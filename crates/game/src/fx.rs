//! Visual effects system for moves, captures, spell casts, and board events.

use std::collections::HashMap;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use tc_core::{Side, SpellId, Sq};

use crate::atlas::Atlas;
use crate::board_view::{HOP_SECS, PX, Quads, full_uv, square_top};
use crate::game::{GameEvent, GameState};
use crate::input::MainCamera;

/// A short-lived camera-facing card.
#[derive(Component, Debug, Clone)]
pub struct Fx {
    pub t: f32,
    pub duration: f32,
    pub rise: f32,
    pub grow: f32,
}

#[derive(Component)]
pub(crate) struct FxOrigin(pub Vec3);

#[derive(Component)]
struct PendingLanding {
    timer: f32,
    to: Sq,
    landed_height_change: bool,
}

#[derive(Resource, Default)]
pub(crate) struct FxMeshes {
    meshes: HashMap<String, Handle<Mesh>>,
}

impl FxMeshes {
    pub fn get_or_create(&mut self, name: &str, meshes: &mut Assets<Mesh>, atlas: &Atlas) -> Handle<Mesh> {
        self.meshes
            .entry(name.to_string())
            .or_insert_with(|| {
                let size = atlas.px(name) * PX;
                let hw = size.x / 2.0;
                let hh = size.y / 2.0;
                let uv = full_uv(atlas.uv(name));
                let mut q = Quads::default();
                q.add(
                    [
                        Vec3::new(-hw, -hh, 0.0),
                        Vec3::new(hw, -hh, 0.0),
                        Vec3::new(hw, hh, 0.0),
                        Vec3::new(-hw, hh, 0.0),
                    ],
                    uv,
                    1.0,
                );
                meshes.add(q.mesh())
            })
            .clone()
    }
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_fx(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    atlas: &Atlas,
    fx_meshes: &mut FxMeshes,
    name: &str,
    pos: Vec3,
    duration: f32,
    rise: f32,
    grow: f32,
) -> Entity {
    let mesh = fx_meshes.get_or_create(name, meshes, atlas);
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(atlas.image.clone()),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        fog_enabled: true,
        ..default()
    });
    commands
        .spawn((
            Fx { t: 0.0, duration, rise, grow },
            FxOrigin(pos),
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_translation(pos).with_scale(Vec3::splat(0.6)),
        ))
        .id()
}

pub fn spawn_smoke(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    atlas: &Atlas,
    fx_meshes: &mut FxMeshes,
    pos: Vec3,
) -> Entity {
    spawn_fx(commands, meshes, materials, atlas, fx_meshes, "fx_smoke", pos, 0.6, 0.3, 1.6)
}

pub fn spawn_slash(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    atlas: &Atlas,
    fx_meshes: &mut FxMeshes,
    pos: Vec3,
    side: Side,
) -> Entity {
    let name = match side {
        Side::White => "fx_slash_gold",
        Side::Black => "fx_slash_violet",
    };
    spawn_fx(commands, meshes, materials, atlas, fx_meshes, name, pos, 0.35, 0.0, 1.0)
}

pub fn spawn_splash(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    atlas: &Atlas,
    fx_meshes: &mut FxMeshes,
    pos: Vec3,
) -> Entity {
    spawn_fx(commands, meshes, materials, atlas, fx_meshes, "fx_splash", pos, 0.5, 0.15, 1.2)
}

pub fn spawn_dust(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    atlas: &Atlas,
    fx_meshes: &mut FxMeshes,
    pos: Vec3,
) -> Entity {
    spawn_fx(commands, meshes, materials, atlas, fx_meshes, "fx_dust", pos, 0.45, 0.1, 0.8)
}

pub fn spawn_dirt(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    atlas: &Atlas,
    fx_meshes: &mut FxMeshes,
    pos: Vec3,
) -> Entity {
    spawn_fx(commands, meshes, materials, atlas, fx_meshes, "fx_dirt", pos, 0.6, 0.3, 1.1)
}

pub fn spawn_snowflake(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    atlas: &Atlas,
    fx_meshes: &mut FxMeshes,
    pos: Vec3,
) -> Entity {
    spawn_fx(commands, meshes, materials, atlas, fx_meshes, "fx_snowflake", pos, 0.8, 0.2, 1.0)
}

pub fn spawn_bubble(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    atlas: &Atlas,
    fx_meshes: &mut FxMeshes,
    pos: Vec3,
) -> Entity {
    spawn_fx(commands, meshes, materials, atlas, fx_meshes, "fx_bubble", pos, 0.6, 0.1, 1.3)
}

pub fn spawn_tornado(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    atlas: &Atlas,
    fx_meshes: &mut FxMeshes,
    pos: Vec3,
) -> Entity {
    spawn_fx(commands, meshes, materials, atlas, fx_meshes, "fx_tornado", pos, 0.6, 0.15, 1.1)
}

pub fn spawn_pillar(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    atlas: &Atlas,
    fx_meshes: &mut FxMeshes,
    pos: Vec3,
) -> Entity {
    spawn_fx(commands, meshes, materials, atlas, fx_meshes, "fx_pillar", pos, 0.9, 0.4, 1.2)
}

pub fn spawn_sparkle(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    atlas: &Atlas,
    fx_meshes: &mut FxMeshes,
    pos: Vec3,
) -> Entity {
    spawn_fx(commands, meshes, materials, atlas, fx_meshes, "fx_sparkle", pos, 0.6, 0.2, 1.1)
}

/// Helper system parameter for spawning visual effects cleanly.
#[derive(SystemParam)]
pub struct FxSpawner<'w, 's> {
    pub commands: Commands<'w, 's>,
    pub meshes: ResMut<'w, Assets<Mesh>>,
    pub materials: ResMut<'w, Assets<StandardMaterial>>,
    pub atlas: Res<'w, Atlas>,
    pub fx_meshes: ResMut<'w, FxMeshes>,
}

impl FxSpawner<'_, '_> {
    pub fn spawn_smoke(&mut self, pos: Vec3) -> Entity {
        spawn_smoke(
            &mut self.commands,
            &mut self.meshes,
            &mut self.materials,
            &self.atlas,
            &mut self.fx_meshes,
            pos,
        )
    }

    pub fn spawn_slash(&mut self, pos: Vec3, side: Side) -> Entity {
        spawn_slash(
            &mut self.commands,
            &mut self.meshes,
            &mut self.materials,
            &self.atlas,
            &mut self.fx_meshes,
            pos,
            side,
        )
    }

    pub fn spawn_splash(&mut self, pos: Vec3) -> Entity {
        spawn_splash(
            &mut self.commands,
            &mut self.meshes,
            &mut self.materials,
            &self.atlas,
            &mut self.fx_meshes,
            pos,
        )
    }

    pub fn spawn_dust(&mut self, pos: Vec3) -> Entity {
        spawn_dust(
            &mut self.commands,
            &mut self.meshes,
            &mut self.materials,
            &self.atlas,
            &mut self.fx_meshes,
            pos,
        )
    }

    pub fn spawn_dirt(&mut self, pos: Vec3) -> Entity {
        spawn_dirt(
            &mut self.commands,
            &mut self.meshes,
            &mut self.materials,
            &self.atlas,
            &mut self.fx_meshes,
            pos,
        )
    }

    pub fn spawn_snowflake(&mut self, pos: Vec3) -> Entity {
        spawn_snowflake(
            &mut self.commands,
            &mut self.meshes,
            &mut self.materials,
            &self.atlas,
            &mut self.fx_meshes,
            pos,
        )
    }

    pub fn spawn_bubble(&mut self, pos: Vec3) -> Entity {
        spawn_bubble(
            &mut self.commands,
            &mut self.meshes,
            &mut self.materials,
            &self.atlas,
            &mut self.fx_meshes,
            pos,
        )
    }

    pub fn spawn_tornado(&mut self, pos: Vec3) -> Entity {
        spawn_tornado(
            &mut self.commands,
            &mut self.meshes,
            &mut self.materials,
            &self.atlas,
            &mut self.fx_meshes,
            pos,
        )
    }

    pub fn spawn_pillar(&mut self, pos: Vec3) -> Entity {
        spawn_pillar(
            &mut self.commands,
            &mut self.meshes,
            &mut self.materials,
            &self.atlas,
            &mut self.fx_meshes,
            pos,
        )
    }

    pub fn spawn_sparkle(&mut self, pos: Vec3) -> Entity {
        spawn_sparkle(
            &mut self.commands,
            &mut self.meshes,
            &mut self.materials,
            &self.atlas,
            &mut self.fx_meshes,
            pos,
        )
    }
}

pub fn process_game_events(state: Res<GameState>, mut spawner: FxSpawner) {
    if state.events.is_empty() {
        return;
    }
    for event in &state.events {
        match event {
            GameEvent::Moved { to, landed_height_change } => {
                spawner.commands.spawn(PendingLanding {
                    timer: HOP_SECS,
                    to: *to,
                    landed_height_change: *landed_height_change,
                });
            }
            GameEvent::Captured { at, by_side } => {
                let spot = square_top(*at, state.game.terrain.height(*at));
                let pos = spot + Vec3::Y * 0.35;
                spawner.spawn_smoke(pos);
                spawner.spawn_slash(pos, *by_side);
            }
            GameEvent::Cast { spell, squares } => match spell {
                SpellId::RaiseEarth | SpellId::LowerEarth => {
                    for &sq in squares {
                        let spot = square_top(sq, state.game.terrain.height(sq));
                        spawner.spawn_dirt(spot + Vec3::Y * 0.2);
                    }
                }
                SpellId::Freeze => {
                    for &sq in squares {
                        let spot = square_top(sq, state.game.terrain.height(sq));
                        spawner.spawn_snowflake(spot + Vec3::Y * 0.25);
                    }
                }
                SpellId::Shield => {
                    for &sq in squares {
                        let spot = square_top(sq, state.game.terrain.height(sq));
                        spawner.spawn_bubble(spot + Vec3::Y * 0.35);
                    }
                }
                SpellId::Swap => {
                    for &sq in squares {
                        let spot = square_top(sq, state.game.terrain.height(sq));
                        spawner.spawn_tornado(spot + Vec3::Y * 0.35);
                    }
                }
                _ => {}
            },
            GameEvent::Promoted { at } => {
                let spot = square_top(*at, state.game.terrain.height(*at));
                spawner.spawn_pillar(spot + Vec3::Y * 0.5);
            }
            GameEvent::Check { king } => {
                let spot = square_top(*king, state.game.terrain.height(*king));
                spawner.spawn_sparkle(spot + Vec3::Y * 1.3);
            }
            GameEvent::Pickup { .. } | GameEvent::Selected { .. } => {}
        }
    }
}

fn update_pending_landings(
    time: Res<Time>,
    state: Res<GameState>,
    mut spawner: FxSpawner,
    mut q: Query<(Entity, &mut PendingLanding)>,
) {
    for (entity, mut pending) in &mut q {
        pending.timer -= time.delta_secs();
        if pending.timer <= 0.0 {
            let sq = pending.to;
            if state.game.pos.get(sq).is_some() {
                let tile = state.game.terrain.get(sq);
                let spot = square_top(sq, tile.height);
                if tile.is_water() {
                    spawner.spawn_splash(spot + Vec3::Y * 0.1);
                } else if pending.landed_height_change {
                    spawner.spawn_dust(spot + Vec3::Y * 0.1);
                }
            }
            spawner.commands.entity(entity).despawn();
        }
    }
}

fn update_fx(
    mut commands: Commands,
    time: Res<Time>,
    camera: Query<&Transform, (With<MainCamera>, Without<Fx>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut q: Query<
        (Entity, &mut Fx, &FxOrigin, &mut Transform, &MeshMaterial3d<StandardMaterial>),
        Without<MainCamera>,
    >,
) {
    let Some(cam_tf) = camera.iter().next() else {
        return;
    };
    let back = cam_tf.back();
    let yaw = back.x.atan2(back.z);
    let pitch = back.y.clamp(-0.99, 0.99).asin();
    let rotation = Quat::from_rotation_y(yaw);
    let cos_pitch = pitch.cos().max(0.01);

    for (entity, mut fx, origin, mut tf, mat_handle) in &mut q {
        fx.t += time.delta_secs();
        if fx.t >= fx.duration {
            commands.entity(entity).despawn();
            continue;
        }

        let p = (fx.t / fx.duration).clamp(0.0, 1.0);
        let s = 0.6 + (fx.grow - 0.6) * p;

        let alpha = if p <= 0.4 { 1.0 } else { ((1.0 - p) / 0.6).clamp(0.0, 1.0) };

        if let Some(ref mut mat) = materials.get_mut(&mat_handle.0) {
            mat.base_color = Color::srgba(1.0, 1.0, 1.0, alpha);
        }

        tf.translation = origin.0 + Vec3::Y * (fx.rise * p);
        tf.rotation = rotation;
        tf.scale = Vec3::new(s, s / cos_pitch, s);
    }
}

pub struct FxPlugin;

impl Plugin for FxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FxMeshes>()
            .add_systems(Update, (process_game_events, update_pending_landings, update_fx));
    }
}
