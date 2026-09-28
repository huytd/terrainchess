//! Draws the board in 3D (PLAN.md §8). Every square is a column block: its top is the
//! square's ground tile and its sides are cliff art, raised `LEVEL` per height level.
//! Pieces and props are upright pixel-art cards that turn to face the camera, so the
//! board can be orbited while everything keeps the sprite look. All materials are
//! unlit; faces are shaded by direction as if lit from the south-east.
//!
//! World axes: +X is east (files), -Z is north (ranks), +Y is up. One square is one unit.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, MeshVertexBufferLayoutRef, PrimitiveTopology};
use bevy::pbr::{ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, CompareFunction, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use std::collections::HashMap;
use tc_core::movegen::Ctx;
use tc_core::terrain::TileKind;
use tc_core::{Feature, MoveKind, Obstacle, PieceKind, Side, Sq};

use crate::atlas::{Atlas, Uv};
use crate::game::GameState;
use crate::input::MainCamera;

/// World height of one terrain level.
pub const LEVEL: f32 = 0.4;
/// Column height below level 0, so the board reads as a slab.
const BASE: f32 = 0.3;
/// World size of one sprite pixel (ground tiles are 32 px across a square).
const PX: f32 = 1.0 / 30.0;
/// Makes a material skip the depth test, so move markers stay visible behind pieces and
/// taller columns. (A second camera layered over the scene did the same, but some WebGL2
/// drivers lost the scene underneath.)
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, Default)]
pub struct OnTop {}

impl MaterialExtension for OnTop {
    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if let Some(depth) = descriptor.depth_stencil.as_mut() {
            depth.depth_compare = Some(CompareFunction::Always);
            depth.depth_write_enabled = Some(false);
        }
        Ok(())
    }
}

type MarkerMaterial = ExtendedMaterial<StandardMaterial, OnTop>;
/// Overlays float this far above a top face to avoid z-fighting.
const LIFT_TINT: f32 = 0.004;
const LIFT_MARK: f32 = 0.008;

/// Height of a square's top face.
pub fn top_y(height: u8) -> f32 {
    height as f32 * LEVEL
}

/// Centre of a square's top face.
pub fn square_top(sq: Sq, height: u8) -> Vec3 {
    Vec3::new(sq.x as f32, top_y(height), -(sq.y as f32))
}

/// Middle of the board at ground level.
pub fn board_center(size: u8) -> Vec3 {
    let c = (size as f32 - 1.0) / 2.0;
    Vec3::new(c, LEVEL, -c)
}

/// Square whose column the ray hits first.
pub fn pick_square(state: &GameState, ray: Ray3d) -> Option<Sq> {
    let t = &state.game.terrain;
    let o = ray.origin;
    let d = *ray.direction;
    let mut best: Option<(f32, Sq)> = None;
    for sq in tc_core::board::squares(state.size) {
        let c = square_top(sq, t.height(sq));
        let lo = Vec3::new(c.x - 0.5, -BASE, c.z - 0.5);
        let hi = Vec3::new(c.x + 0.5, c.y, c.z + 0.5);
        // Slab test against the column's box.
        let (mut t0, mut t1) = (f32::NEG_INFINITY, f32::INFINITY);
        for i in 0..3 {
            if d[i].abs() < 1e-6 {
                if o[i] < lo[i] || o[i] > hi[i] {
                    t1 = f32::NEG_INFINITY;
                }
                continue;
            }
            let (a, b) = ((lo[i] - o[i]) / d[i], (hi[i] - o[i]) / d[i]);
            t0 = t0.max(a.min(b));
            t1 = t1.min(a.max(b));
        }
        if t0 <= t1 && t1 >= 0.0 && best.is_none_or(|(bt, _)| t0 < bt) {
            best = Some((t0, sq));
        }
    }
    best.map(|(_, sq)| sq)
}

/// Square of the piece whose sprite is under screen position `cursor`, nearest first.
pub fn pick_piece(
    state: &GameState,
    atlas: &Atlas,
    camera: (&Camera, &GlobalTransform),
    cursor: Vec2,
) -> Option<Sq> {
    let (cam, gtf) = camera;
    let forward = gtf.forward();
    // Cards are stretched by 1 / cos(tilt) so they keep their full height on screen.
    let stretch = 1.0 / Vec2::new(forward.x, forward.z).length().max(0.1);
    state
        .game
        .pos
        .pieces()
        .filter_map(|(sq, piece)| {
            let feet = piece_spot(state, sq);
            let size = atlas.px(&piece_sprite_name(piece.kind, piece.side)) * PX;
            let p = cam.world_to_viewport(gtf, feet).ok()?;
            let head = cam.world_to_viewport(gtf, feet + Vec3::Y * size.y * stretch).ok()?;
            // The sides of a sprite are mostly transparent, so only the middle counts.
            let side = cam.world_to_viewport(gtf, feet + gtf.right() * size.x * 0.35).ok()?;
            let hit = (cursor.x - p.x).abs() <= (side.x - p.x).abs() && cursor.y <= p.y && cursor.y >= head.y;
            hit.then(|| (sq, forward.dot(feet)))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(sq, _)| sq)
}

#[derive(Component)]
struct TerrainPart;

#[derive(Component)]
struct PieceSprite;

#[derive(Component)]
struct Overlay;

/// One of the two shallow-water frames; the other is hidden.
#[derive(Component)]
struct WaterFrame(usize);

/// Upright card that turns about the vertical axis to face the camera.
#[derive(Component)]
struct Billboard;

/// Hop from one spot to another along a small arc.
#[derive(Component)]
struct Hop {
    from: Vec3,
    to: Vec3,
    t: f32,
    height: f32,
}

const HOP_SECS: f32 = 0.22;

/// Shared materials and card meshes.
#[derive(Resource)]
struct Look {
    terrain: Handle<StandardMaterial>,
    cards: Handle<StandardMaterial>,
    /// Card meshes by sprite name and horizontal flip.
    card_meshes: HashMap<(String, bool), Handle<Mesh>>,
}

fn setup_look(mut commands: Commands, atlas: Res<Atlas>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let terrain = materials.add(StandardMaterial {
        base_color_texture: Some(atlas.image.clone()),
        unlit: true,
        cull_mode: None,
        ..default()
    });
    let cards = materials.add(StandardMaterial {
        base_color_texture: Some(atlas.image.clone()),
        unlit: true,
        cull_mode: None,
        alpha_mode: AlphaMode::Mask(0.5),
        ..default()
    });
    commands.insert_resource(Look { terrain, cards, card_meshes: HashMap::new() });
}

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

/// Collects textured, vertex-shaded quads into one mesh.
#[derive(Default)]
struct Quads {
    pos: Vec<[f32; 3]>,
    normal: Vec<[f32; 3]>,
    uv: Vec<[f32; 2]>,
    color: Vec<[f32; 4]>,
    idx: Vec<u32>,
}

impl Quads {
    /// Corners in order bottom-left, bottom-right, top-right, top-left as seen from
    /// outside, with texture coordinates to match. `shade` darkens the texture.
    fn add(&mut self, corners: [Vec3; 4], uv: [[f32; 2]; 4], shade: f32) {
        let n = (corners[1] - corners[0]).cross(corners[3] - corners[0]).normalize_or_zero();
        let c = Color::srgb(shade, shade, shade).to_linear().to_f32_array();
        let base = self.pos.len() as u32;
        for (p, t) in corners.into_iter().zip(uv) {
            self.pos.push(p.to_array());
            self.normal.push(n.to_array());
            self.uv.push(t);
            self.color.push(c);
        }
        self.idx.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    fn mesh(self) -> Mesh {
        Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.pos)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normal)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uv)
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.color)
            .with_inserted_indices(Indices::U32(self.idx))
    }
}

/// Texture corners for a whole sprite, matching `Quads::add` corner order.
fn full_uv(uv: Uv) -> [[f32; 2]; 4] {
    [uv.at(0.0, 1.0), uv.at(1.0, 1.0), uv.at(1.0, 0.0), uv.at(0.0, 0.0)]
}

/// A flat quad lying on a top face, centred at `c`.
fn flat(c: Vec3, size: Vec2) -> [Vec3; 4] {
    let (hx, hz) = (size.x / 2.0, size.y / 2.0);
    [
        c + Vec3::new(-hx, 0.0, hz),
        c + Vec3::new(hx, 0.0, hz),
        c + Vec3::new(hx, 0.0, -hz),
        c + Vec3::new(-hx, 0.0, -hz),
    ]
}

/// Walls of one column side from `lo` up to `hi`: grass lip under the top edge, stone
/// below. `a` → `b` runs left to right along the bottom as seen from outside.
fn wall(q: &mut Quads, atlas: &Atlas, a: Vec3, b: Vec3, lo: f32, hi: f32, shade: f32) {
    let mut top = hi;
    let mut first = true;
    while top > lo + 1e-4 {
        let bottom = (top - LEVEL).max(lo);
        let uv = atlas.uv(if first { "wall_lip" } else { "wall_stone" });
        let frac = (top - bottom) / LEVEL;
        let at = |p: Vec3, y: f32| Vec3::new(p.x, y, p.z);
        q.add(
            [at(a, bottom), at(b, bottom), at(b, top), at(a, top)],
            [uv.at(0.0, frac), uv.at(1.0, frac), uv.at(1.0, 0.0), uv.at(0.0, 0.0)],
            shade,
        );
        top = bottom;
        first = false;
    }
}

fn spawn_terrain(
    mut commands: Commands,
    mut state: ResMut<GameState>,
    atlas: Res<Atlas>,
    mut look: ResMut<Look>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
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
    let mut solid = Quads::default();
    let mut water = [Quads::default(), Quads::default()];
    for sq in tc_core::board::squares(size) {
        let tile = *t.get(sq);
        let top = square_top(sq, tile.height);

        // Top face, with a light checker and brighter high ground.
        let name = match tile.kind {
            TileKind::Grass => ["grass_0", "grass_1", "grass_0", "grass_dark"][hash(sq, 1) as usize % 4],
            TileKind::Stone => ["cobble", "flagstone"][hash(sq, 2) as usize % 2],
            TileKind::Sand => "sand",
            TileKind::ShallowWater => "shallow_0",
            TileKind::DeepWater => "deep",
            TileKind::Void => "void",
        };
        let checker = if (sq.x + sq.y) % 2 == 0 { 0.88 } else { 1.0 };
        let shade = checker * (0.86 + 0.06 * tile.height as f32).min(1.05);
        let corners = flat(top, Vec2::ONE);
        if tile.kind == TileKind::ShallowWater {
            for (i, frame) in ["shallow_0", "shallow_1"].into_iter().enumerate() {
                water[i].add(corners, full_uv(atlas.uv(frame)), shade);
            }
        } else {
            solid.add(corners, full_uv(atlas.uv(name)), shade);
        }

        // Sides, only where they show: above a lower neighbour or at the board edge.
        let (x, z) = (top.x, top.z);
        for ((dx, dy), a, b, shade) in [
            ((0i8, -1i8), Vec3::new(x - 0.5, 0.0, z + 0.5), Vec3::new(x + 0.5, 0.0, z + 0.5), 0.84),
            ((1, 0), Vec3::new(x + 0.5, 0.0, z + 0.5), Vec3::new(x + 0.5, 0.0, z - 0.5), 0.72),
            ((0, 1), Vec3::new(x + 0.5, 0.0, z - 0.5), Vec3::new(x - 0.5, 0.0, z - 0.5), 0.56),
            ((-1, 0), Vec3::new(x - 0.5, 0.0, z - 0.5), Vec3::new(x - 0.5, 0.0, z + 0.5), 0.62),
        ] {
            let lo = match sq.offset(dx, dy, size) {
                Some(n) => top_y(t.height(n)),
                None => -BASE,
            };
            wall(&mut solid, &atlas, a, b, lo, top.y, shade + 0.03 * tile.height as f32);
        }

        match tile.feature {
            Feature::Cave(link) => {
                let rune = materials.add(StandardMaterial {
                    base_color: cave_color(link),
                    base_color_texture: Some(atlas.image.clone()),
                    unlit: true,
                    alpha_mode: AlphaMode::Mask(0.5),
                    ..default()
                });
                let mut q = Quads::default();
                q.add(
                    flat(top + Vec3::Y * LIFT_TINT, Vec2::new(0.8, 0.5)),
                    full_uv(atlas.uv("cave_rune_0")),
                    1.0,
                );
                commands.spawn((TerrainPart, Mesh3d(meshes.add(q.mesh())), MeshMaterial3d(rune)));
            }
            Feature::Obstacle(kind) => {
                let name = match kind {
                    Obstacle::Rock => ["rock", "rock_mossy"][hash(sq, 4) as usize % 2],
                    Obstacle::Tree => ["pine", "pine", "dead_tree"][hash(sq, 5) as usize % 3],
                };
                let mesh = card_mesh(&mut look, &mut meshes, &atlas, name, false);
                commands.spawn((
                    TerrainPart,
                    Billboard,
                    Mesh3d(mesh),
                    MeshMaterial3d(look.cards.clone()),
                    Transform::from_translation(top),
                ));
            }
            _ => {}
        }
    }
    let material = MeshMaterial3d(look.terrain.clone());
    commands.spawn((TerrainPart, Mesh3d(meshes.add(solid.mesh())), material.clone()));
    for (i, q) in water.into_iter().enumerate() {
        commands.spawn((TerrainPart, WaterFrame(i), Mesh3d(meshes.add(q.mesh())), material.clone()));
    }
}

/// An upright card for a sprite, feet at the origin, facing +Z.
fn card_mesh(
    look: &mut Look,
    meshes: &mut Assets<Mesh>,
    atlas: &Atlas,
    name: &str,
    flip: bool,
) -> Handle<Mesh> {
    look.card_meshes
        .entry((name.to_string(), flip))
        .or_insert_with(|| {
            let size = atlas.px(name) * PX;
            let hw = size.x / 2.0;
            let mut uv = full_uv(atlas.uv(name));
            if flip {
                uv = [uv[1], uv[0], uv[3], uv[2]];
            }
            let mut q = Quads::default();
            q.add(
                [
                    Vec3::new(-hw, 0.0, 0.0),
                    Vec3::new(hw, 0.0, 0.0),
                    Vec3::new(hw, size.y, 0.0),
                    Vec3::new(-hw, size.y, 0.0),
                ],
                uv,
                1.0,
            );
            meshes.add(q.mesh())
        })
        .clone()
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

/// Where a piece's feet go on a square.
fn piece_spot(state: &GameState, sq: Sq) -> Vec3 {
    square_top(sq, state.game.terrain.height(sq))
}

#[allow(clippy::too_many_arguments)] // a Bevy system: one parameter per resource
fn spawn_pieces(
    mut commands: Commands,
    mut state: ResMut<GameState>,
    atlas: Res<Atlas>,
    mut look: ResMut<Look>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut markers: ResMut<Assets<MarkerMaterial>>,
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
        // Characters face right; the undead court faces the other way.
        let name = piece_sprite_name(piece.kind, piece.side);
        let mesh = card_mesh(&mut look, &mut meshes, &atlas, &name, piece.side == Side::Black);
        let spot = piece_spot(&state, sq);
        let mut e = commands.spawn((
            PieceSprite,
            Billboard,
            Mesh3d(mesh),
            MeshMaterial3d(look.cards.clone()),
            Transform::from_translation(spot),
        ));
        if let Some(mv) = animate.filter(|m| m.to == sq) {
            let from = piece_spot(&state, mv.from);
            let height = match (piece.kind, mv.kind) {
                (_, MoveKind::Cave) => 0.0,
                (PieceKind::Knight, _) => 0.8,
                _ => 0.25 + (from.y - spot.y).abs().min(LEVEL) * 0.5,
            };
            e.insert((Transform::from_translation(from), Hop { from, to: spot, t: 0.0, height }));
        }
        if let Some(MoveKind::Castle { rook_to, rook_from }) = animate.map(|m| m.kind)
            && sq == rook_to
        {
            let from = piece_spot(&state, rook_from);
            e.insert((Transform::from_translation(from), Hop { from, to: spot, t: 0.0, height: 0.2 }));
        }
    }

    spawn_overlays(&mut commands, &state, &atlas, &mut meshes, &mut materials, &mut markers);
}

/// Flat highlights and markers lying on top faces.
struct OverlayPainter<'a, 'w, 's> {
    commands: &'a mut Commands<'w, 's>,
    state: &'a GameState,
    atlas: &'a Atlas,
    meshes: &'a mut Assets<Mesh>,
    materials: &'a mut Assets<StandardMaterial>,
    markers: &'a mut Assets<MarkerMaterial>,
}

impl OverlayPainter<'_, '_, '_> {
    fn quad(&mut self, sq: Sq, sprite: Option<&str>, size: f32, color: Color, lift: f32) {
        let base = StandardMaterial {
            base_color: color,
            base_color_texture: sprite.map(|_| self.atlas.image.clone()),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        };
        let uv = sprite.map(|s| full_uv(self.atlas.uv(s))).unwrap_or([[0.0; 2]; 4]);
        let top = square_top(sq, self.state.game.terrain.height(sq)) + Vec3::Y * lift;
        let mut q = Quads::default();
        q.add(flat(top, Vec2::splat(size)), uv, 1.0);
        let mut e = self.commands.spawn((Overlay, Mesh3d(self.meshes.add(q.mesh()))));
        if sprite.is_some() {
            // Markers draw over everything so a column or piece in front never hides
            // where you can move; tints stay on the ground.
            e.insert(MeshMaterial3d(self.markers.add(MarkerMaterial { base, extension: OnTop {} })));
        } else {
            e.insert(MeshMaterial3d(self.materials.add(base)));
        }
    }

    /// Colour wash over a whole square.
    fn tint(&mut self, sq: Sq, color: Color) {
        self.quad(sq, None, 1.0, color, LIFT_TINT);
    }

    fn mark(&mut self, sq: Sq, name: &str, size: f32, color: Color) {
        self.quad(sq, Some(name), size, color, LIFT_MARK);
    }
}

fn spawn_overlays(
    commands: &mut Commands,
    state: &GameState,
    atlas: &Atlas,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    markers: &mut Assets<MarkerMaterial>,
) {
    let mut paint = OverlayPainter { commands, state, atlas, meshes, materials, markers };
    let t = &state.game.terrain;
    if let Some(last) = state.game.moves.last() {
        for sq in [last.from, last.to] {
            paint.tint(sq, Color::srgba(1.0, 0.9, 0.35, 0.3));
        }
    }
    if state.game.in_check()
        && let Some(k) = state.game.pos.king(state.game.pos.side_to_move)
    {
        paint.tint(k, Color::srgba(0.9, 0.1, 0.15, 0.5));
    }
    let Some(sel) = state.selected else { return };
    paint.mark(sel, "ov_select", 0.95, Color::WHITE);
    let moves = state.selected_moves();
    for mv in &moves {
        let capture = state.game.pos.get(mv.to).is_some() || mv.kind == MoveKind::EnPassant;
        if capture {
            paint.mark(mv.to, "ov_capture", 0.95, Color::WHITE);
        } else if mv.kind == MoveKind::Cave {
            paint.mark(mv.to, "ov_ring", 0.7, Color::WHITE);
        } else {
            paint.mark(mv.to, "ov_dot", 0.3, Color::srgba(1.0, 1.0, 1.0, 0.9));
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
                paint.mark(sq, "ov_blocked", 0.5, Color::srgba(1.0, 1.0, 1.0, 0.8));
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

fn animate_water(time: Res<Time>, mut q: Query<(&WaterFrame, &mut Visibility)>) {
    let frame = (time.elapsed_secs() / 0.7) as usize % 2;
    for (water, mut v) in &mut q {
        let want = if water.0 == frame { Visibility::Inherited } else { Visibility::Hidden };
        if *v != want {
            *v = want;
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
        tf.translation = p;
        if k >= 1.0 {
            // The piece may have been rebuilt by a move this same frame.
            commands.entity(e).try_remove::<Hop>();
        }
    }
}

/// Turn cards to face the camera, and stretch them so the tilt doesn't squash them.
fn face_camera(
    camera: Single<&Transform, (With<MainCamera>, Without<Billboard>)>,
    mut cards: Query<&mut Transform, With<Billboard>>,
) {
    let back = camera.back();
    let yaw = back.x.atan2(back.z);
    let pitch = back.y.clamp(-0.99, 0.99).asin();
    let rotation = Quat::from_rotation_y(yaw);
    let scale = Vec3::new(1.0, 1.0 / pitch.cos(), 1.0);
    for mut tf in &mut cards {
        tf.rotation = rotation;
        tf.scale = scale;
    }
}

pub struct BoardViewPlugin;

impl Plugin for BoardViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<MarkerMaterial>::default())
            .add_systems(Startup, setup_look)
            .add_systems(
                Update,
                ((spawn_terrain, spawn_pieces).chain(), animate_water, animate_hops, face_camera),
            );
    }
}
