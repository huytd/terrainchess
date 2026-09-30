use bevy::prelude::*;
use bevy::text::LineHeight;
use bevy::window::PrimaryWindow;
use tc_core::rules::DrawReason;
use tc_core::{Outcome, Side, SpellId};
use tc_run::ItemKind;

use crate::TitleFont;
use crate::atlas::Atlas;
use crate::game::GameState;
use crate::loading::AppState;
use crate::run::{DeckEdit, DeckSlot, PickCard, Run, RunPhase, StartLevel, TitleMenu};

/// Ink colour on light wood.
pub const INK_WOOD: Color = Color::srgb_u8(0xF7, 0xED, 0xD0);
/// Ink colour on aged parchment.
pub const INK_PARCHMENT: Color = Color::srgb_u8(0x3B, 0x2F, 0x2A);

/// Space kept clear between a modal panel and the screen edge.
const OVERLAY_GUTTER: f32 = 16.0;

/// Clamps a preferred panel width so it fits inside the overlay gutter on narrow screens.
fn fit_width(preferred: f32, win_w: f32) -> f32 {
    preferred.min(win_w - 2.0 * OVERLAY_GUTTER).max(0.0)
}

fn button_slicer() -> NodeImageMode {
    NodeImageMode::Sliced(TextureSlicer {
        border: BorderRect::all(6.0),
        center_scale_mode: SliceScaleMode::Stretch,
        sides_scale_mode: SliceScaleMode::Stretch,
        max_corner_scale: 1.0,
    })
}

fn panel_slicer() -> NodeImageMode {
    NodeImageMode::Sliced(TextureSlicer {
        border: BorderRect::all(8.0),
        center_scale_mode: SliceScaleMode::Stretch,
        sides_scale_mode: SliceScaleMode::Stretch,
        max_corner_scale: 1.0,
    })
}

fn banner_slicer() -> NodeImageMode {
    NodeImageMode::Sliced(TextureSlicer {
        border: BorderRect { min_inset: Vec2::new(10.0, 0.0), max_inset: Vec2::new(10.0, 0.0) },
        center_scale_mode: SliceScaleMode::Stretch,
        sides_scale_mode: SliceScaleMode::Stretch,
        max_corner_scale: 1.0,
    })
}

pub fn integer_scaled_size(atlas: &Atlas, name: &str, target_h: f32) -> (Vec2, f32) {
    let px = atlas.px(name);
    let scale = (target_h / px.y).round().max(1.0);
    (px * scale, scale)
}

pub fn spell_item_id(spell: tc_core::SpellId) -> &'static str {
    match spell {
        tc_core::SpellId::RaiseEarth => "raise_earth",
        tc_core::SpellId::LowerEarth => "lower_earth",
        tc_core::SpellId::Freeze => "freeze",
        tc_core::SpellId::Bridge => "bridge",
        tc_core::SpellId::DigTunnel => "dig_tunnel",
        tc_core::SpellId::Shield => "shield",
        tc_core::SpellId::Swap => "swap",
        tc_core::SpellId::Rewind => "rewind",
        tc_core::SpellId::Smite => "smite",
        tc_core::SpellId::Evaporate => "evaporate",
        tc_core::SpellId::Flood => "flood",
        tc_core::SpellId::Featherfall => "featherfall",
        tc_core::SpellId::Curse => "curse",
        tc_core::SpellId::Sprout => "sprout",
        tc_core::SpellId::Blink => "blink",
        tc_core::SpellId::Insight => "insight",
    }
}

pub fn spell_name(spell: tc_core::SpellId) -> &'static str {
    match spell {
        tc_core::SpellId::RaiseEarth => "Raise Earth",
        tc_core::SpellId::LowerEarth => "Lower Earth",
        tc_core::SpellId::Freeze => "Freeze",
        tc_core::SpellId::Shield => "Shield",
        tc_core::SpellId::Swap => "Swap",
        tc_core::SpellId::Bridge => "Bridge",
        tc_core::SpellId::DigTunnel => "Dig Tunnel",
        tc_core::SpellId::Rewind => "Rewind",
        tc_core::SpellId::Smite => "Smite",
        tc_core::SpellId::Evaporate => "Evaporate",
        tc_core::SpellId::Flood => "Flood",
        tc_core::SpellId::Featherfall => "Featherfall",
        tc_core::SpellId::Curse => "Curse",
        tc_core::SpellId::Sprout => "Sprout",
        tc_core::SpellId::Blink => "Blink",
        tc_core::SpellId::Insight => "Insight",
    }
}

#[derive(Component)]
struct FloorBadge;

#[derive(Component)]
struct FloorBadgeText;

#[derive(Component)]
struct FloorBadgeEnemyBox;

#[derive(Component)]
struct FloorBadgeEnemyText;

#[derive(Component)]
struct TitleOverlay;

#[derive(Component)]
struct MenuButton;

#[derive(Component)]
struct LevelButton(u8);

#[derive(Component)]
struct DraftOverlay;

#[derive(Component)]
struct DraftCard {
    slot_idx: usize,
    item_id: String,
}

#[derive(Component)]
struct DraftCardHighlight(usize);

#[derive(Component)]
struct RunOverOverlay;

#[derive(Component)]
struct RetryButton;

#[derive(Component)]
struct LevelsButton;

#[derive(Component)]
struct ClaimButton;

#[derive(Component)]
struct HandBar;

#[derive(Component)]
struct HandCard {
    slot_idx: usize,
    base_angle_deg: f32,
    resting_pivot_offset: Vec2,
    lift_px: f32,
    is_used: bool,
    /// Why the card can't be cast right now, if it can't.
    blocked: Option<&'static str>,
}

#[derive(Component)]
struct HandCardHighlight(usize);

#[derive(Component)]
struct DiscardButton;

#[derive(Component, Clone, Copy)]
struct ButtonVisuals {
    normal: &'static str,
    hover: &'static str,
    pressed: &'static str,
    disabled: Option<&'static str>,
}

impl ButtonVisuals {
    const WOOD: Self = Self {
        normal: "btn_wood",
        hover: "btn_wood_hover",
        pressed: "btn_wood_pressed",
        disabled: Some("btn_wood_disabled"),
    };
    const GOLD: Self = Self {
        normal: "btn_gold",
        hover: "btn_gold_hover",
        pressed: "btn_gold_pressed",
        disabled: Some("btn_gold_disabled"),
    };
    const ROUND: Self = Self {
        normal: "btn_round",
        hover: "btn_round_hover",
        pressed: "btn_round_hover",
        disabled: Some("btn_round_disabled"),
    };
}

#[derive(Component, Default, PartialEq, Eq)]
struct ButtonDisabled(bool);

fn setup_hud(mut commands: Commands, atlas: Res<Atlas>) {
    // Floor badge and enemy modifier in the top-left corner
    let badge_name = "badge_floor";
    let skull_size = atlas.px("icon_skull");

    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            left: Val::Px(16.0),
            top: Val::Px(16.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            align_items: AlignItems::FlexStart,
            ..default()
        })
        .with_children(|col| {
            // Level badge plaque (e.g. "3 · Knight's Field")
            col.spawn((
                FloorBadge,
                Node {
                    min_width: Val::Px(120.0),
                    min_height: Val::Px(48.0),
                    height: Val::Auto,
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(Val::Px(20.0), Val::Px(6.0)),
                    ..default()
                },
                ImageNode {
                    image: atlas.image.clone(),
                    rect: Some(atlas.rect(badge_name)),
                    image_mode: button_slicer(),
                    ..default()
                },
            ))
            .with_children(|badge| {
                badge.spawn((
                    Text::new(""),
                    TextFont { font_size: FontSize::Px(26.0), ..default() },
                    TextColor(INK_WOOD),
                    TextLayout { justify: Justify::Center, linebreak: LineBreak::NoWrap },
                    FloorBadgeText,
                ));
            });

            // Enemy modifier line on panel_gui_glass with icon_skull in front
            col.spawn((
                FloorBadgeEnemyBox,
                Node {
                    display: Display::None,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(8.0),
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                    ..default()
                },
                ImageNode {
                    image: atlas.image.clone(),
                    rect: Some(atlas.rect("panel_gui_glass")),
                    image_mode: panel_slicer(),
                    ..default()
                },
            ))
            .with_children(|row| {
                row.spawn((
                    Node { width: Val::Px(skull_size.x), height: Val::Px(skull_size.y), ..default() },
                    ImageNode {
                        image: atlas.image.clone(),
                        rect: Some(atlas.rect("icon_skull")),
                        image_mode: NodeImageMode::Auto,
                        ..default()
                    },
                ));
                row.spawn((
                    Text::new(""),
                    TextFont { font_size: FontSize::Px(24.0), ..default() },
                    TextColor(INK_WOOD),
                    TextLayout { justify: Justify::Center, linebreak: LineBreak::NoWrap },
                    FloorBadgeEnemyText,
                ));
            });
        });

    // Menu button at the top-right corner
    let (menu_size, _) = integer_scaled_size(&atlas, "btn_round", 35.0);
    let gear_size = atlas.px("icon_gear");
    commands
        .spawn((
            Button,
            Interaction::default(),
            MenuButton,
            ButtonVisuals::ROUND,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(16.0),
                top: Val::Px(16.0),
                width: Val::Px(menu_size.x),
                height: Val::Px(menu_size.y),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ImageNode {
                image: atlas.image.clone(),
                rect: Some(atlas.rect("btn_round")),
                image_mode: NodeImageMode::Stretch,
                ..default()
            },
        ))
        .with_child((
            Node { width: Val::Px(gear_size.x), height: Val::Px(gear_size.y), ..default() },
            ImageNode {
                image: atlas.image.clone(),
                rect: Some(atlas.rect("icon_gear")),
                image_mode: NodeImageMode::Auto,
                ..default()
            },
        ));
}

fn update_floor_badge(
    run: Res<Run>,
    state: Res<GameState>,
    mut q_text: Query<&mut Text, (With<FloorBadgeText>, Without<FloorBadgeEnemyText>)>,
    mut q_enemy_box: Query<&mut Node, (With<FloorBadgeEnemyBox>, Without<FloorBadge>)>,
    mut q_enemy: Query<&mut Text, With<FloorBadgeEnemyText>>,
) {
    let level_name = tc_run::level(run.level)
        .map(|l| format!("{} · {}", l.id, l.name))
        .unwrap_or_else(|| format!("Level {}", run.level));

    for mut text in &mut q_text {
        if text.0 != level_name {
            text.0 = level_name.clone();
        }
    }

    let enemy_label = if !state.enemy_items.is_empty() {
        let names: Vec<_> = state
            .enemy_items
            .iter()
            .map(|id| tc_run::find_item(id).map(|item| item.name.as_str()).unwrap_or(id.as_str()))
            .collect();
        format!("Enemy: {}", names.join(", "))
    } else {
        String::new()
    };

    for mut box_node in &mut q_enemy_box {
        let show = !enemy_label.is_empty();
        box_node.display = if show { Display::Flex } else { Display::None };
    }

    for mut text in &mut q_enemy {
        if text.0 != enemy_label {
            text.0 = enemy_label.clone();
        }
    }
}

/// One line explaining how a match ended, shown on the result screen (the player is White).
fn outcome_reason(outcome: Option<Outcome>) -> &'static str {
    match outcome {
        Some(Outcome::Checkmate { winner: Side::White }) => "Checkmate — White wins",
        Some(Outcome::Checkmate { winner: Side::Black }) => "Checkmate — Black wins",
        Some(Outcome::Draw(DrawReason::Stalemate)) => "Draw by stalemate",
        Some(Outcome::Draw(DrawReason::FiftyMoves)) => "Draw by the fifty-move rule",
        Some(Outcome::Draw(DrawReason::Repetition)) => "Draw by threefold repetition",
        Some(Outcome::Draw(DrawReason::InsufficientMaterial)) => "Draw — insufficient material",
        None => "",
    }
}

#[allow(clippy::too_many_arguments)]
fn sync_overlays(
    mut commands: Commands,
    run: Res<Run>,
    atlas: Res<Atlas>,
    title_font: Res<TitleFont>,
    game_state: Res<GameState>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut last_phase: Local<Option<RunPhase>>,
    draft_query: Query<Entity, With<DraftOverlay>>,
    over_query: Query<Entity, With<RunOverOverlay>>,
) {
    if *last_phase == Some(run.phase.clone()) {
        return;
    }
    *last_phase = Some(run.phase.clone());

    for e in &draft_query {
        commands.entity(e).despawn();
    }
    for e in &over_query {
        commands.entity(e).despawn();
    }

    match &run.phase {
        RunPhase::Playing => {}
        RunPhase::Draft(items) => {
            let win_w = window.iter().next().map(|w| w.width()).unwrap_or(800.0);
            let target_h = if win_w >= 800.0 {
                324.0
            } else if win_w >= 640.0 {
                240.0
            } else {
                win_w * 0.30 * 4.0 / 3.0
            };
            let (card_size, _) = integer_scaled_size(&atlas, "gui_card_common", target_h);
            let card_font_scale: f32 = if win_w < 500.0 {
                0.75
            } else if win_w < 800.0 {
                0.88
            } else {
                1.0
            };

            commands
                .spawn((
                    DraftOverlay,
                    GlobalZIndex(100),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        right: Val::Px(0.0),
                        top: Val::Px(0.0),
                        bottom: Val::Px(0.0),
                        padding: UiRect::all(Val::Px(OVERLAY_GUTTER)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(28.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.72)),
                ))
                .with_children(|parent| {
                    // Title "Choose a reward" on banner
                    parent
                        .spawn((
                            Node {
                                min_width: Val::Px(fit_width(
                                    if win_w < 500.0 { 320.0 } else { 380.0 },
                                    win_w,
                                )),
                                max_width: Val::Percent(100.0),
                                padding: UiRect::axes(
                                    Val::Px(if win_w < 500.0 { 20.0 } else { 36.0 }),
                                    Val::Px(12.0),
                                ),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                overflow: Overflow::visible(),
                                ..default()
                            },
                            ImageNode {
                                image: atlas.image.clone(),
                                rect: Some(atlas.rect("banner")),
                                image_mode: banner_slicer(),
                                ..default()
                            },
                        ))
                        .with_child((
                            Text::new("Choose a reward"),
                            TextFont {
                                font: FontSource::Handle(title_font.0.clone()),
                                font_size: FontSize::Px(if win_w < 500.0 { 40.0 } else { 48.0 }),
                                ..default()
                            },
                            TextColor(INK_WOOD),
                            TextLayout { justify: Justify::Center, ..default() },
                        ));

                    // Row of 3 reward cards
                    parent
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(16.0),
                            margin: UiRect::top(Val::Px(28.0)),
                            padding: UiRect::horizontal(Val::Px(12.0)),
                            max_width: Val::Percent(100.0),
                            ..default()
                        })
                        .with_children(|row| {
                            for (slot_idx, item_id) in items.iter().enumerate() {
                                let Some(item) = tc_run::find_item(item_id) else { continue };
                                let frame_sprite = match item.rarity {
                                    tc_run::Rarity::Common => "gui_card_common",
                                    tc_run::Rarity::Uncommon => "gui_card_uncommon",
                                    tc_run::Rarity::Rare => "gui_card_rare",
                                };
                                let kind_str = match &item.kind {
                                    ItemKind::Enhancement { .. } | ItemKind::Veteran => {
                                        "Enhancement".to_string()
                                    }
                                    ItemKind::Relic { .. } => "Relic".to_string(),
                                    ItemKind::Spell { charges, .. } => {
                                        if *charges == 1 {
                                            "Spell - 1 card".to_string()
                                        } else {
                                            format!("Spell - {charges} cards")
                                        }
                                    }
                                };

                                let art_name = format!("art_{}", item.id);
                                let art_win_h = card_size.y * 0.46;
                                let (art_size, _) = integer_scaled_size(&atlas, &art_name, art_win_h);

                                row.spawn((
                                    Button,
                                    Interaction::default(),
                                    DraftCard { slot_idx, item_id: item.id.clone() },
                                    Node {
                                        width: Val::Px(card_size.x),
                                        height: Val::Px(card_size.y),
                                        position_type: PositionType::Relative,
                                        overflow: Overflow::visible(),
                                        ..default()
                                    },
                                ))
                                .with_children(|card_root| {
                                    // Highlight behind the card, ~10% larger than the card and centred on it
                                    card_root.spawn((
                                        DraftCardHighlight(slot_idx),
                                        Node {
                                            position_type: PositionType::Absolute,
                                            left: Val::Percent(-5.0),
                                            top: Val::Percent(-5.0),
                                            width: Val::Percent(110.0),
                                            height: Val::Percent(110.0),
                                            display: Display::None,
                                            ..default()
                                        },
                                        ImageNode {
                                            image: atlas.image.clone(),
                                            rect: Some(atlas.rect("gui_card_highlight")),
                                            image_mode: NodeImageMode::Stretch,
                                            ..default()
                                        },
                                    ));

                                    // Name plate above the card
                                    card_root.spawn((
                                        Node {
                                            position_type: PositionType::Absolute,
                                            left: Val::Percent(-15.0),
                                            width: Val::Percent(130.0),
                                            bottom: Val::Percent(103.0),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            ..default()
                                        },
                                        children![(
                                            Text::new(&item.name),
                                            TextFont {
                                                font_size: FontSize::Px((26.0 * card_font_scale).round(),),
                                                ..default()
                                            },
                                            TextColor(INK_WOOD),
                                            TextShadow::default(),
                                            TextLayout {
                                                justify: Justify::Center,
                                                linebreak: LineBreak::WordBoundary,
                                            },
                                        )],
                                    ));

                                    // Card body on top of highlight
                                    card_root
                                        .spawn((
                                            Node {
                                                position_type: PositionType::Absolute,
                                                left: Val::Px(0.0),
                                                top: Val::Px(0.0),
                                                width: Val::Percent(100.0),
                                                height: Val::Percent(100.0),
                                                overflow: Overflow::clip(),
                                                ..default()
                                            },
                                            ImageNode {
                                                image: atlas.image.clone(),
                                                rect: Some(atlas.rect(frame_sprite)),
                                                image_mode: NodeImageMode::Auto,
                                                ..default()
                                            },
                                        ))
                                        .with_children(|card| {
                                            // 1. Art window: spans x 12% - 88%, y 8% - 50%
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
                                                Node {
                                                    width: Val::Px(art_size.x),
                                                    height: Val::Px(art_size.y),
                                                    ..default()
                                                },
                                                ImageNode {
                                                    image: atlas.image.clone(),
                                                    rect: Some(atlas.rect(&art_name)),
                                                    image_mode: NodeImageMode::Auto,
                                                    ..default()
                                                },
                                            ));

                                            // 2. Parchment text box: spans x 12% - 88%, y 52% - 94%
                                            card.spawn(Node {
                                                position_type: PositionType::Absolute,
                                                left: Val::Percent(10.0),
                                                width: Val::Percent(80.0),
                                                top: Val::Percent(52.0),
                                                height: Val::Percent(44.0),
                                                flex_direction: FlexDirection::Column,
                                                align_items: AlignItems::Center,
                                                justify_content: JustifyContent::Center,
                                                overflow: Overflow::clip(),
                                                row_gap: Val::Px(2.0),
                                                padding: UiRect::all(Val::Px(2.0)),
                                                ..default()
                                            })
                                            .with_children(
                                                |tb| {
                                                    tb.spawn((
                                                        Text::new(kind_str),
                                                        TextFont {
                                                            font_size: FontSize::Px(
                                                                (24.0 * card_font_scale).round(),
                                                            ),
                                                            ..default()
                                                        },
                                                        TextColor(INK_PARCHMENT),
                                                        TextLayout {
                                                            justify: Justify::Center,
                                                            linebreak: LineBreak::WordBoundary,
                                                        },
                                                    ));
                                                    tb.spawn((
                                                        Text::new(&item.description),
                                                        TextFont {
                                                            font_size: FontSize::Px(
                                                                (22.0 * card_font_scale).round(),
                                                            ),
                                                            ..default()
                                                        },
                                                        TextColor(INK_PARCHMENT),
                                                        TextLayout {
                                                            justify: Justify::Center,
                                                            linebreak: LineBreak::WordBoundary,
                                                        },
                                                        LineHeight::RelativeToFont(1.05),
                                                    ));
                                                },
                                            );
                                        });
                                });
                            }
                        });
                });
        }
        RunPhase::Result { won } => {
            let badge_name = if *won { "badge_victory" } else { "badge_defeat" };
            let (badge_size, _) = integer_scaled_size(&atlas, badge_name, 60.0);
            let win_w = window.iter().next().map(|w| w.width()).unwrap_or(800.0);
            let is_compact = win_w < 500.0;

            commands
                .spawn((
                    RunOverOverlay,
                    GlobalZIndex(100),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        right: Val::Px(0.0),
                        top: Val::Px(0.0),
                        bottom: Val::Px(0.0),
                        padding: UiRect::all(Val::Px(OVERLAY_GUTTER)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.72)),
                ))
                .with_children(|parent| {
                    parent
                        .spawn((
                            Node {
                                padding: UiRect::axes(
                                    Val::Px(if is_compact { 20.0 } else { 28.0 }),
                                    Val::Px(if is_compact { 20.0 } else { 28.0 }),
                                ),
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                row_gap: Val::Px(if is_compact { 12.0 } else { 16.0 }),
                                min_width: Val::Px(fit_width(360.0, win_w)),
                                max_width: Val::Percent(100.0),
                                max_height: Val::Percent(100.0),
                                overflow: Overflow::scroll_y(),
                                min_height: Val::Px(260.0),
                                ..default()
                            },
                            ImageNode {
                                image: atlas.image.clone(),
                                rect: Some(atlas.rect("panel_gui_parchment_curl")),
                                image_mode: panel_slicer(),
                                ..default()
                            },
                        ))
                        .with_children(|panel| {
                            // Badge above the title
                            panel.spawn((
                                Node {
                                    width: Val::Px(badge_size.x),
                                    height: Val::Px(badge_size.y),
                                    ..default()
                                },
                                ImageNode {
                                    image: atlas.image.clone(),
                                    rect: Some(atlas.rect(badge_name)),
                                    image_mode: NodeImageMode::Auto,
                                    ..default()
                                },
                            ));

                            // Title; a draw still counts as a loss but gets its own title.
                            let is_draw = matches!(game_state.outcome, Some(Outcome::Draw(_)));
                            panel.spawn((
                                Text::new(if *won {
                                    "Victory!"
                                } else if is_draw {
                                    "Draw"
                                } else {
                                    "Defeat"
                                }),
                                TextFont {
                                    font: FontSource::Handle(title_font.0.clone()),
                                    font_size: FontSize::Px(if is_compact { 40.0 } else { 48.0 }),
                                    ..default()
                                },
                                TextColor(INK_PARCHMENT),
                                TextLayout { justify: Justify::Center, ..default() },
                            ));

                            // Why the match ended.
                            let reason = outcome_reason(game_state.outcome);
                            if !reason.is_empty() {
                                panel.spawn((
                                    Text::new(reason),
                                    TextFont {
                                        font_size: FontSize::Px(if is_compact { 22.0 } else { 26.0 }),
                                        ..default()
                                    },
                                    TextColor(INK_PARCHMENT),
                                    TextLayout {
                                        justify: Justify::Center,
                                        linebreak: LineBreak::WordBoundary,
                                    },
                                ));
                            }

                            let level_name = tc_run::level(run.level).map(|l| l.name).unwrap_or("Level");
                            panel.spawn((
                                Text::new(if *won {
                                    format!("{level_name} cleared!")
                                } else {
                                    level_name.to_string()
                                }),
                                TextFont {
                                    font_size: FontSize::Px(if is_compact { 22.0 } else { 26.0 }),
                                    ..default()
                                },
                                TextColor(INK_PARCHMENT),
                                TextLayout { justify: Justify::Center, linebreak: LineBreak::WordBoundary },
                            ));

                            // A win with a reward waiting: one button to claim it.
                            if *won && run.pending_draft.is_some() {
                                panel
                                    .spawn((
                                        Button,
                                        Interaction::default(),
                                        ClaimButton,
                                        ButtonVisuals::GOLD,
                                        Node {
                                            min_width: Val::Px(220.0),
                                            min_height: Val::Px(52.0),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            padding: UiRect::axes(Val::Px(16.0), Val::Px(4.0)),
                                            margin: UiRect::top(Val::Px(8.0)),
                                            ..default()
                                        },
                                        ImageNode {
                                            image: atlas.image.clone(),
                                            rect: Some(atlas.rect("btn_gold")),
                                            image_mode: button_slicer(),
                                            ..default()
                                        },
                                    ))
                                    .with_child((
                                        Text::new("Claim reward"),
                                        TextFont { font_size: FontSize::Px(26.0), ..default() },
                                        TextColor(INK_WOOD),
                                        TextLayout { justify: Justify::Center, linebreak: LineBreak::NoWrap },
                                    ));
                                return;
                            }

                            // Action buttons row: Retry and Levels
                            panel
                                .spawn(Node {
                                    flex_direction: FlexDirection::Row,
                                    flex_wrap: FlexWrap::Wrap,
                                    column_gap: Val::Px(16.0),
                                    row_gap: Val::Px(8.0),
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    max_width: Val::Percent(100.0),
                                    margin: UiRect::top(Val::Px(8.0)),
                                    ..default()
                                })
                                .with_children(|btn_row| {
                                    btn_row
                                        .spawn((
                                            Button,
                                            Interaction::default(),
                                            RetryButton,
                                            ButtonVisuals::GOLD,
                                            Node {
                                                width: Val::Px(140.0),
                                                min_height: Val::Px(52.0),
                                                justify_content: JustifyContent::Center,
                                                align_items: AlignItems::Center,
                                                padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                                                ..default()
                                            },
                                            ImageNode {
                                                image: atlas.image.clone(),
                                                rect: Some(atlas.rect("btn_gold")),
                                                image_mode: button_slicer(),
                                                ..default()
                                            },
                                        ))
                                        .with_child((
                                            Text::new("Retry"),
                                            TextFont { font_size: FontSize::Px(26.0), ..default() },
                                            TextColor(INK_WOOD),
                                            TextLayout {
                                                justify: Justify::Center,
                                                linebreak: LineBreak::NoWrap,
                                            },
                                        ));

                                    btn_row
                                        .spawn((
                                            Button,
                                            Interaction::default(),
                                            LevelsButton,
                                            ButtonVisuals::WOOD,
                                            Node {
                                                width: Val::Px(140.0),
                                                min_height: Val::Px(52.0),
                                                justify_content: JustifyContent::Center,
                                                align_items: AlignItems::Center,
                                                padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                                                ..default()
                                            },
                                            ImageNode {
                                                image: atlas.image.clone(),
                                                rect: Some(atlas.rect("btn_wood")),
                                                image_mode: button_slicer(),
                                                ..default()
                                            },
                                        ))
                                        .with_child((
                                            Text::new("Levels"),
                                            TextFont { font_size: FontSize::Px(26.0), ..default() },
                                            TextColor(INK_WOOD),
                                            TextLayout {
                                                justify: Justify::Center,
                                                linebreak: LineBreak::NoWrap,
                                            },
                                        ));
                                });
                        });
                });
        }
    }
}

fn update_draft_card_sizes(
    window: Query<&Window, (With<PrimaryWindow>, Changed<Window>)>,
    atlas: Res<Atlas>,
    mut cards: Query<&mut Node, With<DraftCard>>,
) {
    let Some(win) = window.iter().next() else { return };
    let target_h = if win.width() >= 800.0 {
        324.0
    } else if win.width() >= 640.0 {
        240.0
    } else {
        win.width() * 0.30 * 4.0 / 3.0
    };
    let (card_size, _) = integer_scaled_size(&atlas, "gui_card_common", target_h);
    for mut node in &mut cards {
        node.width = Val::Px(card_size.x);
        node.height = Val::Px(card_size.y);
    }
}

#[allow(clippy::too_many_arguments)]
fn sync_hand_bar(
    mut commands: Commands,
    state: Res<GameState>,
    run: Res<Run>,
    title_menu: Res<TitleMenu>,
    atlas: Res<Atlas>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut last_key: Local<
        Option<(
            bool,
            [Option<tc_core::SpellId>; 3],
            [bool; 3],
            [bool; 3],
            usize,
            u8,
            bool,
            Option<usize>,
            (u32, u32),
        )>,
    >,
    hand_bar_query: Query<Entity, With<HandBar>>,
) {
    let side = state.game.pos.side_to_move;
    let show = !title_menu.open
        && run.phase == RunPhase::Playing
        && !state.ai_to_move()
        && state.outcome.is_none()
        && side == Side::White;

    let hand = state.game.hand(side);
    let hand_cards = hand.hand;
    let hand_used = hand.used;
    let deck_len = state.game.deck_len(side);
    let discards_left = state.game.discards_left(side);
    let armed_slot = state.armed_slot;
    let can_discard = armed_slot.is_some_and(|slot| state.game.can_discard(side, slot));
    let blocks: [Option<&'static str>; 3] = std::array::from_fn(|i| match hand_cards[i] {
        Some(spell) if !hand_used[i] => state.game.cast_block(spell).map(|b| b.message()),
        _ => None,
    });
    let hand_blocked = blocks.map(|b| b.is_some());

    let win = window.iter().next();
    let (win_w, win_h) = win.map(|w| (w.width(), w.height())).unwrap_or((1280.0, 800.0));
    let win_dim = (win_w as u32, win_h as u32);

    let current_key = (
        show,
        hand_cards,
        hand_used,
        hand_blocked,
        deck_len,
        discards_left,
        can_discard,
        armed_slot,
        win_dim,
    );

    if *last_key == Some(current_key) {
        return;
    }
    *last_key = Some(current_key);

    for e in &hand_bar_query {
        commands.entity(e).despawn();
    }

    if !show {
        return;
    }

    // Integer scaling for cards:
    // Fits 3 cards within phone width; desktop card height ~30% of window height.
    let max_scale_w = ((win_w * 0.90) / (2.7 * 59.0)).floor() as u32;
    let target_scale_h = ((win_h * 0.30) / 81.0).round() as u32;
    let scale = target_scale_h.min(max_scale_w).max(1);

    let card_w = 59.0 * scale as f32;
    let card_h = 81.0 * scale as f32;
    let art_win_h = card_h * 0.46;

    // Resting card position: about 20% of card bottom is off screen.
    // Clamped so fan never covers more than bottom ~26% of screen.
    let max_visible_h = win_h * 0.26;
    let base_offscreen = 0.20 * card_h;
    let mut resting_bottom = -base_offscreen;
    let resting_visible_h = card_h + resting_bottom;
    if resting_visible_h > max_visible_h {
        resting_bottom -= resting_visible_h - max_visible_h;
    }
    let lift_px = 0.18 * card_h;

    let present_slots: Vec<usize> = (0..3).filter(|&i| hand_cards[i].is_some()).collect();
    let n = present_slots.len();

    let overlap = if win_w < 500.0 { 0.37 } else { 0.15 };
    let dx = (1.0 - overlap) * card_w;

    let sidebar_w = if win_w < 600.0 { 108.0 } else { 120.0 };
    let is_compact = win_w < 600.0;
    let hand_font_size = if is_compact { 20.0 } else { 26.0 };
    let hand_sub_font_size = if is_compact { 18.0 } else { 22.0 };
    let discard_font_size = if is_compact { 16.0 } else { 20.0 };
    // Fixed button width: an auto-sized button in this absolutely positioned
    // column measured narrower than its label, so the text spilled out.
    let discard_btn_w = if is_compact { 108.0 } else { 136.0 };
    let deck_font_size = if is_compact { 22.0 } else { 26.0 };
    let center_x = if win_w < 600.0 {
        let sidebar_zone = sidebar_w + 6.0 + 8.0;
        (win_w - sidebar_zone) * 0.5
    } else {
        win_w * 0.5
    };

    commands
        .spawn((
            HandBar,
            GlobalZIndex(10),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                bottom: Val::Px(0.0),
                height: Val::Px(card_h + lift_px),
                overflow: Overflow::visible(),
                ..default()
            },
        ))
        .with_children(|root| {
            // Spawn fanned cards
            for (k, &slot_idx) in present_slots.iter().enumerate() {
                let Some(spell) = hand_cards[slot_idx] else { continue };
                let is_used = hand_used[slot_idx];
                let blocked = blocks[slot_idx];
                let is_armed = armed_slot == Some(slot_idx);

                let (base_angle_deg, drop_y) = match n {
                    3 => match k {
                        0 => (-6.0, 0.04 * card_h),
                        1 => (0.0, 0.0),
                        _ => (6.0, 0.04 * card_h),
                    },
                    2 => match k {
                        0 => (-4.0, 0.03 * card_h),
                        _ => (4.0, 0.03 * card_h),
                    },
                    _ => (0.0, 0.0),
                };

                let pivot = Vec2::new(0.0, 0.5 * card_h);
                let rot = Rot2::degrees(base_angle_deg);
                let resting_pivot_offset = pivot - rot * pivot;

                let card_center_x = center_x + (k as f32 - (n as f32 - 1.0) * 0.5) * dx;
                let card_left = card_center_x - 0.5 * card_w;
                let card_bottom = resting_bottom - drop_y;

                let (rotation, translation, z_idx) = if is_armed {
                    (Rot2::IDENTITY, Val2::px(0.0, -lift_px), ZIndex(10))
                } else {
                    (rot, Val2::px(resting_pivot_offset.x, resting_pivot_offset.y), ZIndex(k as i32 + 1))
                };

                let art_name = format!("art_{}", spell_item_id(spell));
                let (art_size, _) = integer_scaled_size(&atlas, &art_name, art_win_h);

                root.spawn((
                    Button,
                    Interaction::default(),
                    HandCard { slot_idx, base_angle_deg, resting_pivot_offset, lift_px, is_used, blocked },
                    UiTransform { translation, rotation, ..default() },
                    z_idx,
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(card_left),
                        bottom: Val::Px(card_bottom),
                        width: Val::Px(card_w),
                        height: Val::Px(card_h),
                        overflow: Overflow::visible(),
                        ..default()
                    },
                ))
                .with_children(|card_root| {
                    // Highlight behind card
                    card_root.spawn((
                        HandCardHighlight(slot_idx),
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Percent(-5.0),
                            top: Val::Percent(-5.0),
                            width: Val::Percent(110.0),
                            height: Val::Percent(110.0),
                            display: if is_armed { Display::Flex } else { Display::None },
                            ..default()
                        },
                        ImageNode {
                            image: atlas.image.clone(),
                            rect: Some(atlas.rect("gui_card_highlight")),
                            image_mode: NodeImageMode::Stretch,
                            ..default()
                        },
                    ));

                    // Card body
                    card_root
                        .spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(0.0),
                                top: Val::Px(0.0),
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                overflow: Overflow::clip(),
                                ..default()
                            },
                            ImageNode {
                                image: atlas.image.clone(),
                                rect: Some(atlas.rect("gui_card_spell")),
                                image_mode: NodeImageMode::Auto,
                                ..default()
                            },
                        ))
                        .with_children(|card| {
                            // Art window: spans x 12%-88%, y 8%-50%
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

                            // Parchment text box: spans x 10%-90%, y 52%-96%
                            card.spawn(Node {
                                position_type: PositionType::Absolute,
                                left: Val::Percent(10.0),
                                width: Val::Percent(80.0),
                                top: Val::Percent(52.0),
                                height: Val::Percent(44.0),
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                overflow: Overflow::clip(),
                                row_gap: Val::Px(2.0),
                                padding: UiRect::all(Val::Px(2.0)),
                                ..default()
                            })
                            .with_children(|tb| {
                                tb.spawn((
                                    Text::new(spell_name(spell)),
                                    TextFont { font_size: FontSize::Px(hand_font_size), ..default() },
                                    TextColor(INK_PARCHMENT),
                                    TextLayout {
                                        justify: Justify::Center,
                                        linebreak: LineBreak::WordBoundary,
                                    },
                                ));
                                let subtext = if is_used {
                                    "Used".to_string()
                                } else if spell.is_quick() {
                                    format!("[{}] Quick", slot_idx + 1)
                                } else {
                                    format!("[{}]", slot_idx + 1)
                                };
                                tb.spawn((
                                    Text::new(subtext),
                                    TextFont { font_size: FontSize::Px(hand_sub_font_size), ..default() },
                                    TextColor(INK_PARCHMENT),
                                    TextLayout { justify: Justify::Center, linebreak: LineBreak::NoWrap },
                                ));
                            });

                            if is_used {
                                card.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: Val::Px(0.0),
                                        top: Val::Px(0.0),
                                        right: Val::Px(0.0),
                                        bottom: Val::Px(0.0),
                                        ..default()
                                    },
                                    ImageNode {
                                        image: atlas.image.clone(),
                                        rect: Some(atlas.rect("gui_card_used")),
                                        color: Color::srgba(1.0, 1.0, 1.0, 0.70),
                                        image_mode: NodeImageMode::Stretch,
                                        ..default()
                                    },
                                ));
                            } else if blocked.is_some() {
                                // Can't be cast now: dim the card and pin a red badge on it.
                                card.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: Val::Px(0.0),
                                        top: Val::Px(0.0),
                                        right: Val::Px(0.0),
                                        bottom: Val::Px(0.0),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.05, 0.03, 0.08, 0.50)),
                                ));
                                let badge = 11.0 * scale as f32;
                                card.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        right: Val::Percent(9.0),
                                        top: Val::Percent(6.0),
                                        width: Val::Px(badge),
                                        height: Val::Px(badge),
                                        ..default()
                                    },
                                    ImageNode {
                                        image: atlas.image.clone(),
                                        rect: Some(atlas.rect("icon_close")),
                                        color: Color::srgb(1.0, 0.45, 0.40),
                                        image_mode: NodeImageMode::Stretch,
                                        ..default()
                                    },
                                ));
                            }
                        });
                });
            }

            // To the right: Discard button above, deck pile below (bottom-right)
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(if win_w < 600.0 { 6.0 } else { 24.0 }),
                    bottom: Val::Px(16.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::FlexEnd,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(8.0),
                    ..default()
                },
                GlobalZIndex(15),
            ))
            .with_children(|sidebar| {
                // 1. Discard button: a real wood button (9-sliced background with
                // hover/pressed/disabled swapping via `ButtonVisuals`) holding the
                // small discard icon on the left and the count label on the right.
                // Auto-sized to its content with padding so the text never
                // overflows; kept compact so it clears the hand fan on phones.
                let (btn_initial, text_color) = if can_discard {
                    ("btn_wood", INK_WOOD)
                } else {
                    ("btn_wood_disabled", Color::srgba(0.97, 0.93, 0.82, 0.40))
                };

                sidebar
                    .spawn((
                        Button,
                        Interaction::default(),
                        DiscardButton,
                        ButtonVisuals::WOOD,
                        ButtonDisabled(!can_discard),
                        Node {
                            width: Val::Px(discard_btn_w),
                            min_height: Val::Px(36.0),
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(6.0),
                            padding: UiRect::axes(Val::Px(8.0), Val::Px(6.0)),
                            ..default()
                        },
                        ImageNode {
                            image: atlas.image.clone(),
                            rect: Some(atlas.rect(btn_initial)),
                            image_mode: button_slicer(),
                            ..default()
                        },
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Node {
                                width: Val::Px(34.0),
                                height: Val::Px(16.0),
                                flex_shrink: 0.0,
                                ..default()
                            },
                            ImageNode {
                                image: atlas.image.clone(),
                                rect: Some(atlas.rect("icon_discard")),
                                image_mode: NodeImageMode::Stretch,
                                ..default()
                            },
                        ));
                        btn.spawn((
                            Text::new(format!("Discard {discards_left}/{}", tc_core::MAX_DISCARDS)),
                            TextFont { font_size: FontSize::Px(discard_font_size), ..default() },
                            TextColor(text_color),
                            TextLayout { justify: Justify::Center, linebreak: LineBreak::NoWrap },
                        ));
                    });

                // 2. Deck pile with count drawn on it in light ink
                let (deck_size, _) = integer_scaled_size(&atlas, "deck_pile", 46.0);
                sidebar
                    .spawn((
                        Node {
                            width: Val::Px(deck_size.x),
                            height: Val::Px(deck_size.y),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        ImageNode {
                            image: atlas.image.clone(),
                            rect: Some(atlas.rect("deck_pile")),
                            image_mode: NodeImageMode::Auto,
                            ..default()
                        },
                    ))
                    .with_child((
                        Text::new(deck_len.to_string()),
                        TextFont { font_size: FontSize::Px(deck_font_size), ..default() },
                        TextColor(INK_WOOD),
                        TextLayout { justify: Justify::Center, linebreak: LineBreak::NoWrap },
                    ));
            });
        });
}

fn handle_hand_card_interaction(
    mut card_query: Query<
        (&Interaction, &mut UiTransform, &mut ZIndex, &HandCard),
        (With<Button>, Or<(Changed<Interaction>, Added<HandCard>)>),
    >,
    mut highlight_query: Query<(&mut Node, &HandCardHighlight), Without<HandCard>>,
    mut state: ResMut<GameState>,
) {
    for (interaction, mut transform, mut z_index, card) in &mut card_query {
        let is_armed = state.armed_slot == Some(card.slot_idx);
        let is_hovered = *interaction == Interaction::Hovered;
        let is_pressed = *interaction == Interaction::Pressed;

        if is_pressed {
            state.arm_slot(card.slot_idx);
        } else if is_hovered && let Some(reason) = card.blocked {
            state.toast = Some(reason.to_string());
        }

        let active = !card.is_used && (is_armed || is_hovered || is_pressed);

        if active {
            transform.rotation = Rot2::IDENTITY;
            transform.translation = Val2::px(0.0, -card.lift_px);
            *z_index = ZIndex(10);
        } else {
            transform.rotation = Rot2::degrees(card.base_angle_deg);
            transform.translation = Val2::px(card.resting_pivot_offset.x, card.resting_pivot_offset.y);
            *z_index = ZIndex(card.slot_idx as i32 + 1);
        }

        for (mut hl_node, hl) in &mut highlight_query {
            if hl.0 == card.slot_idx {
                hl_node.display = if active { Display::Flex } else { Display::None };
            }
        }
    }
}

fn handle_discard_button_interaction(
    button_query: Query<(&Interaction, Option<&ButtonDisabled>), (Changed<Interaction>, With<DiscardButton>)>,
    mut state: ResMut<GameState>,
) {
    for (interaction, disabled) in &button_query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if disabled.is_some_and(|d| d.0) {
            let side = state.game.pos.side_to_move;
            state.toast = Some(
                if state.armed_slot.is_none() {
                    "Pick a card to discard first"
                } else if state.game.discards_left(side) == 0 {
                    "No discards left"
                } else {
                    "Can't discard that card"
                }
                .to_string(),
            );
        } else {
            state.discard();
        }
    }
}

fn handle_card_interaction(
    mut card_query: Query<(&Interaction, &mut Node, &DraftCard), (Changed<Interaction>, With<Button>)>,
    mut highlight_query: Query<(&mut Node, &DraftCardHighlight), Without<DraftCard>>,
    mut pick_writer: MessageWriter<PickCard>,
) {
    for (interaction, mut node, card) in &mut card_query {
        let is_hovered = *interaction == Interaction::Hovered;
        let is_pressed = *interaction == Interaction::Pressed;

        if is_hovered || is_pressed {
            node.top = Val::Px(-8.0);
        } else {
            node.top = Val::Px(0.0);
        }

        for (mut hl_node, hl) in &mut highlight_query {
            if hl.0 == card.slot_idx {
                hl_node.display = if is_hovered || is_pressed { Display::Flex } else { Display::None };
            }
        }

        if is_pressed {
            pick_writer.write(PickCard(card.item_id.clone()));
        }
    }
}

#[derive(Component)]
struct PrepareCard {
    slot: DeckSlot,
    spell: SpellId,
}

#[derive(Component)]
struct PrepareDetail;

#[derive(Component)]
struct PrepareBack;

#[derive(Component)]
struct PrepareShuffle;

#[derive(Component)]
struct PrepareReset;

#[derive(Component)]
struct PrepareStart;

/// What the title panel was last built from; any change rebuilds it.
type TitleKey = (bool, Option<u8>, Option<DeckSlot>, Vec<SpellId>, bool, (u32, u32));

/// Pieces per side on a level: its army, or the standard set for its board size.
fn level_piece_count(level: &tc_run::Level) -> usize {
    level.army.map(|a| a.len()).unwrap_or(match level.size {
        16 => 32,
        _ => 16,
    })
}

/// The held card on the Prepare screen, if any.
fn held_spell(held: Option<DeckSlot>, deck: &[SpellId], reserve: &[SpellId]) -> Option<SpellId> {
    match held? {
        DeckSlot::Deck(i) => deck.get(i).copied(),
        DeckSlot::Reserve(r) => reserve.get(r).copied(),
    }
}

/// A card's name and effect, for the Prepare screen's detail line.
fn card_detail(spell: SpellId) -> String {
    let quick = if spell.is_quick() { " (Quick: doesn't use your turn)" } else { "" };
    format!("{} — {}{quick}", spell_name(spell), spell.effect_text())
}

/// The Prepare screen's detail line when no card is hovered.
fn prepare_hint(held: Option<SpellId>, shuffle: bool) -> String {
    match held {
        Some(spell) => format!("Tap another card to swap it with {}.", spell_name(spell)),
        None if shuffle => "Shuffled at the start of each match. Tap two cards to swap them.".into(),
        None => "Drawn in this order — cards 1–3 are your opening hand.".into(),
    }
}

#[allow(clippy::too_many_arguments)]
fn sync_title_menu(
    mut commands: Commands,
    title_menu: Res<TitleMenu>,
    run: Res<Run>,
    atlas: Res<Atlas>,
    title_font: Res<TitleFont>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut last: Local<Option<TitleKey>>,
    overlay_query: Query<Entity, With<TitleOverlay>>,
) {
    let (win_w, win_h) = window.iter().next().map(|w| (w.width(), w.height())).unwrap_or((800.0, 600.0));
    let deck = if title_menu.prepare.is_some() { run.profile.deck() } else { Vec::new() };
    let key: TitleKey = (
        title_menu.open,
        title_menu.prepare,
        title_menu.held,
        deck,
        run.profile.shuffle_deck,
        (win_w as u32, win_h as u32),
    );
    if last.as_ref() == Some(&key) {
        return;
    }
    *last = Some(key);

    for e in &overlay_query {
        commands.entity(e).despawn();
    }

    if !title_menu.open {
        return;
    }

    let is_compact = win_w < 500.0;
    let panel_padding = if is_compact { 12.0 } else { 20.0 };

    commands
        .spawn((
            TitleOverlay,
            GlobalZIndex(200),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(OVERLAY_GUTTER)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.72)),
        ))
        .with_children(|overlay| {
            // Main stone panel
            overlay
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(8.0),
                        padding: UiRect::all(Val::Px(panel_padding)),
                        max_width: Val::Percent(100.0),
                        max_height: Val::Percent(100.0),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                    ImageNode {
                        image: atlas.image.clone(),
                        rect: Some(atlas.rect("panel_gui_stone")),
                        image_mode: panel_slicer(),
                        ..default()
                    },
                ))
                .with_children(|parent| match title_menu.prepare.and_then(tc_run::level) {
                    Some(level) => {
                        spawn_prepare(parent, level, &run, title_menu.held, &atlas, &title_font, win_w)
                    }
                    None => spawn_level_select(parent, &run, &atlas, &title_font, win_w),
                });
        });
}

/// A title banner with large light ink, as on the level select.
fn spawn_banner(
    parent: &mut ChildSpawnerCommands,
    text: String,
    width: f32,
    font_px: f32,
    atlas: &Atlas,
    title_font: &TitleFont,
) {
    parent
        .spawn((
            Node {
                width: Val::Px(width),
                height: Val::Px(52.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ImageNode {
                image: atlas.image.clone(),
                rect: Some(atlas.rect("banner")),
                image_mode: banner_slicer(),
                ..default()
            },
        ))
        .with_child((
            Text::new(text),
            TextFont {
                font: FontSource::Handle(title_font.0.clone()),
                font_size: FontSize::Px(font_px),
                ..default()
            },
            TextColor(INK_WOOD),
            TextLayout { justify: Justify::Center, ..default() },
        ));
}

fn spawn_level_select(
    parent: &mut ChildSpawnerCommands,
    run: &Run,
    atlas: &Atlas,
    title_font: &TitleFont,
    win_w: f32,
) {
    let is_compact = win_w < 500.0;
    let btn_w = if is_compact { ((win_w - 60.0) / 2.0).clamp(140.0, 180.0) } else { 240.0 };
    let panel_padding = if is_compact { 12.0 } else { 20.0 };

    spawn_banner(
        parent,
        "Terrain Chess".into(),
        fit_width(if is_compact { 320.0 } else { 380.0 }, win_w - 2.0 * panel_padding),
        if is_compact { 42.0 } else { 48.0 },
        atlas,
        title_font,
    );

    // Items owned count
    parent.spawn((
        Text::new(format!("Items: {}", run.profile.owned.len())),
        TextFont { font_size: FontSize::Px(26.0), ..default() },
        TextColor(INK_WOOD),
        TextLayout { justify: Justify::Center, linebreak: LineBreak::NoWrap },
    ));

    // 2 columns of 5 levels each
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(10.0),
            align_items: AlignItems::FlexStart,
            justify_content: JustifyContent::Center,
            margin: UiRect::top(Val::Px(4.0)),
            ..default()
        })
        .with_children(|grid| {
            for col_levels in tc_run::LEVELS.chunks(5) {
                grid.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|col| {
                    for level_def in col_levels {
                        let is_cleared = run.profile.cleared.contains(&level_def.id);
                        let visuals = if is_cleared { ButtonVisuals::GOLD } else { ButtonVisuals::WOOD };
                        let sprite = if is_cleared { "btn_gold" } else { "btn_wood" };

                        col.spawn((
                            Button,
                            Interaction::default(),
                            LevelButton(level_def.id),
                            visuals,
                            Node {
                                width: Val::Px(btn_w),
                                min_height: Val::Px(52.0),
                                height: Val::Auto,
                                flex_direction: FlexDirection::Column,
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                padding: UiRect::axes(Val::Px(6.0), Val::Px(4.0)),
                                ..default()
                            },
                            ImageNode {
                                image: atlas.image.clone(),
                                rect: Some(atlas.rect(sprite)),
                                image_mode: button_slicer(),
                                ..default()
                            },
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new(format!("{} · {}", level_def.id, level_def.name)),
                                TextFont {
                                    font_size: FontSize::Px(if is_compact { 22.0 } else { 26.0 }),
                                    ..default()
                                },
                                TextColor(INK_WOOD),
                                TextLayout { justify: Justify::Center, linebreak: LineBreak::WordBoundary },
                            ));
                            btn.spawn((
                                Text::new(format!(
                                    "{}×{} · {} pcs",
                                    level_def.size,
                                    level_def.size,
                                    level_piece_count(level_def)
                                )),
                                TextFont {
                                    font_size: FontSize::Px(if is_compact { 20.0 } else { 24.0 }),
                                    ..default()
                                },
                                TextColor(INK_WOOD),
                                TextLayout { justify: Justify::Center, linebreak: LineBreak::NoWrap },
                            ));
                        });
                    }
                });
            }
        });
}

/// The Prepare screen: level info, the 15-card deck in draw order, the reserve, and Start.
fn spawn_prepare(
    parent: &mut ChildSpawnerCommands,
    level: &tc_run::Level,
    run: &Run,
    held: Option<DeckSlot>,
    atlas: &Atlas,
    title_font: &TitleFont,
    win_w: f32,
) {
    let is_compact = win_w < 500.0;
    let panel_padding = if is_compact { 12.0 } else { 20.0 };
    let avail = win_w.min(760.0) - 2.0 * OVERLAY_GUTTER - 2.0 * panel_padding;
    let gap = if is_compact { 4.0 } else { 8.0 };
    let tile_w = ((avail - 4.0 * gap) / 5.0).floor().clamp(62.0, 112.0);
    let grid_w = 5.0 * tile_w + 4.0 * gap;
    let shuffle = run.profile.shuffle_deck;
    let deck = run.profile.deck();
    let reserve = run.profile.deck_reserve();
    let px = |big: f32, small: f32| FontSize::Px(if is_compact { small } else { big });

    spawn_banner(
        parent,
        format!("{} · {}", level.id, level.name),
        fit_width(if is_compact { 320.0 } else { 380.0 }, win_w - 2.0 * panel_padding),
        if is_compact { 40.0 } else { 48.0 },
        atlas,
        title_font,
    );

    parent.spawn((
        Text::new(format!("{}×{} · {} pcs", level.size, level.size, level_piece_count(level))),
        TextFont { font_size: px(26.0, 22.0), ..default() },
        TextColor(INK_WOOD),
        TextLayout { justify: Justify::Center, linebreak: LineBreak::NoWrap },
    ));

    let mut header = format!("Your deck · {} cards", deck.len());
    if !reserve.is_empty() {
        header += &format!(" · {} in reserve", reserve.len());
    }
    parent.spawn((
        Text::new(header),
        TextFont { font_size: px(24.0, 20.0), ..default() },
        TextColor(INK_WOOD),
        TextLayout { justify: Justify::Center, linebreak: LineBreak::WordBoundary },
    ));

    let grid = Node {
        width: Val::Px(grid_w),
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        justify_content: JustifyContent::Center,
        column_gap: Val::Px(gap),
        row_gap: Val::Px(gap),
        ..default()
    };

    parent.spawn(grid.clone()).with_children(|grid| {
        for (i, &spell) in deck.iter().enumerate() {
            let slot = DeckSlot::Deck(i);
            let (sprite, slicer) = if held == Some(slot) {
                ("btn_gold", button_slicer())
            } else {
                ("panel_gui_parchment", panel_slicer())
            };
            grid.spawn((
                Button,
                Interaction::default(),
                PrepareCard { slot, spell },
                Node {
                    width: Val::Px(tile_w),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    padding: if is_compact {
                        UiRect::axes(Val::Px(1.0), Val::Px(3.0))
                    } else {
                        UiRect::all(Val::Px(4.0))
                    },
                    row_gap: Val::Px(2.0),
                    position_type: PositionType::Relative,
                    ..default()
                },
                ImageNode {
                    image: atlas.image.clone(),
                    rect: Some(atlas.rect(sprite)),
                    image_mode: slicer,
                    ..default()
                },
            ))
            .with_children(|tile| {
                if !shuffle {
                    // Cards 1–3 are the opening hand.
                    let ink = if i < 3 { Color::srgb_u8(0x9A, 0x6A, 0x12) } else { INK_PARCHMENT };
                    tile.spawn((
                        Text::new(format!("{}", i + 1)),
                        TextFont { font_size: px(18.0, 16.0), ..default() },
                        TextColor(ink),
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(6.0),
                            top: Val::Px(3.0),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ));
                }
                let art = format!("art_{}", spell_item_id(spell));
                // Older art is 60×54, newer art 30×26 drawn at 2×.
                let (art_px, _) = integer_scaled_size(atlas, &art, 54.0);
                tile.spawn((
                    ImageNode { image: atlas.image.clone(), rect: Some(atlas.rect(&art)), ..default() },
                    Node { width: Val::Px(art_px.x), height: Val::Px(art_px.y), ..default() },
                    Pickable::IGNORE,
                ));
                tile.spawn((
                    Text::new(spell_name(spell)),
                    TextFont { font_size: px(20.0, 16.0), ..default() },
                    TextColor(INK_PARCHMENT),
                    TextLayout { justify: Justify::Center, linebreak: LineBreak::WordBoundary },
                    Pickable::IGNORE,
                ));
                // Spawned after the art so it draws on top where the art fills a phone tile.
                if spell.is_quick() {
                    spawn_bolt(tile, atlas);
                }
            });
        }
    });

    parent.spawn((
        PrepareDetail,
        Text::new(prepare_hint(held_spell(held, &deck, &reserve), shuffle)),
        TextFont { font_size: px(22.0, 18.0), ..default() },
        TextColor(INK_WOOD),
        TextLayout { justify: Justify::Center, linebreak: LineBreak::WordBoundary },
        Node {
            width: Val::Px(avail.min(grid_w + 40.0)),
            min_height: Val::Px(if is_compact { 40.0 } else { 50.0 }),
            ..default()
        },
    ));

    if !reserve.is_empty() {
        parent.spawn((
            Text::new("Reserve"),
            TextFont { font_size: px(24.0, 20.0), ..default() },
            TextColor(INK_WOOD),
        ));
        parent.spawn(grid).with_children(|grid| {
            for (r, &spell) in reserve.iter().enumerate() {
                let slot = DeckSlot::Reserve(r);
                let is_held = held == Some(slot);
                let (sprite, slicer) = if is_held {
                    ("btn_gold", button_slicer())
                } else {
                    ("panel_gui_parchment", panel_slicer())
                };
                grid.spawn((
                    Button,
                    Interaction::default(),
                    PrepareCard { slot, spell },
                    Node {
                        width: Val::Px(tile_w),
                        min_height: Val::Px(30.0),
                        column_gap: Val::Px(2.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        padding: UiRect::axes(Val::Px(4.0), Val::Px(3.0)),
                        position_type: PositionType::Relative,
                        ..default()
                    },
                    ImageNode {
                        image: atlas.image.clone(),
                        rect: Some(atlas.rect(sprite)),
                        image_mode: slicer,
                        color: if is_held { Color::WHITE } else { Color::srgb(0.78, 0.74, 0.70) },
                        ..default()
                    },
                ))
                .with_children(|chip| {
                    chip.spawn((
                        Text::new(spell_name(spell)),
                        TextFont { font_size: px(18.0, 16.0), ..default() },
                        TextColor(INK_PARCHMENT),
                        TextLayout { justify: Justify::Center, linebreak: LineBreak::WordBoundary },
                        Pickable::IGNORE,
                    ));
                    // Phone chips are too narrow for a long name plus the bolt; hover still says Quick.
                    if spell.is_quick() && !is_compact {
                        chip.spawn((
                            ImageNode {
                                image: atlas.image.clone(),
                                rect: Some(atlas.rect("icon_bolt")),
                                ..default()
                            },
                            Node {
                                width: Val::Px(15.0),
                                height: Val::Px(13.0),
                                flex_shrink: 0.0,
                                ..default()
                            },
                            Pickable::IGNORE,
                        ));
                    }
                });
            }
        });
    }

    let btn_w = if is_compact { (avail - 12.0) / 2.0 } else { 140.0 };
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(12.0),
            row_gap: Val::Px(8.0),
            justify_content: JustifyContent::Center,
            margin: UiRect::top(Val::Px(4.0)),
            ..default()
        })
        .with_children(|row| {
            let reset_disabled = run.profile.deck.is_empty() && shuffle;
            let shuffle_label = if shuffle { "Shuffle: On" } else { "Shuffle: Off" };
            spawn_prepare_button(row, PrepareBack, "Back", ButtonVisuals::WOOD, false, btn_w, atlas);
            spawn_prepare_button(
                row,
                PrepareShuffle,
                shuffle_label,
                ButtonVisuals::WOOD,
                false,
                btn_w,
                atlas,
            );
            spawn_prepare_button(
                row,
                PrepareReset,
                "Reset",
                ButtonVisuals::WOOD,
                reset_disabled,
                btn_w,
                atlas,
            );
            spawn_prepare_button(row, PrepareStart, "Start", ButtonVisuals::GOLD, false, btn_w, atlas);
        });
}

/// The quick-spell bolt in a card's top-right corner.
fn spawn_bolt(parent: &mut ChildSpawnerCommands, atlas: &Atlas) {
    let size = atlas.px("icon_bolt");
    parent.spawn((
        ImageNode { image: atlas.image.clone(), rect: Some(atlas.rect("icon_bolt")), ..default() },
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(3.0),
            top: Val::Px(2.0),
            width: Val::Px(size.x),
            height: Val::Px(size.y),
            ..default()
        },
        Pickable::IGNORE,
    ));
}

fn spawn_prepare_button(
    parent: &mut ChildSpawnerCommands,
    marker: impl Component,
    label: &str,
    visuals: ButtonVisuals,
    disabled: bool,
    width: f32,
    atlas: &Atlas,
) {
    let sprite = if disabled { visuals.disabled.unwrap_or(visuals.normal) } else { visuals.normal };
    parent
        .spawn((
            Button,
            Interaction::default(),
            marker,
            visuals,
            ButtonDisabled(disabled),
            Node {
                width: Val::Px(width),
                height: Val::Px(52.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ImageNode {
                image: atlas.image.clone(),
                rect: Some(atlas.rect(sprite)),
                image_mode: button_slicer(),
                ..default()
            },
        ))
        .with_child((
            Text::new(label),
            TextFont { font_size: FontSize::Px(26.0), ..default() },
            TextColor(INK_WOOD),
            TextLayout { justify: Justify::Center, linebreak: LineBreak::NoWrap },
            Pickable::IGNORE,
        ));
}

fn handle_level_buttons(
    button_query: Query<(&Interaction, &LevelButton), (Changed<Interaction>, With<Button>)>,
    mut title_menu: ResMut<TitleMenu>,
) {
    for (interaction, btn) in &button_query {
        if *interaction == Interaction::Pressed {
            title_menu.prepare = Some(btn.0);
            title_menu.held = None;
        }
    }
}

/// Deck tiles: tap to pick up / swap, hover to lift the tile and describe the card.
fn handle_prepare_cards(
    mut cards: Query<(&Interaction, &PrepareCard, &mut Node), Changed<Interaction>>,
    mut detail: Query<&mut Text, With<PrepareDetail>>,
    title_menu: Res<TitleMenu>,
    run: Res<Run>,
    mut edits: MessageWriter<DeckEdit>,
) {
    for (interaction, card, mut node) in &mut cards {
        let text = match interaction {
            Interaction::Pressed => {
                edits.write(DeckEdit::Tap(card.slot));
                continue;
            }
            Interaction::Hovered => {
                node.top = Val::Px(-4.0);
                card_detail(card.spell)
            }
            Interaction::None => {
                node.top = Val::Px(0.0);
                let held = held_spell(title_menu.held, &run.profile.deck(), &run.profile.deck_reserve());
                prepare_hint(held, run.profile.shuffle_deck)
            }
        };
        for mut t in &mut detail {
            t.0 = text.clone();
        }
    }
}

#[allow(clippy::type_complexity)]
fn handle_prepare_buttons(
    back: Query<&Interaction, (Changed<Interaction>, With<PrepareBack>)>,
    shuffle: Query<&Interaction, (Changed<Interaction>, With<PrepareShuffle>)>,
    reset: Query<(&Interaction, &ButtonDisabled), (Changed<Interaction>, With<PrepareReset>)>,
    start: Query<&Interaction, (Changed<Interaction>, With<PrepareStart>)>,
    mut title_menu: ResMut<TitleMenu>,
    mut edits: MessageWriter<DeckEdit>,
    mut start_writer: MessageWriter<StartLevel>,
) {
    if back.iter().any(|i| *i == Interaction::Pressed) {
        title_menu.prepare = None;
        title_menu.held = None;
    }
    if shuffle.iter().any(|i| *i == Interaction::Pressed) {
        edits.write(DeckEdit::ToggleShuffle);
    }
    if reset.iter().any(|(i, d)| *i == Interaction::Pressed && !d.0) {
        edits.write(DeckEdit::Reset);
    }
    if start.iter().any(|i| *i == Interaction::Pressed)
        && let Some(level) = title_menu.prepare
    {
        start_writer.write(StartLevel(level));
        title_menu.open = false;
        title_menu.prepare = None;
        title_menu.held = None;
    }
}

fn handle_menu_button(
    button_query: Query<&Interaction, (Changed<Interaction>, With<MenuButton>)>,
    mut title_menu: ResMut<TitleMenu>,
) {
    for interaction in &button_query {
        if *interaction == Interaction::Pressed {
            title_menu.open = !title_menu.open;
            title_menu.prepare = None;
            title_menu.held = None;
        }
    }
}

fn handle_result_buttons(
    claim_query: Query<&Interaction, (Changed<Interaction>, With<ClaimButton>)>,
    retry_query: Query<&Interaction, (Changed<Interaction>, With<RetryButton>)>,
    levels_query: Query<&Interaction, (Changed<Interaction>, With<LevelsButton>)>,
    mut run: ResMut<Run>,
    mut title_menu: ResMut<TitleMenu>,
    mut start_writer: MessageWriter<StartLevel>,
) {
    for interaction in &claim_query {
        if *interaction == Interaction::Pressed
            && let Some(draft) = run.pending_draft.take()
        {
            run.phase = RunPhase::Draft(draft);
        }
    }
    for interaction in &retry_query {
        if *interaction == Interaction::Pressed {
            start_writer.write(StartLevel(run.level));
        }
    }
    for interaction in &levels_query {
        if *interaction == Interaction::Pressed {
            run.phase = RunPhase::Playing;
            title_menu.open = true;
            title_menu.prepare = None;
            title_menu.held = None;
        }
    }
}

fn update_button_visuals(
    atlas: Res<Atlas>,
    mut q: Query<
        (&Interaction, &ButtonVisuals, Option<&ButtonDisabled>, &mut ImageNode),
        Or<(Changed<Interaction>, Changed<ButtonDisabled>)>,
    >,
) {
    for (interaction, visuals, disabled, mut image_node) in &mut q {
        let is_disabled = disabled.is_some_and(|d| d.0);
        let sprite_name = if is_disabled {
            visuals.disabled.unwrap_or(visuals.normal)
        } else {
            match interaction {
                Interaction::Pressed => visuals.pressed,
                Interaction::Hovered => visuals.hover,
                Interaction::None => visuals.normal,
            }
        };
        image_node.rect = Some(atlas.rect(sprite_name));
    }
}

/// A one-line message above the hand bar that fades out on its own.
#[derive(Component)]
struct Toast {
    age: f32,
    pill: Entity,
    text: Entity,
}

const TOAST_SECS: f32 = 2.6;

/// Shows `GameState::toast` as a pill above the hand; a new message replaces the old one.
fn sync_toast(
    mut commands: Commands,
    time: Res<Time>,
    mut state: ResMut<GameState>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut toasts: Query<(Entity, &mut Toast)>,
    mut colors: Query<(Option<&mut BackgroundColor>, Option<&mut TextColor>)>,
) {
    if let Some(msg) = state.toast.take() {
        for (e, _) in &toasts {
            commands.entity(e).despawn();
        }
        let (w, h) = window.iter().next().map(|w| (w.width(), w.height())).unwrap_or((1280.0, 800.0));
        let font_size = if w < 600.0 { 18.0 } else { 22.0 };
        let text = commands
            .spawn((
                Text::new(msg),
                TextFont { font_size: FontSize::Px(font_size), ..default() },
                TextColor(Color::srgba(1.0, 0.93, 0.80, 0.0)),
                TextLayout { justify: Justify::Center, linebreak: LineBreak::WordBoundary },
                Pickable::IGNORE,
            ))
            .id();
        let pill = commands
            .spawn((
                Node {
                    max_width: Val::Vw(90.0),
                    padding: UiRect::axes(Val::Px(16.0), Val::Px(6.0)),
                    border_radius: BorderRadius::all(Val::Px(8.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.08, 0.05, 0.10, 0.0)),
                Pickable::IGNORE,
            ))
            .add_child(text)
            .id();
        commands
            .spawn((
                Toast { age: 0.0, pill, text },
                GlobalZIndex(40),
                Node {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(h * 0.30),
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .add_child(pill);
        return;
    }
    for (e, mut toast) in &mut toasts {
        toast.age += time.delta_secs();
        let t = toast.age;
        if t >= TOAST_SECS {
            commands.entity(e).despawn();
            continue;
        }
        // Fade in over 0.15 s, hold, fade out over the last 0.4 s.
        let a = (t / 0.15).min(1.0).min((TOAST_SECS - t) / 0.4);
        if let Ok((Some(mut bg), _)) = colors.get_mut(toast.pill) {
            bg.0.set_alpha(0.85 * a);
        }
        if let Ok((_, Some(mut color))) = colors.get_mut(toast.text) {
            color.0.set_alpha(a);
        }
    }
}

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Ready), setup_hud).add_systems(
            Update,
            (
                update_floor_badge,
                sync_overlays,
                sync_title_menu,
                update_draft_card_sizes,
                handle_card_interaction,
                handle_result_buttons,
                handle_level_buttons,
                handle_prepare_cards,
                handle_prepare_buttons,
                handle_menu_button,
                handle_hand_card_interaction,
                handle_discard_button_interaction,
                update_button_visuals,
                sync_hand_bar,
                sync_toast,
            )
                .chain()
                .run_if(in_state(AppState::Ready)),
        );
    }
}
