//! Draws the board in a top-down 3/4 view (PLAN.md §8): each height level lifts a
//! tile by 16 px, and a cliff face is drawn below any tile whose south neighbour is
//! lower. Rows are layered back to front so cliffs hide what stands behind them.

use bevy::prelude::*;
use bevy::sprite::Anchor;
use tc_core::movegen::Ctx;
use tc_core::terrain::TileKind;
use tc_core::{Feature, MoveKind, Obstacle, PieceKind, Side, Sq};

use crate::atlas::Atlas;
use crate::game::GameState;

pub const TILE: f32 = 32.0;
pub const LIFT: f32 = 16.0;

/// Depth layers within one board row.
mod layer {
    pub const GROUND: f32 = 0.0;
    pub const FACE: f32 = 0.1;
    pub const DECAL: f32 = 0.2;
    pub const TINT: f32 = 0.3;
    pub const ACTOR: f32 = 0.6;
}

/// Move markers and height badges sit above every row, so a cliff or a tall piece in
/// front never hides where you can move.
const MARKER_Z: f32 = 100.0;

/// Screen position of the centre of a tile's top surface.
pub fn tile_top(sq: Sq, height: u8) -> Vec2 {
    Vec2::new(sq.x as f32 * TILE, sq.y as f32 * TILE + height as f32 * LIFT)
}

/// Rows further north are drawn first; everything in a row shares its depth band.
pub fn row_z(y: u8, layer: f32) -> f32 {
    -(y as f32) + layer
}

/// Tile whose top surface or cliff face is under `p`, front-most first.
pub fn pick_tile(state: &GameState, p: Vec2) -> Option<Sq> {
    let t = &state.game.terrain;
    for y in 0..state.size {
        for x in 0..state.size {
            let sq = Sq::new(x, y);
            let top = tile_top(sq, t.height(sq));
            let below = face_depth(state, sq) as f32 * LIFT;
            let inside_x = (p.x - top.x).abs() <= TILE / 2.0;
            if inside_x && p.y <= top.y + TILE / 2.0 && p.y >= top.y - TILE / 2.0 - below {
                return Some(sq);
            }
        }
    }
    None
}

/// Height levels of cliff face shown below a tile (the front row stands on a plinth).
fn face_depth(state: &GameState, sq: Sq) -> u8 {
    let t = &state.game.terrain;
    let h = t.height(sq);
    match sq.offset(0, -1, state.size) {
        Some(s) => h.saturating_sub(t.height(s)),
        None => h + 1,
    }
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
            TileKind::ShallowWater => ("shallow_0", Some(["shallow_0", "shallow_1"])),
            TileKind::DeepWater => ("deep", None),
            TileKind::Void => ("void", None),
        };
        let checker = if (sq.x + sq.y) % 2 == 0 { 0.86 } else { 1.0 };
        let shade = checker * (0.88 + 0.06 * tile.height as f32);
        let (mut sprite, anchor) = atlas.sprite(name);
        sprite.color = Color::srgb(shade, shade, shade);
        let mut ground = commands.spawn((
            TerrainPart,
            sprite,
            anchor,
            Transform::from_translation(top.extend(row_z(sq.y, layer::GROUND))),
        ));
        if let Some(frames) = water {
            ground.insert(AnimatedWater { frames });
        }

        // Cliff face below the tile.
        let depth = face_depth(&state, sq);
        if depth > 0 {
            let cave = matches!(tile.feature, Feature::Cave(_));
            let name = match tile.feature {
                Feature::Cave(link) => ["cave_door_0", "cave_door_1"][link as usize % 2],
                _ => ["cliff_0", "cliff_1", "cliff_2", "cliff_3", "cliff_4"][hash(sq, 3) as usize % 5],
            };
            let (mut sprite, _) = atlas.sprite(name);
            sprite.custom_size = Some(Vec2::new(TILE, depth as f32 * LIFT));
            if cave && depth < 2 {
                // A door squashed into a one-level ledge reads badly; use plain cliff.
                sprite = atlas.sprite("cliff_0").0;
                sprite.custom_size = Some(Vec2::new(TILE, depth as f32 * LIFT));
            }
            let shade = 0.8 + 0.07 * tile.height as f32;
            sprite.color = Color::srgb(shade, shade, shade);
            commands.spawn((
                TerrainPart,
                sprite,
                Anchor::TOP_CENTER,
                Transform::from_translation(Vec3::new(top.x, top.y - TILE / 2.0, row_z(sq.y, layer::FACE))),
            ));
        }

        match tile.feature {
            Feature::Cave(link) => {
                let (mut sprite, anchor) = atlas.sprite("cave_rune_0");
                sprite.color = cave_color(link);
                sprite.custom_size = Some(Vec2::new(26.0, 18.0));
                commands.spawn((
                    TerrainPart,
                    sprite,
                    anchor,
                    Transform::from_translation(top.extend(row_z(sq.y, layer::DECAL))),
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
                    Transform::from_translation(Vec3::new(top.x, top.y - 10.0, row_z(sq.y, layer::ACTOR))),
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
            Transform::from_translation(Vec3::new(top.x + 9.0, top.y + 8.0, MARKER_Z + 1.0))
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
    Vec3::new(top.x, top.y - 9.0, row_z(sq.y, layer::ACTOR + 0.05))
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
                _ => 8.0 + (from.y - spot.y).abs().min(16.0) * 0.5,
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
    let at = |sq: Sq, layer: f32| tile_top(sq, t.height(sq)).extend(row_z(sq.y, layer));
    let tint = |commands: &mut Commands, sq: Sq, color: Color| {
        commands.spawn((
            Overlay,
            Sprite::from_color(color, Vec2::splat(TILE)),
            Transform::from_translation(at(sq, layer::TINT)),
        ));
    };
    let mark = |commands: &mut Commands, sq: Sq, name: &str, size: f32, color: Color| {
        let (mut sprite, anchor) = atlas.sprite(name);
        sprite.custom_size = Some(Vec2::splat(size));
        sprite.color = color;
        let p = tile_top(sq, t.height(sq)).extend(MARKER_Z - sq.y as f32 * 0.01);
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
    mark(commands, sel, "ov_select", TILE, Color::WHITE);
    let moves = state.selected_moves();
    for mv in &moves {
        let capture = state.game.pos.get(mv.to).is_some() || mv.kind == MoveKind::EnPassant;
        if capture {
            mark(commands, mv.to, "ov_capture", TILE, Color::WHITE);
        } else if mv.kind == MoveKind::Cave {
            mark(commands, mv.to, "ov_ring", 22.0, Color::WHITE);
        } else {
            mark(commands, mv.to, "ov_dot", 10.0, Color::srgba(1.0, 1.0, 1.0, 0.85));
        }
    }
    // Neighbouring squares the piece could reach on flat ground but a cliff blocks.
    if let Some(piece) = state.game.pos.get(sel) {
        let ctx = Ctx { terrain: t, rules: &state.game.rules };
        for sq in cliff_blocked(&ctx, sel, piece) {
            if !moves.iter().any(|m| m.to == sq) {
                mark(commands, sq, "ov_blocked", 16.0, Color::srgba(1.0, 1.0, 1.0, 0.7));
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

fn toggle_height_badges(keys: Res<ButtonInput<KeyCode>>, mut q: Query<&mut Visibility, With<HeightBadge>>) {
    let show = keys.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]);
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
        app.add_systems(
            Update,
            ((spawn_terrain, spawn_pieces).chain(), animate_water, animate_hops, toggle_height_badges),
        );
    }
}
