//! Announces the AI's spells: a banner across the screen with the card, its name and
//! what it does, then the effect plays on the board. A pill under the level badge
//! keeps the latest enemy spell in view.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use tc_core::SpellId;

use crate::TitleFont;
use crate::atlas::Atlas;
use crate::board_view::square_top;
use crate::fx::FxSpawner;
use crate::game::GameState;
use crate::hud::{INK_WOOD, integer_scaled_size, spell_item_id, spell_name};
use crate::loading::AppState;

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

fn ease_out_back(x: f32) -> f32 {
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    1.0 + c3 * (x - 1.0).powi(3) + c1 * (x - 1.0).powi(2)
}

fn spawn_banner(commands: &mut Commands, atlas: &Atlas, title_font: &TitleFont, spell: SpellId, win: Vec2) {
    let compact = win.x < 600.0;
    let full_h = (win.y * if compact { 0.26 } else { 0.34 }).max(180.0);
    let card_h = (full_h * 0.82).round();
    let card_w = (card_h * 59.0 / 81.0).round();
    let art_name = format!("art_{}", spell_item_id(spell));
    let (art_size, _) = integer_scaled_size(atlas, &art_name, card_h * 0.40);
    let (label_px, title_px, desc_px) = if compact { (18.0, 40.0, 18.0) } else { (24.0, 72.0, 24.0) };
    let from_x = -(card_w + 60.0);
    let subtitle = spell.effect_text();

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
                border: UiRect::vertical(Val::Px(3.0)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.01, 0.03, 0.82)),
            BorderColor::all(Color::srgb(0.85, 0.18, 0.15)),
        ))
        .with_children(|strip| {
            // The spell card, sliding in from the left.
            strip
                .spawn((
                    BannerCard { from_x },
                    UiTransform { translation: Val2::px(from_x, 0.0), ..default() },
                    Node { width: Val::Px(card_w), height: Val::Px(card_h), flex_shrink: 0.0, ..default() },
                    ImageNode {
                        image: atlas.image.clone(),
                        rect: Some(atlas.rect("gui_card_spell")),
                        image_mode: NodeImageMode::Stretch,
                        ..default()
                    },
                ))
                .with_children(|card| {
                    card.spawn(Node {
                        position_type: PositionType::Absolute,
                        left: Val::Percent(12.0),
                        width: Val::Percent(76.0),
                        top: Val::Percent(8.0),
                        height: Val::Percent(42.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        overflow: Overflow::clip(),
                        ..default()
                    })
                    .with_child((
                        Node { width: Val::Px(art_size.x), height: Val::Px(art_size.y), ..default() },
                        ImageNode {
                            image: atlas.image.clone(),
                            rect: Some(atlas.rect(&art_name)),
                            image_mode: NodeImageMode::Auto,
                            ..default()
                        },
                    ));
                    card.spawn(Node {
                        position_type: PositionType::Absolute,
                        left: Val::Percent(10.0),
                        width: Val::Percent(80.0),
                        top: Val::Percent(52.0),
                        height: Val::Percent(44.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        overflow: Overflow::clip(),
                        ..default()
                    })
                    .with_child((
                        Text::new(spell_name(spell)),
                        TextFont { font_size: FontSize::Px((card_h * 0.10).round().max(12.0)), ..default() },
                        TextColor(crate::hud::INK_PARCHMENT),
                        TextLayout { justify: Justify::Center, linebreak: LineBreak::WordBoundary },
                    ));
                });

            // "Enemy casts", the spell name and what it does.
            strip
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    row_gap: Val::Px(if compact { 2.0 } else { 6.0 }),
                    flex_shrink: 1.0,
                    max_width: Val::Vw(if compact { 58.0 } else { 46.0 }),
                    ..default()
                })
                .with_children(|col| {
                    col.spawn((
                        Text::new("Enemy casts"),
                        TextFont { font_size: FontSize::Px(label_px), ..default() },
                        TextColor(Color::srgb(1.0, 0.55, 0.50)),
                    ));
                    col.spawn((
                        BannerTitle,
                        Text::new(spell_name(spell).to_uppercase()),
                        TextFont {
                            font: FontSource::Handle(title_font.0.clone()),
                            font_size: FontSize::Px(title_px),
                            ..default()
                        },
                        TextColor(INK_WOOD),
                        TextLayout { linebreak: LineBreak::WordBoundary, ..default() },
                    ));
                    col.spawn((
                        Text::new(subtitle),
                        TextFont { font_size: FontSize::Px(desc_px), ..default() },
                        TextColor(Color::srgb(0.92, 0.88, 0.80)),
                        TextLayout { linebreak: LineBreak::WordBoundary, ..default() },
                    ));
                });
        });
}

/// Opens the banner for a held-back AI cast, animates it, and releases the cast when it closes.
#[allow(clippy::too_many_arguments)]
fn run_banner(
    mut commands: Commands,
    time: Res<Time>,
    mouse: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    atlas: Res<Atlas>,
    title_font: Res<TitleFont>,
    mut state: ResMut<GameState>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut banners: Query<(Entity, &mut Banner, &mut Node)>,
    mut cards: Query<(&BannerCard, &mut UiTransform), Without<BannerTitle>>,
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
        spawn_banner(&mut commands, &atlas, &title_font, spell, win);
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
    for (card, mut tf) in &mut cards {
        tf.translation = Val2::px(card.from_x * (1.0 - ease_out_back(slide)), 0.0);
    }
    // Name punches in from 1.4× to 1×.
    let punch = ((t - 0.15) / 0.2).clamp(0.0, 1.0);
    for mut tf in &mut titles {
        tf.scale = Vec2::splat(1.4 - 0.4 * punch);
    }
}

fn setup_pill(mut commands: Commands, atlas: Res<Atlas>) {
    let (icon, _) = integer_scaled_size(&atlas, "icon_skull", 20.0);
    commands
        .spawn((
            LastSpellPill,
            GlobalZIndex(5),
            Node {
                display: Display::None,
                position_type: PositionType::Absolute,
                left: Val::Px(16.0),
                top: Val::Px(72.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                padding: UiRect::axes(Val::Px(12.0), Val::Px(4.0)),
                border_radius: BorderRadius::all(Val::Px(12.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.04, 0.06, 0.75)),
            Button,
        ))
        .with_children(|row| {
            row.spawn((
                Node { width: Val::Px(icon.x), height: Val::Px(icon.y), ..default() },
                ImageNode {
                    image: atlas.image.clone(),
                    rect: Some(atlas.rect("icon_skull")),
                    image_mode: NodeImageMode::Auto,
                    ..default()
                },
            ));
            row.spawn((
                LastSpellText,
                Text::new(""),
                TextFont { font_size: FontSize::Px(20.0), ..default() },
                TextColor(Color::srgb(1.0, 0.80, 0.74)),
                TextLayout { linebreak: LineBreak::NoWrap, ..default() },
            ));
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
