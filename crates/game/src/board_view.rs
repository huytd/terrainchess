//! Draws the board in 3D (specs/game-design.md §6). Every square is a column block: its top is the
//! square's ground tile and its sides are cliff art, raised `LEVEL` per height level.
//! Pieces and props are upright pixel-art cards that turn to face the camera, so the
//! board can be orbited while everything keeps the sprite look. All materials are
//! unlit; faces are shaded by direction as if lit from the south-east.
//!
//! World axes: +X is east (files), -Z is north (ranks), +Y is up. One square is one unit.

use bevy::asset::{RenderAssetUsages, load_internal_asset, uuid_handle};
use bevy::mesh::{Indices, MeshVertexBufferLayoutRef, PrimitiveTopology};
use bevy::pbr::{ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, CompareFunction, DepthBiasState, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;
use std::collections::{HashMap, HashSet};
use tc_core::movegen::Ctx;
use tc_core::terrain::TileKind;
use tc_core::{Feature, MoveKind, Obstacle, PieceKind, Side, Sq};

use crate::atlas::{Atlas, Uv};
use crate::game::GameState;
use crate::input::{ArrowsEnabled, MainCamera};
use crate::run::{Run, RunPhase, TitleMenu};

/// World height of one terrain level.
pub const LEVEL: f32 = 0.4;
/// Column height below level 0, so the board reads as a slab.
pub(crate) const BASE: f32 = 0.3;
/// World size of one sprite pixel (ground tiles are 32 px across a square).
pub(crate) const PX: f32 = 1.0 / 30.0;
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
            depth.bias = DepthBiasState::default();
        }
        Ok(())
    }
}

type MarkerMaterial = ExtendedMaterial<StandardMaterial, OnTop>;

const OCCLUDED_SHADER_HANDLE: Handle<Shader> = uuid_handle!("b1a9f62c-8821-4f16-953e-2f9e4210d654");

/// Draws a faction-coloured silhouette only where a piece is occluded by nearer geometry.
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, Default)]
pub struct Occluded {}

impl MaterialExtension for Occluded {
    fn fragment_shader() -> ShaderRef {
        OCCLUDED_SHADER_HANDLE.into()
    }

    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if let Some(depth) = descriptor.depth_stencil.as_mut() {
            depth.depth_compare = Some(CompareFunction::Less);
            depth.depth_write_enabled = Some(false);
            depth.bias = DepthBiasState::default();
        }
        Ok(())
    }
}

type OccludedMaterial = ExtendedMaterial<StandardMaterial, Occluded>;

/// Disables depth write while keeping the normal depth test, so piece cards are drawn in
/// the transparent pass, sorted back to front, and never write depth.
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, Default)]
pub struct PieceCard {}

impl MaterialExtension for PieceCard {
    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if let Some(depth) = descriptor.depth_stencil.as_mut() {
            depth.depth_write_enabled = Some(false);
            depth.bias = DepthBiasState::default();
        }
        Ok(())
    }
}

pub type PieceMaterial = ExtendedMaterial<StandardMaterial, PieceCard>;

/// Overlays float this far above a top face to avoid z-fighting.
const LIFT_GRID: f32 = 0.001;
const LIFT_TINT: f32 = 0.007;
const LIFT_SHADOW: f32 = 0.008;
const LIFT_MARK: f32 = 0.014;
const GRID_WIDTH: f32 = 0.035;

/// Height of a square's top face.
pub fn top_y(height: i8) -> f32 {
    height as f32 * LEVEL
}

/// Centre of a square's top face.
pub fn square_top(sq: Sq, height: u8) -> Vec3 {
    Vec3::new(sq.x as f32, top_y(height as i8), -(sq.y as f32))
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

/// Square of the obstacle whose sprite is under screen position `cursor`, nearest first.
pub fn pick_obstacle(
    state: &GameState,
    atlas: &Atlas,
    camera: (&Camera, &GlobalTransform),
    cursor: Vec2,
) -> Option<Sq> {
    let (cam, gtf) = camera;
    let forward = gtf.forward();
    let stretch = 1.0 / Vec2::new(forward.x, forward.z).length().max(0.1);
    let t = &state.game.terrain;
    tc_core::board::squares(state.size)
        .filter_map(|sq| {
            let tile = t.get(sq);
            let kind = match tile.feature {
                Feature::Obstacle(k) => k,
                _ => return None,
            };
            let name = match kind {
                Obstacle::Rock => ["rock", "rock_mossy"][hash(sq.x as i32, sq.y as i32, 4) as usize % 2],
                Obstacle::Tree => {
                    ["pine", "pine", "dead_tree"][hash(sq.x as i32, sq.y as i32, 5) as usize % 3]
                }
            };
            let feet = square_top(sq, tile.height);
            let size = atlas.px(name) * PX;
            let p = cam.world_to_viewport(gtf, feet).ok()?;
            let head = cam.world_to_viewport(gtf, feet + Vec3::Y * size.y * stretch).ok()?;
            let side = cam.world_to_viewport(gtf, feet + gtf.right() * size.x * 0.4).ok()?;
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
struct PieceSilhouette;

/// Small intent badge floating above threatened pieces.
#[derive(Component)]
struct ThreatBadge {
    base_y: f32,
    sprite_h: f32,
}

#[derive(Component)]
struct PickupSprite;

/// Up-and-down oscillation for board pickups (specs/game-design.md §6).
#[derive(Component)]
struct PickupBob {
    base_y: f32,
    phase: f32,
}

#[derive(Component)]
struct Overlay;

/// One of the two shallow-water frames; the other is hidden.
#[derive(Component)]
struct WaterFrame(usize);

/// Upright card that turns about the vertical axis to face the camera.
#[derive(Component)]
pub(crate) struct Billboard;

/// Hop from one spot to another along a small arc.
#[derive(Component)]
struct Hop {
    from: Vec3,
    to: Vec3,
    t: f32,
    height: f32,
}

pub(crate) const HOP_SECS: f32 = 0.22;

/// Shared materials and card meshes.
#[derive(Resource)]
pub(crate) struct Look {
    pub(crate) terrain: Handle<StandardMaterial>,
    pub(crate) cards: Handle<StandardMaterial>,
    pub(crate) piece_cards: Handle<PieceMaterial>,
    pub(crate) grid: Handle<StandardMaterial>,
    pub(crate) shadows: Handle<StandardMaterial>,
    pub(crate) shadow_mesh_small: Handle<Mesh>,
    pub(crate) shadow_mesh_large: Handle<Mesh>,
    /// Card meshes by sprite name and horizontal flip.
    pub(crate) card_meshes: HashMap<(String, bool), Handle<Mesh>>,
    /// Shared ground material for threat victim frame.
    pub(crate) threat_victim: Handle<StandardMaterial>,
    /// Shared ground material for threat attacker frame.
    pub(crate) threat_attacker: Handle<StandardMaterial>,
    /// Shared on-top material for threat intent badge.
    pub(crate) threat_badge: Handle<MarkerMaterial>,
    pub(crate) victim_frame_mesh: Handle<Mesh>,
    pub(crate) attacker_frame_mesh: Handle<Mesh>,
    pub(crate) badge_mesh: Handle<Mesh>,
    pub(crate) occluded_white: Handle<OccludedMaterial>,
    pub(crate) occluded_black: Handle<OccludedMaterial>,
}

/// Creates a 1.0 x 1.0 top-face frame decal consisting of four thin strips.
fn frame_mesh(inset: f32, width: f32) -> Mesh {
    let ro = 0.5 - inset;
    let ri = ro - width;
    let mut q = Quads::default();
    let uv = [[0.0, 0.0]; 4];
    // North strip (-Z)
    q.add(
        [
            Vec3::new(-ro, 0.0, -ri),
            Vec3::new(ro, 0.0, -ri),
            Vec3::new(ro, 0.0, -ro),
            Vec3::new(-ro, 0.0, -ro),
        ],
        uv,
        1.0,
    );
    // South strip (+Z)
    q.add(
        [Vec3::new(-ro, 0.0, ro), Vec3::new(ro, 0.0, ro), Vec3::new(ro, 0.0, ri), Vec3::new(-ro, 0.0, ri)],
        uv,
        1.0,
    );
    // West strip (-X)
    q.add(
        [
            Vec3::new(-ro, 0.0, ri),
            Vec3::new(-ri, 0.0, ri),
            Vec3::new(-ri, 0.0, -ri),
            Vec3::new(-ro, 0.0, -ri),
        ],
        uv,
        1.0,
    );
    // East strip (+X)
    q.add(
        [Vec3::new(ri, 0.0, ri), Vec3::new(ro, 0.0, ri), Vec3::new(ro, 0.0, -ri), Vec3::new(ri, 0.0, -ri)],
        uv,
        1.0,
    );
    q.mesh()
}

fn setup_look(
    mut commands: Commands,
    atlas: Res<Atlas>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut markers: ResMut<Assets<MarkerMaterial>>,
    mut occluded_materials: ResMut<Assets<OccludedMaterial>>,
    mut piece_materials: ResMut<Assets<PieceMaterial>>,
) {
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
    let piece_cards = piece_materials.add(PieceMaterial {
        base: StandardMaterial {
            base_color_texture: Some(atlas.image.clone()),
            unlit: true,
            cull_mode: None,
            alpha_mode: AlphaMode::Blend,
            depth_bias: 100.0,
            ..default()
        },
        extension: PieceCard {},
    });
    let grid = materials.add(StandardMaterial {
        base_color: Color::srgb_u8(0x2A, 0x26, 0x21),
        unlit: true,
        cull_mode: None,
        alpha_mode: AlphaMode::Opaque,
        fog_enabled: true,
        ..default()
    });
    let shadows = materials.add(StandardMaterial {
        base_color_texture: Some(atlas.image.clone()),
        unlit: true,
        cull_mode: None,
        alpha_mode: AlphaMode::Blend,
        fog_enabled: true,
        ..default()
    });
    let mut q_small = Quads::default();
    q_small.add(flat(Vec3::ZERO, Vec2::new(0.55, 0.28)), full_uv(atlas.uv("shadow_ellipse")), 1.0);
    let shadow_mesh_small = meshes.add(q_small.mesh());

    let mut q_large = Quads::default();
    q_large.add(flat(Vec3::ZERO, Vec2::new(0.8, 0.4)), full_uv(atlas.uv("shadow_ellipse")), 1.0);
    let shadow_mesh_large = meshes.add(q_large.mesh());

    let victim_frame_mesh = meshes.add(frame_mesh(0.13, 0.07));
    let attacker_frame_mesh = meshes.add(frame_mesh(0.06, 0.05));

    let px = atlas.px("icon_sword");
    let badge_h = 0.32;
    let badge_w = badge_h * (px.x / px.y);
    let hw = badge_w / 2.0;
    let mut q_badge = Quads::default();
    q_badge.add(
        [
            Vec3::new(-hw, 0.0, 0.0),
            Vec3::new(hw, 0.0, 0.0),
            Vec3::new(hw, badge_h, 0.0),
            Vec3::new(-hw, badge_h, 0.0),
        ],
        full_uv(atlas.uv("icon_sword")),
        1.0,
    );
    let badge_mesh = meshes.add(q_badge.mesh());

    let threat_victim = materials.add(StandardMaterial {
        base_color: Color::srgb_u8(0xE0, 0x3A, 0x2F).with_alpha(1.0),
        unlit: true,
        cull_mode: None,
        alpha_mode: AlphaMode::Blend,
        depth_bias: 0.0,
        ..default()
    });
    let threat_attacker = materials.add(StandardMaterial {
        base_color: Color::srgb_u8(0xF2, 0xB3, 0x3D).with_alpha(0.85),
        unlit: true,
        cull_mode: None,
        alpha_mode: AlphaMode::Blend,
        depth_bias: 0.0,
        ..default()
    });
    let threat_badge = markers.add(MarkerMaterial {
        base: StandardMaterial {
            base_color: Color::srgb_u8(0xFF, 0x6B, 0x5E),
            base_color_texture: Some(atlas.image.clone()),
            unlit: true,
            cull_mode: None,
            alpha_mode: AlphaMode::Blend,
            depth_bias: 200.0,
            ..default()
        },
        extension: OnTop {},
    });

    let occluded_white = occluded_materials.add(OccludedMaterial {
        base: StandardMaterial {
            base_color: Color::srgb_u8(0xD9, 0xBF, 0x40).with_alpha(0.55),
            base_color_texture: Some(atlas.image.clone()),
            unlit: true,
            cull_mode: None,
            alpha_mode: AlphaMode::Blend,
            depth_bias: 0.0,
            ..default()
        },
        extension: Occluded {},
    });
    let occluded_black = occluded_materials.add(OccludedMaterial {
        base: StandardMaterial {
            base_color: Color::srgb_u8(0xA4, 0x92, 0xC9).with_alpha(0.55),
            base_color_texture: Some(atlas.image.clone()),
            unlit: true,
            cull_mode: None,
            alpha_mode: AlphaMode::Blend,
            depth_bias: 0.0,
            ..default()
        },
        extension: Occluded {},
    });

    commands.insert_resource(Look {
        terrain,
        cards,
        piece_cards,
        grid,
        shadows,
        shadow_mesh_small,
        shadow_mesh_large,
        card_meshes: HashMap::new(),
        threat_victim,
        threat_attacker,
        threat_badge,
        victim_frame_mesh,
        attacker_frame_mesh,
        badge_mesh,
        occluded_white,
        occluded_black,
    });
}

/// A per-cell number for picking tile variants without flicker.
pub(crate) fn hash(x: i32, y: i32, salt: u32) -> u32 {
    let mut h = (x as u32).wrapping_mul(73_856_093) ^ (y as u32).wrapping_mul(19_349_663) ^ salt;
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
pub(crate) struct Quads {
    pub(crate) pos: Vec<[f32; 3]>,
    pub(crate) normal: Vec<[f32; 3]>,
    pub(crate) uv: Vec<[f32; 2]>,
    pub(crate) color: Vec<[f32; 4]>,
    pub(crate) idx: Vec<u32>,
}

impl Quads {
    /// Corners in order bottom-left, bottom-right, top-right, top-left as seen from
    /// outside, with texture coordinates to match. `shade` darkens the texture.
    pub(crate) fn add(&mut self, corners: [Vec3; 4], uv: [[f32; 2]; 4], shade: f32) {
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

    pub(crate) fn add_tinted(&mut self, corners: [Vec3; 4], uv: [[f32; 2]; 4], tint: Color) {
        let n = (corners[1] - corners[0]).cross(corners[3] - corners[0]).normalize_or_zero();
        let c = tint.to_linear().to_f32_array();
        let base = self.pos.len() as u32;
        for (p, t) in corners.into_iter().zip(uv) {
            self.pos.push(p.to_array());
            self.normal.push(n.to_array());
            self.uv.push(t);
            self.color.push(c);
        }
        self.idx.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    pub(crate) fn mesh(self) -> Mesh {
        Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.pos)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normal)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uv)
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.color)
            .with_inserted_indices(Indices::U32(self.idx))
    }
}

/// Texture corners for a whole sprite, matching `Quads::add` corner order.
pub(crate) fn full_uv(uv: Uv) -> [[f32; 2]; 4] {
    [uv.at(0.0, 1.0), uv.at(1.0, 1.0), uv.at(1.0, 0.0), uv.at(0.0, 0.0)]
}

pub(crate) fn rotate_uv(uv: [[f32; 2]; 4], step: u32) -> [[f32; 2]; 4] {
    let [bl, br, tr, tl] = uv;
    match step % 4 {
        0 => [bl, br, tr, tl],
        1 => [br, tr, tl, bl],
        2 => [tr, tl, bl, br],
        _ => [tl, bl, br, tr],
    }
}

/// A flat quad lying on a top face, centred at `c`.
pub(crate) fn flat(c: Vec3, size: Vec2) -> [Vec3; 4] {
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
pub(crate) fn wall(q: &mut Quads, atlas: &Atlas, a: Vec3, b: Vec3, lo: f32, hi: f32, shade: f32) {
    wall_tinted(q, atlas, a, b, lo, hi, Color::srgb(shade, shade, shade));
}

pub(crate) fn wall_tinted(q: &mut Quads, atlas: &Atlas, a: Vec3, b: Vec3, lo: f32, hi: f32, tint: Color) {
    let mut top = hi;
    let mut first = true;
    while top > lo + 1e-4 {
        let bottom = (top - LEVEL).max(lo);
        let uv = atlas.uv(if first { "wall_lip" } else { "wall_stone" });
        let frac = (top - bottom) / LEVEL;
        let at = |p: Vec3, y: f32| Vec3::new(p.x, y, p.z);
        q.add_tinted(
            [at(a, bottom), at(b, bottom), at(b, top), at(a, top)],
            [uv.at(0.0, frac), uv.at(1.0, frac), uv.at(1.0, 0.0), uv.at(0.0, 0.0)],
            tint,
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
    let mut grid_quads = Quads::default();
    for sq in tc_core::board::squares(size) {
        let tile = *t.get(sq);
        let top = square_top(sq, tile.height);

        // Top face: board land tops use board_stone; water/ice use dark navy without checker.
        let name = match tile.kind {
            TileKind::Grass | TileKind::Stone | TileKind::Sand | TileKind::Bridge => "board_stone",
            TileKind::ShallowWater => "shallow_0",
            TileKind::DeepWater => "deep",
            TileKind::Ice => "ice",
            TileKind::Void => "void",
        };
        let tint = if tile.kind == TileKind::DeepWater {
            Color::srgb(0.506, 0.283, 0.344)
        } else if tile.kind == TileKind::ShallowWater {
            Color::srgb(0.410, 0.284, 0.427)
        } else if tile.kind == TileKind::Ice {
            Color::srgb(0.150, 0.184, 0.264)
        } else {
            let base_tint = if (sq.x + sq.y) % 2 != 0 {
                Color::srgb(1.097, 1.060, 0.927)
            } else {
                Color::srgb(0.869, 0.846, 0.755)
            };
            let bright = 1.0 + 0.03 * tile.height as f32;
            let c = base_tint.to_srgba();
            Color::srgb(c.red * bright, c.green * bright, c.blue * bright)
        };
        let corners = flat(top, Vec2::ONE);
        if tile.kind == TileKind::ShallowWater {
            for (i, frame) in ["shallow_0", "shallow_1"].into_iter().enumerate() {
                water[i].add_tinted(corners, full_uv(atlas.uv(frame)), tint);
            }
        } else {
            solid.add_tinted(corners, full_uv(atlas.uv(name)), tint);
        }

        if tile.is_water() {
            let patch_uv = |name: &str| {
                let uv = atlas.uv(name);
                [uv.at(0.2, 0.52), uv.at(0.8, 0.52), uv.at(0.8, 0.44), uv.at(0.2, 0.44)]
            };
            let patch_0 = patch_uv("shore_0");
            let patch_1 = patch_uv("shore_1");
            let foam_tint = Color::srgb(0.287, 0.298, 0.471);
            for (dir, (dx, dy), offset, edge_size) in [
                (0, (0i8, 1i8), Vec3::new(0.0, 0.003, -0.45), Vec2::new(1.0, 0.10)),
                (1, (1, 0), Vec3::new(0.45, 0.003, 0.0), Vec2::new(0.10, 1.0)),
                (2, (0, -1), Vec3::new(0.0, 0.003, 0.45), Vec2::new(1.0, 0.10)),
                (3, (-1, 0), Vec3::new(-0.45, 0.003, 0.0), Vec2::new(0.10, 1.0)),
            ] {
                let is_water_neighbour = sq.offset(dx, dy, size).is_some_and(|n| t.get(n).is_water());
                if !is_water_neighbour {
                    let foam_corners = flat(top + offset, edge_size);
                    let uv_0 = rotate_uv(patch_0, dir);
                    let uv_1 = rotate_uv(patch_1, dir);
                    water[0].add_tinted(foam_corners, uv_0, foam_tint);
                    water[1].add_tinted(foam_corners, uv_1, foam_tint);
                }
            }
        }

        // Sides: south/east 30% darker (0.588, 0.504), north/west 15% darker (0.476, 0.527).
        let (x, z) = (top.x, top.z);
        for ((dx, dy), a, b, shade) in [
            ((0i8, -1i8), Vec3::new(x - 0.5, 0.0, z + 0.5), Vec3::new(x + 0.5, 0.0, z + 0.5), 0.588),
            ((1, 0), Vec3::new(x + 0.5, 0.0, z + 0.5), Vec3::new(x + 0.5, 0.0, z - 0.5), 0.504),
            ((0, 1), Vec3::new(x + 0.5, 0.0, z - 0.5), Vec3::new(x - 0.5, 0.0, z - 0.5), 0.476),
            ((-1, 0), Vec3::new(x - 0.5, 0.0, z - 0.5), Vec3::new(x - 0.5, 0.0, z + 0.5), 0.527),
        ] {
            let lo = match sq.offset(dx, dy, size) {
                Some(n) => top_y(t.height(n) as i8),
                None => -BASE,
            };
            wall(&mut solid, &atlas, a, b, lo, top.y, shade + 0.03 * tile.height as f32);
        }

        // Grid lines along square edges and column top rims:
        let y_lift = top.y + LIFT_GRID;
        let is_water = tile.is_water();
        for (dx, dy) in [(0i8, 1i8), (1, 0), (0, -1), (-1, 0)] {
            let n_opt = sq.offset(dx, dy, size);
            match n_opt {
                None => {
                    // Board outer edge: sitting on this square's own top edge.
                    let quad = match (dx, dy) {
                        (0, 1) => {
                            flat(Vec3::new(x, y_lift, z - 0.5 + GRID_WIDTH / 2.0), Vec2::new(1.0, GRID_WIDTH))
                        }
                        (1, 0) => {
                            flat(Vec3::new(x + 0.5 - GRID_WIDTH / 2.0, y_lift, z), Vec2::new(GRID_WIDTH, 1.0))
                        }
                        (0, -1) => {
                            flat(Vec3::new(x, y_lift, z + 0.5 - GRID_WIDTH / 2.0), Vec2::new(1.0, GRID_WIDTH))
                        }
                        (-1, 0) => {
                            flat(Vec3::new(x - 0.5 + GRID_WIDTH / 2.0, y_lift, z), Vec2::new(GRID_WIDTH, 1.0))
                        }
                        _ => unreachable!(),
                    };
                    grid_quads.add(quad, [[0.0, 0.0]; 4], 1.0);
                }
                Some(n) => {
                    let n_tile = *t.get(n);
                    let n_is_water = n_tile.is_water();
                    if is_water && n_is_water {
                        // Water squares' interiors: no grid lines.
                        continue;
                    }
                    if tile.height == n_tile.height {
                        // Shared flat edge: draw once (North and East).
                        if dx == 1 || dy == 1 {
                            let quad = match (dx, dy) {
                                (0, 1) => flat(Vec3::new(x, y_lift, z - 0.5), Vec2::new(1.0, GRID_WIDTH)),
                                (1, 0) => flat(Vec3::new(x + 0.5, y_lift, z), Vec2::new(GRID_WIDTH, 1.0)),
                                _ => unreachable!(),
                            };
                            grid_quads.add(quad, [[0.0, 0.0]; 4], 1.0);
                        }
                    } else if !is_water {
                        // Height difference: sitting on this square's own top edge.
                        let quad = match (dx, dy) {
                            (0, 1) => flat(
                                Vec3::new(x, y_lift, z - 0.5 + GRID_WIDTH / 2.0),
                                Vec2::new(1.0, GRID_WIDTH),
                            ),
                            (1, 0) => flat(
                                Vec3::new(x + 0.5 - GRID_WIDTH / 2.0, y_lift, z),
                                Vec2::new(GRID_WIDTH, 1.0),
                            ),
                            (0, -1) => flat(
                                Vec3::new(x, y_lift, z + 0.5 - GRID_WIDTH / 2.0),
                                Vec2::new(1.0, GRID_WIDTH),
                            ),
                            (-1, 0) => flat(
                                Vec3::new(x - 0.5 + GRID_WIDTH / 2.0, y_lift, z),
                                Vec2::new(GRID_WIDTH, 1.0),
                            ),
                            _ => unreachable!(),
                        };
                        grid_quads.add(quad, [[0.0, 0.0]; 4], 1.0);
                    }
                }
            }
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
                    Obstacle::Rock => ["rock", "rock_mossy"][hash(sq.x as i32, sq.y as i32, 4) as usize % 2],
                    Obstacle::Tree => {
                        ["pine", "pine", "dead_tree"][hash(sq.x as i32, sq.y as i32, 5) as usize % 3]
                    }
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
        if !q.pos.is_empty() {
            commands.spawn((TerrainPart, WaterFrame(i), Mesh3d(meshes.add(q.mesh())), material.clone()));
        }
    }
    if !grid_quads.pos.is_empty() {
        commands.spawn((
            TerrainPart,
            Mesh3d(meshes.add(grid_quads.mesh())),
            MeshMaterial3d(look.grid.clone()),
        ));
    }
}

/// An upright card for a sprite, feet at the origin, facing +Z.
pub(crate) fn card_mesh(
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

#[derive(Component)]
struct PieceShadow;

#[derive(Component)]
struct ShadowHop {
    from: Vec3,
    to: Vec3,
    t: f32,
    y: f32,
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
    run: Res<Run>,
    title_menu: Res<TitleMenu>,
    arrows_enabled: Res<ArrowsEnabled>,
    old: Query<
        Entity,
        Or<(With<PieceSprite>, With<PieceShadow>, With<Overlay>, With<PickupSprite>, With<PieceSilhouette>)>,
    >,
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
        let occluded_mat = match piece.side {
            Side::White => look.occluded_white.clone(),
            Side::Black => look.occluded_black.clone(),
        };
        let piece_e = commands
            .spawn((
                PieceSprite,
                Billboard,
                Mesh3d(mesh.clone()),
                MeshMaterial3d(look.piece_cards.clone()),
                Transform::from_translation(spot),
            ))
            .with_children(|parent| {
                parent.spawn((
                    PieceSilhouette,
                    Mesh3d(mesh),
                    MeshMaterial3d(occluded_mat),
                    Transform::from_translation(Vec3::new(0.0, 0.0, 0.06)),
                ));
            })
            .id();

        let shadow_mesh = match piece.kind {
            PieceKind::Knight | PieceKind::Rook => look.shadow_mesh_large.clone(),
            _ => look.shadow_mesh_small.clone(),
        };
        let shadow_spot = Vec3::new(spot.x, spot.y + LIFT_SHADOW, spot.z);
        let shadow_e = commands
            .spawn((
                PieceShadow,
                Mesh3d(shadow_mesh),
                MeshMaterial3d(look.shadows.clone()),
                Transform::from_translation(shadow_spot),
            ))
            .id();

        if let Some(mv) = animate.filter(|m| m.to == sq) {
            let from = piece_spot(&state, mv.from);
            let height = match (piece.kind, mv.kind) {
                (_, MoveKind::Cave) => 0.0,
                (PieceKind::Knight, _) => 0.8,
                _ => 0.25 + (from.y - spot.y).abs().min(LEVEL) * 0.5,
            };
            commands
                .entity(piece_e)
                .insert((Transform::from_translation(from), Hop { from, to: spot, t: 0.0, height }));
            let hop_y = from.y.min(spot.y) + LIFT_SHADOW;
            commands.entity(shadow_e).insert((
                Transform::from_translation(Vec3::new(from.x, hop_y, from.z)),
                ShadowHop { from, to: spot, t: 0.0, y: hop_y },
            ));
        }
        if let Some(MoveKind::Castle { rook_to, rook_from }) = animate.map(|m| m.kind)
            && sq == rook_to
        {
            let from = piece_spot(&state, rook_from);
            commands
                .entity(piece_e)
                .insert((Transform::from_translation(from), Hop { from, to: spot, t: 0.0, height: 0.2 }));
            let hop_y = from.y.min(spot.y) + LIFT_SHADOW;
            commands.entity(shadow_e).insert((
                Transform::from_translation(Vec3::new(from.x, hop_y, from.z)),
                ShadowHop { from, to: spot, t: 0.0, y: hop_y },
            ));
        }
    }

    // Pickups on the board: RunItem -> chest, bobbing at 1.5 Hz.
    for &(sq, pickup) in &state.game.pickups {
        let sprite_name = match pickup {
            tc_core::Pickup::RunItem => "chest",
        };
        let mesh = card_mesh(&mut look, &mut meshes, &atlas, sprite_name, false);
        let spot = square_top(sq, state.game.terrain.height(sq));
        let phase = sq.x as f32 * 1.3 + sq.y as f32 * 0.9;
        commands.spawn((
            PickupSprite,
            Billboard,
            Mesh3d(mesh),
            MeshMaterial3d(look.cards.clone()),
            Transform::from_translation(spot + Vec3::Y * 0.06),
            PickupBob { base_y: spot.y + 0.06, phase },
        ));
    }

    // No threat markers behind a title/draft/run-over overlay; red ones only on the turn of the
    // side they warn, amber ones (attackers of the selection) whenever selecting is possible.
    let overlay_open = title_menu.open || run.phase != RunPhase::Playing;
    let amber_threats = arrows_enabled.0 && !overlay_open;
    let red_threats = amber_threats && !state.ai_to_move() && state.outcome.is_none();
    spawn_overlays(
        &mut commands,
        &state,
        &atlas,
        &mut meshes,
        &mut materials,
        &mut markers,
        &look,
        red_threats,
        amber_threats,
    );
}

/// Flat highlights and markers lying on top faces.
struct OverlayPainter<'a, 'w, 's> {
    commands: &'a mut Commands<'w, 's>,
    state: &'a GameState,
    atlas: &'a Atlas,
    meshes: &'a mut Assets<Mesh>,
    materials: &'a mut Assets<StandardMaterial>,
    markers: &'a mut Assets<MarkerMaterial>,
    look: &'a Look,
}

impl OverlayPainter<'_, '_, '_> {
    fn quad(&mut self, sq: Sq, sprite: Option<&str>, size: f32, color: Color, lift: f32) {
        let base = StandardMaterial {
            base_color: color,
            base_color_texture: sprite.map(|_| self.atlas.image.clone()),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            depth_bias: if sprite.is_some() { 200.0 } else { 0.0 },
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

    fn victim_frame(&mut self, sq: Sq) {
        let top = square_top(sq, self.state.game.terrain.height(sq)) + Vec3::Y * LIFT_TINT;
        self.commands.spawn((
            Overlay,
            Mesh3d(self.look.victim_frame_mesh.clone()),
            MeshMaterial3d(self.look.threat_victim.clone()),
            Transform::from_translation(top),
        ));
    }

    fn attacker_frame(&mut self, sq: Sq) {
        let top = square_top(sq, self.state.game.terrain.height(sq)) + Vec3::Y * LIFT_TINT;
        self.commands.spawn((
            Overlay,
            Mesh3d(self.look.attacker_frame_mesh.clone()),
            MeshMaterial3d(self.look.threat_attacker.clone()),
            Transform::from_translation(top),
        ));
    }

    fn intent_badge(&mut self, sq: Sq, sprite_h: f32) {
        let spot = piece_spot(self.state, sq);
        self.commands.spawn((
            Overlay,
            Billboard,
            ThreatBadge { base_y: spot.y, sprite_h },
            Mesh3d(self.look.badge_mesh.clone()),
            MeshMaterial3d(self.look.threat_badge.clone()),
            Transform::from_translation(spot),
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_overlays(
    commands: &mut Commands,
    state: &GameState,
    atlas: &Atlas,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    markers: &mut Assets<MarkerMaterial>,
    look: &Look,
    red_threats: bool,
    amber_threats: bool,
) {
    let mut paint = OverlayPainter { commands, state, atlas, meshes, materials, markers, look };
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
    if let Some((sq, _)) = state.game.pos.shield {
        paint.mark(sq, "ov_ring", 0.7, Color::srgb_u8(0xFF, 0xD3, 0x5A));
    }
    if let Some(f) = state.game.pos.featherfall {
        paint.mark(f.sq, "ov_ring", 0.7, Color::srgb_u8(0x7D, 0xE8, 0x8B));
    }
    for curse in &state.game.pos.curses {
        paint.mark(curse.sq, "ov_ring", 0.7, Color::srgb_u8(0xB0, 0x3A, 0x5A));
    }
    let mut victims = HashSet::new();
    let mut attackers = HashSet::new();
    if red_threats {
        // Threats onto whichever side is to move, so hotseat/sandbox always reads as
        // "danger to the player about to act."
        for (from, to) in state.game.threats(state.game.pos.side_to_move) {
            attackers.insert(from);
            victims.insert(to);
        }
    }
    if amber_threats
        && let Some(sel) = state.selected
        && let Some(piece) = state.game.pos.get(sel)
    {
        for (from, to) in state.game.threats(piece.side) {
            if to == sel {
                attackers.insert(from);
                victims.insert(to);
            }
        }
    }
    for &from in &attackers {
        paint.attacker_frame(from);
    }
    for &to in &victims {
        paint.victim_frame(to);
        if let Some(piece) = state.game.pos.get(to) {
            let piece_h = atlas.px(&piece_sprite_name(piece.kind, piece.side)).y;
            paint.intent_badge(to, piece_h);
        }
    }
    if let Some(spell) = state.armed_spell {
        if matches!(spell, tc_core::SpellId::Swap | tc_core::SpellId::DigTunnel | tc_core::SpellId::Blink)
            && let Some(first) = state.swap_first
        {
            paint.mark(first, "ov_select", 0.95, Color::WHITE);
        }
        for sq in state.cast_target_squares() {
            paint.mark(sq, "ov_rune", 0.7, Color::WHITE);
        }
        return;
    }
    let Some(sel) = state.selected else { return };
    paint.mark(sel, "ov_select", 0.95, Color::WHITE);
    let moves = state.selected_moves();
    for mv in &moves {
        let capture = state.game.pos.get(mv.to).is_some() || mv.kind == MoveKind::EnPassant;
        if capture || mv.kind == MoveKind::Clear {
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

fn animate_shadow_hops(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut ShadowHop, &mut Transform)>,
) {
    for (e, mut hop, mut tf) in &mut q {
        hop.t = (hop.t + time.delta_secs() / HOP_SECS).min(1.0);
        let k = hop.t;
        let eased = k * k * (3.0 - 2.0 * k);
        let p = hop.from.lerp(hop.to, eased);
        let shrink = 1.0 - 0.3 * (4.0 * k * (1.0 - k));
        tf.translation = Vec3::new(p.x, hop.y, p.z);
        tf.scale = Vec3::splat(shrink);
        if k >= 1.0 {
            tf.translation = Vec3::new(hop.to.x, hop.to.y + LIFT_SHADOW, hop.to.z);
            tf.scale = Vec3::ONE;
            commands.entity(e).try_remove::<ShadowHop>();
        }
    }
}

/// Oscillate pickups at 1.5 Hz with an amplitude of 0.06 world units (specs/game-design.md §4).
fn animate_pickups(time: Res<Time>, mut q: Query<(&PickupBob, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (bob, mut tf) in &mut q {
        let offset = (t * 1.5 * std::f32::consts::TAU + bob.phase).sin() * 0.06;
        tf.translation.y = bob.base_y + offset;
    }
}

/// Pulse the shared threat frame materials' alpha at 1.6 Hz.
fn animate_threat_materials(
    time: Res<Time>,
    look: Res<Look>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let t = time.elapsed_secs();
    let phase = t * 1.6 * std::f32::consts::TAU;
    let s = phase.sin();
    // Victim: red #E03A2F, pulsing alpha 0.55 -> 1.0 at 1.6 Hz.
    let alpha_victim = 0.775 + 0.225 * s;
    if let Some(mut mat) = materials.get_mut(&look.threat_victim) {
        mat.base_color = Color::srgb_u8(0xE0, 0x3A, 0x2F).with_alpha(alpha_victim);
    }
    // Attacker: amber #F2B33D, pulsing in counter-phase, alpha 0.4 -> 0.85.
    let alpha_attacker = 0.625 - 0.225 * s;
    if let Some(mut mat) = materials.get_mut(&look.threat_attacker) {
        mat.base_color = Color::srgb_u8(0xF2, 0xB3, 0x3D).with_alpha(alpha_attacker);
    }
}

/// Float the intent badge 0.18 above the piece sprite, bobbing 0.04 at 1.2 Hz.
fn animate_threat_badges(
    time: Res<Time>,
    camera: Single<&Transform, (With<MainCamera>, Without<Billboard>)>,
    mut badges: Query<(&ThreatBadge, &mut Transform), (With<Billboard>, Without<MainCamera>)>,
) {
    let back = camera.back();
    let pitch = back.y.clamp(-0.99, 0.99).asin();
    let stretch = 1.0 / pitch.cos();
    let bob = (time.elapsed_secs() * 1.2 * std::f32::consts::TAU).sin() * 0.04;
    for (badge, mut tf) in &mut badges {
        tf.translation.y = badge.base_y + badge.sprite_h * PX * stretch + 0.18 + bob;
    }
}

/// Turn cards to face the camera, and stretch them so the tilt doesn't squash them.
fn face_camera(
    camera: Single<&Transform, (With<MainCamera>, Without<Billboard>)>,
    mut cards: Query<&mut Transform, (With<Billboard>, Without<MainCamera>)>,
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
        load_internal_asset!(app, OCCLUDED_SHADER_HANDLE, "occluded.wgsl", Shader::from_wgsl);
        app.add_plugins(MaterialPlugin::<MarkerMaterial>::default())
            .add_plugins(MaterialPlugin::<OccludedMaterial>::default())
            .add_plugins(MaterialPlugin::<PieceMaterial>::default())
            .add_systems(Startup, setup_look)
            .add_systems(
                Update,
                (
                    (spawn_terrain, spawn_pieces).chain(),
                    animate_water,
                    animate_hops,
                    animate_shadow_hops,
                    animate_pickups,
                    animate_threat_materials,
                    animate_threat_badges,
                    face_camera,
                ),
            );
    }
}
