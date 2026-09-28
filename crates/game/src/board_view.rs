//! Draws the board as 2:1 isometric blocks (PLAN.md §8): every square is a column
//! with a diamond top and two shaded side faces, raised `LIFT` px per height level.
//! a1 is the bottom corner, files run up-right and ranks up-left. Columns are drawn
//! back to front, so nearer and taller columns hide what stands behind them.

use bevy::prelude::*;
use tc_core::movegen::Ctx;
use tc_core::terrain::TileKind;
use tc_core::{Feature, MoveKind, Obstacle, PieceKind, Side, Sq};

use crate::atlas::Atlas;
use crate::game::GameState;

/// Diamond top of one square (matches ISO_* in tools/process_sprites.py).
pub const TILE_W: f32 = 48.0;
pub const TILE_H: f32 = 24.0;
/// Screen px per height level.
pub const LIFT: f32 = 12.0;
/// Heights 0..=3 have face sprites.
const MAX_LEVEL: u8 = 3;

/// Depth layers within one depth band (squares with the same x + y).
mod layer {
    pub const FACE: f32 = 0.0;
    pub const GROUND: f32 = 0.05;
    pub const EDGE: f32 = 0.1;
    pub const DECAL: f32 = 0.2;
    pub const TINT: f32 = 0.3;
    pub const ACTOR: f32 = 0.6;
}

/// Rim drawn around every diamond top so neighbouring squares stay distinct.
const OUTLINE: Color = Color::srgba(0.05, 0.06, 0.10, 0.45);

/// Move markers and height badges sit above every column, so a cliff or a tall piece
/// in front never hides where you can move.
const MARKER_Z: f32 = 100.0;

/// Screen position of the centre of a square's diamond top.
pub fn tile_top(sq: Sq, height: u8) -> Vec2 {
    let (x, y) = (sq.x as f32, sq.y as f32);
    Vec2::new((x - y) * TILE_W / 2.0, (x + y) * TILE_H / 2.0 + height as f32 * LIFT)
}

/// Squares further from the viewer (larger x + y) are drawn first.
pub fn depth_z(sq: Sq, layer: f32) -> f32 {
    -((sq.x + sq.y) as f32) + layer
}

/// Centre of the board and the screen size it covers at zoom 1, for fitting the camera.
pub fn board_bounds(size: u8) -> (Vec2, Vec2) {
    let n = size as f32;
    let center = Vec2::new(0.0, (n - 1.0) * TILE_H / 2.0 + LIFT);
    // Diamond span, plus room for the tallest columns and pieces.
    (center, Vec2::new(n * TILE_W, n * TILE_H + 4.0 * LIFT + 40.0))
}

/// Screen px of side face under a column of this height.
fn column_depth(height: u8) -> f32 {
    6.0 + height as f32 * LIFT
}

/// Square whose top or side face is under `p`. Nearest columns are tested first
/// since they are drawn on top.
pub fn pick_tile(state: &GameState, p: Vec2) -> Option<Sq> {
    let t = &state.game.terrain;
    let mut squares: Vec<Sq> = tc_core::board::squares(state.size).collect();
    squares.sort_by_key(|sq| sq.x + sq.y);
    squares.into_iter().find(|&sq| {
        let d = p - tile_top(sq, t.height(sq));
        let (hw, hh) = (TILE_W / 2.0, TILE_H / 2.0);
        if d.x.abs() > hw {
            return false;
        }
        // The lower diamond edge at this x; the column hangs below it.
        let lower = -hh + d.x.abs() * hh / hw;
        d.y <= -lower && d.y >= lower - column_depth(t.height(sq))
    })
}

#[derive(Component)]
struct TerrainPart;

#[derive(Component)]
struct PieceSprite;

#[derive(Component)]
struct Overlay;

#[derive(Component)]
struct AnimatedWater {
    frames: [&'static str; 2],
}

#[derive(Component)]
pub struct HeightBadge;

/// Hop from one spot to another along a small arc.
#[derive(Component)]
struct Hop {
    from: Vec3,
    to: Vec3,
    t: f32,
    height: f32,
}

const HOP_SECS: f32 = 0.22;

/// A per-square number for picking tile variants without flicker.
fn hash(sq: Sq, salt: u32) -> u32 {
    let mut h = (sq.x as u32).wrapping_mul(73_856_093) ^ (sq.y as u32).wrapping_mul(19_349_663) ^ salt;
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1_e995);
    h ^ (h >> 15)
}

/// Rune colours for linked cave pairs (teal, violet, amber, crimson).
fn cave_color(link: u8) -> Color {
    [
        Color::srgb_u8(0x7F, 0xF3, 0xE0),
        Color::srgb_u8(0xD9, 0xA6, 0xFF),
        Color::srgb_u8(0xFF, 0xD3, 0x5A),
        Color::srgb_u8(0xFF, 0x8A, 0x8A),
    ][link as usize % 4]
}

fn spawn_terrain(
    mut commands: Commands,
    mut state: ResMut<GameState>,
    atlas: Res<Atlas>,
    old: Query<Entity, With<TerrainPart>>,
) {
    if !state.terrain_dirty {
        return;
    }
    state.terrain_dirty = false;
    for e in &old {
        commands.entity(e).despawn();
    }
    let size = state.size;
    let t = state.game.terrain.clone();
    for sq in tc_core::board::squares(size) {
        let tile = *t.get(sq);
        let top = tile_top(sq, tile.height);

        // Ground, with a light checker and brighter high ground for readability.
        let (name, water) = match tile.kind {
            TileKind::Grass => {
                (["grass_0", "grass_1", "grass_0", "grass_dark"][hash(sq, 1) as usize % 4], None)
            }
            TileKind::Stone => (["cobble", "flagstone"][hash(sq, 2) as usize % 2], None),
            TileKind::Sand => ("sand", None),
            TileKind::ShallowWater => ("shallow_0", Some(["iso_shallow_0", "iso_shallow_1"])),
            TileKind::DeepWater => ("deep", None),
            TileKind::Void => ("void", None),
        };
        let checker = if (sq.x + sq.y) % 2 == 0 { 0.9 } else { 1.0 };
        let shade = checker * (0.84 + 0.07 * tile.height as f32);
        let (mut sprite, anchor) = atlas.sprite(&format!("iso_{name}"));
        sprite.color = Color::srgb(shade, shade, shade);
        let mut ground = commands.spawn((
            TerrainPart,
            sprite,
            anchor,
            Transform::from_translation(top.extend(depth_z(sq, layer::GROUND))),
        ));
        if let Some(frames) = water {
            ground.insert(AnimatedWater { frames });
        }
        let (mut rim, anchor) = atlas.sprite("iso_outline");
        rim.color = OUTLINE;
        commands.spawn((
            TerrainPart,
            rim,
            anchor,
            Transform::from_translation(top.extend(depth_z(sq, layer::EDGE))),
        ));

        // Side faces: the left one in shade, the right one half lit.
        let level = tile.height.min(MAX_LEVEL);
        for (side, x, light) in [("l", -TILE_W / 2.0, 0.58), ("r", 0.0, 0.8)] {
            let (mut face, anchor) = atlas.sprite(&format!("iso_face_{side}_{level}"));
            let light = light + 0.04 * tile.height as f32;
            face.color = Color::srgb(light, light, light);
            commands.spawn((
                TerrainPart,
                face,
                anchor,
                Transform::from_translation(Vec3::new(top.x + x, top.y, depth_z(sq, layer::FACE))),
            ));
        }

        match tile.feature {
            Feature::Cave(link) => {
                let (mut sprite, anchor) = atlas.sprite("cave_rune_0");
                sprite.color = cave_color(link);
                sprite.custom_size = Some(Vec2::new(24.0, 13.0));
                commands.spawn((
                    TerrainPart,
                    sprite,
                    anchor,
                    Transform::from_translation(top.extend(depth_z(sq, layer::DECAL))),
                ));
            }
            Feature::Obstacle(kind) => {
                let name = match kind {
                    Obstacle::Rock => ["rock", "rock_mossy"][hash(sq, 4) as usize % 2],
                    Obstacle::Tree => ["pine", "pine", "dead_tree"][hash(sq, 5) as usize % 3],
                };
                let (sprite, anchor) = atlas.sprite(name);
                commands.spawn((
                    TerrainPart,
                    sprite,
                    anchor,
                    Transform::from_translation(Vec3::new(top.x, top.y - 3.0, depth_z(sq, layer::ACTOR))),
                ));
            }
            _ => {}
        }

        commands.spawn((
            TerrainPart,
            HeightBadge,
            Text2d::new(tile.height.to_string()),
            TextFont { font_size: 14.0.into(), ..default() },
            TextColor(Color::WHITE),
            Visibility::Hidden,
            Transform::from_translation(Vec3::new(top.x, top.y + 1.0, MARKER_Z + 1.0))
                .with_scale(Vec3::splat(0.75)),
        ));
    }
}

fn piece_sprite_name(kind: PieceKind, side: Side) -> String {
    let side = match side {
        Side::White => "white",
        Side::Black => "black",
    };
    let kind = match kind {
        PieceKind::Pawn => "pawn",
        PieceKind::Knight => "knight",
        PieceKind::Bishop => "bishop",
        PieceKind::Rook => "rook",
        PieceKind::Queen => "queen",
        PieceKind::King => "king",
    };
    format!("{side}_{kind}")
}

/// Where a piece's feet go on a tile.
fn piece_spot(state: &GameState, sq: Sq) -> Vec3 {
    let top = tile_top(sq, state.game.terrain.height(sq));
    Vec3::new(top.x, top.y - 3.0, depth_z(sq, layer::ACTOR + 0.05))
}

fn spawn_pieces(
    mut commands: Commands,
    mut state: ResMut<GameState>,
    atlas: Res<Atlas>,
    old: Query<Entity, Or<(With<PieceSprite>, With<Overlay>)>>,
) {
    if !state.pieces_dirty {
        return;
    }
    state.pieces_dirty = false;
    for e in &old {
        commands.entity(e).despawn();
    }
    let animate = state.animate.take();

    for (sq, piece) in state.game.pos.pieces() {
        let (mut sprite, anchor) = atlas.sprite(&piece_sprite_name(piece.kind, piece.side));
        // Characters face right; the undead court faces the other way.
        sprite.flip_x = piece.side == Side::Black;
        let spot = piece_spot(&state, sq);
        let mut e = commands.spawn((PieceSprite, sprite, anchor, Transform::from_translation(spot)));
        if let Some(mv) = animate.filter(|m| m.to == sq) {
            let from = piece_spot(&state, mv.from);
            // Draw the moving piece above everything it passes over.
            let top_z = spot.z.max(from.z) + 0.01;
            let height = match (piece.kind, mv.kind) {
                (_, MoveKind::Cave) => 0.0,
                (PieceKind::Knight, _) => 26.0,
                _ => 8.0 + (from.y - spot.y).abs().min(LIFT) * 0.5,
            };
            e.insert((
                Transform::from_translation(from.with_z(top_z)),
                Hop { from: from.with_z(top_z), to: spot, t: 0.0, height },
            ));
        }
        if let Some(MoveKind::Castle { rook_to, rook_from }) = animate.map(|m| m.kind)
            && sq == rook_to
        {
            let from = piece_spot(&state, rook_from);
            e.insert((Transform::from_translation(from), Hop { from, to: spot, t: 0.0, height: 6.0 }));
        }
    }

    spawn_overlays(&mut commands, &state, &atlas);
}

fn spawn_overlays(commands: &mut Commands, state: &GameState, atlas: &Atlas) {
    let t = &state.game.terrain;
    let tint = |commands: &mut Commands, sq: Sq, color: Color| {
        let (mut sprite, anchor) = atlas.sprite("iso_fill");
        sprite.color = color;
        let p = tile_top(sq, t.height(sq)).extend(depth_z(sq, layer::TINT));
        commands.spawn((Overlay, sprite, anchor, Transform::from_translation(p)));
    };
    // Markers are flattened to lie on the diamond.
    let mark = |commands: &mut Commands, sq: Sq, name: &str, size: Vec2, color: Color| {
        let (mut sprite, anchor) = atlas.sprite(name);
        sprite.custom_size = Some(size);
        sprite.color = color;
        let p = tile_top(sq, t.height(sq)).extend(MARKER_Z - (sq.x + sq.y) as f32 * 0.01);
        commands.spawn((Overlay, sprite, anchor, Transform::from_translation(p)));
    };

    if let Some(last) = state.game.moves.last() {
        for sq in [last.from, last.to] {
            tint(commands, sq, Color::srgba(1.0, 0.9, 0.35, 0.28));
        }
    }
    if state.game.in_check()
        && let Some(k) = state.game.pos.king(state.game.pos.side_to_move)
    {
        tint(commands, k, Color::srgba(0.9, 0.1, 0.15, 0.45));
    }
    let Some(sel) = state.selected else { return };
    mark(commands, sel, "ov_select", Vec2::new(40.0, 22.0), Color::WHITE);
    let moves = state.selected_moves();
    for mv in &moves {
        let capture = state.game.pos.get(mv.to).is_some() || mv.kind == MoveKind::EnPassant;
        if capture {
            mark(commands, mv.to, "ov_capture", Vec2::new(40.0, 22.0), Color::WHITE);
        } else if mv.kind == MoveKind::Cave {
            mark(commands, mv.to, "ov_ring", Vec2::new(26.0, 14.0), Color::WHITE);
        } else {
            mark(commands, mv.to, "ov_dot", Vec2::new(10.0, 6.0), Color::srgba(1.0, 1.0, 1.0, 0.85));
        }
    }
    // Neighbouring squares the piece could reach on flat ground but a cliff blocks, and
    // enemies in reach that stand too high to capture.
    if let Some(piece) = state.game.pos.get(sel) {
        let ctx = Ctx { terrain: t, rules: &state.game.rules };
        let mut blocked = cliff_blocked(&ctx, sel, piece);
        ctx.for_each_attack(&state.game.pos, sel, piece, &mut |sq| {
            if state.game.pos.get(sq).is_some_and(|p| p.side != piece.side) && t.height(sq) > t.height(sel) {
                blocked.push(sq);
            }
            false
        });
        for sq in blocked {
            if !moves.iter().any(|m| m.to == sq) {
                mark(commands, sq, "ov_blocked", Vec2::new(16.0, 12.0), Color::srgba(1.0, 1.0, 1.0, 0.7));
            }
        }
    }
}

/// First-step squares that are only unreachable because of height.
fn cliff_blocked(ctx: &Ctx, from: Sq, piece: tc_core::Piece) -> Vec<Sq> {
    use tc_core::board::{DIAG, KING, KNIGHT, ORTHO};
    let t = ctx.terrain;
    let prof = ctx.rules.profile(piece);
    let size = t.size;
    let h = t.height(from) as i16;
    let (dirs, limit): (&[(i8, i8)], i16) = match piece.kind {
        PieceKind::Knight => (&KNIGHT, prof.jump_max_dh as i16),
        PieceKind::Bishop => (&DIAG, prof.max_climb as i16),
        PieceKind::Rook => (&ORTHO, prof.max_climb as i16),
        PieceKind::Pawn => (
            match piece.side {
                Side::White => &[(0, 1), (1, 1), (-1, 1)],
                Side::Black => &[(0, -1), (1, -1), (-1, -1)],
            },
            prof.max_climb as i16,
        ),
        _ => (&KING, prof.max_climb as i16),
    };
    dirs.iter()
        .filter_map(|&(dx, dy)| from.offset(dx, dy, size))
        .filter(|&sq| {
            let dh = t.height(sq) as i16 - h;
            let too_far = if piece.kind == PieceKind::Knight { dh.abs() > limit } else { dh > limit };
            too_far && !t.get(sq).is_blocked()
        })
        .collect()
}

fn animate_water(time: Res<Time>, atlas: Res<Atlas>, mut q: Query<(&AnimatedWater, &mut Sprite)>) {
    let frame = (time.elapsed_secs() / 0.7) as usize % 2;
    for (water, mut sprite) in &mut q {
        let rect = atlas.sprite(water.frames[frame]).0.rect;
        if sprite.rect != rect {
            sprite.rect = rect;
        }
    }
}

fn animate_hops(mut commands: Commands, time: Res<Time>, mut q: Query<(Entity, &mut Hop, &mut Transform)>) {
    for (e, mut hop, mut tf) in &mut q {
        hop.t = (hop.t + time.delta_secs() / HOP_SECS).min(1.0);
        let k = hop.t;
        let eased = k * k * (3.0 - 2.0 * k);
        let mut p = hop.from.lerp(hop.to, eased);
        p.y += hop.height * 4.0 * k * (1.0 - k);
        p.z = if k >= 1.0 { hop.to.z } else { hop.from.z };
        tf.translation = p;
        if k >= 1.0 {
            // The piece may have been rebuilt by a move this same frame.
            commands.entity(e).try_remove::<Hop>();
        }
    }
}

/// Height badges stay on while set (toolbar / T); holding Alt shows them briefly.
#[derive(Resource, Default)]
pub struct ShowHeights(pub bool);

fn toggle_height_badges(
    keys: Res<ButtonInput<KeyCode>>,
    pinned: Res<ShowHeights>,
    mut q: Query<&mut Visibility, With<HeightBadge>>,
) {
    let show = pinned.0 || keys.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]);
    let want = if show { Visibility::Inherited } else { Visibility::Hidden };
    for mut v in &mut q {
        if *v != want {
            *v = want;
        }
    }
}

pub struct BoardViewPlugin;

impl Plugin for BoardViewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShowHeights>().add_systems(
            Update,
            ((spawn_terrain, spawn_pieces).chain(), animate_water, animate_hops, toggle_height_badges),
        );
    }
}
