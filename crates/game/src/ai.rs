//! Runs the AI opponent in small slices each frame, so the window (and the web
//! build, which has no threads) stays responsive while it thinks.

use bevy::platform::time::Instant;
use bevy::prelude::*;
use tc_ai::{Limits, SearchJob};

use crate::game::GameState;
use crate::run::TitleMenu;

/// Search time per frame.
const SLICE_MS: f64 = 8.0;
/// Let the player's move land before the AI starts.
const THINK_DELAY_SECS: f32 = 0.35;

#[derive(Resource)]
struct Thinker {
    job: Option<SearchJob>,
    /// Position the job was started for: (seed, plies played).
    key: (u64, usize),
    waited: f32,
    epoch: Instant,
}

impl Default for Thinker {
    fn default() -> Self {
        Thinker { job: None, key: (0, usize::MAX), waited: 0.0, epoch: Instant::now() }
    }
}

fn think(
    time: Res<Time>,
    title_menu: Res<TitleMenu>,
    mut thinker: ResMut<Thinker>,
    mut state: ResMut<GameState>,
) {
    if title_menu.open {
        return;
    }
    let key = (state.seed, state.game.moves.len());
    if !state.ai_to_move() {
        if thinker.job.is_some() {
            thinker.job = None;
        }
        return;
    }
    if thinker.key != key {
        // New position (a move, undo or new game): start over after a short pause.
        thinker.key = key;
        thinker.job = None;
        thinker.waited = 0.0;
    }
    if thinker.job.is_none() {
        thinker.waited += time.delta_secs();
        if thinker.waited < THINK_DELAY_SECS {
            return;
        }
        let limits = Limits { seed: state.seed ^ key.1 as u64, ..Limits::for_floor(state.ai_level) };
        thinker.job = Some(SearchJob::new(&state.game, limits));
    }
    let epoch = thinker.epoch;
    let clock = move || epoch.elapsed().as_secs_f64() * 1000.0;
    let job = thinker.job.as_mut().unwrap();
    let chosen = job.step(&clock, SLICE_MS);
    let finished = job.is_finished();
    if let Some(mv) = chosen {
        thinker.job = None;
        state.play(mv);
    } else if finished {
        thinker.job = None;
    }
}

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Thinker>().add_systems(Update, think);
    }
}
