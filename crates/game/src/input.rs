//! Camera (integer zoom only, so pixels stay crisp) and board interaction, by mouse,
//! keyboard or touch (tap to select / move, drag to pan, pinch to zoom).

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::input::touch::Touches;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::board_view::{LIFT, ShowHeights, TILE, pick_tile, tile_top};
use tc_core::Side;

use crate::game::{GameState, MAX_AI_LEVEL};

const MAX_ZOOM: f32 = 6.0;
/// Screen px kept clear of the board for the status panel (top) and toolbar (bottom).
const HUD_TOP: f32 = 30.0;
const HUD_BOTTOM: f32 = 60.0;
/// A touch that moves further than this (screen px) is a drag, not a tap.
const TAP_SLOP: f32 = 12.0;
/// Pinch distance ratio that steps the zoom by one.
const PINCH_STEP: f32 = 1.3;

/// Game commands shared by the keyboard and the HUD toolbar.
#[derive(Message, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Undo,
    /// A fresh board of this size.
    NewBoard(u8),
    ToggleAi,
    SwapSides,
    AiLevel(i8),
    ToggleHeights,
    Deselect,
}

/// The touch gesture in progress, from the first finger down until the last one lifts.
#[derive(Resource, Default)]
struct Gesture {
    active: bool,
    /// Started on a HUD button, so it never reaches the board.
    on_ui: bool,
    /// Moved past the tap slop or used a second finger: not a tap.
    dragged: bool,
    /// Finger spread at the last zoom step, while pinching.
    pinch_base: Option<f32>,
}

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
    let zoom = (win.x / board_px).min((win.y - HUD_TOP - HUD_BOTTOM) / board_px).floor().clamp(1.0, MAX_ZOOM);
    if let Projection::Orthographic(o) = &mut *proj {
        o.scale = 1.0 / zoom;
    }
    // Centre the board in the space between the HUD bars.
    let shift = Vec2::new(0.0, (HUD_BOTTOM - HUD_TOP) / 2.0 / zoom);
    tf.translation = (board_center(state.size) - shift).extend(tf.translation.z);
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

fn touch_gestures(
    touches: Res<Touches>,
    buttons: Query<&Interaction>,
    mut gesture: ResMut<Gesture>,
    mut state: ResMut<GameState>,
    camera: Single<(&Camera, &GlobalTransform, &mut Transform, &mut Projection), With<MainCamera>>,
) {
    let (cam, gtf, mut tf, mut proj) = camera.into_inner();
    let Projection::Orthographic(o) = &mut *proj else { return };
    if touches.any_just_pressed() && !gesture.active {
        // UI interaction is updated before Update, so a press on a button shows here.
        let on_ui = buttons.iter().any(|i| *i != Interaction::None);
        *gesture = Gesture { active: true, on_ui, ..default() };
    }
    if !gesture.active {
        return;
    }
    let down: Vec<_> = touches.iter().collect();
    match down.as_slice() {
        [t] if !gesture.on_ui => {
            if t.distance().length() > TAP_SLOP {
                gesture.dragged = true;
            }
            if gesture.dragged {
                tf.translation.x -= t.delta().x * o.scale;
                tf.translation.y += t.delta().y * o.scale;
            }
            gesture.pinch_base = None;
        }
        [a, b, ..] => {
            gesture.dragged = true;
            let spread = a.position().distance(b.position());
            let base = *gesture.pinch_base.get_or_insert(spread);
            let zoom = (1.0 / o.scale).round();
            let step = if spread > base * PINCH_STEP {
                1.0
            } else if spread < base / PINCH_STEP {
                -1.0
            } else {
                0.0
            };
            if step != 0.0 {
                o.scale = 1.0 / (zoom + step).clamp(1.0, MAX_ZOOM);
                gesture.pinch_base = Some(spread);
            }
            let mid = (a.delta() + b.delta()) / 2.0;
            tf.translation.x -= mid.x * o.scale;
            tf.translation.y += mid.y * o.scale;
        }
        _ => {}
    }
    if down.is_empty() {
        let tap = touches.iter_just_released().next().filter(|_| !gesture.dragged && !gesture.on_ui);
        if let Some(t) = tap
            && let Ok(p) = cam.viewport_to_world_2d(gtf, t.position())
        {
            tap_board(&mut state, p);
        }
        *gesture = Gesture::default();
    }
}

fn click_board(
    mouse: Res<ButtonInput<MouseButton>>,
    buttons: Query<&Interaction>,
    mut state: ResMut<GameState>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
) {
    if !mouse.just_pressed(MouseButton::Left) || buttons.iter().any(|i| *i != Interaction::None) {
        return;
    }
    if let Some(p) = cursor_world(&window, *camera) {
        tap_board(&mut state, p);
    }
}

/// Select or move with a click / tap at world position `p`.
fn tap_board(state: &mut GameState, p: Vec2) {
    if state.pending_promotion.is_some() || state.outcome.is_some() || state.ai_to_move() {
        return;
    }
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
    let Some(sq) = marked.or_else(|| pick_tile(state, p)) else {
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

fn hotkeys(keys: Res<ButtonInput<KeyCode>>, state: Res<GameState>, mut actions: MessageWriter<Action>) {
    for (key, action) in [
        (KeyCode::Digit1, Action::NewBoard(8)),
        (KeyCode::Digit2, Action::NewBoard(16)),
        (KeyCode::Digit3, Action::NewBoard(32)),
        (KeyCode::KeyN, Action::NewBoard(state.size)),
        (KeyCode::KeyH, Action::ToggleAi),
        (KeyCode::KeyF, Action::SwapSides),
        (KeyCode::Minus, Action::AiLevel(-1)),
        (KeyCode::Equal, Action::AiLevel(1)),
        (KeyCode::Backspace, Action::Undo),
        (KeyCode::KeyU, Action::Undo),
        (KeyCode::KeyT, Action::ToggleHeights),
        (KeyCode::Escape, Action::Deselect),
    ] {
        if keys.just_pressed(key) {
            actions.write(action);
        }
    }
}

fn apply_actions(
    mut actions: MessageReader<Action>,
    mut state: ResMut<GameState>,
    mut heights: ResMut<ShowHeights>,
    time: Res<Time>,
) {
    for &action in actions.read() {
        match action {
            Action::NewBoard(size) => {
                // Time since start is a good enough source of a fresh seed on every platform.
                let seed = time.elapsed().as_nanos() as u64 ^ state.seed.rotate_left(17);
                *state = state.restart(size, seed);
            }
            Action::ToggleAi => {
                // Toggle between playing the AI (as the Ashen Sun) and hotseat.
                state.ai_side = match state.ai_side {
                    Some(_) => None,
                    None => Some(Side::Black),
                };
            }
            Action::SwapSides if state.ai_side.is_some() => {
                state.ai_side = state.ai_side.map(Side::opposite);
                state.selected = None;
                state.pieces_dirty = true;
            }
            Action::SwapSides => {}
            Action::AiLevel(d) => {
                state.ai_level = state.ai_level.saturating_add_signed(d).min(MAX_AI_LEVEL);
            }
            Action::Undo => state.undo(),
            Action::ToggleHeights => heights.0 = !heights.0,
            Action::Deselect => {
                state.selected = None;
                state.pending_promotion = None;
                state.pieces_dirty = true;
            }
        }
    }
}

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FitCamera>()
            .init_resource::<Gesture>()
            .add_message::<Action>()
            .add_systems(Startup, setup_camera)
            .add_systems(
                Update,
                (hotkeys, apply_actions, click_board, touch_gestures, pan_zoom, fit_camera).chain(),
            );
    }
}
