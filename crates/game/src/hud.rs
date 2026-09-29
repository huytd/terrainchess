use bevy::prelude::*;
use bevy::text::LineHeight;
use bevy::window::PrimaryWindow;
use tc_core::Side;
use tc_run::ItemKind;

use crate::atlas::Atlas;
use crate::game::GameState;
use crate::loading::AppState;
use crate::run::{PickCard, Run, RunPhase, StartCampaign, TitleMenu};
use crate::save;

/// Ink colour on light wood.
pub const INK_WOOD: Color = Color::srgb_u8(0xF7, 0xED, 0xD0);
/// Ink colour on aged parchment.
pub const INK_PARCHMENT: Color = Color::srgb_u8(0x3B, 0x2F, 0x2A);

pub fn button_slicer() -> NodeImageMode {
    NodeImageMode::Sliced(TextureSlicer {
        border: BorderRect::all(6.0),
        center_scale_mode: SliceScaleMode::Stretch,
        sides_scale_mode: SliceScaleMode::Stretch,
        max_corner_scale: 1.0,
    })
}

pub fn panel_slicer() -> NodeImageMode {
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

fn integer_scaled_size(atlas: &Atlas, name: &str, target_h: f32) -> (Vec2, f32) {
    let px = atlas.px(name);
    let scale = (target_h / px.y).round().max(1.0);
    (px * scale, scale)
}

fn spell_item_id(spell: tc_core::SpellId) -> &'static str {
    match spell {
        tc_core::SpellId::RaiseEarth => "raise_earth",
        tc_core::SpellId::LowerEarth => "lower_earth",
        tc_core::SpellId::Freeze => "freeze",
        tc_core::SpellId::Bridge => "bridge",
        tc_core::SpellId::DigTunnel => "dig_tunnel",
        tc_core::SpellId::Shield => "shield",
        tc_core::SpellId::Swap => "swap",
        tc_core::SpellId::Rewind => "rewind",
    }
}

fn spell_name(spell: tc_core::SpellId) -> &'static str {
    match spell {
        tc_core::SpellId::RaiseEarth => "Raise Earth",
        tc_core::SpellId::LowerEarth => "Lower Earth",
        tc_core::SpellId::Freeze => "Freeze",
        tc_core::SpellId::Shield => "Shield",
        tc_core::SpellId::Swap => "Swap",
        tc_core::SpellId::Bridge => "Bridge",
        tc_core::SpellId::DigTunnel => "Dig Tunnel",
        tc_core::SpellId::Rewind => "Rewind",
    }
}

#[derive(Component)]
struct FloorBadge;

#[derive(Component)]
struct FloorBadgeTitle;

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
pub struct TitleCampaignButton(pub bool);

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
struct HandBar;

#[derive(Component)]
struct HandCard {
    slot_idx: usize,
    base_angle_deg: f32,
    resting_pivot_offset: Vec2,
    lift_px: f32,
    is_used: bool,
}

#[derive(Component)]
struct HandCardHighlight(usize);

#[derive(Component)]
struct DiscardButton;

#[derive(Component, Clone, Copy)]
pub struct ButtonVisuals {
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
    pub(crate) const GOLD: Self = Self {
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
pub(crate) struct ButtonDisabled(pub(crate) bool);

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
            // Floor badge plaque (100 x 112 px)
            col.spawn((
                FloorBadge,
                Node {
                    width: Val::Px(100.0),
                    height: Val::Px(112.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    padding: UiRect { bottom: Val::Px(8.0), ..default() },
                    ..default()
                },
                ImageNode {
                    image: atlas.image.clone(),
                    rect: Some(atlas.rect(badge_name)),
                    image_mode: NodeImageMode::Stretch,
                    ..default()
                },
            ))
            .with_children(|badge| {
                badge.spawn((
                    Text::new("Floor"),
                    TextFont { font_size: FontSize::Px(24.0), ..default() },
                    TextColor(INK_WOOD),
                    TextLayout { justify: Justify::Center, linebreak: LineBreak::NoWrap },
                    FloorBadgeTitle,
                ));
                badge.spawn((
                    Text::new("1/8"),
                    TextFont { font_size: FontSize::Px(24.0), ..default() },
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

#[allow(clippy::too_many_arguments)]
fn update_floor_badge(
    run: Res<Run>,
    mode: Res<State<crate::game::Mode>>,
    title_menu: Res<TitleMenu>,
    state: Res<GameState>,
    atlas: Res<Atlas>,
    mut q_badge: Query<(&mut ImageNode, &mut Node), With<FloorBadge>>,
    mut q_title: Query<
        &mut Text,
        (With<FloorBadgeTitle>, Without<FloorBadgeText>, Without<FloorBadgeEnemyText>),
    >,
    mut q_text: Query<
        &mut Text,
        (With<FloorBadgeText>, Without<FloorBadgeTitle>, Without<FloorBadgeEnemyText>),
    >,
    mut q_enemy_box: Query<&mut Node, (With<FloorBadgeEnemyBox>, Without<FloorBadge>)>,
    mut q_enemy: Query<&mut Text, With<FloorBadgeEnemyText>>,
) {
    let is_boss = run.state.floor == 7;
    let badge_name = if is_boss { "badge_boss" } else { "badge_floor" };

    for (mut img, mut node) in &mut q_badge {
        let show = !title_menu.open && *mode.get() == crate::game::Mode::Classic;
        node.display = if show { Display::Flex } else { Display::None };
        let r = Some(atlas.rect(badge_name));
        if img.rect != r {
            img.rect = r;
        }
        node.width = Val::Px(100.0);
        node.height = Val::Px(112.0);
    }

    let (title, label) =
        if is_boss { ("", "Boss".to_string()) } else { ("Floor", format!("{}/8", run.state.floor + 1)) };

    for mut text in &mut q_title {
        if text.0 != title {
            text.0 = title.to_string();
        }
    }

    for mut text in &mut q_text {
        if text.0 != label {
            text.0 = label.clone();
        }
    }

    let enemy_label = if run.state.floor >= 3 && !state.enemy_items.is_empty() {
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

fn sync_overlays(
    mut commands: Commands,
    run: Res<Run>,
    atlas: Res<Atlas>,
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
                                min_width: Val::Px(380.0),
                                padding: UiRect::axes(Val::Px(36.0), Val::Px(12.0)),
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
                            TextFont { font_size: FontSize::Px(48.0), ..default() },
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
                                            left: Val::Percent(-10.0),
                                            width: Val::Percent(120.0),
                                            bottom: Val::Percent(103.0),
                                            justify_content: JustifyContent::Center,
                                            ..default()
                                        },
                                        children![(
                                            Text::new(&item.name),
                                            TextFont { font_size: FontSize::Px(24.0), ..default() },
                                            TextColor(INK_WOOD),
                                            TextShadow::default(),
                                            TextLayout { justify: Justify::Center, ..default() },
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
                                                left: Val::Percent(12.0),
                                                width: Val::Percent(76.0),
                                                top: Val::Percent(52.0),
                                                height: Val::Percent(42.0),
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
                                                            font_size: FontSize::Px(24.0),
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
                                                            font_size: FontSize::Px(24.0),
                                                            ..default()
                                                        },
                                                        TextColor(INK_PARCHMENT),
                                                        TextLayout {
                                                            justify: Justify::Center,
                                                            linebreak: LineBreak::WordBoundary,
                                                        },
                                                        LineHeight::RelativeToFont(1.1),
                                                    ));
                                                },
                                            );
                                        });
                                });
                            }
                        });
                });
        }
        RunPhase::Over { won } => {
            let badge_name = if *won { "badge_victory" } else { "badge_defeat" };
            let (badge_size, _) = integer_scaled_size(&atlas, badge_name, 60.0);

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
                                padding: UiRect::all(Val::Px(28.0)),
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                row_gap: Val::Px(16.0),
                                min_width: Val::Px(420.0),
                                min_height: Val::Px(300.0),
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

                            // Title
                            panel.spawn((
                                Text::new(if *won { "Victory!" } else { "Run lost" }),
                                TextFont { font_size: FontSize::Px(48.0), ..default() },
                                TextColor(INK_PARCHMENT),
                                TextLayout { justify: Justify::Center, ..default() },
                            ));

                            let floor_num = if *won { 8 } else { run.state.floor + 1 };
                            panel.spawn((
                                Text::new(format!("Reached floor {floor_num}")),
                                TextFont { font_size: FontSize::Px(24.0), ..default() },
                                TextColor(INK_PARCHMENT),
                                TextLayout { justify: Justify::Center, ..default() },
                            ));
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
    mode: Res<State<crate::game::Mode>>,
    title_menu: Res<TitleMenu>,
    atlas: Res<Atlas>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut last_key: Local<
        Option<(bool, [Option<tc_core::SpellId>; 3], [bool; 3], usize, u8, bool, Option<usize>, (u32, u32))>,
    >,
    hand_bar_query: Query<Entity, With<HandBar>>,
) {
    let side = state.game.pos.side_to_move;
    let show = !title_menu.open
        && run.phase == RunPhase::Playing
        && !state.ai_to_move()
        && state.outcome.is_none()
        && side == Side::White
        && (*mode.get() == crate::game::Mode::Classic || *mode.get() == crate::game::Mode::OverworldBattle);

    let hand = state.game.hand(side);
    let hand_cards = hand.hand;
    let hand_used = hand.used;
    let deck_len = state.game.deck_len(side);
    let discards_left = state.game.discards_left(side);
    let armed_slot = state.armed_slot;
    let can_discard = armed_slot.is_some_and(|slot| state.game.can_discard(side, slot));

    let win = window.iter().next();
    let (win_w, win_h) = win.map(|w| (w.width(), w.height())).unwrap_or((1280.0, 800.0));
    let win_dim = (win_w as u32, win_h as u32);

    let current_key =
        (show, hand_cards, hand_used, deck_len, discards_left, can_discard, armed_slot, win_dim);

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
                    HandCard { slot_idx, base_angle_deg, resting_pivot_offset, lift_px, is_used },
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

                            // Parchment text box: spans x 12%-88%, y 52%-94%
                            card.spawn(Node {
                                position_type: PositionType::Absolute,
                                left: Val::Percent(12.0),
                                width: Val::Percent(76.0),
                                top: Val::Percent(52.0),
                                height: Val::Percent(42.0),
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
                                    TextFont { font_size: FontSize::Px(24.0), ..default() },
                                    TextColor(INK_PARCHMENT),
                                    TextLayout {
                                        justify: Justify::Center,
                                        linebreak: LineBreak::WordBoundary,
                                    },
                                ));
                                let subtext = if is_used {
                                    "Used".to_string()
                                } else if spell.is_quick() {
                                    format!("[{}] Quick", slot_idx + 5)
                                } else {
                                    format!("[{}]", slot_idx + 5)
                                };
                                tb.spawn((
                                    Text::new(subtext),
                                    TextFont { font_size: FontSize::Px(24.0), ..default() },
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
                // 1. Discard button
                let (icon_discard_size, _) = integer_scaled_size(&atlas, "icon_discard", 44.0);
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
                            width: Val::Px(sidebar_w),
                            height: Val::Px(if win_w < 600.0 { 70.0 } else { 78.0 }),
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            row_gap: Val::Px(2.0),
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
                                width: Val::Px(icon_discard_size.x),
                                height: Val::Px(icon_discard_size.y),
                                ..default()
                            },
                            ImageNode {
                                image: atlas.image.clone(),
                                rect: Some(atlas.rect("icon_discard")),
                                image_mode: NodeImageMode::Auto,
                                ..default()
                            },
                        ));
                        btn.spawn((
                            Text::new(format!("Discard {discards_left}/{}", tc_core::MAX_DISCARDS)),
                            TextFont { font_size: FontSize::Px(24.0), ..default() },
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
                        TextFont { font_size: FontSize::Px(24.0), ..default() },
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
        if disabled.is_some_and(|d| d.0) {
            continue;
        }
        if *interaction == Interaction::Pressed {
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

fn sync_title_menu(
    mut commands: Commands,
    title_menu: Res<TitleMenu>,
    atlas: Res<Atlas>,
    mut last_open: Local<Option<bool>>,
    overlay_query: Query<Entity, With<TitleOverlay>>,
) {
    if *last_open == Some(title_menu.open) {
        return;
    }
    *last_open = Some(title_menu.open);

    for e in &overlay_query {
        commands.entity(e).despawn();
    }

    if !title_menu.open {
        return;
    }

    let has_campaign_save = save::has_campaign_save();

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
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .with_children(|overlay| {
            // Main stone panel
            overlay
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(10.0),
                        padding: UiRect::all(Val::Px(20.0)),
                        flex_shrink: 0.0,
                        ..default()
                    },
                    ImageNode {
                        image: atlas.image.clone(),
                        rect: Some(atlas.rect("panel_gui_stone")),
                        image_mode: panel_slicer(),
                        ..default()
                    },
                ))
                .with_children(|parent| {
                    // Title banner: large light ink on a banner
                    parent
                        .spawn((
                            Node {
                                width: Val::Px(380.0),
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
                            Text::new("Terrain Chess"),
                            TextFont { font_size: FontSize::Px(48.0), ..default() },
                            TextColor(INK_WOOD),
                            TextLayout { justify: Justify::Center, ..default() },
                        ));

                    // Column of buttons (min 280 x 56 px, 24 px text)
                    parent
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(10.0),
                            align_items: AlignItems::Center,
                            margin: UiRect { top: Val::Px(16.0), bottom: Val::Px(16.0), ..default() },
                            flex_shrink: 0.0,
                            ..default()
                        })
                        .with_children(|col| {
                            if has_campaign_save {
                                col.spawn((
                                    Button,
                                    Interaction::default(),
                                    TitleCampaignButton(true),
                                    ButtonVisuals::GOLD,
                                    Node {
                                        min_width: Val::Px(280.0),
                                        width: Val::Px(280.0),
                                        height: Val::Px(56.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        padding: UiRect::horizontal(Val::Px(12.0)),
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
                                    Text::new("Continue campaign"),
                                    TextFont { font_size: FontSize::Px(24.0), ..default() },
                                    TextColor(INK_WOOD),
                                    TextLayout { justify: Justify::Center, linebreak: LineBreak::NoWrap },
                                ));
                            }

                            col.spawn((
                                Button,
                                Interaction::default(),
                                TitleCampaignButton(false),
                                ButtonVisuals::WOOD,
                                Node {
                                    min_width: Val::Px(280.0),
                                    width: Val::Px(280.0),
                                    height: Val::Px(56.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
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
                                Text::new("New campaign"),
                                TextFont { font_size: FontSize::Px(24.0), ..default() },
                                TextColor(INK_WOOD),
                                TextLayout { justify: Justify::Center, linebreak: LineBreak::NoWrap },
                            ));
                        });
                });
        });
}

fn handle_title_buttons(
    menu_q: Query<&Interaction, (Changed<Interaction>, With<MenuButton>)>,
    mut title_menu: ResMut<TitleMenu>,
    campaign_q: Query<(&Interaction, &TitleCampaignButton, &Children), (Changed<Interaction>, With<Button>)>,
    mut text_q: Query<&mut Text>,
    mut campaign_writer: MessageWriter<StartCampaign>,
) {
    for (interaction, btn, children) in &campaign_q {
        if *interaction == Interaction::Pressed && !title_menu.pending {
            title_menu.pending = true;
            campaign_writer.write(StartCampaign(btn.0));
            for child in children.iter() {
                if let Ok(mut text) = text_q.get_mut(child) {
                    text.0 = "Loading…".to_string();
                }
            }
        }
    }

    for interaction in &menu_q {
        if *interaction == Interaction::Pressed {
            title_menu.open = true;
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
                handle_title_buttons,
                handle_hand_card_interaction,
                handle_discard_button_interaction,
                update_button_visuals,
                sync_hand_bar,
            )
                .chain()
                .run_if(in_state(AppState::Ready)),
        );
    }
}
