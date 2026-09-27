//! Terrain Chess AI (PLAN.md §7): negamax with alpha-beta, iterative deepening,
//! move ordering (TT move, MVV-LVA, killers), quiescence and a transposition table.
//!
//! The search is a [`SearchJob`] that runs in slices, so a single-threaded web build
//! can think across frames without freezing.

mod eval;
mod search;
mod tt;

pub use eval::evaluate;
pub use search::{SearchInfo, SearchJob, search_blocking};

/// How hard the AI tries. Scales with the run's floor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Limits {
    pub max_depth: u8,
    /// Thinking time budget in milliseconds.
    pub time_ms: f64,
    /// Chance of picking one of the other top moves instead of the best.
    pub blunder_chance: f64,
    /// Seed for blunder rolls, so replays are deterministic.
    pub seed: u64,
}

impl Limits {
    /// Floor 0 is the first match of a run; floor 7 is the boss.
    pub fn for_floor(floor: u8) -> Self {
        let f = floor.min(7);
        Limits {
            max_depth: 2 + f,
            time_ms: 400.0 + 200.0 * f as f64,
            blunder_chance: (0.3 - 0.06 * f as f64).max(0.0),
            seed: 0,
        }
    }

    /// Full strength with a time cap; useful for tests and self-play.
    pub fn depth(max_depth: u8, time_ms: f64) -> Self {
        Limits { max_depth, time_ms, blunder_chance: 0.0, seed: 0 }
    }
}
