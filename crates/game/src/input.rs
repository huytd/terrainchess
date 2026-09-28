//! Orbit camera (perspective) and board interaction, by mouse, keyboard or touch.
//!
//! Mouse: click to select / move, right-drag to turn and tilt the board, middle-drag or WASD to
//! pan, wheel to zoom, Q / E to turn by 45°, Z / X to tilt. Touch: tap to select / move,
//! drag to pan, pinch to zoom, twist two fingers to turn, two-finger vertical drag to tilt.

use std::f32::consts::{FRAC_PI_4, PI};

use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::input::touch::Touches;
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::atlas::Atlas;
use crate::board_view::{board_center, pick_obstacle, pick_piece, pick_square, square_top};
use tc_core::{Outcome, PieceKind, Side};

use crate::game::{GameEvent, GameState, MAX_AI_LEVEL};
use crate::run::{self, Run, RunPhase, TitleMenu};
use crate::save;

/// Camera distance from the point it looks at, the zoom.
const MIN_DISTANCE: f32 = 3.0;
const MAX_DISTANCE: f32 = 150.0;
/// Vertical field of view: enough for depth without distorting the board edges.
const FOV: f32 = PI / 6.0;
/// Screen px kept clear of the board for the status panel (top) and toolbar (bottom).
const HUD_TOP: f32 = 30.0;
const HUD_BOTTOM: f32 = 60.0;
/// A touch that moves further than this (screen px) is a drag, not a tap.
const TAP_SLOP: f32 = 12.0;
/// Minimum and maximum camera tilt above the board.
const MIN_PITCH: f32 = 12.0 * PI / 180.0;
const MAX_PITCH: f32 = 70.0 * PI / 180.0;
/// Starting view: from White's side with a1 as the nearest corner.
const START_YAW: f32 = -FRAC_PI_4;

/// Game commands from the keyboard.
#[derive(Message, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Undo,
    /// Start a new run of this size.
    NewRun(u8),
    ToggleSandbox,
    SwapSides,
    AiLevel(i8),
    Deselect,
    /// Turn the view by this many eighths of a full turn.
    Turn(i8),
    /// Arm the hand slot at this 0-based index.
    ArmSlot(usize),
    /// Discard current hand and draw a new one.
    Discard,
    /// Dev cheat: refill hand with 3 random castable spells.
    CheatSpells,
    /// Dev cheat: win current run match.
    CheatWin,
    /// Dev cheat: lose current run match.
    CheatLoss,
}

#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct DevMode(pub bool);

fn read_dev_flag() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window().and_then(|w| w.location().search().ok()).is_some_and(|s| s.contains("dev"))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::var("TC_DEV").is_ok_and(|v| v == "1")
    }
}

/// The touch gesture in progress, from the first finger down until the last one lifts.
#[derive(Resource, Default)]
struct Gesture {
    active: bool,
    /// Moved past the tap slop or used a second finger: not a tap.
    dragged: bool,
}

#[derive(Component)]
pub struct MainCamera;

/// Where the camera looks and from which side. The camera circles `focus`; `yaw` eases
/// toward `target_yaw` so turns animate.
#[derive(Resource)]
struct Orbit {
    focus: Vec3,
    yaw: f32,
    target_yaw: f32,
    pitch: f32,
    distance: f32,
    /// Window height in logical px, for converting drags to world distances.
    view_h: f32,
}

impl Default for Orbit {
    fn default() -> Self {
        Orbit {
            focus: Vec3::ZERO,
            yaw: START_YAW,
            target_yaw: START_YAW,
            pitch: 30.0 * PI / 180.0,
            distance: 20.0,
            view_h: 800.0,
        }
    }
}

impl Orbit {
    /// Ground directions that point screen-right and screen-up.
    fn ground_axes(&self) -> (Vec3, Vec3) {
        let right = Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin());
        let forward = -Vec3::new(self.yaw.sin(), 0.0, self.yaw.cos());
        (right, forward)
    }

    /// Screen px per world unit at the focus point.
    fn px_per_unit(&self) -> f32 {
        self.view_h / (2.0 * self.distance * (FOV / 2.0).tan())
    }

    /// Move the view so the board follows a drag of `delta` screen px.
    fn pan(&mut self, delta: Vec2) {
        let (right, forward) = self.ground_axes();
        let k = self.px_per_unit();
        self.focus -= right * delta.x / k;
        // The ground is foreshortened on screen by the tilt.
        self.focus += forward * delta.y / (k * self.pitch.sin());
    }

    /// Zoom in by `factor` (above 1) or out (below 1).
    fn zoom_by(&mut self, factor: f32) {
        self.distance = (self.distance / factor).clamp(MIN_DISTANCE, MAX_DISTANCE);
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
    // Tonemapping off keeps the pixel-art colours exact; no MSAA keeps edges crisp.
    commands.spawn((
        Camera3d::default(),
        MainCamera,
        Projection::Perspective(PerspectiveProjection { fov: FOV, ..default() }),
        Tonemapping::None,
        Msaa::Off,
        DistanceFog {
            color: Color::srgb_u8(0xA9, 0xB8, 0xC4),
            falloff: FogFalloff::Linear { start: 0.0, end: 100.0 },
            ..default()
        },
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
    let px_per_unit =
        (win.x / (span + 1.0)).min((win.y - HUD_TOP - HUD_BOTTOM) / (span * orbit.pitch.sin() + 3.0));
    // The near side of the board looks bigger in perspective; leave it some room.
    orbit.view_h = win.y;
    orbit.distance =
        (1.1 * win.y / (2.0 * px_per_unit * (FOV / 2.0).tan())).clamp(MIN_DISTANCE, MAX_DISTANCE);
    orbit.focus = board_center(state.size);
    // Centre the board in the space between the HUD bars.
    orbit.pan(Vec2::new(0.0, (HUD_BOTTOM - HUD_TOP) / 2.0));
}

fn apply_orbit(
    time: Res<Time>,
    window: Single<&Window, With<PrimaryWindow>>,
    title_menu: Res<TitleMenu>,
    mut orbit: ResMut<Orbit>,
    mut camera: Single<&mut Transform, With<MainCamera>>,
) {
    if title_menu.open {
        let rot_speed = 4.0 * std::f32::consts::PI / 180.0;
        orbit.target_yaw += rot_speed * time.delta_secs();
    }
    orbit.view_h = window.height().max(1.0);
    let ease = (time.delta_secs() * 12.0).min(1.0);
    orbit.yaw += (orbit.target_yaw - orbit.yaw) * ease;
    let dir = Vec3::new(
        orbit.yaw.sin() * orbit.pitch.cos(),
        orbit.pitch.sin(),
        orbit.yaw.cos() * orbit.pitch.cos(),
    );
    **camera =
        Transform::from_translation(orbit.focus + dir * orbit.distance).looking_at(orbit.focus, Vec3::Y);
}

#[allow(clippy::too_many_arguments)]
fn mouse_camera(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    time: Res<Time>,
    run: Res<Run>,
    title_menu: Res<TitleMenu>,
    ui_query: Query<&Interaction>,
    mut orbit: ResMut<Orbit>,
) {
    if title_menu.open || run.phase != RunPhase::Playing {
        return;
    }
    if ui_query.iter().any(|&i| i != Interaction::None)
        && (mouse.just_pressed(MouseButton::Right) || mouse.just_pressed(MouseButton::Middle))
    {
        return;
    }
    if scroll.delta.y != 0.0 {
        orbit.zoom_by(1.15f32.powf(scroll.delta.y.signum()));
    }
    if mouse.pressed(MouseButton::Right) {
        orbit.turn(-motion.delta.x * 0.01);
        orbit.pitch = (orbit.pitch + motion.delta.y * 0.005).clamp(MIN_PITCH, MAX_PITCH);
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

    let mut key_tilt = 0.0;
    if keys.pressed(KeyCode::KeyZ) {
        key_tilt -= 1.0;
    }
    if keys.pressed(KeyCode::KeyX) {
        key_tilt += 1.0;
    }
    if key_tilt != 0.0 {
        let tilt_speed = 60.0 * PI / 180.0;
        orbit.pitch = (orbit.pitch + key_tilt * tilt_speed * time.delta_secs()).clamp(MIN_PITCH, MAX_PITCH);
    }
}

#[allow(clippy::too_many_arguments)]
fn touch_gestures(
    touches: Res<Touches>,
    mut gesture: ResMut<Gesture>,
    mut orbit: ResMut<Orbit>,
    mut state: ResMut<GameState>,
    atlas: Res<Atlas>,
    run: Res<Run>,
    title_menu: Res<TitleMenu>,
    ui_query: Query<&Interaction>,
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
) {
    if title_menu.open || run.phase != RunPhase::Playing {
        if gesture.active {
            *gesture = Gesture::default();
        }
        return;
    }
    if touches.any_just_pressed() && !gesture.active {
        if ui_query.iter().any(|&i| i != Interaction::None) {
            return;
        }
        *gesture = Gesture { active: true, ..default() };
    }
    if !gesture.active {
        return;
    }
    let down: Vec<_> = touches.iter().collect();
    match down.as_slice() {
        [t] => {
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
            let avg_delta = (a.delta() + b.delta()) / 2.0;
            orbit.pan(Vec2::new(avg_delta.x, 0.0));
            orbit.pitch = (orbit.pitch + avg_delta.y * 0.005).clamp(MIN_PITCH, MAX_PITCH);
        }
        _ => {}
    }
    if down.is_empty() {
        let tap = touches.iter_just_released().next().filter(|_| !gesture.dragged);
        if let Some(t) = tap {
            tap_board(&mut state, &atlas, *camera, orbit.px_per_unit(), t.position());
        }
        *gesture = Gesture::default();
    }
}

#[allow(clippy::too_many_arguments)]
fn click_board(
    mouse: Res<ButtonInput<MouseButton>>,
    orbit: Res<Orbit>,
    mut state: ResMut<GameState>,
    atlas: Res<Atlas>,
    run: Res<Run>,
    title_menu: Res<TitleMenu>,
    ui_query: Query<&Interaction>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
) {
    if title_menu.open || run.phase != RunPhase::Playing {
        return;
    }
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    if ui_query.iter().any(|&i| i != Interaction::None) {
        return;
    }
    if let Some(cursor) = window.cursor_position() {
        tap_board(&mut state, &atlas, *camera, orbit.px_per_unit(), cursor);
    }
}

/// Select or move with a click / tap at screen position `cursor`.
fn tap_board(
    state: &mut GameState,
    atlas: &Atlas,
    camera: (&Camera, &GlobalTransform),
    px_per_unit: f32,
    cursor: Vec2,
) {
    if state.outcome.is_some() || state.ai_to_move() {
        return;
    }
    let (cam, gtf) = camera;
    // A tap close to a move or spell marker means that square, even when a column in front
    // covers part of it.
    let marked_squares = if state.armed_spell.is_some() {
        state.cast_target_squares()
    } else {
        state.selected_moves().into_iter().map(|m| m.to).collect()
    };
    let marked = marked_squares
        .into_iter()
        .filter_map(|sq| {
            let p = cam.world_to_viewport(gtf, square_top(sq, state.game.terrain.height(sq))).ok()?;
            Some((sq, p.distance(cursor)))
        })
        .filter(|&(_, d)| d < px_per_unit * 0.35)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(sq, _)| sq);
    // Then a piece's sprite, which stands in front of the ground behind it, then obstacle sprite, then terrain.
    let piece = pick_piece(state, atlas, camera, cursor);
    let obstacle = pick_obstacle(state, atlas, camera, cursor);
    let ground = || cam.viewport_to_world(gtf, cursor).ok().and_then(|ray| pick_square(state, ray));
    let Some(sq) = marked.or(piece).or(obstacle).or_else(ground) else {
        if state.armed_spell.is_some() {
            state.disarm();
        } else {
            state.selected = None;
            state.pieces_dirty = true;
        }
        return;
    };

    if let Some(spell) = state.armed_spell {
        let targets = state.cast_target_squares();
        if targets.contains(&sq) {
            match spell {
                tc_core::SpellId::RaiseEarth => state.cast(tc_core::SpellCast::RaiseEarth(sq)),
                tc_core::SpellId::LowerEarth => state.cast(tc_core::SpellCast::LowerEarth(sq)),
                tc_core::SpellId::Freeze => state.cast(tc_core::SpellCast::Freeze(sq)),
                tc_core::SpellId::Shield => state.cast(tc_core::SpellCast::Shield(sq)),
                tc_core::SpellId::Bridge => state.cast(tc_core::SpellCast::Bridge(sq)),
                tc_core::SpellId::DigTunnel => {
                    if let Some(first) = state.swap_first {
                        let (a, b) = if first < sq { (first, sq) } else { (sq, first) };
                        state.cast(tc_core::SpellCast::DigTunnel(a, b));
                    } else {
                        state.swap_first = Some(sq);
                        state.pieces_dirty = true;
                    }
                }
                tc_core::SpellId::Swap => {
                    if let Some(first) = state.swap_first {
                        state.cast(tc_core::SpellCast::Swap(first, sq));
                    } else {
                        state.swap_first = Some(sq);
                        state.pieces_dirty = true;
                    }
                }
                tc_core::SpellId::Rewind => state.cast(tc_core::SpellCast::Rewind),
            }
        } else {
            state.disarm();
        }
        return;
    }

    let side = state.game.pos.side_to_move;
    let moves: Vec<_> = state.selected_moves().into_iter().filter(|m| m.to == sq).collect();
    // Several moves to one square only happens with promotions; there is no picker yet,
    // so pawns become queens.
    let chosen = moves.iter().find(|m| m.promotion.is_none_or(|k| k == PieceKind::Queen));
    match chosen {
        Some(&mv) => state.play(mv),
        None => {
            let own = state.game.pos.get(sq).is_some_and(|p| p.side == side);
            if own && state.selected != Some(sq) {
                state.selected = Some(sq);
                state.events.push(GameEvent::Selected { sq });
            } else {
                state.selected = None;
            }
            state.pieces_dirty = true;
        }
    }
}

fn hotkeys(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<GameState>,
    dev: Res<DevMode>,
    mut title_menu: ResMut<TitleMenu>,
    mut actions: MessageWriter<Action>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        if title_menu.open {
            if save::has_save() {
                title_menu.open = false;
            }
        } else if state.armed_spell.is_some() || state.selected.is_some() {
            actions.write(Action::Deselect);
        } else {
            title_menu.open = true;
        }
    }

    if title_menu.open {
        return;
    }

    for (key, action) in [
        (KeyCode::Digit1, Action::NewRun(8)),
        (KeyCode::Digit2, Action::NewRun(16)),
        (KeyCode::Digit3, Action::NewRun(32)),
        (KeyCode::Digit5, Action::ArmSlot(0)),
        (KeyCode::Digit6, Action::ArmSlot(1)),
        (KeyCode::Digit7, Action::ArmSlot(2)),
        (KeyCode::KeyD, Action::Discard),
        (KeyCode::KeyN, Action::NewRun(state.size)),
        (KeyCode::KeyH, Action::ToggleSandbox),
        (KeyCode::KeyF, Action::SwapSides),
        (KeyCode::Minus, Action::AiLevel(-1)),
        (KeyCode::Equal, Action::AiLevel(1)),
        (KeyCode::Backspace, Action::Undo),
        (KeyCode::KeyU, Action::Undo),
        (KeyCode::KeyQ, Action::Turn(1)),
        (KeyCode::KeyE, Action::Turn(-1)),
    ] {
        if keys.just_pressed(key) {
            actions.write(action);
        }
    }

    if dev.0 {
        if keys.just_pressed(KeyCode::F7) {
            actions.write(Action::CheatSpells);
        }
        if keys.just_pressed(KeyCode::F8) {
            actions.write(Action::CheatWin);
        }
        if keys.just_pressed(KeyCode::F9) {
            actions.write(Action::CheatLoss);
        }
    }
}

fn apply_actions(
    mut actions: MessageReader<Action>,
    mut state: ResMut<GameState>,
    mut run: ResMut<Run>,
    mut orbit: ResMut<Orbit>,
    time: Res<Time>,
) {
    for &action in actions.read() {
        if run.phase != RunPhase::Playing {
            if let Action::NewRun(size) = action {
                let seed = time.elapsed().as_nanos() as u64 ^ run::time_seed();
                run::start_new_run(size, &mut run, &mut state, seed);
            }
            continue;
        }

        match action {
            Action::NewRun(size) => {
                let seed = time.elapsed().as_nanos() as u64 ^ run::time_seed();
                run::start_new_run(size, &mut run, &mut state, seed);
            }
            Action::ToggleSandbox => {
                let seed = time.elapsed().as_nanos() as u64 ^ run::time_seed();
                run::toggle_sandbox(&mut run, &mut state, seed);
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
            Action::Turn(eighths) => orbit.target_yaw += eighths as f32 * FRAC_PI_4,
            Action::ArmSlot(slot) => {
                state.arm_slot(slot);
            }
            Action::Discard => {
                state.discard();
            }
            Action::CheatSpells => {
                const ALL_CASTABLE: [tc_core::SpellId; 8] = [
                    tc_core::SpellId::RaiseEarth,
                    tc_core::SpellId::LowerEarth,
                    tc_core::SpellId::Freeze,
                    tc_core::SpellId::Bridge,
                    tc_core::SpellId::DigTunnel,
                    tc_core::SpellId::Shield,
                    tc_core::SpellId::Swap,
                    tc_core::SpellId::Rewind,
                ];
                let mut seed = time.elapsed().as_nanos() as u64 ^ state.seed;
                let mut pick = || {
                    seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                    ALL_CASTABLE[(seed as usize) % ALL_CASTABLE.len()]
                };
                let hand = &mut state.game.hands[Side::White.index()];
                hand.hand = [Some(pick()), Some(pick()), Some(pick())];
                hand.used = [false, false, false];
                state.disarm();
                state.pieces_dirty = true;
            }
            Action::CheatWin => {
                state.outcome = Some(Outcome::Checkmate { winner: Side::White });
                state.pieces_dirty = true;
            }
            Action::CheatLoss => {
                state.outcome = Some(Outcome::Checkmate { winner: Side::Black });
                state.pieces_dirty = true;
            }
            Action::Deselect => {
                state.disarm();
                state.selected = None;
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
            .insert_resource(DevMode(read_dev_flag()))
            .add_message::<Action>()
            .add_systems(Startup, setup_camera)
            .add_systems(
                Update,
                (hotkeys, apply_actions, click_board, touch_gestures, mouse_camera, fit_camera, apply_orbit)
                    .chain(),
            );
    }
}
