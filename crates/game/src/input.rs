//! Camera (integer zoom only, so pixels stay crisp) and board interaction.

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::board_view::{LIFT, TILE, pick_tile, tile_top};
use tc_core::Side;

use crate::game::{GameState, MAX_AI_LEVEL};

const MAX_ZOOM: f32 = 6.0;

#[derive(Component)]
pub struct MainCamera;

/// Window size and board (seed, size) the camera was last fitted to.
#[derive(Resource, Default)]
struct FitCamera(Option<(Vec2, u64, u8)>);

fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, MainCamera));
}

fn board_center(size: u8) -> Vec2 {
    let span = (size as f32 - 1.0) * TILE;
    Vec2::new(span / 2.0, span / 2.0 + LIFT)
}

fn fit_camera(
    mut fit: ResMut<FitCamera>,
    state: Res<GameState>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&mut Transform, &mut Projection), With<MainCamera>>,
) {
    let win = window.size();
    let key = (win, state.seed, state.size);
    if fit.0 == Some(key) || win.x < 1.0 {
        return;
    }
    fit.0 = Some(key);
    let (mut tf, mut proj) = camera.into_inner();
    let board_px = state.size as f32 * TILE + 3.0 * TILE;
    let zoom = (win.x / board_px).min((win.y - 90.0) / board_px).floor().clamp(1.0, MAX_ZOOM);
    if let Projection::Orthographic(o) = &mut *proj {
        o.scale = 1.0 / zoom;
    }
    tf.translation = board_center(state.size).extend(tf.translation.z);
}

fn pan_zoom(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    time: Res<Time>,
    camera: Single<(&mut Transform, &mut Projection), With<MainCamera>>,
) {
    let (mut tf, mut proj) = camera.into_inner();
    let Projection::Orthographic(o) = &mut *proj else { return };
    if scroll.delta.y != 0.0 {
        let zoom = (1.0 / o.scale).round();
        let zoom = (zoom + scroll.delta.y.signum()).clamp(1.0, MAX_ZOOM);
        o.scale = 1.0 / zoom;
    }
    if mouse.pressed(MouseButton::Right) || mouse.pressed(MouseButton::Middle) {
        tf.translation.x -= motion.delta.x * o.scale;
        tf.translation.y += motion.delta.y * o.scale;
    }
    let mut dir = Vec2::ZERO;
    for (key, d) in [
        (KeyCode::KeyW, Vec2::Y),
        (KeyCode::ArrowUp, Vec2::Y),
        (KeyCode::KeyS, -Vec2::Y),
        (KeyCode::ArrowDown, -Vec2::Y),
        (KeyCode::KeyA, -Vec2::X),
        (KeyCode::ArrowLeft, -Vec2::X),
        (KeyCode::KeyD, Vec2::X),
        (KeyCode::ArrowRight, Vec2::X),
    ] {
        if keys.pressed(key) {
            dir += d;
        }
    }
    tf.translation += (dir * 400.0 * o.scale * time.delta_secs()).extend(0.0);
}

/// World position under the cursor.
pub fn cursor_world(window: &Window, camera: (&Camera, &GlobalTransform)) -> Option<Vec2> {
    let cursor = window.cursor_position()?;
    camera.0.viewport_to_world_2d(camera.1, cursor).ok()
}

fn click_board(
    mouse: Res<ButtonInput<MouseButton>>,
    mut state: ResMut<GameState>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
) {
    if !mouse.just_pressed(MouseButton::Left)
        || state.pending_promotion.is_some()
        || state.outcome.is_some()
        || state.ai_to_move()
    {
        return;
    }
    let Some(p) = cursor_world(&window, *camera) else { return };
    // Markers draw above everything, so a click near one means that square even
    // when a higher tile in front covers it.
    let marked = state
        .selected_moves()
        .into_iter()
        .map(|m| m.to)
        .map(|sq| (sq, tile_top(sq, state.game.terrain.height(sq)).distance(p)))
        .filter(|&(_, d)| d < TILE * 0.4)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(sq, _)| sq);
    let Some(sq) = marked.or_else(|| pick_tile(&state, p)) else {
        state.selected = None;
        state.pieces_dirty = true;
        return;
    };
    let side = state.game.pos.side_to_move;
    let moves: Vec<_> = state.selected_moves().into_iter().filter(|m| m.to == sq).collect();
    match moves.len() {
        0 => {
            let own = state.game.pos.get(sq).is_some_and(|p| p.side == side);
            state.selected = if own && state.selected != Some(sq) { Some(sq) } else { None };
            state.pieces_dirty = true;
        }
        1 => state.play(moves[0]),
        // Several moves to one square only happens with promotions.
        _ => state.pending_promotion = Some(moves),
    }
}

fn hotkeys(keys: Res<ButtonInput<KeyCode>>, mut state: ResMut<GameState>, time: Res<Time>) {
    let new_size = if keys.just_pressed(KeyCode::Digit1) {
        Some(8)
    } else if keys.just_pressed(KeyCode::Digit2) {
        Some(16)
    } else if keys.just_pressed(KeyCode::Digit3) {
        Some(32)
    } else if keys.just_pressed(KeyCode::KeyN) {
        Some(state.size)
    } else {
        None
    };
    if let Some(size) = new_size {
        // Time since start is a good enough source of a fresh seed on every platform.
        let seed = time.elapsed().as_nanos() as u64 ^ state.seed.rotate_left(17);
        *state = state.restart(size, seed);
    }
    if keys.just_pressed(KeyCode::KeyH) {
        // Toggle between playing the AI (as the Ashen Sun) and hotseat.
        state.ai_side = match state.ai_side {
            Some(_) => None,
            None => Some(Side::Black),
        };
    }
    if keys.just_pressed(KeyCode::KeyF) && state.ai_side.is_some() {
        // Swap sides with the AI.
        state.ai_side = state.ai_side.map(Side::opposite);
        state.selected = None;
        state.pieces_dirty = true;
    }
    if keys.just_pressed(KeyCode::Minus) {
        state.ai_level = state.ai_level.saturating_sub(1);
    }
    if keys.just_pressed(KeyCode::Equal) {
        state.ai_level = (state.ai_level + 1).min(MAX_AI_LEVEL);
    }
    if keys.just_pressed(KeyCode::Backspace) || keys.just_pressed(KeyCode::KeyU) {
        state.undo();
    }
    if keys.just_pressed(KeyCode::Escape) {
        state.selected = None;
        state.pending_promotion = None;
        state.pieces_dirty = true;
    }
}

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FitCamera>()
            .add_systems(Startup, setup_camera)
            .add_systems(Update, (hotkeys, click_board, pan_zoom, fit_camera).chain());
    }
}
