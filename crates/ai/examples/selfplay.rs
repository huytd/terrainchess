//! AI vs AI on generated boards (specs/project-plan.md §4): spots boards or rules that break chess.
//!
//! `cargo run --release -p tc_ai --example selfplay -- [games] [size] [ms_per_move]`

use std::time::Instant;

use tc_ai::{Limits, search_blocking};
use tc_core::worldgen::{GenParams, generate};
use tc_core::{Match, Outcome, Position, Rules, Side};

fn main() {
    let arg =
        |i: usize, default: u64| std::env::args().nth(i).and_then(|s| s.parse().ok()).unwrap_or(default);
    let (games, size, ms) = (arg(1, 20), arg(2, 8) as u8, arg(3, 100) as f64);
    let start = Instant::now();
    let clock = move || start.elapsed().as_secs_f64() * 1000.0;

    let (mut white, mut black, mut draws, mut plies_total, mut nodes, mut depth_sum, mut searches) =
        (0, 0, 0, 0usize, 0u64, 0u64, 0u64);
    for g in 0..games {
        let (terrain, seed) = generate(g * 7919, &GenParams::for_floor(size, (g % 8) as u8));
        let mut game = Match::new(terrain, Rules::standard(size), Position::start(size));
        let outcome = loop {
            if let Some(o) = game.outcome() {
                break Some(o);
            }
            if game.moves.len() >= 300 {
                break None; // move limit: counts as a draw
            }
            let (mv, info) = search_blocking(&game, Limits::depth(12, ms), &clock);
            nodes += info.nodes;
            depth_sum += info.depth as u64;
            searches += 1;
            game.play(mv.expect("side to move has moves")).unwrap();
        };
        plies_total += game.moves.len();
        let result = match outcome {
            Some(Outcome::Checkmate { winner: Side::White }) => {
                white += 1;
                "1-0".to_string()
            }
            Some(Outcome::Checkmate { winner: Side::Black }) => {
                black += 1;
                "0-1".to_string()
            }
            Some(Outcome::Draw(r)) => {
                draws += 1;
                format!("draw ({r:?})")
            }
            None => {
                draws += 1;
                "draw (move limit)".to_string()
            }
        };
        println!("game {g:3} seed {seed:6} {:3} plies  {result}", game.moves.len());
    }
    let secs = start.elapsed().as_secs_f64();
    println!(
        "\n{games} games on {size}x{size}: white {white}, black {black}, draws {draws}; \
         avg {:.0} plies, avg depth {:.1}, {:.0}k nodes/s",
        plies_total as f64 / games as f64,
        depth_sum as f64 / searches.max(1) as f64,
        nodes as f64 / secs / 1000.0
    );
}
