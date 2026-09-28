use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use tc_core::Side;
use tc_run::ItemKind;

use crate::atlas::Atlas;
use crate::game::GameState;
use crate::run::{PickCard, Run, RunPhase, StartRun, TitleMenu, ToggleSandbox};
use crate::save;

/// Ink colour on light wood.
pub const INK_WOOD: Color = Color::srgb_u8(0xF7, 0xED, 0xD0);
/// Ink colour on aged parchment.
pub const INK_PARCHMENT: Color = Color::srgb_u8(0x3B, 0x2F, 0x2A);
/// Gold tint for armed spell cards.
pub const GOLD_TINT: Color = Color::srgb_u8(0xFF, 0xD3, 0x5A);

#[derive(Component)]
struct FloorBadge;

#[derive(Component)]
struct FloorBadgeText;

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
    item_id: String,
}

#[derive(Component)]
struct RunOverOverlay;

#[derive(Component)]
struct NewRunButton;

#[derive(Component)]
struct SpellBar;

#[derive(Component)]
struct SpellCard {
    spell: tc_core::SpellId,
}

fn setup_hud(mut commands: Commands, atlas: Res<Atlas>) {
    // Floor badge in the top-left corner
    commands
        .spawn((
            FloorBadge,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(16.0),
                top: Val::Px(16.0),
                padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ImageNode {
                image: atlas.image.clone(),
                rect: Some(atlas.rect("panel_wood")),
                image_mode: NodeImageMode::Stretch,
                ..default()
            },
        ))
        .with_child((
            Text::new(""),
            TextFont { font_size: FontSize::Px(16.0), ..default() },
            TextColor(INK_WOOD),
            FloorBadgeText,
        ));

    // Menu button at the top-right corner
    commands
        .spawn((
            Button,
            Interaction::default(),
            MenuButton,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(16.0),
                top: Val::Px(16.0),
                padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ImageNode {
                image: atlas.image.clone(),
                rect: Some(atlas.rect("panel_wood")),
                image_mode: NodeImageMode::Stretch,
                ..default()
            },
        ))
        .with_child((
            Text::new("Menu"),
            TextFont { font_size: FontSize::Px(16.0), ..default() },
            TextColor(INK_WOOD),
        ));
}

fn update_floor_badge(run: Res<Run>, mut q: Query<&mut Text, With<FloorBadgeText>>) {
    for mut text in &mut q {
        let label = if run.sandbox {
            "Sandbox".to_string()
        } else if run.state.floor == 7 {
            "Boss floor".to_string()
        } else {
            format!("Floor {} / 8", run.state.floor + 1)
        };
        if text.0 != label {
            text.0 = label;
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
            let (card_w, card_h) = if win_w >= 640.0 {
                (Val::Px(180.0), Val::Px(240.0))
            } else {
                let w = win_w * 0.30;
                (Val::Px(w), Val::Px(w * 4.0 / 3.0))
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
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(28.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.72)),
                ))
                .with_children(|parent| {
                    // Title "Choose a reward" on panel_parchment
                    parent
                        .spawn((
                            Node {
                                min_width: Val::Px(320.0),
                                padding: UiRect::axes(Val::Px(24.0), Val::Px(12.0)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                overflow: Overflow::visible(),
                                ..default()
                            },
                            ImageNode {
                                image: atlas.image.clone(),
                                rect: Some(atlas.rect("panel_parchment")),
                                image_mode: NodeImageMode::Stretch,
                                ..default()
                            },
                        ))
                        .with_child((
                            Text::new("Choose a reward"),
                            TextFont { font_size: FontSize::Px(22.0), ..default() },
                            TextColor(INK_PARCHMENT),
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
                            for item_id in items {
                                let Some(item) = tc_run::find_item(item_id) else { continue };
                                let frame_sprite = match item.rarity {
                                    tc_run::Rarity::Common => "card_common",
                                    tc_run::Rarity::Uncommon => "card_uncommon",
                                    tc_run::Rarity::Rare => "card_rare",
                                };
                                let ink = match item.rarity {
                                    tc_run::Rarity::Common => INK_PARCHMENT,
                                    tc_run::Rarity::Uncommon | tc_run::Rarity::Rare => INK_WOOD,
                                };
                                let kind_str = match &item.kind {
                                    ItemKind::Enhancement { .. } => "Enhancement".to_string(),
                                    ItemKind::Relic { .. } => "Relic".to_string(),
                                    ItemKind::Spell { charges, .. } => {
                                        if *charges == 1 {
                                            "Spell - 1 charge".to_string()
                                        } else {
                                            format!("Spell - {charges} charges")
                                        }
                                    }
                                };

                                row.spawn((
                                    Button,
                                    Interaction::default(),
                                    DraftCard { item_id: item.id.clone() },
                                    Node {
                                        width: card_w,
                                        height: card_h,
                                        position_type: PositionType::Relative,
                                        flex_direction: FlexDirection::Column,
                                        align_items: AlignItems::Center,
                                        justify_content: JustifyContent::FlexStart,
                                        padding: UiRect {
                                            left: Val::Px(12.0),
                                            right: Val::Px(12.0),
                                            top: Val::Px(30.0),
                                            ..default()
                                        },
                                        row_gap: Val::Px(6.0),
                                        overflow: Overflow::clip(),
                                        ..default()
                                    },
                                    ImageNode {
                                        image: atlas.image.clone(),
                                        rect: Some(atlas.rect(frame_sprite)),
                                        image_mode: NodeImageMode::Stretch,
                                        ..default()
                                    },
                                ))
                                .with_children(|card| {
                                    // Item name (18 px, centred, wrapping)
                                    card.spawn((
                                        Text::new(&item.name),
                                        TextFont { font_size: FontSize::Px(18.0), ..default() },
                                        TextColor(ink),
                                        TextLayout {
                                            justify: Justify::Center,
                                            linebreak: LineBreak::WordBoundary,
                                        },
                                    ));
                                    // Item kind (13 px)
                                    card.spawn((
                                        Text::new(kind_str),
                                        TextFont { font_size: FontSize::Px(13.0), ..default() },
                                        TextColor(ink),
                                        TextLayout {
                                            justify: Justify::Center,
                                            linebreak: LineBreak::WordBoundary,
                                        },
                                    ));
                                    // Description (14 px, wrapping)
                                    card.spawn((
                                        Text::new(&item.description),
                                        TextFont { font_size: FontSize::Px(14.0), ..default() },
                                        TextColor(ink),
                                        TextLayout {
                                            justify: Justify::Center,
                                            linebreak: LineBreak::WordBoundary,
                                        },
                                    ));
                                });
                            }
                        });
                });
        }
        RunPhase::Over { won } => {
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
                                row_gap: Val::Px(16.0),
                                min_width: Val::Px(360.0),
                                min_height: Val::Px(260.0),
                                ..default()
                            },
                            ImageNode {
                                image: atlas.image.clone(),
                                rect: Some(atlas.rect("panel_parchment")),
                                image_mode: NodeImageMode::Stretch,
                                ..default()
                            },
                        ))
                        .with_children(|panel| {
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

                            panel
                                .spawn((
                                    Button,
                                    Interaction::default(),
                                    NewRunButton,
                                    Node {
                                        min_width: Val::Px(160.0),
                                        min_height: Val::Px(48.0),
                                        padding: UiRect::axes(Val::Px(24.0), Val::Px(10.0)),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    ImageNode {
                                        image: atlas.image.clone(),
                                        rect: Some(atlas.rect("panel_wood")),
                                        image_mode: NodeImageMode::Stretch,
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
    mut cards: Query<&mut Node, With<DraftCard>>,
) {
    let Some(win) = window.iter().next() else { return };
    let (card_w, card_h) = if win.width() >= 640.0 {
        (Val::Px(180.0), Val::Px(240.0))
    } else {
        let w = win.width() * 0.30;
        (Val::Px(w), Val::Px(w * 4.0 / 3.0))
    };
    for mut node in &mut cards {
        node.width = card_w;
        node.height = card_h;
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

fn sync_spell_bar(
    mut commands: Commands,
    state: Res<GameState>,
    run: Res<Run>,
    title_menu: Res<TitleMenu>,
    atlas: Res<Atlas>,
    mut last_key: Local<Option<(bool, Vec<(tc_core::SpellId, u8)>, Option<tc_core::SpellId>)>>,
    spell_bar_query: Query<Entity, With<SpellBar>>,
) {
    let show = !title_menu.open
        && !run.sandbox
        && run.phase == RunPhase::Playing
        && state.game.pos.side_to_move == Side::White
        && !state.ai_to_move()
        && state.outcome.is_none()
        && !state.castable_spells().is_empty();

    let spells = if show { state.castable_spells() } else { Vec::new() };
    let current_key = (show, spells.clone(), state.armed_spell);

    if *last_key == Some(current_key.clone()) {
        return;
    }
    *last_key = Some(current_key);

    for e in &spell_bar_query {
        commands.entity(e).despawn();
    }

    if !show {
        return;
    }

    commands
        .spawn((
            SpellBar,
            GlobalZIndex(10),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                bottom: Val::Px(16.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexEnd,
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(8.0),
                padding: UiRect::horizontal(Val::Px(12.0)),
                ..default()
            },
        ))
        .with_children(|row| {
            for (spell, count) in spells {
                let is_armed = state.armed_spell == Some(spell);
                let (top, tint) =
                    if is_armed { (Val::Px(-8.0), GOLD_TINT) } else { (Val::Px(0.0), Color::WHITE) };

                row.spawn((
                    Button,
                    Interaction::default(),
                    SpellCard { spell },
                    Node {
                        width: Val::Px(84.0),
                        height: Val::Px(112.0),
                        position_type: PositionType::Relative,
                        top,
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::FlexStart,
                        padding: UiRect {
                            left: Val::Px(12.0),
                            right: Val::Px(12.0),
                            top: Val::Px(22.0),
                            ..default()
                        },
                        row_gap: Val::Px(6.0),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    ImageNode {
                        image: atlas.image.clone(),
                        rect: Some(atlas.rect("card_uncommon")),
                        color: tint,
                        image_mode: NodeImageMode::Stretch,
                        ..default()
                    },
                ))
                .with_children(|card| {
                    // Spell name (12 px, wrapped)
                    card.spawn((
                        Text::new(spell_name(spell)),
                        TextFont { font_size: FontSize::Px(12.0), ..default() },
                        TextColor(INK_WOOD),
                        TextLayout { justify: Justify::Center, linebreak: LineBreak::WordBoundary },
                    ));

                    // Quick tag if quick spell
                    if spell.is_quick() {
                        card.spawn((
                            Text::new("Quick"),
                            TextFont { font_size: FontSize::Px(11.0), ..default() },
                            TextColor(INK_WOOD),
                            TextLayout { justify: Justify::Center, linebreak: LineBreak::WordBoundary },
                        ));
                    }

                    // Charges "xN"
                    card.spawn((
                        Text::new(format!("x{count}")),
                        TextFont { font_size: FontSize::Px(14.0), ..default() },
                        TextColor(INK_WOOD),
                        TextLayout { justify: Justify::Center, linebreak: LineBreak::WordBoundary },
                        Node { margin: UiRect { top: Val::Auto, ..default() }, ..default() },
                    ));
                });
            }
        });
}

fn handle_spell_card_interaction(
    mut card_query: Query<(&Interaction, &mut Node, &SpellCard), (Changed<Interaction>, With<Button>)>,
    mut state: ResMut<GameState>,
) {
    for (interaction, mut node, card) in &mut card_query {
        let is_armed = state.armed_spell == Some(card.spell);
        match *interaction {
            Interaction::Pressed => {
                if is_armed {
                    state.disarm();
                } else {
                    state.arm_spell(card.spell);
                }
            }
            Interaction::Hovered => {
                if !is_armed {
                    node.top = Val::Px(-4.0);
                }
            }
            Interaction::None => {
                if !is_armed {
                    node.top = Val::Px(0.0);
                }
            }
        }
    }
}

fn handle_card_interaction(
    mut card_query: Query<(&Interaction, &mut Node, &DraftCard), (Changed<Interaction>, With<Button>)>,
    mut pick_writer: MessageWriter<PickCard>,
) {
    for (interaction, mut node, card) in &mut card_query {
        match *interaction {
            Interaction::Hovered => {
                node.top = Val::Px(-8.0);
            }
            Interaction::Pressed => {
                node.top = Val::Px(-8.0);
                pick_writer.write(PickCard(card.item_id.clone()));
            }
            Interaction::None => {
                node.top = Val::Px(0.0);
            }
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
    let floor_label = format!("Floor {} / 8", run.state.floor + 1);

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
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(14.0),
                padding: UiRect::all(Val::Px(24.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.09, 0.14, 0.72)),
        ))
        .with_children(|parent| {
            // Title banner: large light ink on a panel_wood banner
            parent
                .spawn((
                    Node {
                        min_width: Val::Px(320.0),
                        padding: UiRect::axes(Val::Px(32.0), Val::Px(14.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    ImageNode {
                        image: atlas.image.clone(),
                        rect: Some(atlas.rect("panel_wood")),
                        image_mode: NodeImageMode::Stretch,
                        ..default()
                    },
                ))
                .with_child((
                    Text::new("Terrain Chess"),
                    TextFont { font_size: FontSize::Px(36.0), ..default() },
                    TextColor(INK_WOOD),
                    TextLayout { justify: Justify::Center, ..default() },
                ));

            // Column of panel_wood buttons (min 220 x 48 px, 18 px text)
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(10.0),
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|col| {
                    if has_save {
                        col.spawn((
                            Button,
                            Interaction::default(),
                            TitleContinueButton,
                            Node {
                                min_width: Val::Px(220.0),
                                min_height: Val::Px(48.0),
                                padding: UiRect::axes(Val::Px(20.0), Val::Px(10.0)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            ImageNode {
                                image: atlas.image.clone(),
                                rect: Some(atlas.rect("panel_wood")),
                                image_mode: NodeImageMode::Stretch,
                                ..default()
                            },
                        ))
                        .with_child((
                            Text::new(format!("Continue ({floor_label})")),
                            TextFont { font_size: FontSize::Px(18.0), ..default() },
                            TextColor(INK_WOOD),
                        ));
                    }

                    for (size, label) in [(8, "New run 8x8"), (16, "New run 16x16"), (32, "New run 32x32")] {
                        col.spawn((
                            Button,
                            Interaction::default(),
                            TitleNewRunButton(size),
                            Node {
                                min_width: Val::Px(220.0),
                                min_height: Val::Px(48.0),
                                padding: UiRect::axes(Val::Px(20.0), Val::Px(10.0)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            ImageNode {
                                image: atlas.image.clone(),
                                rect: Some(atlas.rect("panel_wood")),
                                image_mode: NodeImageMode::Stretch,
                                ..default()
                            },
                        ))
                        .with_child((
                            Text::new(label),
                            TextFont { font_size: FontSize::Px(18.0), ..default() },
                            TextColor(INK_WOOD),
                        ));
                    }

                    col.spawn((
                        Button,
                        Interaction::default(),
                        TitleSandboxButton,
                        Node {
                            min_width: Val::Px(220.0),
                            min_height: Val::Px(48.0),
                            padding: UiRect::axes(Val::Px(20.0), Val::Px(10.0)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        ImageNode {
                            image: atlas.image.clone(),
                            rect: Some(atlas.rect("panel_wood")),
                            image_mode: NodeImageMode::Stretch,
                            ..default()
                        },
                    ))
                    .with_child((
                        Text::new("Sandbox"),
                        TextFont { font_size: FontSize::Px(18.0), ..default() },
                        TextColor(INK_WOOD),
                    ));
                });

            // Control hints under it (keys, mouse, touch in two short lines, 13 px)
            parent.spawn((
                Text::new("Mouse: Left-click select/move  |  Right-drag orbit  |  Wheel zoom  |  WASD pan\nTouch: Tap select  |  Drag pan  |  Pinch zoom  |  Keys: 5-9 Spells  |  M Mute  |  Esc Menu"),
                TextFont { font_size: FontSize::Px(13.0), ..default() },
                TextColor(INK_WOOD),
                TextLayout { justify: Justify::Center, linebreak: LineBreak::WordBoundary },
                Node { max_width: Val::Percent(90.0), ..default() },
            ));
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

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_hud).add_systems(
            Update,
            (
                update_floor_badge,
                sync_overlays,
                sync_title_menu,
                update_draft_card_sizes,
                handle_card_interaction,
                handle_new_run_button,
                handle_title_buttons,
                handle_spell_card_interaction,
                sync_spell_bar,
            )
                .chain(),
        );
    }
}
