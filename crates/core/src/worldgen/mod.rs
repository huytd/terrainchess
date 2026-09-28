//! Seeded terrain generator (PLAN.md §3). The board is mirrored across the middle
//! rank so neither side gets an unfair board, then checked by the validator.

mod validate;

pub use validate::{Reject, validate};

use noise::{Fbm, MultiFractal, NoiseFn, Perlin};
use serde::{Deserialize, Serialize};

use crate::board::{KING, Sq};
use crate::position::Position;
use crate::rng::Rng;
use crate::rules::Rules;
use crate::terrain::{Feature, MAX_HEIGHT, Obstacle, Terrain, Tile, TileKind};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GenParams {
    pub size: u8,
    /// 0 = gentle rolling hills, 1 = rugged cliffs.
    pub roughness: f64,
    /// Water basins per 8×8 area of the board.
    pub basins: f64,
    /// Linked cave pairs.
    pub cave_pairs: u8,
    /// Chance that a free middle tile holds a rock or tree.
    pub obstacle_density: f64,
    /// Height of the flattened home ranks.
    pub home_height: u8,
}

impl GenParams {
    /// Parameters that get rougher as the run goes deeper (floor 0 = first match).
    pub fn for_floor(size: u8, floor: u8) -> Self {
        let f = floor.min(7) as f64 / 7.0;
        let area = (size as f64 / 8.0).powi(2);
        GenParams {
            size,
            roughness: 0.35 + 0.5 * f,
            basins: 0.6 + 0.4 * f,
            cave_pairs: (area.sqrt() as u8).max(1),
            obstacle_density: 0.04 + 0.04 * f,
            home_height: 1,
        }
    }
}

/// Generate a board that passes validation, trying `seed`, `seed + 1`, … in turn.
/// Returns the terrain and the seed that produced it. Falls back to a flat board.
pub fn generate(seed: u64, params: &GenParams) -> (Terrain, u64) {
    let rules = Rules::standard(params.size);
    let pos = Position::start(params.size);
    for attempt in 0..256 {
        let s = seed.wrapping_add(attempt);
        let terrain = generate_unchecked(s, params);
        if validate(&terrain, &rules, &pos).is_ok() {
            return (terrain, s);
        }
    }
    (Terrain::flat(params.size), seed)
}

/// One generation pass without validation.
pub fn generate_unchecked(seed: u64, params: &GenParams) -> Terrain {
    let size = params.size;
    let half = size / 2;
    let home = Position::home_rows(size);
    let mut rng = Rng::new(seed);
    let mut t = Terrain::flat(size);

    // 1. Heightmap from fBm noise.
    let fbm = Fbm::<Perlin>::new(rng.next_u64() as u32)
        .set_octaves(3)
        .set_frequency(0.19)
        .set_persistence(0.45 + 0.25 * params.roughness);
    let (ox, oy) = (rng.unit() * 1000.0, rng.unit() * 1000.0);
    let amp = 1.4 + 2.0 * params.roughness;
    for y in 0..half {
        for x in 0..size {
            let n = fbm.get([x as f64 + ox, y as f64 + oy]);
            let h = (1.5 + n * amp * 2.0).round().clamp(0.0, MAX_HEIGHT as f64);
            t.get_mut(Sq::new(x, y)).height = h as u8;
        }
    }

    // 2. Flatten the home ranks so the starting setup is legal.
    for y in 0..home {
        for x in 0..size {
            *t.get_mut(Sq::new(x, y)) = Tile::flat(params.home_height);
        }
    }

    let middle = |sq: Sq| sq.y >= home && sq.y < half;
    let area = (size as f64 / 8.0).powi(2);

    // 3. Water basins, centred near the middle line so the mirror joins them into lakes.
    let basins = (params.basins * area + rng.unit()).floor() as u32;
    for _ in 0..basins {
        let cx = rng.below(size as u32) as f64;
        let cy = (half - 1) as f64 - rng.below((half - home).min(2) as u32) as f64;
        let r = 1.0 + rng.unit() * 0.8 * (size as f64 / 8.0).sqrt();
        for sq in crate::board::squares(size).filter(|&s| middle(s)) {
            let d = ((sq.x as f64 - cx).powi(2) + (sq.y as f64 - cy).powi(2)).sqrt();
            if d <= r {
                let tile = t.get_mut(sq);
                tile.height = 0;
                tile.kind = if d <= r - 1.0 { TileKind::DeepWater } else { TileKind::ShallowWater };
            }
        }
    }

    // 4. Surface kinds: stone on peaks, sand on shores.
    for sq in crate::board::squares(size).filter(|&s| middle(s)) {
        if t.get(sq).is_water() {
            continue;
        }
        let shore = KING.iter().filter_map(|&(dx, dy)| sq.offset(dx, dy, size)).any(|n| t.get(n).is_water());
        let tile = t.get_mut(sq);
        tile.kind = if shore && tile.height <= 1 {
            TileKind::Sand
        } else if tile.height == MAX_HEIGHT {
            TileKind::Stone
        } else {
            TileKind::Grass
        };
    }

    // 5. Cave entrances, preferably cut into a cliff face (south neighbour lower).
    //    Each entrance links to its mirror image on the other half.
    let free =
        |t: &Terrain, sq: Sq| middle(sq) && !t.get(sq).is_water() && t.get(sq).feature == Feature::None;
    for link in 0..params.cave_pairs {
        let candidates: Vec<Sq> = crate::board::squares(size).filter(|&s| free(&t, s)).collect();
        let cliffs: Vec<Sq> = candidates
            .iter()
            .copied()
            .filter(|s| s.offset(0, -1, size).is_some_and(|n| t.height(n) < t.height(*s)))
            .collect();
        let pool = if cliffs.is_empty() { &candidates } else { &cliffs };
        if pool.is_empty() {
            break;
        }
        let sq = pool[rng.below(pool.len() as u32) as usize];
        t.get_mut(sq).feature = Feature::Cave(link);
    }

    // 6. Obstacles: rocks on high ground, trees elsewhere.
    for sq in crate::board::squares(size) {
        if free(&t, sq) && rng.unit() < params.obstacle_density {
            let rock = t.height(sq) >= 2 || t.get(sq).kind == TileKind::Stone;
            t.get_mut(sq).feature = Feature::Obstacle(if rock { Obstacle::Rock } else { Obstacle::Tree });
        }
    }

    // 7. Mirror across the middle rank.
    for y in half..size {
        for x in 0..size {
            let src = *t.get(Sq::new(x, size - 1 - y));
            *t.get_mut(Sq::new(x, y)) = src;
        }
    }
    t
}
