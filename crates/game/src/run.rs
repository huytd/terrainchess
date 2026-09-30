//! Level select mode, match progression, and drafting (specs/game-design.md §4).

use bevy::prelude::*;
use tc_core::{Outcome, Side};
use tc_run::Profile;

use crate::game::GameState;
use crate::loading::AppState;
use crate::save;

/// The current phase of the level run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunPhase {
    /// In an active match.
    Playing,
    /// Drafting a reward after a win: 1–3 item IDs to choose from.
    Draft(Vec<String>),
    /// Match has concluded in victory or defeat.
    Result { won: bool },
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

/// Active profile state and current level.
#[derive(Resource)]
pub struct Run {
    pub profile: Profile,
    pub level: u8,
    pub phase: RunPhase,
    /// Reward cards earned by the last win, claimed from the victory screen.
    pub pending_draft: Option<Vec<String>>,
}

/// Message to select a reward card from the active draft.
#[derive(Message, Clone, Debug)]
pub struct PickCard(pub String);

/// Message to start a level with the given level id.
#[derive(Message, Clone, Copy, Debug)]
pub struct StartLevel(pub u8);

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
        fn splitmix64(mut x: u64) -> u64 {
            x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = x;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        }

        // `Instant::now().elapsed()` is ~0 on wasm, so gather real entropy instead:
        // wall-clock ms, 64 bits from `Math.random()`, and a sub-ms monotonic clock.
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let date_bits = js_sys::Date::now().to_bits();
        let hi = (js_sys::Math::random() * 4_294_967_296.0) as u64;
        let lo = (js_sys::Math::random() * 4_294_967_296.0) as u64;
        let rand_bits = (hi << 32) | (lo & 0xFFFF_FFFF);
        let perf_bits = web_sys::window()
            .and_then(|w| w.performance())
            .map(|p| p.now().to_bits())
            .unwrap_or(0x243F_6A88_85A3_08D3);
        let count = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        splitmix64(date_bits ^ rand_bits ^ perf_bits ^ count)
    }
}

/// Starts the specified level, building the match.
pub fn start_level(level_id: u8, run: &mut Run, game_state: &mut GameState, seed: u64) {
    let Some(level_def) = tc_run::level(level_id) else {
        bevy::log::warn!("unknown level id {level_id}");
        return;
    };
    run.level = level_id;
    run.phase = RunPhase::Playing;
    run.pending_draft = None;
    let setup = run.profile.match_setup(level_def, seed);
    *game_state = GameState::from_setup(&setup);
}

/// Picks a drafted item, saving the profile and returning to the level select.
pub fn apply_pick(item_id: &str, run: &mut Run, title_menu: &mut TitleMenu) {
    if let RunPhase::Draft(_) = &run.phase {
        if let Err(e) = run.profile.pick(item_id) {
            bevy::log::warn!("failed to pick item '{item_id}': {e}");
            return;
        }
        save::store(&run.profile);
        run.phase = RunPhase::Playing;
        title_menu.open = true;
    }
}

fn check_run_match_outcome(mut run: ResMut<Run>, game_state: Res<GameState>) {
    if run.phase != RunPhase::Playing {
        return;
    }

    if let Some(outcome) = game_state.outcome {
        // Draws are losses; checkmate with White as winner is victory
        let won = matches!(outcome, Outcome::Checkmate { winner: Side::White });
        if won {
            if let Some(level_def) = tc_run::level(run.level) {
                let draft = run.profile.record_win(level_def);
                save::store(&run.profile);
                // The victory screen comes first; its button opens the reward draft.
                run.pending_draft = (!draft.is_empty()).then_some(draft);
                run.phase = RunPhase::Result { won: true };
            } else {
                run.phase = RunPhase::Result { won: true };
            }
        } else {
            run.phase = RunPhase::Result { won: false };
        }
    }
}

fn handle_run_messages(
    mut pick_events: MessageReader<PickCard>,
    mut start_events: MessageReader<StartLevel>,
    mut run: ResMut<Run>,
    mut game_state: ResMut<GameState>,
    mut title_menu: ResMut<TitleMenu>,
) {
    for pick in pick_events.read() {
        apply_pick(&pick.0, &mut run, &mut title_menu);
    }

    for start in start_events.read() {
        // Fresh entropy per match: `Time::elapsed` is frame-quantized on web,
        // so it adds almost no entropy over `time_seed()` itself.
        let seed = time_seed();
        start_level(start.0, &mut run, &mut game_state, seed);
    }
}

fn initial_run_and_game() -> (Run, GameState) {
    let profile = save::load().unwrap_or_else(|| Profile::new(time_seed()));
    let level_id = 1;
    let level_def = tc_run::level(level_id).unwrap_or(&tc_run::LEVELS[0]);
    let seed = time_seed();
    let setup = profile.match_setup(level_def, seed);
    let game_state = GameState::from_setup(&setup);
    let run = Run { profile, level: level_id, phase: RunPhase::Playing, pending_draft: None };
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
            .add_message::<StartLevel>()
            .add_systems(
                Update,
                (check_run_match_outcome, handle_run_messages).chain().run_if(in_state(AppState::Ready)),
            );
    }
}
