use bevy::prelude::*;
use bevy::text::LineHeight;
use bevy::window::PrimaryWindow;
use tc_core::Side;
use tc_run::ItemKind;

use crate::atlas::Atlas;
use crate::game::GameState;
use crate::loading::AppState;
use crate::run::{PickCard, Run, RunPhase, StartRun, TitleMenu, ToggleSandbox};
use crate::save;

/// Ink colour on light wood.
pub const INK_WOOD: Color = Color::srgb_u8(0xF7, 0xED, 0xD0);
/// Ink colour on aged parchment.
pub const INK_PARCHMENT: Color = Color::srgb_u8(0x3B, 0x2F, 0x2A);

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
struct TitleContinueButton;

#[derive(Component)]
struct TitleNewRunButton(u8);

#[derive(Component)]
struct TitleSandboxButton;

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
struct NewRunButton;

#[derive(Component)]
struct HandBar;

#[derive(Component)]
struct HandCard {
    slot_idx: usize,
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
            // Floor badge plaque (72 x 84 px)
            col.spawn((
                FloorBadge,
                Node {
                    width: Val::Px(72.0),
                    height: Val::Px(84.0),
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
                    TextFont { font_size: FontSize::Px(11.0), ..default() },
                    TextColor(INK_WOOD),
                    TextLayout { justify: Justify::Center, linebreak: LineBreak::NoWrap },
                    FloorBadgeTitle,
                ));
                badge.spawn((
                    Text::new("1/8"),
                    TextFont { font_size: FontSize::Px(18.0), ..default() },
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
                    column_gap: Val::Px(6.0),
                    padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
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
                    TextFont { font_size: FontSize::Px(13.0), ..default() },
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
    let is_boss = !run.sandbox && run.state.floor == 7;
    let badge_name = if is_boss { "badge_boss" } else { "badge_floor" };

    for (mut img, mut node) in &mut q_badge {
        let r = Some(atlas.rect(badge_name));
        if img.rect != r {
            img.rect = r;
        }
        node.width = Val::Px(72.0);
        node.height = Val::Px(84.0);
    }

    let (title, label) = if run.sandbox {
        ("", "Sandbox".to_string())
    } else if is_boss {
        ("", "Boss".to_string())
    } else {
        ("Floor", format!("{}/8", run.state.floor + 1))
    };

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

    let enemy_label = if !run.sandbox && run.state.floor >= 3 && !state.enemy_items.is_empty() {
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
                                min_width: Val::Px(320.0),
                                padding: UiRect::axes(Val::Px(32.0), Val::Px(12.0)),
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
                            TextFont { font_size: FontSize::Px(22.0), ..default() },
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

                                let box_w = card_size.x * 0.70 - 8.0;
                                let box_h = card_size.y * 0.31 - 8.0;

                                let wrap_count = |text: &str, font_sz: f32| -> usize {
                                    let max_chars = ((box_w / (font_sz * 0.6)).floor() as usize).max(1);
                                    let mut lines = 0;
                                    let mut cur = 0;
                                    for w in text.split_whitespace() {
                                        let len = w.chars().count();
                                        if cur == 0 {
                                            cur = len;
                                            lines += 1;
                                        } else if cur + 1 + len <= max_chars {
                                            cur += 1 + len;
                                        } else {
                                            cur = len;
                                            lines += 1;
                                        }
                                    }
                                    lines.max(1)
                                };

                                let name_h = wrap_count(&item.name, 15.0) as f32 * (15.0 * 1.2);
                                let kind_h = wrap_count(&kind_str, 11.0) as f32 * (11.0 * 1.2);
                                let gaps = 4.0;
                                let desc_h_12 = wrap_count(&item.description, 12.0) as f32 * (12.0 * 1.1);

                                let desc_font =
                                    if name_h + kind_h + gaps + desc_h_12 <= box_h { 12.0 } else { 11.0 };

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
                                            // 1. Art window: spans x 12% - 88%, y 8% - 54%
                                            card.spawn(Node {
                                                position_type: PositionType::Absolute,
                                                left: Val::Percent(12.0),
                                                width: Val::Percent(76.0),
                                                top: Val::Percent(8.0),
                                                height: Val::Percent(46.0),
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

                                            // 2. Parchment text box: spans x 15% - 85%, y 60% - 91%
                                            card.spawn(Node {
                                                position_type: PositionType::Absolute,
                                                left: Val::Percent(15.0),
                                                width: Val::Percent(70.0),
                                                top: Val::Percent(60.0),
                                                height: Val::Percent(31.0),
                                                flex_direction: FlexDirection::Column,
                                                align_items: AlignItems::Center,
                                                justify_content: JustifyContent::FlexStart,
                                                overflow: Overflow::clip(),
                                                row_gap: Val::Px(2.0),
                                                padding: UiRect::all(Val::Px(4.0)),
                                                ..default()
                                            })
                                            .with_children(
                                                |tb| {
                                                    // Name: 15 px, dark ink #3B2F2A
                                                    tb.spawn((
                                                        Text::new(&item.name),
                                                        TextFont {
                                                            font_size: FontSize::Px(15.0),
                                                            ..default()
                                                        },
                                                        TextColor(INK_PARCHMENT),
                                                        TextLayout {
                                                            justify: Justify::Center,
                                                            linebreak: LineBreak::WordBoundary,
                                                        },
                                                    ));
                                                    // Kind: 11 px, dark ink #3B2F2A
                                                    tb.spawn((
                                                        Text::new(kind_str),
                                                        TextFont {
                                                            font_size: FontSize::Px(11.0),
                                                            ..default()
                                                        },
                                                        TextColor(INK_PARCHMENT),
                                                        TextLayout {
                                                            justify: Justify::Center,
                                                            linebreak: LineBreak::WordBoundary,
                                                        },
                                                    ));
                                                    // Description: 12 px (or 11 if doesn't fit) with line height 1.1, dark ink #3B2F2A
                                                    tb.spawn((
                                                        Text::new(&item.description),
                                                        TextFont {
                                                            font_size: FontSize::Px(desc_font),
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
                                padding: UiRect::all(Val::Px(24.0)),
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                row_gap: Val::Px(14.0),
                                min_width: Val::Px(360.0),
                                min_height: Val::Px(280.0),
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
                                TextFont { font_size: FontSize::Px(28.0), ..default() },
                                TextColor(INK_PARCHMENT),
                                TextLayout { justify: Justify::Center, ..default() },
                            ));

                            let floor_num = if *won { 8 } else { run.state.floor + 1 };
                            panel.spawn((
                                Text::new(format!("Reached floor {floor_num}")),
                                TextFont { font_size: FontSize::Px(16.0), ..default() },
                                TextColor(INK_PARCHMENT),
                                TextLayout { justify: Justify::Center, ..default() },
                            ));

                            // Primary action button: btn_gold*
                            panel
                                .spawn((
                                    Button,
                                    Interaction::default(),
                                    NewRunButton,
                                    ButtonVisuals::GOLD,
                                    Node {
                                        width: Val::Px(160.0),
                                        height: Val::Px(48.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
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
                                    Text::new("New run"),
                                    TextFont { font_size: FontSize::Px(18.0), ..default() },
                                    TextColor(INK_WOOD),
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

fn sync_hand_bar(
    mut commands: Commands,
    state: Res<GameState>,
    run: Res<Run>,
    title_menu: Res<TitleMenu>,
    atlas: Res<Atlas>,
    mut last_key: Local<Option<(bool, [Option<tc_core::SpellId>; 3], [bool; 3], usize, bool, Option<usize>)>>,
    hand_bar_query: Query<Entity, With<HandBar>>,
) {
    let side = state.game.pos.side_to_move;
    let show = !title_menu.open
        && run.phase == RunPhase::Playing
        && !state.ai_to_move()
        && state.outcome.is_none()
        && (run.sandbox || side == Side::White);

    let hand = state.game.hand(side);
    let hand_cards = hand.hand;
    let hand_used = hand.used;
    let deck_len = state.game.deck_len(side);
    let can_discard = state.game.can_discard_hand(side);
    let armed_slot = state.armed_slot;

    let current_key = (show, hand_cards, hand_used, deck_len, can_discard, armed_slot);

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

    let (card_size, _) = integer_scaled_size(&atlas, "gui_card_spell", 160.0);
    let art_win_h = card_size.y * 0.46;
    let (slot_empty_size, _) = integer_scaled_size(&atlas, "card_slot_empty", card_size.y);

    commands
        .spawn((
            HandBar,
            GlobalZIndex(10),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                bottom: Val::Px(16.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexEnd,
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(12.0),
                padding: UiRect::horizontal(Val::Px(12.0)),
                ..default()
            },
        ))
        .with_children(|hand_bar| {
            // Sliced hand_tray panel behind the 3 cards
            hand_bar
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::FlexEnd,
                        justify_content: JustifyContent::Center,
                        column_gap: Val::Px(8.0),
                        padding: UiRect {
                            left: Val::Px(12.0),
                            right: Val::Px(12.0),
                            top: Val::Px(10.0),
                            bottom: Val::Px(10.0),
                        },
                        ..default()
                    },
                    ImageNode {
                        image: atlas.image.clone(),
                        rect: Some(atlas.rect("hand_tray")),
                        image_mode: panel_slicer(),
                        ..default()
                    },
                ))
                .with_children(|tray| {
                    for slot_idx in 0..3 {
                        let card_opt = hand_cards[slot_idx];
                        let is_used = hand_used[slot_idx];
                        let is_armed = armed_slot == Some(slot_idx);

                        if let Some(spell) = card_opt {
                            let art_name = format!("art_{}", spell_item_id(spell));
                            let (art_size, _) = integer_scaled_size(&atlas, &art_name, art_win_h);

                            if !is_used {
                                let top = if is_armed { Val::Px(-8.0) } else { Val::Px(0.0) };

                                tray.spawn((
                                    Button,
                                    Interaction::default(),
                                    HandCard { slot_idx },
                                    Node {
                                        width: Val::Px(card_size.x),
                                        height: Val::Px(card_size.y),
                                        position_type: PositionType::Relative,
                                        top,
                                        overflow: Overflow::visible(),
                                        ..default()
                                    },
                                ))
                                .with_children(|card_root| {
                                    // Armed / hovered highlight overlay BEHIND the card, ~10% larger and centred on it
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
                                                rect: Some(atlas.rect("gui_card_spell")),
                                                image_mode: NodeImageMode::Auto,
                                                ..default()
                                            },
                                        ))
                                        .with_children(|card| {
                                            // Art window: spans x 12%-88%, y 8%-54%
                                            card.spawn(Node {
                                                position_type: PositionType::Absolute,
                                                left: Val::Percent(12.0),
                                                width: Val::Percent(76.0),
                                                top: Val::Percent(8.0),
                                                height: Val::Percent(46.0),
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

                                            // Parchment text box: spans x 15%-85%, y 60%-91%
                                            card.spawn(Node {
                                                position_type: PositionType::Absolute,
                                                left: Val::Percent(15.0),
                                                width: Val::Percent(70.0),
                                                top: Val::Percent(60.0),
                                                height: Val::Percent(31.0),
                                                flex_direction: FlexDirection::Column,
                                                align_items: AlignItems::Center,
                                                justify_content: JustifyContent::FlexStart,
                                                overflow: Overflow::clip(),
                                                row_gap: Val::Px(1.0),
                                                padding: UiRect::all(Val::Px(1.0)),
                                                ..default()
                                            })
                                            .with_children(
                                                |tb| {
                                                    tb.spawn((
                                                        Text::new(spell_name(spell)),
                                                        TextFont {
                                                            font_size: FontSize::Px(11.0),
                                                            ..default()
                                                        },
                                                        TextColor(INK_PARCHMENT),
                                                        TextLayout {
                                                            justify: Justify::Center,
                                                            linebreak: LineBreak::WordBoundary,
                                                        },
                                                    ));
                                                    if spell.is_quick() {
                                                        tb.spawn((
                                                            Text::new("Quick"),
                                                            TextFont {
                                                                font_size: FontSize::Px(10.0),
                                                                ..default()
                                                            },
                                                            TextColor(INK_PARCHMENT),
                                                            TextLayout {
                                                                justify: Justify::Center,
                                                                linebreak: LineBreak::NoWrap,
                                                            },
                                                        ));
                                                    }
                                                    tb.spawn((
                                                        Text::new(format!("[{}]", slot_idx + 5)),
                                                        TextFont {
                                                            font_size: FontSize::Px(10.0),
                                                            ..default()
                                                        },
                                                        TextColor(Color::srgba(0.23, 0.18, 0.16, 0.60)),
                                                        TextLayout {
                                                            justify: Justify::Center,
                                                            linebreak: LineBreak::NoWrap,
                                                        },
                                                    ));
                                                },
                                            );
                                        });
                                });
                            } else {
                                // Used hand slot: overlay gui_card_used at 70% alpha
                                tray.spawn((
                                    Node {
                                        width: Val::Px(card_size.x),
                                        height: Val::Px(card_size.y),
                                        position_type: PositionType::Relative,
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
                                    card.spawn(Node {
                                        position_type: PositionType::Absolute,
                                        left: Val::Percent(12.0),
                                        width: Val::Percent(76.0),
                                        top: Val::Percent(8.0),
                                        height: Val::Percent(46.0),
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

                                    card.spawn(Node {
                                        position_type: PositionType::Absolute,
                                        left: Val::Percent(15.0),
                                        width: Val::Percent(70.0),
                                        top: Val::Percent(60.0),
                                        height: Val::Percent(31.0),
                                        flex_direction: FlexDirection::Column,
                                        align_items: AlignItems::Center,
                                        justify_content: JustifyContent::FlexStart,
                                        overflow: Overflow::clip(),
                                        row_gap: Val::Px(1.0),
                                        padding: UiRect::all(Val::Px(1.0)),
                                        ..default()
                                    })
                                    .with_children(|tb| {
                                        tb.spawn((
                                            Text::new(spell_name(spell)),
                                            TextFont { font_size: FontSize::Px(11.0), ..default() },
                                            TextColor(INK_PARCHMENT),
                                            TextLayout {
                                                justify: Justify::Center,
                                                linebreak: LineBreak::WordBoundary,
                                            },
                                        ));
                                        tb.spawn((
                                            Text::new("Used"),
                                            TextFont { font_size: FontSize::Px(10.0), ..default() },
                                            TextColor(INK_PARCHMENT),
                                            TextLayout {
                                                justify: Justify::Center,
                                                linebreak: LineBreak::NoWrap,
                                            },
                                        ));
                                    });

                                    // gui_card_used at 70% alpha
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
                                });
                            }
                        } else {
                            // Empty slot: card_slot_empty
                            tray.spawn(Node {
                                width: Val::Px(card_size.x),
                                height: Val::Px(card_size.y),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                overflow: Overflow::clip(),
                                ..default()
                            })
                            .with_child((
                                Node {
                                    width: Val::Px(slot_empty_size.x),
                                    height: Val::Px(slot_empty_size.y),
                                    ..default()
                                },
                                ImageNode {
                                    image: atlas.image.clone(),
                                    rect: Some(atlas.rect("card_slot_empty")),
                                    image_mode: NodeImageMode::Auto,
                                    ..default()
                                },
                            ));
                        }
                    }
                });

            // To the right: deck_pile and Discard button
            hand_bar
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::FlexEnd,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(8.0),
                    ..default()
                })
                .with_children(|sidebar| {
                    // Deck pile with count drawn on it in light ink
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
                            TextFont { font_size: FontSize::Px(16.0), ..default() },
                            TextColor(INK_WOOD),
                            TextLayout { justify: Justify::Center, linebreak: LineBreak::NoWrap },
                        ));

                    // Discard button showing icon_discard (and text "Discard" below, 12 px) using btn_wood states
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
                                width: Val::Px(104.0),
                                height: Val::Px(68.0),
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
                                Text::new("Discard"),
                                TextFont { font_size: FontSize::Px(12.0), ..default() },
                                TextColor(text_color),
                                TextLayout { justify: Justify::Center, linebreak: LineBreak::NoWrap },
                            ));
                        });
                });
        });
}

fn handle_hand_card_interaction(
    mut card_query: Query<(&Interaction, &mut Node, &HandCard), (Changed<Interaction>, With<Button>)>,
    mut highlight_query: Query<(&mut Node, &HandCardHighlight), Without<HandCard>>,
    mut state: ResMut<GameState>,
) {
    for (interaction, mut node, card) in &mut card_query {
        let is_armed = state.armed_slot == Some(card.slot_idx);
        let is_hovered = *interaction == Interaction::Hovered;
        let is_pressed = *interaction == Interaction::Pressed;

        if is_pressed {
            state.arm_slot(card.slot_idx);
        }

        if is_armed || is_hovered || is_pressed {
            node.top = Val::Px(-8.0);
        } else {
            node.top = Val::Px(0.0);
        }

        for (mut hl_node, hl) in &mut highlight_query {
            if hl.0 == card.slot_idx {
                let show = is_armed || is_hovered || is_pressed;
                hl_node.display = if show { Display::Flex } else { Display::None };
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

fn handle_new_run_button(
    button_query: Query<&Interaction, (Changed<Interaction>, With<NewRunButton>)>,
    run: Res<Run>,
    mut start_writer: MessageWriter<StartRun>,
) {
    for interaction in &button_query {
        if *interaction == Interaction::Pressed {
            start_writer.write(StartRun(run.state.size));
        }
    }
}

fn sync_title_menu(
    mut commands: Commands,
    title_menu: Res<TitleMenu>,
    run: Res<Run>,
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

    let has_save = save::has_save();

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
                                width: Val::Px(360.0),
                                height: Val::Px(48.0),
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
                            TextFont { font_size: FontSize::Px(32.0), ..default() },
                            TextColor(INK_WOOD),
                            TextLayout { justify: Justify::Center, ..default() },
                        ));

                    // Column of buttons (min 260 x 52 px, 18 px text)
                    parent
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(10.0),
                            align_items: AlignItems::Center,
                            margin: UiRect::top(Val::Px(36.0)),
                            ..default()
                        })
                        .with_children(|col| {
                            if has_save {
                                col.spawn((
                                    Button,
                                    Interaction::default(),
                                    TitleContinueButton,
                                    ButtonVisuals::GOLD,
                                    Node {
                                        min_width: Val::Px(260.0),
                                        width: Val::Px(260.0),
                                        height: Val::Px(52.0),
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
                                    Text::new(format!("Continue - Floor {}/8", run.state.floor + 1)),
                                    TextFont { font_size: FontSize::Px(18.0), ..default() },
                                    TextColor(INK_WOOD),
                                    TextLayout { justify: Justify::Center, linebreak: LineBreak::NoWrap },
                                ));
                            }

                            for (size, label) in [(8, "New run 8x8"), (16, "New run 16x16"), (32, "New run 32x32")] {
                                col.spawn((
                                    Button,
                                    Interaction::default(),
                                    TitleNewRunButton(size),
                                    ButtonVisuals::WOOD,
                                    Node {
                                        min_width: Val::Px(260.0),
                                        width: Val::Px(260.0),
                                        height: Val::Px(52.0),
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
                                    Text::new(label),
                                    TextFont { font_size: FontSize::Px(18.0), ..default() },
                                    TextColor(INK_WOOD),
                                    TextLayout { justify: Justify::Center, linebreak: LineBreak::NoWrap },
                                ));
                            }

                            col.spawn((
                                Button,
                                Interaction::default(),
                                TitleSandboxButton,
                                ButtonVisuals::WOOD,
                                Node {
                                    min_width: Val::Px(260.0),
                                    width: Val::Px(260.0),
                                    height: Val::Px(52.0),
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
                                Text::new("Sandbox"),
                                TextFont { font_size: FontSize::Px(18.0), ..default() },
                                TextColor(INK_WOOD),
                                TextLayout { justify: Justify::Center, linebreak: LineBreak::NoWrap },
                            ));
                        });

                    // Control hints box in panel_gui_glass
                    parent
                        .spawn((
                            Node {
                                padding: UiRect::axes(Val::Px(14.0), Val::Px(6.0)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                max_width: Val::Px(560.0),
                                margin: UiRect::top(Val::Px(4.0)),
                                ..default()
                            },
                            ImageNode {
                                image: atlas.image.clone(),
                                rect: Some(atlas.rect("panel_gui_glass")),
                                image_mode: panel_slicer(),
                                ..default()
                            },
                        ))
                        .with_child((
                            Text::new("Mouse: Left-click select/move  |  Right-drag orbit  |  Wheel zoom  |  WASD pan\nTouch: Tap select  |  Drag pan  |  Pinch zoom  |  Keys: 5-7 Spells  |  D Discard  |  M Mute  |  Esc Menu"),
                            TextFont { font_size: FontSize::Px(13.0), ..default() },
                            TextColor(INK_WOOD),
                            TextLayout { justify: Justify::Center, linebreak: LineBreak::WordBoundary },
                        ));
                });
        });
}

#[allow(clippy::too_many_arguments)]
fn handle_title_buttons(
    continue_q: Query<&Interaction, (Changed<Interaction>, With<TitleContinueButton>)>,
    new_run_q: Query<(&Interaction, &TitleNewRunButton), (Changed<Interaction>, With<Button>)>,
    sandbox_q: Query<&Interaction, (Changed<Interaction>, With<TitleSandboxButton>)>,
    menu_q: Query<&Interaction, (Changed<Interaction>, With<MenuButton>)>,
    mut title_menu: ResMut<TitleMenu>,
    mut start_writer: MessageWriter<StartRun>,
    mut toggle_writer: MessageWriter<ToggleSandbox>,
    run: Res<Run>,
) {
    for interaction in &continue_q {
        if *interaction == Interaction::Pressed {
            title_menu.open = false;
        }
    }

    for (interaction, btn) in &new_run_q {
        if *interaction == Interaction::Pressed {
            start_writer.write(StartRun(btn.0));
            title_menu.open = false;
        }
    }

    for interaction in &sandbox_q {
        if *interaction == Interaction::Pressed {
            if !run.sandbox {
                toggle_writer.write(ToggleSandbox);
            }
            title_menu.open = false;
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
                handle_new_run_button,
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
