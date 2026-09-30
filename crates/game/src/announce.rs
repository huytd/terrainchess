//! Announces the AI's spells: a banner across the screen with the card, its name and
//! what it does, then the effect plays on the board. A pill under the level badge
//! keeps the latest enemy spell in view.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use tc_core::SpellId;

use crate::atlas::Atlas;
use crate::board_view::square_top;
use crate::fx::FxSpawner;
use crate::game::GameState;
use crate::loading::AppState;
use crate::theme::{
    self, PanelKind, RED, S, SLATE, TEXT, TEXT_DIM, XL, XS, card_root, label, label_nowrap, panel,
    spawn_card_face, spell_accent, spell_item_id, spell_name,
};
use crate::ui_fx::{CardMotion, ease_out_back};

/// Banner length in seconds; a click or tap after `SKIP_AFTER` closes it early.
const BANNER_SECS: f32 = 1.8;
const SKIP_AFTER: f32 = 0.3;
const OPEN_SECS: f32 = 0.25;
const CLOSE_SECS: f32 = 0.3;

#[derive(Component)]
struct Banner {
    t: f32,
    full_h: f32,
}

#[derive(Component)]
struct BannerCard {
    from_x: f32,
}

#[derive(Component)]
struct BannerTitle;

#[derive(Component)]
struct LastSpellPill;

#[derive(Component)]
struct LastSpellText;

fn spawn_banner(commands: &mut Commands, atlas: &Atlas, spell: SpellId, win: Vec2) {
    let compact = win.x < 600.0;
    let full_h = (win.y * if compact { 0.26 } else { 0.34 }).max(180.0);
    let card_h = (full_h * 0.82).round();
    let card_w = (card_h * 59.0 / 81.0).round();
    let from_x = -(card_w + 60.0);
    let name_px = if card_w < 170.0 { XS } else { S };

    commands
        .spawn((
            Banner { t: 0.0, full_h },
            GlobalZIndex(60),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(((win.y - full_h) * 0.5).round()),
                height: Val::Px(0.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                column_gap: Val::Px(if compact { 14.0 } else { 40.0 }),
                padding: UiRect::horizontal(Val::Px(if compact { 12.0 } else { 32.0 })),
                border: UiRect::vertical(Val::Px(4.0)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(SLATE.with_alpha(0.92)),
            BorderColor::all(RED),
        ))
        .with_children(|strip| {
            // The spell card, sliding in from the left.
            let mut motion = CardMotion::new(0.0, 1.0, true, 0.0);
            motion.extra = Vec2::new(from_x, 0.0);
            strip
                .spawn((
                    BannerCard { from_x },
                    motion,
                    card_root(
                        Node {
                            width: Val::Px(card_w),
                            height: Val::Px(card_h),
                            flex_shrink: 0.0,
                            ..default()
                        },
                        0.09 * card_w,
                    ),
                ))
                .with_children(|card| {
                    let art = format!("art_{}", spell_item_id(spell));
                    spawn_card_face(
                        card,
                        atlas,
                        &art,
                        spell_name(spell),
                        spell_accent(spell),
                        &[("Enemy", RED)],
                        card_w,
                        name_px,
                    );
                });

            // "Enemy casts", the spell name and what it does.
            strip
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::FlexStart,
                    row_gap: Val::Px(if compact { 2.0 } else { 6.0 }),
                    flex_shrink: 1.0,
                    max_width: Val::Vw(if compact { 58.0 } else { 46.0 }),
                    ..default()
                })
                .with_children(|col| {
                    col.spawn(label_nowrap("Enemy casts", if compact { XS } else { S }, RED))
                        .insert(left_text());
                    col.spawn((
                        BannerTitle,
                        label(spell_name(spell).to_uppercase(), if compact { theme::M } else { XL }, TEXT),
                    ))
                    .insert(left_text());
                    col.spawn(label(spell.effect_text(), if compact { XS } else { S }, TEXT_DIM))
                        .insert(left_text());
                });
        });
}

fn left_text() -> TextLayout {
    TextLayout { justify: Justify::Left, linebreak: LineBreak::WordBoundary }
}

/// Opens the banner for a held-back AI cast, animates it, and releases the cast when it closes.
#[allow(clippy::too_many_arguments)]
fn run_banner(
    mut commands: Commands,
    time: Res<Time>,
    mouse: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    atlas: Res<Atlas>,
    mut state: ResMut<GameState>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut banners: Query<(Entity, &mut Banner, &mut Node)>,
    mut cards: Query<(&BannerCard, &mut CardMotion)>,
    mut titles: Query<&mut UiTransform, With<BannerTitle>>,
    mut spawner: FxSpawner,
) {
    let Some(spell) = state.announce.as_ref().map(|a| a.spell) else {
        for (e, ..) in &banners {
            commands.entity(e).despawn();
        }
        return;
    };
    let Ok((entity, mut banner, mut node)) = banners.single_mut() else {
        let win = window.iter().next().map(|w| w.size()).unwrap_or(Vec2::new(1280.0, 800.0));
        spawn_banner(&mut commands, &atlas, spell, win);
        return;
    };

    banner.t += time.delta_secs();
    // A click or tap jumps to the closing animation (so that click never reaches the board).
    let skip = banner.t > SKIP_AFTER && (mouse.just_pressed(MouseButton::Left) || touches.any_just_pressed());
    if skip && banner.t < BANNER_SECS - CLOSE_SECS {
        banner.t = BANNER_SECS - CLOSE_SECS;
    }
    let t = banner.t;
    if t >= BANNER_SECS {
        commands.entity(entity).despawn();
        let squares = state.announce.as_ref().map(|a| a.squares.clone()).unwrap_or_default();
        state.finish_announce();
        // Mark where it landed.
        for sq in squares {
            let top = square_top(sq, state.game.terrain.height(sq));
            spawner.spawn_sparkle(top + Vec3::Y * 0.3);
        }
        return;
    }

    // Strip opens, holds, then closes.
    let k = if t < OPEN_SECS {
        (t / OPEN_SECS).powf(0.6)
    } else if t > BANNER_SECS - CLOSE_SECS {
        ((BANNER_SECS - t) / CLOSE_SECS).max(0.0)
    } else {
        1.0
    };
    node.height = Val::Px((banner.full_h * k).round());
    if let Val::Px(top) = node.top {
        let win_h = window.iter().next().map(|w| w.height()).unwrap_or(800.0);
        let want = ((win_h - banner.full_h * k) * 0.5).round();
        if (top - want).abs() > 0.5 {
            node.top = Val::Px(want);
        }
    }

    let slide = ((t - 0.05) / 0.3).clamp(0.0, 1.0);
    for (card, mut motion) in &mut cards {
        motion.extra = Vec2::new(card.from_x * (1.0 - ease_out_back(slide)), 0.0);
    }
    // Name punches in from 1.4× to 1×.
    let punch = ((t - 0.15) / 0.2).clamp(0.0, 1.0);
    for mut tf in &mut titles {
        tf.scale = Vec2::splat(1.4 - 0.4 * punch);
    }
}

fn setup_pill(mut commands: Commands, atlas: Res<Atlas>) {
    let (icon, _) = theme::integer_scaled_size(&atlas, "icon_skull", 20.0);
    commands
        .spawn((
            LastSpellPill,
            GlobalZIndex(5),
            panel(
                PanelKind::Pill,
                Node {
                    display: Display::None,
                    position_type: PositionType::Absolute,
                    left: Val::Px(16.0),
                    top: Val::Px(76.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(8.0),
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(4.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
            ),
            Button,
        ))
        .insert(BorderColor::all(RED))
        .with_children(|row| {
            row.spawn((
                Node { width: Val::Px(icon.x), height: Val::Px(icon.y), ..default() },
                ImageNode { image: atlas.image.clone(), rect: Some(atlas.rect("icon_skull")), ..default() },
                Pickable::IGNORE,
            ));
            row.spawn((LastSpellText, label_nowrap("", XS, RED.lighter(0.15))));
        });
}

/// Keeps the "Enemy: <spell>" pill current; tapping it shows what the spell does.
fn sync_pill(
    mut state: ResMut<GameState>,
    title_menu: Res<crate::run::TitleMenu>,
    mut pill: Query<(&mut Node, &Interaction), With<LastSpellPill>>,
    mut text: Query<&mut Text, With<LastSpellText>>,
) {
    // Revealed once the banner has closed.
    let spell = if state.announce.is_some() { None } else { state.last_enemy_spell };
    let shown = spell.filter(|_| !title_menu.open && state.outcome.is_none());
    let Ok((mut node, interaction)) = pill.single_mut() else { return };
    let display = if shown.is_some() { Display::Flex } else { Display::None };
    if node.display != display {
        node.display = display;
    }
    let Some(spell) = shown else { return };
    let label = format!("Enemy: {}", spell_name(spell));
    for mut t in &mut text {
        if t.0 != label {
            t.0 = label.clone();
        }
    }
    if *interaction == Interaction::Pressed {
        state.toast = Some(spell.effect_text().to_string());
    }
}

pub struct AnnouncePlugin;

impl Plugin for AnnouncePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Ready), setup_pill).add_systems(
            Update,
            (
                // Released events must reach the effects before the sound system drains them.
                run_banner.before(crate::fx::process_game_events).before(crate::particles::cast_particles),
                sync_pill,
            )
                .run_if(in_state(AppState::Ready)),
        );
    }
}
