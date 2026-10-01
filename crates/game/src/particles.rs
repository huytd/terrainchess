//! Small CPU particle bursts for spell casts. The web build runs on WebGL2, so no
//! compute-shader particles: each particle is a camera-facing atlas quad that shares
//! its mesh and a tinted material with every other particle of the same look.

use std::collections::HashMap;
use std::f32::consts::TAU;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use tc_core::{Side, SpellId, Sq};

use crate::atlas::Atlas;
use crate::board_view::square_top;
use crate::fx::FxMeshes;
use crate::game::GameState;
use crate::input::MainCamera;

/// Live particle caps; phones get fewer.
const MAX_PARTICLES: usize = 400;
const MAX_PARTICLES_COMPACT: usize = 150;
/// Every particle is drawn this much bigger than its recipe says.
const SIZE_K: f32 = 1.5;
/// Bursts spread this much wider than one square's worth, reaching the neighbours.
const SPREAD_K: f32 = 1.8;
/// Each burst spawns this many times as many particles, to stay dense over the wider spread.
const COUNT_K: f32 = 1.5;
/// Particles are drawn pulled this far toward the camera (and shrunk to match), so they cover
/// the pieces and terrain they fly through without looking any different.
const PULL: f32 = 3.0;

fn more(n: usize) -> usize {
    (n as f32 * COUNT_K).round() as usize
}

#[derive(Clone, Copy)]
enum Motion {
    /// Velocity, gravity and drag.
    Ballistic,
    /// Circles `center` at `radius`, drifting up by `rise` per second; radius shrinks by `shrink`.
    Orbit { center: Vec3, radius: f32, angle: f32, speed: f32, rise: f32, shrink: f32 },
}

#[derive(Component)]
struct Particle {
    motion: Motion,
    /// Where the particle really is; its `Transform` is this pulled toward the camera.
    pos: Vec3,
    vel: Vec3,
    gravity: f32,
    drag: f32,
    /// Seconds before it shows up, for trails and staggered bursts.
    delay: f32,
    age: f32,
    life: f32,
    size: f32,
    /// Size at the end of life, as a fraction of `size`.
    end_size: f32,
    spin: f32,
    angle: f32,
    /// Horizontal sway amplitude (feathers, leaves).
    sway: f32,
}

#[derive(Resource, Default)]
struct ParticleMats(HashMap<(String, [u8; 4]), Handle<StandardMaterial>>);

#[derive(Resource)]
struct ParticleRng(u64);

impl Default for ParticleRng {
    fn default() -> Self {
        ParticleRng(0x9E37_79B9_7F4A_7C15)
    }
}

impl ParticleRng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.next()
    }
    /// A random horizontal unit vector.
    fn dir(&mut self) -> Vec3 {
        let a = self.next() * TAU;
        Vec3::new(a.cos(), 0.0, a.sin())
    }
}

/// One particle's starting state; fields left at default mean "not used".
#[derive(Clone, Copy)]
struct Spec {
    sprite: &'static str,
    tint: [u8; 4],
    pos: Vec3,
    vel: Vec3,
    gravity: f32,
    drag: f32,
    delay: f32,
    life: f32,
    size: f32,
    end_size: f32,
    spin: f32,
    sway: f32,
    motion: Motion,
}

impl Spec {
    fn new(sprite: &'static str, tint: [u8; 4], pos: Vec3) -> Self {
        Spec {
            sprite,
            tint,
            pos,
            vel: Vec3::ZERO,
            gravity: 0.0,
            drag: 0.0,
            delay: 0.0,
            life: 0.8,
            size: 0.3,
            end_size: 0.0,
            spin: 0.0,
            sway: 0.0,
            motion: Motion::Ballistic,
        }
    }
}

const WHITE: [u8; 4] = [255, 255, 255, 255];
const GOLD: [u8; 4] = [255, 214, 110, 255];
const ICE: [u8; 4] = [170, 230, 255, 255];
const BLUE: [u8; 4] = [120, 190, 255, 255];
const VIOLET: [u8; 4] = [200, 130, 255, 255];
const PURPLE: [u8; 4] = [150, 70, 190, 230];
const GREEN: [u8; 4] = [130, 230, 110, 255];
const WOOD: [u8; 4] = [200, 140, 80, 255];
const WATER: [u8; 4] = [140, 200, 255, 230];
const STEAM: [u8; 4] = [235, 240, 245, 200];
const AMBER: [u8; 4] = [255, 170, 70, 255];

#[derive(SystemParam)]
pub struct Particles<'w, 's> {
    commands: Commands<'w, 's>,
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    atlas: Res<'w, Atlas>,
    fx_meshes: ResMut<'w, FxMeshes>,
    mats: ResMut<'w, ParticleMats>,
    rng: ResMut<'w, ParticleRng>,
    live: Query<'w, 's, (), With<Particle>>,
    window: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
}

impl Particles<'_, '_> {
    fn spawn(&mut self, s: Spec, budget: &mut usize) {
        if *budget == 0 {
            return;
        }
        *budget -= 1;
        let mesh = self.fx_meshes.get_or_create(s.sprite, &mut self.meshes, &self.atlas);
        let image = self.atlas.image.clone();
        let materials = &mut self.materials;
        let material = self
            .mats
            .0
            .entry((s.sprite.to_string(), s.tint))
            .or_insert_with(|| {
                let [r, g, b, a] = s.tint;
                materials.add(StandardMaterial {
                    base_color: Color::srgba_u8(r, g, b, a),
                    base_color_texture: Some(image),
                    unlit: true,
                    alpha_mode: AlphaMode::Blend,
                    cull_mode: None,
                    fog_enabled: true,
                    ..default()
                })
            })
            .clone();
        let angle = self.rng.range(0.0, TAU);
        self.commands.spawn((
            Particle {
                motion: s.motion,
                pos: s.pos,
                vel: s.vel,
                gravity: s.gravity,
                drag: s.drag,
                delay: s.delay,
                age: 0.0,
                life: s.life,
                size: s.size * SIZE_K,
                end_size: s.end_size,
                spin: s.spin,
                angle,
                sway: s.sway,
            },
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_translation(s.pos).with_scale(Vec3::ZERO),
        ));
    }

    fn budget(&self) -> usize {
        let compact = self.window.iter().next().is_some_and(|w| w.width() < 600.0);
        let cap = if compact { MAX_PARTICLES_COMPACT } else { MAX_PARTICLES };
        cap.saturating_sub(self.live.iter().count())
    }

    /// A burst flying up and out from `at`, falling back under gravity.
    #[allow(clippy::too_many_arguments)]
    fn fountain(
        &mut self,
        b: &mut usize,
        sprite: &'static str,
        tint: [u8; 4],
        at: Vec3,
        n: usize,
        up: (f32, f32),
        out: (f32, f32),
        size: f32,
    ) {
        for _ in 0..more(n) {
            let dir = self.rng.dir();
            let vel = dir * self.rng.range(out.0, out.1) * SPREAD_K + Vec3::Y * self.rng.range(up.0, up.1);
            let mut s = Spec::new(sprite, tint, at + dir * 0.1 * SPREAD_K);
            s.vel = vel;
            s.gravity = 7.0;
            s.life = self.rng.range(0.6, 1.0);
            s.size = size * self.rng.range(0.7, 1.2);
            s.end_size = 0.4;
            s.spin = self.rng.range(-6.0, 6.0);
            self.spawn(s, b);
        }
    }

    /// A flat ring pushed outward along the ground.
    #[allow(clippy::too_many_arguments)]
    fn ring(
        &mut self,
        b: &mut usize,
        sprite: &'static str,
        tint: [u8; 4],
        at: Vec3,
        n: usize,
        speed: f32,
        size: f32,
    ) {
        let n = more(n);
        for i in 0..n {
            let a = i as f32 / n as f32 * TAU;
            let dir = Vec3::new(a.cos(), 0.0, a.sin());
            let mut s = Spec::new(sprite, tint, at + dir * 0.15 * SPREAD_K);
            s.vel = dir * speed * SPREAD_K + Vec3::Y * 0.2;
            s.drag = 3.0;
            s.life = 0.55;
            s.size = size;
            s.end_size = 0.2;
            self.spawn(s, b);
        }
    }

    /// Particles pulled in toward `at` from a ring around it.
    fn implode(
        &mut self,
        b: &mut usize,
        sprite: &'static str,
        tint: [u8; 4],
        at: Vec3,
        n: usize,
        radius: f32,
    ) {
        for _ in 0..more(n) {
            let dir = self.rng.dir();
            let from = at + dir * radius * SPREAD_K + Vec3::Y * self.rng.range(-0.1, 0.4);
            let life = self.rng.range(0.35, 0.5);
            let mut s = Spec::new(sprite, tint, from);
            s.vel = (at - from) / life;
            s.life = life;
            s.size = 0.28;
            s.end_size = 0.3;
            self.spawn(s, b);
        }
    }

    /// Particles spiralling around `center`.
    #[allow(clippy::too_many_arguments)]
    fn swirl(
        &mut self,
        b: &mut usize,
        sprite: &'static str,
        tint: [u8; 4],
        center: Vec3,
        n: usize,
        radius: f32,
        rise: f32,
        shrink: f32,
    ) {
        let n = more(n);
        for i in 0..n {
            let mut s = Spec::new(sprite, tint, center);
            s.motion = Motion::Orbit {
                center: center + Vec3::Y * self.rng.range(0.0, 0.3),
                radius: radius * SPREAD_K * self.rng.range(0.8, 1.1),
                angle: i as f32 / n as f32 * TAU,
                speed: 5.0,
                rise,
                shrink: shrink * SPREAD_K,
            };
            s.life = self.rng.range(0.8, 1.1);
            s.size = 0.26;
            s.end_size = 0.1;
            s.delay = i as f32 * 0.015;
            self.spawn(s, b);
        }
    }

    /// Dots appearing one after another along an arc from `a` to `b`.
    #[allow(clippy::too_many_arguments)]
    fn trail(
        &mut self,
        budget: &mut usize,
        sprite: &'static str,
        tint: [u8; 4],
        a: Vec3,
        b: Vec3,
        n: usize,
        height: f32,
    ) {
        for i in 0..n {
            let k = i as f32 / (n - 1).max(1) as f32;
            let p = a.lerp(b, k) + Vec3::Y * (height * 4.0 * k * (1.0 - k));
            let mut s = Spec::new(sprite, tint, p);
            s.delay = k * 0.35;
            s.life = 0.55;
            s.size = 0.24;
            s.end_size = 0.1;
            s.vel = Vec3::Y * 0.3;
            self.spawn(s, budget);
        }
    }

    /// Things drifting down from above `at` with a side-to-side sway.
    #[allow(clippy::too_many_arguments)]
    fn drift_down(
        &mut self,
        b: &mut usize,
        sprite: &'static str,
        tint: [u8; 4],
        at: Vec3,
        n: usize,
        spread: f32,
        size: f32,
    ) {
        for i in 0..more(n) {
            let off = self.rng.dir() * self.rng.range(0.0, spread * SPREAD_K);
            let mut s = Spec::new(sprite, tint, at + off + Vec3::Y * self.rng.range(0.9, 1.4));
            s.vel = Vec3::Y * -self.rng.range(0.6, 0.9);
            s.life = self.rng.range(1.1, 1.4);
            s.size = size;
            s.end_size = 0.6;
            s.sway = 0.15;
            s.spin = self.rng.range(-2.0, 2.0);
            s.delay = i as f32 * 0.05;
            self.spawn(s, b);
        }
    }

    /// Things rising from `at`, spreading and growing (smoke, steam).
    #[allow(clippy::too_many_arguments)]
    fn rise(
        &mut self,
        b: &mut usize,
        sprite: &'static str,
        tint: [u8; 4],
        at: Vec3,
        n: usize,
        size: f32,
        grow: f32,
    ) {
        for i in 0..more(n) {
            let dir = self.rng.dir();
            let mut s = Spec::new(sprite, tint, at + dir * self.rng.range(0.0, 0.3) * SPREAD_K);
            s.vel = dir * 0.2 * SPREAD_K + Vec3::Y * self.rng.range(0.6, 1.2);
            s.drag = 0.6;
            s.life = self.rng.range(0.9, 1.3);
            s.size = size;
            s.end_size = grow;
            s.spin = self.rng.range(-1.5, 1.5);
            s.delay = i as f32 * 0.03;
            self.spawn(s, b);
        }
    }

    /// The particle recipe for one spell cast.
    pub fn cast(&mut self, state: &GameState, side: Side, spell: SpellId, squares: &[Sq]) {
        let mut b = self.budget();
        let b = &mut b;
        let top = |sq: Sq| square_top(sq, state.game.terrain.height(sq));
        match spell {
            SpellId::RaiseEarth => {
                for &sq in squares {
                    self.fountain(
                        b,
                        "fx_dirt",
                        WHITE,
                        top(sq) + Vec3::Y * 0.1,
                        14,
                        (2.0, 3.6),
                        (0.4, 1.2),
                        0.3,
                    );
                    self.ring(b, "fx_dust", WHITE, top(sq), 10, 2.2, 0.45);
                }
            }
            SpellId::LowerEarth => {
                for &sq in squares {
                    self.implode(b, "fx_dust", WHITE, top(sq) + Vec3::Y * 0.1, 12, 0.7);
                    self.rise(b, "fx_smoke", STEAM, top(sq), 6, 0.25, 1.4);
                }
            }
            SpellId::Freeze => {
                for &sq in squares {
                    self.drift_down(b, "fx_snowflake", ICE, top(sq), 4, 0.4, 0.28);
                    self.fountain(b, "fx_sparkle", ICE, top(sq), 3, (0.8, 1.4), (0.2, 0.5), 0.3);
                }
            }
            SpellId::Bridge => {
                for &sq in squares {
                    let c = top(sq);
                    self.trail(b, "fx_sparkle", WOOD, c - Vec3::X * 0.5, c + Vec3::X * 0.5, 10, 0.2);
                    self.fountain(b, "fx_splash", WATER, c, 6, (1.2, 2.0), (0.4, 0.9), 0.25);
                }
            }
            SpellId::DigTunnel => {
                for &sq in squares {
                    self.fountain(b, "fx_dirt", WHITE, top(sq), 10, (1.6, 2.8), (0.3, 1.0), 0.28);
                }
                if let [a, c] = squares {
                    self.trail(b, "fx_sparkle", AMBER, top(*a), top(*c), 12, 1.0);
                }
            }
            SpellId::Shield => {
                for &sq in squares {
                    self.swirl(b, "fx_sparkle", BLUE, top(sq) + Vec3::Y * 0.2, 12, 0.45, 0.6, 0.0);
                }
            }
            SpellId::Swap => {
                if let [a, c] = squares {
                    let (pa, pc) = (top(*a) + Vec3::Y * 0.4, top(*c) + Vec3::Y * 0.4);
                    self.trail(b, "fx_sparkle", GOLD, pa, pc, 14, 1.2);
                    self.trail(b, "fx_sparkle", VIOLET, pc, pa, 14, 0.7);
                }
            }
            SpellId::Rewind => {
                let n = state.size as f32;
                let center = Vec3::new((n - 1.0) * 0.5, 1.0, -(n - 1.0) * 0.5);
                self.swirl(b, "fx_sparkle", ICE, center, 24, n * 0.35, 0.2, n * 0.3);
                for &sq in squares.iter().take(12) {
                    self.implode(b, "fx_sparkle", WHITE, top(sq) + Vec3::Y * 0.3, 5, 0.5);
                }
            }
            SpellId::Smite => {
                for &sq in squares {
                    let c = top(sq) + Vec3::Y * 0.2;
                    self.fountain(b, "fx_sparkle", GOLD, c, 16, (1.5, 3.2), (1.2, 2.4), 0.3);
                    self.fountain(b, "fx_dirt", WHITE, c, 8, (1.8, 3.0), (0.6, 1.4), 0.3);
                }
            }
            SpellId::Evaporate => {
                for &sq in squares {
                    self.rise(b, "fx_smoke", STEAM, top(sq), 14, 0.22, 1.8);
                }
            }
            SpellId::Flood => {
                for &sq in squares {
                    self.fountain(b, "fx_splash", WATER, top(sq), 16, (1.8, 3.0), (0.3, 1.0), 0.22);
                    self.ring(b, "fx_splash", WATER, top(sq), 10, 1.8, 0.25);
                }
            }
            SpellId::Featherfall => {
                for &sq in squares {
                    self.drift_down(b, "fx_feather", WHITE, top(sq), 8, 0.45, 0.3);
                }
            }
            SpellId::Curse => {
                for &sq in squares {
                    self.rise(b, "fx_smoke", PURPLE, top(sq) + Vec3::Y * 0.2, 10, 0.25, 1.6);
                    self.rise(b, "fx_skull", WHITE, top(sq) + Vec3::Y * 0.4, 3, 0.22, 0.9);
                }
            }
            SpellId::Sprout => {
                for &sq in squares {
                    self.fountain(b, "icon_leaf", GREEN, top(sq), 12, (1.8, 3.0), (0.3, 1.0), 0.28);
                    self.fountain(b, "fx_sparkle", GREEN, top(sq), 6, (1.0, 1.8), (0.2, 0.6), 0.25);
                }
            }
            SpellId::Blink => {
                if let [from, to] = squares {
                    self.implode(b, "fx_sparkle", VIOLET, top(*from) + Vec3::Y * 0.4, 12, 0.6);
                    let at = top(*to) + Vec3::Y * 0.4;
                    for _ in 0..more(14) {
                        let dir = (self.rng.dir() + Vec3::Y * self.rng.range(-0.3, 0.8)).normalize();
                        let mut s = Spec::new("fx_sparkle", VIOLET, at);
                        s.vel = dir * self.rng.range(1.5, 2.5) * SPREAD_K;
                        s.drag = 2.5;
                        s.delay = 0.3;
                        s.life = 0.6;
                        s.size = 0.28;
                        self.spawn(s, b);
                    }
                }
            }
            SpellId::Insight => {
                if let Some(king) = state.game.pos.king(side) {
                    self.swirl(b, "fx_star", GOLD, top(king) + Vec3::Y * 0.3, 14, 0.5, 1.2, 0.2);
                }
            }
        }
    }
}

/// Bursts particles for every spell cast this frame.
pub fn cast_particles(state: Res<GameState>, mut particles: Particles) {
    for event in &state.events {
        if let crate::game::GameEvent::Cast { side, spell, squares } = event {
            particles.cast(&state, *side, *spell, squares);
        }
    }
}

fn update_particles(
    mut commands: Commands,
    time: Res<Time>,
    camera: Query<&Transform, (With<MainCamera>, Without<Particle>)>,
    mut q: Query<(Entity, &mut Particle, &mut Transform), Without<MainCamera>>,
) {
    let Some(cam_tf) = camera.iter().next() else { return };
    let back = cam_tf.back();
    let yaw = back.x.atan2(back.z);
    let cos_pitch = back.y.clamp(-0.99, 0.99).asin().cos().max(0.01);
    let face = Quat::from_rotation_y(yaw);
    let right = face * Vec3::X;
    let eye = cam_tf.translation;
    let dt = time.delta_secs();

    for (e, mut p, mut tf) in &mut q {
        if p.delay > 0.0 {
            p.delay -= dt;
            continue;
        }
        p.age += dt;
        if p.age >= p.life {
            commands.entity(e).despawn();
            continue;
        }
        let k = p.age / p.life;
        match p.motion {
            Motion::Ballistic => {
                p.vel.y -= p.gravity * dt;
                let damp = (1.0 - p.drag * dt).max(0.0);
                p.vel *= damp;
                let step = p.vel * dt;
                p.pos += step;
            }
            Motion::Orbit { center, radius, angle, speed, rise, shrink } => {
                let a = angle + speed * p.age;
                let r = (radius - shrink * p.age).max(0.0);
                p.pos = center + Vec3::new(a.cos() * r, rise * p.age, a.sin() * r);
            }
        }
        if p.sway != 0.0 {
            let step = right * (p.age * 5.0).cos() * p.sway * 5.0 * dt;
            p.pos += step;
        }
        p.angle += p.spin * dt;
        // Pop in over the first 10% of life, then ease toward the end size.
        let pop = (k / 0.1).min(1.0);
        let s = p.size * pop * (1.0 + (p.end_size - 1.0) * k);
        // Same spot on screen, but nearer: shrinking by the pulled-in distance keeps its size.
        let dist = (p.pos - eye).length().max(0.01);
        let near = (dist - PULL).max(dist * 0.3) / dist;
        tf.translation = eye + (p.pos - eye) * near;
        tf.rotation = face * Quat::from_rotation_z(p.angle);
        tf.scale = Vec3::new(s, s / cos_pitch, s) * near;
    }
}

pub struct ParticlesPlugin;

impl Plugin for ParticlesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ParticleMats>()
            .init_resource::<ParticleRng>()
            .add_systems(Update, (cast_particles, update_particles));
    }
}
