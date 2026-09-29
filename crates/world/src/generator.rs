use crate::hero::{Faction, Hero};
use crate::map::{Biome, CampKind, MapObject, MapPos, ObjectKind, WorldMap};
use crate::world::WorldParams;
use noise::{NoiseFn, OpenSimplex};
use tc_core::piece::PieceKind;
use tc_core::rng::Rng;

fn find_path_for_generation(map: &WorldMap, start: MapPos, end: MapPos) -> Option<Vec<MapPos>> {
    use std::collections::{BinaryHeap, HashMap};
    #[derive(Clone, Copy, PartialEq)]
    struct State {
        cost: f32,
        pos: MapPos,
    }
    impl Eq for State {}
    impl Ord for State {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            other.cost.partial_cmp(&self.cost).unwrap_or(std::cmp::Ordering::Equal)
        }
    }
    impl PartialOrd for State {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }

    let mut heap = BinaryHeap::new();
    let mut dist = HashMap::new();
    let mut came_from = HashMap::new();
    heap.push(State { cost: 0.0, pos: start });
    dist.insert(start, 0.0f32);
    while let Some(State { cost, pos }) = heap.pop() {
        if pos == end {
            let mut path = Vec::new();
            let mut current = pos;
            while current != start {
                path.push(current);
                current = came_from[&current];
            }
            path.reverse();
            return Some(path);
        }
        let neighbors = [
            MapPos::new(pos.x.saturating_sub(1), pos.y),
            MapPos::new(pos.x + 1, pos.y),
            MapPos::new(pos.x, pos.y.saturating_sub(1)),
            MapPos::new(pos.x, pos.y + 1),
        ];
        for &n in &neighbors {
            if n.x >= map.size.0 || n.y >= map.size.1 || n == pos {
                continue;
            }
            let tile = map.get(n).unwrap();
            let c = match tile.biome {
                Biome::Mountain => 100.0,
                Biome::Water => 10.0, // bridge
                Biome::Forest | Biome::Hills => 2.0,
                _ => 1.0,
            };
            let next_cost = cost + c;
            if next_cost < *dist.get(&n).unwrap_or(&f32::INFINITY) {
                dist.insert(n, next_cost);
                came_from.insert(n, pos);
                heap.push(State { cost: next_cost + n.manhattan(end) as f32, pos: n });
            }
        }
    }
    None
}

pub fn generate_world(mut seed: u64, params: WorldParams) -> (WorldMap, Vec<Hero>) {
    let num_heroes = params.heroes.clamp(2, 10);
    let mut map_size = params.width;

    // Default map size scales with hero count
    if params.width == 48 && params.height == 48 {
        let size_scale = ((num_heroes as f32) / 2.0).sqrt();
        map_size = (48.0 * size_scale).round() as u16;
        map_size = map_size.clamp(48, 128);
    }
    let width = map_size;
    let height = map_size;

    let mut retries = 0;
    loop {
        retries += 1;
        if retries > 100 {
            panic!("Could not generate world after 100 retries (seed {})", seed);
        }
        let mut rng = Rng::new(seed);
        let mut map = WorldMap::new(width, height);

        let noise = OpenSimplex::new(seed as u32);

        let center_x = width as f32 / 2.0;
        let center_y = height as f32 / 2.0;
        let max_r = center_x.min(center_y);

        let sector = std::f32::consts::TAU / num_heroes as f32;
        let angle_offset = (rng.next_u64() % 360) as f32 * std::f32::consts::PI / 180.0;

        // 1. Biomes
        for y in 0..height {
            for x in 0..width {
                let dx = x as f32 - center_x;
                let dy = y as f32 - center_y;
                let r = (dx * dx + dy * dy).sqrt();
                let dist_norm = r / max_r;

                let a = dy.atan2(dx);
                // a_eff is angle relative to the start of sector 0
                let a_eff = (a - angle_offset).rem_euclid(std::f32::consts::TAU);
                let a_sec = a_eff.rem_euclid(sector);
                let a_folded = a_sec.min(sector - a_sec);

                // Sample noise at folded polar coordinates
                let nx = r * a_folded.cos() * 0.1;
                let ny = r * a_folded.sin() * 0.1;
                let val = noise.get([nx as f64, ny as f64]);

                let edge_factor = (dist_norm - 0.8).max(0.0) * 5.0;
                let adjusted = val - edge_factor as f64;

                let biome = if adjusted < -0.4 {
                    Biome::Water
                } else if adjusted < -0.2 {
                    Biome::Coast
                } else if adjusted < 0.3 {
                    Biome::Grass
                } else if adjusted < 0.6 {
                    Biome::Forest
                } else if adjusted < 0.8 {
                    Biome::Hills
                } else {
                    Biome::Mountain
                };

                map.get_mut(MapPos::new(x, y)).unwrap().biome = biome;
            }
        }

        // 2. Hero starts
        let ring_radius = max_r * 0.6;
        let mut starts = Vec::new();
        for i in 0..num_heroes {
            // Middle of the sector
            let angle = angle_offset + (i as f32 * sector) + sector / 2.0;
            let hx = (center_x + angle.cos() * ring_radius).clamp(2.0, width as f32 - 3.0);
            let hy = (center_y + angle.sin() * ring_radius).clamp(2.0, height as f32 - 3.0);

            let pos = MapPos::new(hx as u16, hy as u16);
            starts.push(pos);

            // Force grass around start
            for dy in -3..=3 {
                for dx in -3..=3 {
                    if dx * dx + dy * dy <= 9 {
                        let nx = pos.x as i32 + dx;
                        let ny = pos.y as i32 + dy;
                        if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                            map.get_mut(MapPos::new(nx as u16, ny as u16)).unwrap().biome = Biome::Grass;
                        }
                    }
                }
            }
        }

        // 3. Template camps/objects
        let neighbor_dist = 2.0 * ring_radius * (sector / 2.0).sin();
        let d_max = 10.0f32.min((0.45 * neighbor_dist).floor());

        let mut template = Vec::new();
        let total_camps = params.camps_per_region as usize;
        let mut camp_kinds = Vec::new();
        let villages = (total_camps * 5) / 12;
        let knights = (total_camps * 3) / 12;
        let bishops = (total_camps * 3) / 12;
        let fortress = total_camps - villages - knights - bishops;

        for _ in 0..villages {
            camp_kinds.push((CampKind::Village, 1));
        }
        for _ in 0..knights {
            camp_kinds.push((CampKind::KnightCamp, 2));
        }
        for _ in 0..bishops {
            camp_kinds.push((CampKind::BishopCamp, 2));
        }
        for _ in 0..fortress {
            camp_kinds.push((CampKind::Fortress, 3));
        }

        let start_0 = starts[0];
        let mut objects_to_place = Vec::new();
        for (kind, tier) in camp_kinds {
            objects_to_place.push((ObjectKind::Camp(kind), tier));
        }
        for _ in 0..3 {
            objects_to_place.push((ObjectKind::Chest, 1));
        }
        for _ in 0..2 {
            objects_to_place.push((ObjectKind::Shrine, 1));
        }

        // Generate template points relative to start 0
        let mut all_valid_template = true;
        for (obj_kind, tier) in objects_to_place {
            let mut found = false;
            for _ in 0..100 {
                let dist = 3.0 + (rng.next_u64() % 100) as f32 / 100.0 * (d_max - 3.0);
                // Try angles around start
                let angle_from_start = (rng.next_u64() % 360) as f32 * std::f32::consts::PI / 180.0;
                let lx = dist * angle_from_start.cos();
                let ly = dist * angle_from_start.sin();

                let abs_x = start_0.x as f32 + lx;
                let abs_y = start_0.y as f32 + ly;

                // Check if it stays inside hero 0's sector
                let a = (abs_y - center_y).atan2(abs_x - center_x);
                let a_eff = (a - angle_offset).rem_euclid(std::f32::consts::TAU);
                if a_eff >= 0.0 && a_eff < sector {
                    // Valid template point! We store the offset relative to map center, but wait,
                    // "rotation about the map centre by k*sector". So let's store absolute offset from center.
                    let ox = abs_x - center_x;
                    let oy = abs_y - center_y;
                    template.push((ox, oy, obj_kind, tier));
                    found = true;
                    break;
                }
            }
            if !found {
                all_valid_template = false;
                break;
            }
        }
        if !all_valid_template {
            seed = seed.wrapping_add(1);
            continue;
        }

        // Force grass at center for Citadel
        for dy in -2..=2 {
            for dx in -2..=2 {
                let nx = center_x as i32 + dx;
                let ny = center_y as i32 + dy;
                if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                    map.get_mut(MapPos::new(nx as u16, ny as u16)).unwrap().biome = Biome::Grass;
                }
            }
        }

        map.objects.push(MapObject {
            pos: MapPos::new(center_x as u16, center_y as u16),
            kind: ObjectKind::Camp(CampKind::Citadel),
            owner: None,
            cleared: false,
            guards: vec![PieceKind::King, PieceKind::Queen, PieceKind::Pawn, PieceKind::Pawn],
            tier: 5,
        });

        let mut all_valid_placement = true;

        // Search order for snapping
        let mut search_offsets = Vec::new();
        for dy in -3..=3 {
            for dx in -3..=3 {
                search_offsets.push((dx, dy));
            }
        }
        // Deterministic sort by distance
        search_offsets.sort_by_key(|&(dx, dy)| dx * dx + dy * dy);

        for (h_idx, _start) in starts.iter().enumerate() {
            let rot_angle = h_idx as f32 * sector;
            let cos_r = rot_angle.cos();
            let sin_r = rot_angle.sin();

            for (ox, oy, kind, tier) in &template {
                let rx = ox * cos_r - oy * sin_r;
                let ry = ox * sin_r + oy * cos_r;

                let tx = center_x + rx;
                let ty = center_y + ry;

                let mut best_pos = None;

                for &(dx, dy) in &search_offsets {
                    let nx = (tx.round() as i32) + dx;
                    let ny = (ty.round() as i32) + dy;
                    if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                        let pos = MapPos::new(nx as u16, ny as u16);
                        let tile = map.get(pos).unwrap();
                        if tile.biome != Biome::Mountain
                            && tile.biome != Biome::Water
                            && !map.objects.iter().any(|o| o.pos == pos)
                            && !starts.contains(&pos)
                        {
                            // Ensure it stays in sector
                            let a = (ny as f32 - center_y).atan2(nx as f32 - center_x);
                            let a_eff = (a - angle_offset).rem_euclid(std::f32::consts::TAU);
                            // The sector for this hero is [h_idx*sector, (h_idx+1)*sector]
                            let min_a = h_idx as f32 * sector;
                            let max_a = min_a + sector;
                            if a_eff >= min_a && a_eff < max_a {
                                best_pos = Some(pos);
                                break;
                            }
                        }
                    }
                }

                if let Some(pos) = best_pos {
                    let guards = match kind {
                        ObjectKind::Camp(k) => {
                            let boss = match k {
                                CampKind::Village => PieceKind::Pawn,
                                CampKind::KnightCamp => PieceKind::Knight,
                                CampKind::BishopCamp => PieceKind::Bishop,
                                CampKind::Fortress => PieceKind::Rook,
                                _ => PieceKind::Queen,
                            };
                            vec![PieceKind::King, boss, PieceKind::Pawn, PieceKind::Pawn]
                        }
                        _ => vec![],
                    };

                    map.objects.push(MapObject {
                        pos,
                        kind: kind.clone(),
                        owner: None,
                        cleared: false,
                        guards,
                        tier: *tier,
                    });
                } else {
                    all_valid_placement = false;
                    break;
                }
            }
            if !all_valid_placement {
                break;
            }
        }

        if !all_valid_placement {
            seed = seed.wrapping_add(1);
            continue;
        }

        // 4. Roads (A*)
        let center_pos = MapPos::new(center_x as u16, center_y as u16);
        let mut roads_to_build = Vec::new();
        for start in &starts {
            roads_to_build.push((*start, center_pos));
        }
        for obj in &map.objects {
            let mut best_start = starts[0];
            let mut best_dist = i32::MAX;
            for start in &starts {
                let dx = start.x as i32 - obj.pos.x as i32;
                let dy = start.y as i32 - obj.pos.y as i32;
                let dist = dx * dx + dy * dy;
                if dist < best_dist {
                    best_dist = dist;
                    best_start = *start;
                }
            }
            roads_to_build.push((obj.pos, best_start));
        }

        let mut path_failed = false;
        for (from, to) in roads_to_build {
            if let Some(path) = find_path_for_generation(&map, from, to) {
                for p in path {
                    map.get_mut(p).unwrap().road = true;
                }
            } else {
                path_failed = true;
                break;
            }
        }

        if path_failed {
            seed = seed.wrapping_add(1);
            continue;
        }

        // 5. Validation (BFS)
        let mut visited = vec![false; map.size.0 as usize * map.size.1 as usize];
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(starts[0]);
        visited[starts[0].y as usize * map.size.0 as usize + starts[0].x as usize] = true;

        while let Some(p) = queue.pop_front() {
            let neighbors = [
                MapPos::new(p.x.saturating_sub(1), p.y),
                MapPos::new(p.x + 1, p.y),
                MapPos::new(p.x, p.y.saturating_sub(1)),
                MapPos::new(p.x, p.y + 1),
            ];
            for n in neighbors {
                if n.x < map.size.0 && n.y < map.size.1 {
                    let tile = map.get(n).unwrap();
                    if tile.biome != Biome::Mountain && (tile.biome != Biome::Water || tile.road) {
                        let idx = n.y as usize * map.size.0 as usize + n.x as usize;
                        if !visited[idx] {
                            visited[idx] = true;
                            queue.push_back(n);
                        }
                    }
                }
            }
        }

        let all_objects_reachable =
            map.objects.iter().all(|o| visited[o.pos.y as usize * map.size.0 as usize + o.pos.x as usize])
                && starts.iter().all(|s| visited[s.y as usize * map.size.0 as usize + s.x as usize]);

        if !all_objects_reachable {
            seed = seed.wrapping_add(1);
            continue;
        }

        let mut heroes = Vec::new();
        for (i, &pos) in starts.iter().enumerate() {
            let faction = if i == 0 { Faction::AshenSun } else { Faction::HollowCrown };
            let name = if i == 0 { "Lord Caelen".to_string() } else { "Lich-King Malakor".to_string() };
            heroes.push(Hero::new(i as u8, faction, name, pos, i > 0));
        }

        let mut road_tiles = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let pos = MapPos::new(x, y);
                if map.get(pos).unwrap().road
                    && !map.objects.iter().any(|o| o.pos == pos)
                    && !starts.contains(&pos)
                {
                    road_tiles.push(pos);
                }
            }
        }
        for _ in 0..(num_heroes * 2) {
            if road_tiles.is_empty() {
                break;
            }
            let idx = (rng.next_u64() as usize) % road_tiles.len();
            let pos = road_tiles.remove(idx);
            map.objects.push(MapObject {
                pos,
                kind: ObjectKind::Signpost,
                owner: None,
                cleared: false,
                guards: vec![],
                tier: 1,
            });
        }

        return (map, heroes);
    }
}
