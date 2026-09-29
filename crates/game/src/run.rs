//! Roguelike run flow, match progression, and drafting (specs/game-design.md §4).

use bevy::prelude::*;

#[derive(Resource)]
pub struct Campaign {
    pub world: tc_world::World,
}
use tc_core::{Outcome, Side};
use tc_run::RunState;

use crate::game::GameState;
use crate::loading::AppState;
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
    pub pending: bool,
}

impl Default for TitleMenu {
    fn default() -> Self {
        Self { open: true, pending: false }
    }
}

/// Active run state and progression.
#[derive(Resource)]
pub struct Run {
    pub state: RunState,
    pub phase: RunPhase,
}

/// Message to select a reward card from the active draft.
#[derive(Message, Clone, Debug)]
pub struct PickCard(pub String);

#[derive(Message, Clone, Copy, Debug)]
pub struct StartCampaign(pub bool); // true for continue, false for new

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

fn check_run_match_outcome(
    mut run: ResMut<Run>,
    game_state: Res<GameState>,
    mode: Res<State<crate::game::Mode>>,
) {
    if *mode.get() != crate::game::Mode::Classic {
        return;
    }
    if run.phase != RunPhase::Playing {
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

#[allow(clippy::too_many_arguments)]
fn handle_run_messages(
    mut commands: Commands,
    mut pick_events: MessageReader<PickCard>,
    mut campaign_events: MessageReader<StartCampaign>,
    mut next_mode: ResMut<NextState<crate::game::Mode>>,
    mode: Res<State<crate::game::Mode>>,
    mut run: ResMut<Run>,
    mut game_state: ResMut<GameState>,
    time: Res<Time>,
    mut title_menu: ResMut<TitleMenu>,
    button_q: Query<(&crate::hud::TitleCampaignButton, &Children)>,
    mut text_q: Query<&mut Text>,
) {
    for pick in pick_events.read() {
        if *mode.get() == crate::game::Mode::Classic {
            apply_pick(&pick.0, &mut run, &mut game_state);
        }
    }

    for campaign in campaign_events.read() {
        if campaign.0 {
            if let Some(world) = crate::save::load_campaign() {
                commands.insert_resource(Campaign { world });
                next_mode.set(crate::game::Mode::Overworld);
            } else {
                title_menu.pending = false;
                for (btn, children) in &button_q {
                    if btn.0 {
                        for child in children.iter() {
                            if let Ok(mut text) = text_q.get_mut(child) {
                                text.0 = "Continue campaign".to_string();
                            }
                        }
                    }
                }
            }
        } else {
            let seed = time.elapsed().as_nanos() as u64 ^ time_seed();
            let mut world = tc_world::World::new(seed, tc_world::WorldParams::default());
            world.heroes[0].name = "Ashen Sun".to_string();
            world.heroes[1].name = "Hollow Crown".to_string();
            crate::save::store_campaign(&world);
            commands.insert_resource(Campaign { world });
            next_mode.set(crate::game::Mode::Overworld);
        }
    }
}

fn initial_run_and_game() -> (Run, GameState) {
    let run_state = RunState::new(time_seed(), 8);
    let setup = run_state.match_setup();
    let game_state = GameState::from_setup(&setup);
    let run = Run { state: run_state, phase: RunPhase::Playing };
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
            .add_message::<StartCampaign>()
            .add_systems(
                Update,
                (check_run_match_outcome, handle_run_messages).chain().run_if(
                    in_state(AppState::Ready).and_then(not(in_state(crate::game::Mode::OverworldBattle))),
                ),
            );
    }
}
