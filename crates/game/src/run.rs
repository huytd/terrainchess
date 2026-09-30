//! The endless run: stage progression, menus and drafting (specs/game-design.md §4).

use bevy::prelude::*;
use tc_core::{Outcome, Side};
use tc_run::{Profile, Stage};

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
    /// The Prepare (deck) screen for the current stage is showing instead of the main menu.
    pub prepare: bool,
    /// Card picked up on the Prepare screen; the next card tapped swaps with it.
    pub held: Option<DeckSlot>,
    /// "Abandon run" / "New run" was pressed once; the next press confirms.
    pub confirm: bool,
}

impl Default for TitleMenu {
    fn default() -> Self {
        Self { open: true, prepare: false, held: None, confirm: false }
    }
}

impl TitleMenu {
    /// Show the main menu (`prepare = false`) or the Prepare screen.
    pub fn show(&mut self, prepare: bool) {
        self.open = true;
        self.prepare = prepare;
        self.held = None;
        self.confirm = false;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.prepare = false;
        self.held = None;
        self.confirm = false;
    }
}

/// A card position on the Prepare screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DeckSlot {
    Deck(usize),
    Reserve(usize),
}

/// Deck arrangement edits from the Prepare screen.
#[derive(Message, Clone, Copy, Debug)]
pub enum DeckEdit {
    Tap(DeckSlot),
    Reset,
}

/// Active profile state and the stage on the board.
#[derive(Resource)]
pub struct Run {
    pub profile: Profile,
    /// The stage on the board (the backdrop stage while no match is in progress).
    pub stage: Stage,
    pub phase: RunPhase,
    /// Reward cards earned by the last win, claimed from the result strip.
    pub pending_draft: Option<Vec<String>>,
    /// A real match is being played (not the idle backdrop board behind the menu).
    pub in_match: bool,
    /// Stage the last run ended on, for the defeat strip.
    pub ended_at: Option<u32>,
}

impl Run {
    /// Whether the menu can close: there's a match or a result strip behind it.
    pub fn menu_closable(&self) -> bool {
        self.in_match || matches!(self.phase, RunPhase::Result { .. })
    }
}

impl RunPhase {
    /// The camera can orbit and zoom (during play and while reviewing the final board).
    pub fn camera_free(&self) -> bool {
        matches!(self, RunPhase::Playing | RunPhase::Result { .. })
    }
}

/// Message to select a reward card from the active draft.
#[derive(Message, Clone, Debug)]
pub struct PickCard(pub String);

/// Run-level commands from the menus and the result strip.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunCommand {
    /// Play the profile's current stage (from Prepare).
    Start,
    /// Begin a new run at stage 1 and open its Prepare screen (ends any current run).
    New,
    /// Give up the current run (it ends like a loss).
    Abandon,
}

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

/// Starts the profile's current stage on its fixed board.
pub fn start_stage(run: &mut Run, game_state: &mut GameState) {
    let stage = run.profile.stage();
    let seed = run.profile.run.map_or_else(time_seed, |r| r.match_seed());
    *game_state = GameState::from_setup(&run.profile.stage_setup(&stage, seed));
    run.stage = stage;
    run.phase = RunPhase::Playing;
    run.pending_draft = None;
    run.in_match = true;
    run.ended_at = None;
}

/// Puts an idle board for the profile's current stage behind the menu.
fn show_backdrop(run: &mut Run, game_state: &mut GameState) {
    let stage = run.profile.stage();
    *game_state = GameState::from_setup(&run.profile.stage_setup(&stage, time_seed()));
    run.stage = stage;
    run.phase = RunPhase::Playing;
    run.pending_draft = None;
    run.in_match = false;
}

/// Picks a drafted item, saving the profile and opening the next stage's Prepare screen.
pub fn apply_pick(item_id: &str, run: &mut Run, title_menu: &mut TitleMenu) {
    if let RunPhase::Draft(_) = &run.phase {
        if let Err(e) = run.profile.pick(item_id) {
            bevy::log::warn!("failed to pick item '{item_id}': {e}");
            return;
        }
        save::store(&run.profile);
        run.phase = RunPhase::Playing;
        title_menu.show(true);
    }
}

/// Applies a Prepare screen edit: tapping picks a card up; tapping a card on the other side
/// (deck vs reserve) swaps the two, so a card leaves the deck and another joins it.
pub fn apply_deck_edit(edit: DeckEdit, run: &mut Run, title_menu: &mut TitleMenu) {
    use DeckSlot::{Deck, Reserve};
    match edit {
        DeckEdit::Tap(slot) => match (title_menu.held, slot) {
            (Some(h), s) if h == s => title_menu.held = None,
            (Some(Deck(i)), Reserve(r)) | (Some(Reserve(r)), Deck(i)) => {
                run.profile.swap_with_reserve(i, r);
                title_menu.held = None;
            }
            (_, s) => {
                // Picking up (or switching the held card) doesn't change the saved deck.
                title_menu.held = Some(s);
                return;
            }
        },
        DeckEdit::Reset => {
            run.profile.reset_deck();
            title_menu.held = None;
        }
    }
    save::store(&run.profile);
}

fn check_run_match_outcome(mut run: ResMut<Run>, game_state: Res<GameState>) {
    if run.phase != RunPhase::Playing || !run.in_match {
        return;
    }
    let Some(outcome) = game_state.outcome else {
        return;
    };
    run.in_match = false;
    // Draws are losses; the player is White.
    let won = matches!(outcome, Outcome::Checkmate { winner: Side::White });
    if won {
        let draft = run.profile.win_stage();
        run.pending_draft = (!draft.is_empty()).then_some(draft);
    } else {
        run.ended_at = Some(run.profile.end_run());
    }
    save::store(&run.profile);
    run.phase = RunPhase::Result { won };
}

fn handle_run_messages(
    mut pick_events: MessageReader<PickCard>,
    mut commands: MessageReader<RunCommand>,
    mut deck_edits: MessageReader<DeckEdit>,
    mut run: ResMut<Run>,
    mut game_state: ResMut<GameState>,
    mut title_menu: ResMut<TitleMenu>,
) {
    for pick in pick_events.read() {
        apply_pick(&pick.0, &mut run, &mut title_menu);
    }

    for &edit in deck_edits.read() {
        apply_deck_edit(edit, &mut run, &mut title_menu);
    }

    for &cmd in commands.read() {
        match cmd {
            RunCommand::Start => {
                if run.profile.run.is_none() {
                    run.profile.new_run(time_seed());
                    save::store(&run.profile);
                }
                start_stage(&mut run, &mut game_state);
                title_menu.close();
            }
            RunCommand::New => {
                run.profile.new_run(time_seed());
                save::store(&run.profile);
                run.ended_at = None;
                show_backdrop(&mut run, &mut game_state);
                title_menu.show(true);
            }
            RunCommand::Abandon => {
                if run.profile.run.is_some() {
                    run.ended_at = Some(run.profile.end_run());
                    save::store(&run.profile);
                }
                show_backdrop(&mut run, &mut game_state);
                title_menu.show(false);
            }
        }
    }
}

fn initial_run_and_game() -> (Run, GameState) {
    let profile = save::load().unwrap_or_else(|| Profile::new(time_seed()));
    let stage = profile.stage();
    let game_state = GameState::from_setup(&profile.stage_setup(&stage, time_seed()));
    let run = Run {
        profile,
        stage,
        phase: RunPhase::Playing,
        pending_draft: None,
        in_match: false,
        ended_at: None,
    };
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
            .add_message::<RunCommand>()
            .add_message::<DeckEdit>()
            .add_systems(
                Update,
                (check_run_match_outcome, handle_run_messages).chain().run_if(in_state(AppState::Ready)),
            );
    }
}
