//! Orbit camera and board interaction, by mouse, keyboard or touch.
//!
//! Mouse: click to select / move, right-drag to turn the board, middle-drag or WASD to
//! pan, wheel to zoom, Q / E to turn by 45°. Touch: tap to select / move, drag to pan,
//! pinch to zoom, twist two fingers to turn.

use std::f32::consts::{FRAC_PI_4, PI};

use bevy::camera::ScalingMode;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::input::touch::Touches;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::atlas::Atlas;
use crate::board_view::{MARKER_LAYER, ShowHeights, board_center, pick_piece, pick_square, square_top};
use tc_core::Side;

use crate::game::{GameState, MAX_AI_LEVEL};

/// Screen px per square, the camera's zoom.
const MIN_ZOOM: f32 = 8.0;
const MAX_ZOOM: f32 = 180.0;
/// Screen px kept clear of the board for the status panel (top) and toolbar (bottom).
const HUD_TOP: f32 = 30.0;
const HUD_BOTTOM: f32 = 60.0;
/// A touch that moves further than this (screen px) is a drag, not a tap.
const TAP_SLOP: f32 = 12.0;
/// Camera tilt above the board; 30° gives the 2:1 isometric look.
const PITCH: f32 = PI / 6.0;
/// Starting view: from White's side with a1 as the nearest corner.
const START_YAW: f32 = -FRAC_PI_4;

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
    /// Turn the view by this many eighths of a full turn.
    Turn(i8),
}

/// The touch gesture in progress, from the first finger down until the last one lifts.
#[derive(Resource, Default)]
struct Gesture {
    active: bool,
    /// Started on a HUD button, so it never reaches the board.
    on_ui: bool,
    /// Moved past the tap slop or used a second finger: not a tap.
    dragged: bool,
}

/// The scene camera, used for picking.
#[derive(Component)]
pub struct MainCamera;

/// Both cameras (scene and markers) follow the orbit.
#[derive(Component)]
struct OrbitCamera;

/// Where the camera looks and from which side. The camera circles `focus` at a fixed
/// tilt; `yaw` eases toward `target_yaw` so turns animate.
#[derive(Resource)]
struct Orbit {
    focus: Vec3,
    yaw: f32,
    target_yaw: f32,
    zoom: f32,
}

impl Default for Orbit {
    fn default() -> Self {
        Orbit { focus: Vec3::ZERO, yaw: START_YAW, target_yaw: START_YAW, zoom: 32.0 }
    }
}

impl Orbit {
    /// Ground directions that point screen-right and screen-up.
    fn ground_axes(&self) -> (Vec3, Vec3) {
        let right = Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin());
        let forward = -Vec3::new(self.yaw.sin(), 0.0, self.yaw.cos());
        (right, forward)
    }

    /// Move the view so the board follows a drag of `delta` screen px.
    fn pan(&mut self, delta: Vec2) {
        let (right, forward) = self.ground_axes();
        self.focus -= right * delta.x / self.zoom;
        // The ground is foreshortened on screen by the tilt.
        self.focus += forward * delta.y / (self.zoom * PITCH.sin());
    }

    fn zoom_by(&mut self, factor: f32) {
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
    }

    /// Turn immediately (dragging), keeping any pending animated turn relative.
    fn turn(&mut self, angle: f32) {
        self.yaw += angle;
        self.target_yaw += angle;
    }
}

/// Window size and board (seed, size) the camera was last fitted to.
#[derive(Resource, Default)]
struct FitCamera(Option<(Vec2, u64, u8)>);

fn setup_camera(mut commands: Commands) {
    let projection = || {
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::WindowSize,
            far: 500.0,
            ..OrthographicProjection::default_3d()
        })
    };
    // Tonemapping off keeps the pixel-art colours exact; no MSAA keeps edges crisp.
    commands.spawn((
        Camera3d::default(),
        MainCamera,
        OrbitCamera,
        projection(),
        Tonemapping::None,
        Msaa::Off,
    ));
    // Move markers draw last, over the scene, with a fresh depth buffer.
    commands.spawn((
        Camera3d::default(),
        Camera { order: 1, clear_color: ClearColorConfig::None, ..default() },
        OrbitCamera,
        MARKER_LAYER,
        projection(),
        Tonemapping::None,
        Msaa::Off,
    ));
}

fn fit_camera(
    mut fit: ResMut<FitCamera>,
    mut orbit: ResMut<Orbit>,
    state: Res<GameState>,
    window: Single<&Window, With<PrimaryWindow>>,
) {
    let win = window.size();
    let key = (win, state.seed, state.size);
    if fit.0 == Some(key) || win.x < 1.0 {
        return;
    }
    fit.0 = Some(key);
    // Size the board for its widest view (a diagonal), with room for tall pieces.
    let n = state.size as f32;
    let span = n * std::f32::consts::SQRT_2;
    orbit.zoom = (win.x / (span + 1.0))
        .min((win.y - HUD_TOP - HUD_BOTTOM) / (span * PITCH.sin() + 3.0))
        .clamp(MIN_ZOOM, MAX_ZOOM);
    orbit.focus = board_center(state.size);
    // Centre the board in the space between the HUD bars.
    orbit.pan(Vec2::new(0.0, (HUD_BOTTOM - HUD_TOP) / 2.0));
}

fn apply_orbit(
    time: Res<Time>,
    mut orbit: ResMut<Orbit>,
    mut cameras: Query<(&mut Transform, &mut Projection), With<OrbitCamera>>,
) {
    let ease = (time.delta_secs() * 12.0).min(1.0);
    orbit.yaw += (orbit.target_yaw - orbit.yaw) * ease;
    let dir = Vec3::new(orbit.yaw.sin() * PITCH.cos(), PITCH.sin(), orbit.yaw.cos() * PITCH.cos());
    let view = Transform::from_translation(orbit.focus + dir * 100.0).looking_at(orbit.focus, Vec3::Y);
    for (mut tf, mut proj) in &mut cameras {
        *tf = view;
        if let Projection::Orthographic(o) = &mut *proj {
            o.scale = 1.0 / orbit.zoom;
        }
    }
}

fn mouse_camera(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    time: Res<Time>,
    mut orbit: ResMut<Orbit>,
) {
    if scroll.delta.y != 0.0 {
        orbit.zoom_by(1.15f32.powf(scroll.delta.y.signum()));
    }
    if mouse.pressed(MouseButton::Right) {
        orbit.turn(-motion.delta.x * 0.01);
    }
    if mouse.pressed(MouseButton::Middle) {
        orbit.pan(motion.delta);
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
    // Keys move the view, which is the opposite of dragging the board.
    orbit.pan(-dir * Vec2::new(1.0, -1.0) * 400.0 * time.delta_secs());
}

fn touch_gestures(
    touches: Res<Touches>,
    buttons: Query<&Interaction>,
    mut gesture: ResMut<Gesture>,
    mut orbit: ResMut<Orbit>,
    mut state: ResMut<GameState>,
    atlas: Res<Atlas>,
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
) {
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
                orbit.pan(t.delta());
            }
        }
        [a, b, ..] => {
            gesture.dragged = true;
            let before = b.previous_position() - a.previous_position();
            let now = b.position() - a.position();
            if before.length() > 1.0 && now.length() > 1.0 {
                orbit.zoom_by(now.length() / before.length());
                orbit.turn(before.angle_to(now));
            }
            orbit.pan((a.delta() + b.delta()) / 2.0);
        }
        _ => {}
    }
    if down.is_empty() {
        let tap = touches.iter_just_released().next().filter(|_| !gesture.dragged && !gesture.on_ui);
        if let Some(t) = tap {
            tap_board(&mut state, &atlas, *camera, orbit.zoom, t.position());
        }
        *gesture = Gesture::default();
    }
}

fn click_board(
    mouse: Res<ButtonInput<MouseButton>>,
    buttons: Query<&Interaction>,
    orbit: Res<Orbit>,
    mut state: ResMut<GameState>,
    atlas: Res<Atlas>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
) {
    if !mouse.just_pressed(MouseButton::Left) || buttons.iter().any(|i| *i != Interaction::None) {
        return;
    }
    if let Some(cursor) = window.cursor_position() {
        tap_board(&mut state, &atlas, *camera, orbit.zoom, cursor);
    }
}

/// Select or move with a click / tap at screen position `cursor`.
fn tap_board(
    state: &mut GameState,
    atlas: &Atlas,
    camera: (&Camera, &GlobalTransform),
    zoom: f32,
    cursor: Vec2,
) {
    if state.pending_promotion.is_some() || state.outcome.is_some() || state.ai_to_move() {
        return;
    }
    let (cam, gtf) = camera;
    // A tap close to a move marker means that square, even when a column in front
    // covers part of it.
    let marked = state
        .selected_moves()
        .into_iter()
        .map(|m| m.to)
        .filter_map(|sq| {
            let p = cam.world_to_viewport(gtf, square_top(sq, state.game.terrain.height(sq))).ok()?;
            Some((sq, p.distance(cursor)))
        })
        .filter(|&(_, d)| d < zoom * 0.35)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(sq, _)| sq);
    // Then a piece's sprite, which stands in front of the ground behind it, then terrain.
    let piece = pick_piece(state, atlas, camera, zoom, cursor);
    let ground = || cam.viewport_to_world(gtf, cursor).ok().and_then(|ray| pick_square(state, ray));
    let Some(sq) = marked.or(piece).or_else(ground) else {
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
        (KeyCode::KeyQ, Action::Turn(1)),
        (KeyCode::KeyE, Action::Turn(-1)),
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
    mut orbit: ResMut<Orbit>,
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
                // Look from the player's new side.
                orbit.target_yaw += PI;
            }
            Action::SwapSides => {}
            Action::AiLevel(d) => {
                state.ai_level = state.ai_level.saturating_add_signed(d).min(MAX_AI_LEVEL);
            }
            Action::Undo => state.undo(),
            Action::ToggleHeights => heights.0 = !heights.0,
            Action::Turn(eighths) => orbit.target_yaw += eighths as f32 * FRAC_PI_4,
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
            .init_resource::<Orbit>()
            .add_message::<Action>()
            .add_systems(Startup, setup_camera)
            .add_systems(
                Update,
                (hotkeys, apply_actions, click_board, touch_gestures, mouse_camera, fit_camera, apply_orbit)
                    .chain(),
            );
    }
}
