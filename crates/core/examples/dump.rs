//! Print a generated board and its legal opening moves: `cargo run -p tc_core --example dump -- <seed> <size>`.

use tc_core::worldgen::*;
use tc_core::*;
fn main() {
    let seed: u64 = std::env::args().nth(1).map(|s| s.parse().unwrap()).unwrap_or(1);
    let size: u8 = std::env::args().nth(2).map(|s| s.parse().unwrap()).unwrap_or(8);
    let (t, s) = generate(seed, &GenParams::for_floor(size, 2));
    println!("seed {s}");
    for y in (0..size).rev() {
        let row: String = (0..size)
            .map(|x| {
                let tile = t.get(Sq::new(x, y));
                let c = match (tile.kind, tile.feature) {
                    (_, Feature::Cave(_)) => 'C',
                    (_, Feature::Obstacle(_)) => 'X',
                    (TileKind::ShallowWater, _) => 'w',
                    (TileKind::DeepWater, _) => 'W',
                    _ => ' ',
                };
                format!("{}{} ", tile.height, c)
            })
            .collect();
        println!("{} {row}", y + 1);
    }
    let m = Match::new(t, Rules::standard(size), Position::start(size));
    let mv: Vec<String> = m.legal_moves().iter().map(|m| m.uci()).collect();
    println!("{} moves: {}", mv.len(), mv.join(" "));
}
