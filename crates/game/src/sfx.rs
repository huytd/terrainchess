//! Synthesized retro chiptune sound effects.

use bevy::audio::{AudioPlayer, AudioSource, PlaybackSettings, Volume};
use bevy::prelude::*;

use crate::board_view::HOP_SECS;
use crate::game::{GameEvent, GameState};
use crate::run::{PickCard, Run, RunPhase};

pub const SFX_VOLUME: f32 = 0.6;

#[derive(Resource, Clone)]
pub struct SfxHandles {
    pub select: Handle<AudioSource>,
    pub move_sound: Handle<AudioSource>,
    pub capture: Handle<AudioSource>,
    pub splash: Handle<AudioSource>,
    pub spell: Handle<AudioSource>,
    pub pickup: Handle<AudioSource>,
    pub check: Handle<AudioSource>,
    pub win: Handle<AudioSource>,
    pub lose: Handle<AudioSource>,
    pub card: Handle<AudioSource>,
}

#[derive(Resource, Default)]
pub struct SfxMuted(pub bool);

#[derive(Component)]
struct PendingSplash {
    timer: f32,
}

fn setup_sfx(mut commands: Commands, mut audio_sources: ResMut<Assets<AudioSource>>) {
    let handles = SfxHandles {
        select: audio_sources
            .add(AudioSource { bytes: include_bytes!("../../../assets/sfx/select.wav").as_slice().into() }),
        move_sound: audio_sources
            .add(AudioSource { bytes: include_bytes!("../../../assets/sfx/move.wav").as_slice().into() }),
        capture: audio_sources
            .add(AudioSource { bytes: include_bytes!("../../../assets/sfx/capture.wav").as_slice().into() }),
        splash: audio_sources
            .add(AudioSource { bytes: include_bytes!("../../../assets/sfx/splash.wav").as_slice().into() }),
        spell: audio_sources
            .add(AudioSource { bytes: include_bytes!("../../../assets/sfx/spell.wav").as_slice().into() }),
        pickup: audio_sources
            .add(AudioSource { bytes: include_bytes!("../../../assets/sfx/pickup.wav").as_slice().into() }),
        check: audio_sources
            .add(AudioSource { bytes: include_bytes!("../../../assets/sfx/check.wav").as_slice().into() }),
        win: audio_sources
            .add(AudioSource { bytes: include_bytes!("../../../assets/sfx/win.wav").as_slice().into() }),
        lose: audio_sources
            .add(AudioSource { bytes: include_bytes!("../../../assets/sfx/lose.wav").as_slice().into() }),
        card: audio_sources
            .add(AudioSource { bytes: include_bytes!("../../../assets/sfx/card.wav").as_slice().into() }),
    };
    commands.insert_resource(handles);
    commands.insert_resource(SfxMuted(false));
}

fn play_sound(commands: &mut Commands, handle: &Handle<AudioSource>, muted: bool) {
    if muted {
        return;
    }
    commands.spawn((
        AudioPlayer::new(handle.clone()),
        PlaybackSettings::DESPAWN.with_volume(Volume::Linear(SFX_VOLUME)),
    ));
}

fn handle_mute_toggle(keys: Res<ButtonInput<KeyCode>>, mut muted: ResMut<SfxMuted>) {
    if keys.just_pressed(KeyCode::KeyM) {
        muted.0 = !muted.0;
    }
}

fn play_game_events_sfx(
    mut commands: Commands,
    mut state: ResMut<GameState>,
    sfx: Res<SfxHandles>,
    muted: Res<SfxMuted>,
) {
    if state.events.is_empty() {
        return;
    }
    let events: Vec<_> = state.events.drain(..).collect();
    for event in events {
        match event {
            GameEvent::Selected { .. } => {
                play_sound(&mut commands, &sfx.select, muted.0);
            }
            GameEvent::Moved { to, .. } => {
                play_sound(&mut commands, &sfx.move_sound, muted.0);
                if state.game.terrain.get(to).is_water() {
                    commands.spawn(PendingSplash { timer: HOP_SECS });
                }
            }
            GameEvent::Captured { .. } => {
                play_sound(&mut commands, &sfx.capture, muted.0);
            }
            GameEvent::Cast { .. } => {
                play_sound(&mut commands, &sfx.spell, muted.0);
            }
            GameEvent::Pickup { .. } => {
                play_sound(&mut commands, &sfx.pickup, muted.0);
            }
            GameEvent::Check { .. } => {
                play_sound(&mut commands, &sfx.check, muted.0);
            }
            GameEvent::Cleared { .. } => {
                play_sound(&mut commands, &sfx.capture, muted.0);
            }
            GameEvent::Promoted { .. } => {}
        }
    }
}

fn update_pending_splash(
    mut commands: Commands,
    time: Res<Time>,
    sfx: Res<SfxHandles>,
    muted: Res<SfxMuted>,
    mut q: Query<(Entity, &mut PendingSplash)>,
) {
    for (entity, mut pending) in &mut q {
        pending.timer -= time.delta_secs();
        if pending.timer <= 0.0 {
            play_sound(&mut commands, &sfx.splash, muted.0);
            commands.entity(entity).despawn();
        }
    }
}

fn handle_run_phase_sfx(
    mut commands: Commands,
    run: Res<Run>,
    sfx: Res<SfxHandles>,
    muted: Res<SfxMuted>,
    mut last_phase: Local<Option<RunPhase>>,
    mut pick_reader: MessageReader<PickCard>,
) {
    for _ in pick_reader.read() {
        play_sound(&mut commands, &sfx.card, muted.0);
    }

    if *last_phase == Some(run.phase.clone()) {
        return;
    }
    let prev = last_phase.clone();
    *last_phase = Some(run.phase.clone());

    let Some(_) = prev else {
        return;
    };

    match &run.phase {
        RunPhase::Draft(_) => {
            play_sound(&mut commands, &sfx.card, muted.0);
        }
        RunPhase::Over { won } => {
            if *won {
                play_sound(&mut commands, &sfx.win, muted.0);
            } else {
                play_sound(&mut commands, &sfx.lose, muted.0);
            }
        }
        RunPhase::Playing => {}
    }
}

pub struct SfxPlugin;

impl Plugin for SfxPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_sfx).add_systems(
            Update,
            (
                handle_mute_toggle,
                play_game_events_sfx.after(crate::fx::process_game_events),
                update_pending_splash,
                handle_run_phase_sfx,
            ),
        );
    }
}
