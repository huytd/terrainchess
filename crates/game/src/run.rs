//! Roguelike run flow, match progression, and drafting (PLAN.md §6).

use bevy::prelude::*;
use tc_core::{Outcome, Side};
use tc_run::RunState;

use crate::game::GameState;
use crate::save;

/// The current phase of a roguelike run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunPhase {
    /// In an active match.
    Playing,
    /// Drafting a reward after a win: 3 item IDs to choose from.
    Draft([String; 3]),
    /// Run has concluded in victory or defeat.
    Over { won: bool },
}

/// Title menu display state.
#[derive(Resource, Debug, Clone)]
pub struct TitleMenu {
    pub open: bool,
}

impl Default for TitleMenu {
    fn default() -> Self {
        Self { open: true }
    }
}

/// Active run state and progression.
#[derive(Resource)]
pub struct Run {
    pub state: RunState,
    pub phase: RunPhase,
    pub sandbox: bool,
}

/// Message to select a reward card from the active draft.
#[derive(Message, Clone, Debug)]
pub struct PickCard(pub String);

/// Message to start a new run with the given board size.
#[derive(Message, Clone, Copy, Debug)]
pub struct StartRun(pub u8);

/// Message to toggle sandbox hotseat mode.
#[derive(Message, Clone, Copy, Debug)]
pub struct ToggleSandbox;

/// Derive a seed from system time / clock without panicking on WASM.
pub fn time_seed() -> u64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| (d.as_secs() << 32) ^ d.subsec_nanos() as u64)
            .unwrap_or(0xCAFE_BABE_1234_5678)
    }

    #[cfg(target_arch = "wasm32")]
    {
        let t = bevy::platform::time::Instant::now();
        (t.elapsed().as_nanos() as u64) ^ 0x9E37_79B9_7F4A_7C15
    }
}

/// Starts a new run of the specified board size, building the match and saving.
pub fn start_new_run(size: u8, run: &mut Run, game_state: &mut GameState, seed: u64) {
    run.sandbox = false;
    run.state = RunState::new(seed, size);
    run.phase = RunPhase::Playing;
    let setup = run.state.match_setup();
    *game_state = GameState::from_setup(&setup);
    save::store(&run.state);
}

/// Toggles sandbox (hotseat on a fresh random board; run paused and not saved).
pub fn toggle_sandbox(run: &mut Run, game_state: &mut GameState, seed: u64) {
    if !run.sandbox {
        run.sandbox = true;
        // Hotseat on a fresh random board, AI side None
        *game_state = GameState::new(run.state.size, seed, None, 2);
    } else {
        run.sandbox = false;
        // Return to the run's current match rebuilt from its setup
        let setup = run.state.match_setup();
        *game_state = GameState::from_setup(&setup);
    }
}

/// Picks a drafted item, progressing through bonus drafts or starting the next floor.
pub fn apply_pick(item_id: &str, run: &mut Run, game_state: &mut GameState) {
    if let RunPhase::Draft(_) = run.phase {
        if let Err(e) = run.state.pick(item_id) {
            bevy::log::warn!("failed to pick item '{item_id}': {e}");
            return;
        }
        if run.state.bonus_picks > 0 {
            run.state.bonus_picks -= 1;
            let next_draft = run.state.draft();
            run.phase = RunPhase::Draft(next_draft);
        } else {
            let setup = run.state.match_setup();
            *game_state = GameState::from_setup(&setup);
            save::store(&run.state);
            run.phase = RunPhase::Playing;
        }
    }
}

fn check_run_match_outcome(mut run: ResMut<Run>, game_state: Res<GameState>) {
    if run.sandbox || run.phase != RunPhase::Playing {
        return;
    }

    if let Some(outcome) = game_state.outcome {
        // Draws are losses; checkmate with White as winner is victory
        let won = matches!(outcome, Outcome::Checkmate { winner: Side::White });
        let collected = game_state.game.run_items_collected[Side::White.index()];
        run.state.record_result(won, collected);

        if !won || run.state.outcome.is_some() {
            save::clear();
            run.phase = RunPhase::Over { won };
        } else {
            let draft = run.state.draft();
            run.phase = RunPhase::Draft(draft);
        }
    }
}

fn handle_run_messages(
    mut pick_events: MessageReader<PickCard>,
    mut start_events: MessageReader<StartRun>,
    mut toggle_events: MessageReader<ToggleSandbox>,
    mut run: ResMut<Run>,
    mut game_state: ResMut<GameState>,
    time: Res<Time>,
) {
    for pick in pick_events.read() {
        apply_pick(&pick.0, &mut run, &mut game_state);
    }

    for start in start_events.read() {
        let seed = time.elapsed().as_nanos() as u64 ^ time_seed();
        start_new_run(start.0, &mut run, &mut game_state, seed);
    }

    for _ in toggle_events.read() {
        let seed = time.elapsed().as_nanos() as u64 ^ time_seed();
        toggle_sandbox(&mut run, &mut game_state, seed);
    }
}

fn initial_run_and_game() -> (Run, GameState) {
    let saved = save::load();
    let has_save = saved.is_some();
    let run_state = match saved {
        Some(state) => state,
        None => {
            let seed = time_seed();
            RunState::new(seed, 8)
        }
    };
    let setup = run_state.match_setup();
    let game_state = GameState::from_setup(&setup);
    if has_save {
        save::store(&run_state);
    }
    let run = Run { state: run_state, phase: RunPhase::Playing, sandbox: false };
    (run, game_state)
}

pub struct RunPlugin;

impl Plugin for RunPlugin {
    fn build(&self, app: &mut App) {
        let (run, game_state) = initial_run_and_game();
        app.insert_resource(run)
            .insert_resource(game_state)
            .init_resource::<TitleMenu>()
            .add_message::<PickCard>()
            .add_message::<StartRun>()
            .add_message::<ToggleSandbox>()
            .add_systems(Update, (check_run_match_outcome, handle_run_messages).chain());
    }
}
