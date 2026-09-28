//! Runs the AI opponent in small slices each frame, so the window (and the web
//! build, which has no threads) stays responsive while it thinks.

use bevy::prelude::*;
use tc_ai::{Action, Limits, choose_action};

use crate::game::GameState;
use crate::loading::AppState;
use crate::run::TitleMenu;

/// Let the player's move land before the AI starts.
const THINK_DELAY_SECS: f32 = 0.35;

#[derive(Resource)]
struct Thinker {
    /// Position the job was started for: (seed, undo plies).
    key: (u64, usize),
    waited: f32,
}

impl Default for Thinker {
    fn default() -> Self {
        Thinker { key: (0, usize::MAX), waited: 0.0 }
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
    let key = (state.seed, state.undo.len());
    if !state.ai_to_move() {
        return;
    }
    if thinker.key != key {
        // New position (a move, undo or new game): start over after a short pause.
        thinker.key = key;
        thinker.waited = 0.0;
    }
    thinker.waited += time.delta_secs();
    if thinker.waited < THINK_DELAY_SECS {
        return;
    }
    let limits = Limits { seed: state.seed ^ key.1 as u64, ..Limits::for_floor(state.ai_level) };
    match choose_action(&state.game, limits) {
        Action::Move(mv) => state.play(mv),
        Action::Cast(cast) => state.cast(cast),
    }
}

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Thinker>().add_systems(Update, think.run_if(in_state(AppState::Ready)));
    }
}
