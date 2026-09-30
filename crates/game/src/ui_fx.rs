//! UI juice: card sway, hover lift and tilt, and pop-in.

use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;

use crate::loading::AppState;

/// Hover and idle motion for a card; the only writer of the card's `UiTransform`.
#[derive(Component, Clone, Copy)]
pub struct CardMotion {
    /// Fan angle at rest (hand cards); 0 elsewhere.
    pub base_rot_deg: f32,
    /// Rotation pivot from the node center, y down (hand cards pivot at their bottom edge).
    pub pivot: Vec2,
    /// Rise when active.
    pub lift_px: f32,
    pub hover_scale: f32,
    /// Idle sway and bob.
    pub sway: bool,
    /// Offsets the idle motion so cards don't move in lockstep.
    pub phase: f32,
    /// Hovered, pressed or armed; set by the owner's interaction handler.
    pub active: bool,
    /// Extra offset added on top (the banner's slide-in).
    pub extra: Vec2,
    /// -1..1 from the cursor's x over the card.
    pub tilt: f32,
    h: f32,
}

impl CardMotion {
    pub fn new(lift_px: f32, hover_scale: f32, sway: bool, phase: f32) -> Self {
        CardMotion {
            base_rot_deg: 0.0,
            pivot: Vec2::ZERO,
            lift_px,
            hover_scale,
            sway,
            phase,
            active: false,
            extra: Vec2::ZERO,
            tilt: 0.0,
            h: 0.0,
        }
    }

    /// Starts fully active (e.g. a card that was armed before the hand rebuilt).
    pub fn settled(mut self, active: bool) -> Self {
        self.active = active;
        self.h = if active { 1.0 } else { 0.0 };
        self
    }

    /// The transform for the current state at time `t`.
    fn transform(&self, t: f32, pop: f32) -> UiTransform {
        let rest = 1.0 - self.h;
        let (sway, bob) = if self.sway {
            ((1.3 * t + self.phase).sin() * 1.2 * rest, (1.7 * t + self.phase).sin() * 1.5 * rest)
        } else {
            (0.0, 0.0)
        };
        let rot_deg = self.base_rot_deg * rest + sway - self.tilt * 5.0 * self.h;
        let rot = Rot2::degrees(rot_deg);
        let offset = self.pivot - rot * self.pivot + Vec2::new(0.0, bob - self.lift_px * self.h) + self.extra;
        UiTransform {
            translation: Val2::px(offset.x, offset.y),
            rotation: rot,
            scale: Vec2::splat((1.0 + (self.hover_scale - 1.0) * self.h) * pop),
        }
    }
}

/// Scale-in with ease-out-back after `delay` seconds; removed when done.
#[derive(Component)]
pub struct PopIn {
    pub delay: f32,
    pub dur: f32,
    t: f32,
}

impl PopIn {
    pub fn new(delay: f32, dur: f32) -> Self {
        PopIn { delay, dur, t: 0.0 }
    }

    fn scale(&self) -> f32 {
        let k = ((self.t - self.delay) / self.dur).clamp(0.0, 1.0);
        if k <= 0.0 { 0.0 } else { 0.85 + 0.15 * ease_out_back(k) }
    }
}

pub fn ease_out_back(x: f32) -> f32 {
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    1.0 + c3 * (x - 1.0).powi(3) + c1 * (x - 1.0).powi(2)
}

fn track_cursor_tilt(mut q: Query<(&RelativeCursorPosition, &mut CardMotion)>) {
    for (rel, mut motion) in &mut q {
        let tilt = match rel.normalized {
            Some(p) if rel.cursor_over => (p.x * 2.0).clamp(-1.0, 1.0),
            _ => 0.0,
        };
        if motion.tilt != tilt {
            motion.tilt = tilt;
        }
    }
}

fn animate_cards(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut CardMotion, &mut UiTransform, Option<&mut BoxShadow>, Option<&mut PopIn>)>,
) {
    let dt = time.delta_secs();
    let t = time.elapsed_secs();
    let k = 1.0 - (-14.0 * dt).exp();
    for (e, mut motion, mut tf, shadow, pop) in &mut q {
        let target = if motion.active { 1.0 } else { 0.0 };
        motion.h += (target - motion.h) * k;
        let pop = match pop {
            Some(mut p) => {
                p.t += dt;
                let s = p.scale();
                if p.t >= p.delay + p.dur {
                    commands.entity(e).remove::<PopIn>();
                }
                s
            }
            None => 1.0,
        };
        *tf = motion.transform(t, pop);
        if let Some(mut shadow) = shadow
            && let Some(s) = shadow.0.first_mut()
        {
            s.x_offset = Val::Px(3.0 + 4.0 * motion.h * motion.tilt);
            s.y_offset = Val::Px(5.0 + 6.0 * motion.h);
        }
    }
}

fn pop_in(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut PopIn, &mut UiTransform), Without<CardMotion>>,
) {
    for (e, mut pop, mut tf) in &mut q {
        pop.t += time.delta_secs();
        tf.scale = Vec2::splat(pop.scale());
        if pop.t >= pop.delay + pop.dur {
            tf.scale = Vec2::ONE;
            commands.entity(e).remove::<PopIn>();
        }
    }
}

pub struct UiFxPlugin;

impl Plugin for UiFxPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (track_cursor_tilt, animate_cards, pop_in)
                .chain()
                .before(bevy::ui::UiSystems::Layout)
                .run_if(in_state(AppState::Ready)),
        );
    }
}
