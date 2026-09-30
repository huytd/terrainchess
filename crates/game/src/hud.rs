use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;
use bevy::window::PrimaryWindow;
use tc_core::rules::DrawReason;
use tc_core::{Outcome, Side, SpellId};
use tc_run::ItemKind;

use crate::atlas::Atlas;
use crate::game::GameState;
use crate::loading::AppState;
use crate::run::{DeckEdit, DeckSlot, PickCard, Run, RunCommand, RunPhase, TitleMenu};
use crate::theme::{
    self, ButtonDisabled, GOLD, GREEN, GREY, INFO_BG, L, M, ORANGE, OVERLAY_GUTTER, PanelKind, RED, S, SLATE,
    SLATE_DARK, SLATE_LIGHT, TEXT, TEXT_DARK, TEXT_DIM, XL, XS, button, card_root, ink, integer_scaled_size,
    label, label_nowrap, number_chip, panel, scrim, spawn_card_face, spell_accent, spell_item_id, spell_name,
    tag_chip, title_bar,
};
use crate::ui_fx::{CardMotion, PopIn};

/// Clamps a preferred panel width so it fits inside the overlay gutter on narrow screens.
fn fit_width(preferred: f32, win_w: f32) -> f32 {
    preferred.min(win_w - 2.0 * OVERLAY_GUTTER).max(0.0)
}

#[derive(Component)]
struct FloorBadgeNumber;

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

/// Main menu buttons.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum MenuChoice {
    Resume,
    Continue,
    NewRun,
    Abandon,
}

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

/// The big "YOU WIN!" / "CHECKMATED" / "STALEMATE" title: pops in at the centre, then
/// shrinks up toward the top so the board can be reviewed.
#[derive(Component, Default)]
struct EndTitle {
    t: f32,
}

#[derive(Component)]
struct MenuOpenButton;

#[derive(Component)]
struct NextStageButton;

#[derive(Component)]
struct ClaimButton;

#[derive(Component)]
struct HandBar;

#[derive(Component)]
struct HandCard {
    slot_idx: usize,
    is_used: bool,
}

/// The info box above a hand card, shown while it's hovered or armed.
#[derive(Component)]
struct HandTooltip(usize);

#[derive(Component)]
struct DiscardButton;

fn setup_hud(mut commands: Commands, atlas: Res<Atlas>) {
    // Stage badge and enemy modifier in the top-left corner.
    let skull_size = atlas.px("icon_skull");
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            left: Val::Px(16.0),
            top: Val::Px(16.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.0),
            align_items: AlignItems::FlexStart,
            ..default()
        })
        .with_children(|col| {
            col.spawn(panel(
                PanelKind::Outer,
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(10.0),
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(6.0)),
                    ..default()
                },
            ))
            .with_children(|badge| {
                badge.spawn(number_chip(RED, 32.0)).with_child((label_nowrap("", S, TEXT), FloorBadgeNumber));
                badge.spawn((label_nowrap("", S, TEXT), FloorBadgeText));
            });

            col.spawn((
                FloorBadgeEnemyBox,
                panel(
                    PanelKind::Inner,
                    Node {
                        display: Display::None,
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(8.0),
                        padding: UiRect::axes(Val::Px(10.0), Val::Px(4.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                ),
            ))
            .insert(BorderColor::all(RED.with_alpha(0.6)))
            .with_children(|row| {
                row.spawn((
                    Node { width: Val::Px(skull_size.x), height: Val::Px(skull_size.y), ..default() },
                    ImageNode {
                        image: atlas.image.clone(),
                        rect: Some(atlas.rect("icon_skull")),
                        ..default()
                    },
                ));
                row.spawn((label_nowrap("", XS, RED.lighter(0.1)), FloorBadgeEnemyText));
            });
        });

    // Menu button at the top-right corner.
    let gear_size = atlas.px("icon_gear");
    let mut gear = commands.spawn((MenuButton, button(SLATE_LIGHT, Val::Px(44.0), 44.0)));
    gear.entry::<Node>().and_modify(|mut n| {
        n.position_type = PositionType::Absolute;
        n.right = Val::Px(16.0);
        n.top = Val::Px(16.0);
        n.padding = UiRect::ZERO;
        n.border_radius = BorderRadius::MAX;
    });
    gear.with_child((
        Node { width: Val::Px(gear_size.x), height: Val::Px(gear_size.y), ..default() },
        ImageNode { image: atlas.image.clone(), rect: Some(atlas.rect("icon_gear")), ..default() },
        Pickable::IGNORE,
    ));
}

fn update_floor_badge(
    run: Res<Run>,
    state: Res<GameState>,
    mut q_text: Query<
        (&mut Text, Has<FloorBadgeNumber>, Has<FloorBadgeText>, Has<FloorBadgeEnemyText>),
        Or<(With<FloorBadgeNumber>, With<FloorBadgeText>, With<FloorBadgeEnemyText>)>,
    >,
    mut q_enemy_box: Query<&mut Node, With<FloorBadgeEnemyBox>>,
) {
    let name = run.stage.name();
    let enemy_label = if state.enemy_items.is_empty() {
        String::new()
    } else {
        let names: Vec<_> = state
            .enemy_items
            .iter()
            .map(|id| tc_run::find_item(id).map(|item| item.name.as_str()).unwrap_or(id.as_str()))
            .collect();
        format!("Enemy: {}", names.join(", "))
    };

    for (mut text, is_number, is_name, is_enemy) in &mut q_text {
        let want = if is_number {
            run.stage.number.to_string()
        } else if is_name {
            name.clone()
        } else if is_enemy {
            enemy_label.clone()
        } else {
            continue;
        };
        if text.0 != want {
            text.0 = want;
        }
    }

    for mut box_node in &mut q_enemy_box {
        let display = if enemy_label.is_empty() { Display::None } else { Display::Flex };
        if box_node.display != display {
            box_node.display = display;
        }
    }
}

/// One line explaining how a match ended, shown on the result screen (the player is White).
fn outcome_reason(outcome: Option<Outcome>) -> &'static str {
    match outcome {
        Some(Outcome::Checkmate { winner: Side::White }) => "Checkmate, White wins",
        Some(Outcome::Checkmate { winner: Side::Black }) => "Checkmate, Black wins",
        Some(Outcome::Draw(DrawReason::Stalemate)) => "Draw by stalemate",
        Some(Outcome::Draw(DrawReason::FiftyMoves)) => "Draw by the fifty-move rule",
        Some(Outcome::Draw(DrawReason::Repetition)) => "Draw by threefold repetition",
        Some(Outcome::Draw(DrawReason::InsufficientMaterial)) => "Draw: insufficient material",
        None => "",
    }
}

/// Draft card height for a window width.
fn draft_card_h(win_w: f32) -> f32 {
    if win_w >= 800.0 {
        324.0
    } else if win_w >= 640.0 {
        240.0
    } else {
        (win_w * 0.40).floor()
    }
}

#[allow(clippy::too_many_arguments)]
fn sync_overlays(
    mut commands: Commands,
    run: Res<Run>,
    atlas: Res<Atlas>,
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

    let win_w = window.iter().next().map(|w| w.width()).unwrap_or(800.0);
    let compact = win_w < 500.0;
    match &run.phase {
        RunPhase::Playing => {}
        RunPhase::Draft(items) => {
            let card_h = draft_card_h(win_w);
            let card_w = (card_h * 59.0 / 81.0).round();
            let body_px = if card_w < 170.0 { XS } else { S };

            commands.spawn((DraftOverlay, GlobalZIndex(100), scrim())).with_children(|parent| {
                parent
                    .spawn((title_bar(RED, RED.darker(0.18)), UiTransform::default(), PopIn::new(0.0, 0.25)))
                    .with_child(label("Choose a reward", theme::size(win_w, L, M), TEXT));

                parent
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::FlexStart,
                        column_gap: Val::Px(if compact { 10.0 } else { 24.0 }),
                        margin: UiRect::top(Val::Px(if compact { 20.0 } else { 36.0 })),
                        max_width: Val::Percent(100.0),
                        ..default()
                    })
                    .with_children(|row| {
                        for (i, item_id) in items.iter().enumerate() {
                            let Some(item) = tc_run::find_item(item_id) else { continue };
                            let kind = match &item.kind {
                                ItemKind::Enhancement { .. } | ItemKind::Veteran => "Upgrade".to_string(),
                                ItemKind::Relic { .. } => "Relic".to_string(),
                                ItemKind::Spell { charges, .. } => format!("Spell x{charges}"),
                            };
                            let rarity = match item.rarity {
                                tc_run::Rarity::Common => "Common",
                                tc_run::Rarity::Uncommon => "Uncommon",
                                tc_run::Rarity::Rare => "Rare",
                            };
                            let accent = theme::rarity_color(item.rarity);
                            let art = format!("art_{}", item.id);

                            row.spawn(Node {
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                row_gap: Val::Px(12.0),
                                width: Val::Px(card_w + if compact { 8.0 } else { 40.0 }),
                                ..default()
                            })
                            .with_children(|col| {
                                col.spawn((
                                    Button,
                                    Interaction::default(),
                                    DraftCard { item_id: item.id.clone() },
                                    CardMotion::new(10.0, 1.08, true, i as f32 * 1.7),
                                    PopIn::new(0.08 * i as f32, 0.3),
                                    RelativeCursorPosition::default(),
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
                                    // Phone cards only have room for the rarity tag.
                                    let all_tags = [(kind.as_str(), SLATE_LIGHT), (rarity, accent)];
                                    spawn_card_face(
                                        card,
                                        &atlas,
                                        &art,
                                        &item.name,
                                        accent,
                                        &all_tags[usize::from(compact)..],
                                        card_w,
                                        body_px,
                                    );
                                });

                                col.spawn((
                                    Node {
                                        width: Val::Percent(100.0),
                                        padding: UiRect::axes(Val::Px(8.0), Val::Px(6.0)),
                                        justify_content: JustifyContent::Center,
                                        border_radius: BorderRadius::all(Val::Px(6.0)),
                                        ..default()
                                    },
                                    BackgroundColor(INFO_BG),
                                    UiTransform::default(),
                                    PopIn::new(0.08 * i as f32 + 0.1, 0.25),
                                ))
                                .with_child(ink(
                                    item.description.clone(),
                                    body_px,
                                    TEXT_DARK,
                                ));
                            });
                        }
                    });
            });
        }
        RunPhase::Result { won } => {
            let is_draw = matches!(game_state.outcome, Some(Outcome::Draw(_)));
            let (title, fill, lip) = if *won {
                ("VICTORY", GREEN, GREEN.darker(0.18))
            } else if is_draw {
                ("DRAW", SLATE_LIGHT, SLATE_DARK)
            } else {
                ("DEFEAT", SLATE_LIGHT, RED)
            };
            // A compact strip at the bottom with no scrim, so the final board stays in view.
            let narrow = win_w < 640.0;
            let btn_h = if narrow { 44.0 } else { 48.0 };
            let btn_px = theme::size(win_w, M, S);
            let stage_n = run.stage.number;
            let detail = if *won {
                format!("Stage {stage_n} cleared. Next: {}", run.profile.stage().name())
            } else if is_draw {
                format!("A draw counts as a loss. Run over at stage {}.", run.ended_at.unwrap_or(stage_n))
            } else {
                format!(
                    "Run over at stage {}. Best: stage {}",
                    run.ended_at.unwrap_or(stage_n),
                    run.profile.best_stage
                )
            };
            let edge = if narrow { 8.0 } else { 16.0 };

            let (big, big_color) = match game_state.outcome {
                Some(Outcome::Checkmate { winner: Side::White }) => ("YOU WIN!", GOLD),
                Some(Outcome::Checkmate { .. }) => ("CHECKMATED", RED),
                Some(Outcome::Draw(DrawReason::Stalemate)) => ("STALEMATE", TEXT),
                _ if *won => ("YOU WIN!", GOLD),
                _ if is_draw => ("DRAW", TEXT),
                _ => ("DEFEAT", RED),
            };
            commands
                .spawn((
                    RunOverOverlay,
                    GlobalZIndex(90),
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
                    Pickable::IGNORE,
                ))
                .with_child((
                    label_nowrap(big, if narrow { 63.0 } else { 108.0 }, big_color),
                    EndTitle::default(),
                    UiTransform { scale: Vec2::ZERO, ..default() },
                ));

            commands
                .spawn((
                    RunOverOverlay,
                    GlobalZIndex(100),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        right: Val::Px(0.0),
                        bottom: Val::Px(edge),
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .with_children(|parent| {
                    parent
                        .spawn((
                            panel(
                                PanelKind::Outer,
                                Node {
                                    width: Val::Px(fit_width(720.0, win_w).min(win_w - 2.0 * edge)),
                                    flex_direction: if narrow {
                                        FlexDirection::Column
                                    } else {
                                        FlexDirection::Row
                                    },
                                    align_items: if narrow {
                                        AlignItems::FlexStart
                                    } else {
                                        AlignItems::Center
                                    },
                                    column_gap: Val::Px(16.0),
                                    row_gap: Val::Px(8.0),
                                    padding: if narrow {
                                        UiRect::all(Val::Px(10.0))
                                    } else {
                                        UiRect::axes(Val::Px(16.0), Val::Px(12.0))
                                    },
                                    ..default()
                                },
                            ),
                            PopIn::new(2.4, 0.3),
                        ))
                        .with_children(|panel| {
                            panel
                                .spawn(title_bar(fill, lip))
                                .insert(Node { flex_shrink: 0.0, ..default() })
                                .with_child(label_nowrap(title, theme::size(win_w, M, S), TEXT));
                            panel
                                .spawn(Node {
                                    flex_direction: FlexDirection::Column,
                                    flex_grow: 1.0,
                                    flex_shrink: 1.0,
                                    row_gap: Val::Px(2.0),
                                    ..default()
                                })
                                .with_children(|col| {
                                    let reason = outcome_reason(game_state.outcome);
                                    if !reason.is_empty() {
                                        col.spawn(label(reason, S, TEXT));
                                    }
                                    col.spawn(label(detail, XS, TEXT_DIM));
                                });
                            panel
                                .spawn(Node {
                                    flex_direction: FlexDirection::Row,
                                    column_gap: Val::Px(12.0),
                                    flex_shrink: 0.0,
                                    align_self: if narrow { AlignSelf::Center } else { AlignSelf::Auto },
                                    ..default()
                                })
                                .with_children(|row| {
                                    if *won && run.pending_draft.is_some() {
                                        row.spawn((ClaimButton, button(ORANGE, Val::Px(200.0), btn_h)))
                                            .with_child(label_nowrap("Claim reward", btn_px, TEXT));
                                    } else if *won {
                                        row.spawn((NextStageButton, button(ORANGE, Val::Px(180.0), btn_h)))
                                            .with_child(label_nowrap("Next stage", btn_px, TEXT));
                                    } else {
                                        row.spawn((NewRunButton, button(ORANGE, Val::Px(150.0), btn_h)))
                                            .with_child(label_nowrap("New run", btn_px, TEXT));
                                        row.spawn((
                                            MenuOpenButton,
                                            button(SLATE_LIGHT, Val::Px(120.0), btn_h),
                                        ))
                                        .with_child(label_nowrap("Menu", btn_px, TEXT));
                                    }
                                });
                        });
                });
        }
    }
}

/// Timeline: hidden while the last move lands, pops in, holds, then shrinks up and out of the
/// board's way.
fn animate_end_title(
    time: Res<Time>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut q: Query<(&mut EndTitle, &mut UiTransform)>,
) {
    let win_h = window.iter().next().map_or(800.0, |w| w.height());
    for (mut title, mut tf) in &mut q {
        title.t += time.delta_secs();
        let t = title.t;
        let pop = ((t - 0.8) / 0.45).clamp(0.0, 1.0);
        let settle = ((t - 3.2) / 0.9).clamp(0.0, 1.0);
        let settle = settle * settle * (3.0 - 2.0 * settle);
        let scale = if pop <= 0.0 { 0.0 } else { crate::ui_fx::ease_out_back(pop) } * (1.0 - 0.45 * settle);
        tf.scale = Vec2::splat(scale);
        tf.translation = Val2::px(0.0, -win_h * 0.33 * settle);
    }
}

fn update_draft_card_sizes(
    window: Query<&Window, (With<PrimaryWindow>, Changed<Window>)>,
    mut cards: Query<&mut Node, With<DraftCard>>,
) {
    let Some(win) = window.iter().next() else { return };
    let card_h = draft_card_h(win.width());
    let card_w = (card_h * 59.0 / 81.0).round();
    for mut node in &mut cards {
        node.width = Val::Px(card_w);
        node.height = Val::Px(card_h);
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

    // Integer scaling keeps the card art crisp: 3 cards fit a phone's width, and on desktop a card
    // is about 30% of the window height.
    let max_scale_w = ((win_w * 0.90) / (2.7 * 59.0)).floor() as u32;
    let target_scale_h = ((win_h * 0.30) / 81.0).round() as u32;
    let scale = target_scale_h.min(max_scale_w).max(1);
    let card_w = 59.0 * scale as f32;
    let card_h = 81.0 * scale as f32;

    // Resting card position: about 20% of the card is below the screen edge, and the fan never
    // covers more than the bottom ~26% of the screen.
    let max_visible_h = win_h * 0.26;
    let mut resting_bottom = -0.20 * card_h;
    let resting_visible_h = card_h + resting_bottom;
    if resting_visible_h > max_visible_h {
        resting_bottom -= resting_visible_h - max_visible_h;
    }
    let lift_px = 0.18 * card_h;

    let present_slots: Vec<usize> = (0..3).filter(|&i| hand_cards[i].is_some()).collect();
    let n = present_slots.len();
    let overlap = if win_w < 500.0 { 0.37 } else { 0.15 };
    let dx = (1.0 - overlap) * card_w;

    let is_compact = win_w < 600.0;
    let sidebar_w = if is_compact { 108.0 } else { 120.0 };
    let name_px = if card_w < 170.0 { XS } else { S };
    // Fixed button width: an auto-sized button in this absolutely positioned column measured
    // narrower than its label, so the text spilled out.
    let discard_btn_w = if is_compact { 108.0 } else { 150.0 };
    let center_x = if is_compact { (win_w - (sidebar_w + 14.0)) * 0.5 } else { win_w * 0.5 };
    let tip_w = if is_compact { (win_w - 24.0).min(260.0) } else { (1.7 * card_w).max(260.0) };

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
            Pickable::IGNORE,
        ))
        .with_children(|root| {
            for (k, &slot_idx) in present_slots.iter().enumerate() {
                let Some(spell) = hand_cards[slot_idx] else { continue };
                let is_used = hand_used[slot_idx];
                let blocked = blocks[slot_idx];
                let is_armed = armed_slot == Some(slot_idx);

                let (base_angle_deg, drop_y) = match (n, k) {
                    (3, 0) => (-6.0, 0.04 * card_h),
                    (3, 2) => (6.0, 0.04 * card_h),
                    (2, 0) => (-4.0, 0.03 * card_h),
                    (2, _) => (4.0, 0.03 * card_h),
                    _ => (0.0, 0.0),
                };
                let card_center_x = center_x + (k as f32 - (n as f32 - 1.0) * 0.5) * dx;
                let card_left = card_center_x - 0.5 * card_w;

                let mut motion =
                    CardMotion::new(lift_px, 1.08, true, slot_idx as f32 * 1.7).settled(is_armed && !is_used);
                motion.base_rot_deg = base_angle_deg;
                motion.pivot = Vec2::new(0.0, 0.5 * card_h);

                let mut tags: Vec<(&str, Color)> = Vec::new();
                let key = ["1", "2", "3"][slot_idx];
                if is_used {
                    tags.push(("Used", GREY));
                } else {
                    tags.push((key, SLATE_LIGHT));
                    if spell.is_quick() {
                        tags.push(("Quick", GOLD));
                    }
                }

                // Keep the tooltip on screen.
                let tip_left =
                    (card_left + (card_w - tip_w) * 0.5).clamp(8.0, (win_w - tip_w - 8.0).max(8.0));

                root.spawn((
                    Button,
                    Interaction::default(),
                    HandCard { slot_idx, is_used },
                    motion,
                    RelativeCursorPosition::default(),
                    if is_armed { ZIndex(10) } else { ZIndex(k as i32 + 1) },
                    card_root(
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(card_left),
                            bottom: Val::Px(resting_bottom - drop_y),
                            width: Val::Px(card_w),
                            height: Val::Px(card_h),
                            overflow: Overflow::visible(),
                            ..default()
                        },
                        5.0 * scale as f32,
                    ),
                ))
                .insert(Outline {
                    width: Val::Px(3.0),
                    offset: Val::Px(2.0),
                    color: if is_armed { GOLD } else { Color::NONE },
                })
                .with_children(|card| {
                    let art = format!("art_{}", spell_item_id(spell));
                    spawn_card_face(
                        card,
                        &atlas,
                        &art,
                        spell_name(spell),
                        spell_accent(spell),
                        &tags,
                        card_w,
                        name_px,
                    );

                    let cover = Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        top: Val::Px(0.0),
                        right: Val::Px(0.0),
                        bottom: Val::Px(0.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border_radius: BorderRadius::all(Val::Px(4.0 * scale as f32)),
                        ..default()
                    };
                    if is_used {
                        card.spawn((cover, BackgroundColor(SLATE.with_alpha(0.65)), Pickable::IGNORE))
                            .with_children(|c| {
                                c.spawn((
                                    Node {
                                        padding: UiRect::axes(Val::Px(12.0), Val::Px(2.0)),
                                        border_radius: BorderRadius::all(Val::Px(6.0)),
                                        ..default()
                                    },
                                    BackgroundColor(RED),
                                    BoxShadow(vec![theme::lip(RED, 3.0)]),
                                    UiTransform { rotation: Rot2::degrees(-12.0), ..default() },
                                    Pickable::IGNORE,
                                ))
                                .with_child(label_nowrap("USED", name_px, TEXT));
                            });
                    } else if blocked.is_some() {
                        // Can't be cast now: dim the card and pin a red "!" on it.
                        card.spawn((cover, BackgroundColor(SLATE_DARK.with_alpha(0.45)), Pickable::IGNORE));
                        let badge = 8.0 * scale as f32;
                        card.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                right: Val::Px(-badge * 0.3),
                                top: Val::Px(-badge * 0.3),
                                width: Val::Px(badge),
                                height: Val::Px(badge),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border_radius: BorderRadius::MAX,
                                ..default()
                            },
                            BackgroundColor(RED),
                            Pickable::IGNORE,
                        ))
                        .with_child(label_nowrap("!", S, TEXT));
                    }

                    // Info box above the card.
                    card.spawn((
                        HandTooltip(slot_idx),
                        panel(
                            PanelKind::Outer,
                            Node {
                                display: if is_armed && !is_used { Display::Flex } else { Display::None },
                                position_type: PositionType::Absolute,
                                bottom: Val::Percent(104.0),
                                left: Val::Px(tip_left - card_left),
                                width: Val::Px(tip_w),
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                row_gap: Val::Px(6.0),
                                padding: UiRect::all(Val::Px(8.0)),
                                ..default()
                            },
                        ),
                        Pickable::IGNORE,
                    ))
                    .with_children(|tip| {
                        tip.spawn(label(spell_name(spell), theme::size(win_w, M, S), TEXT));
                        tip.spawn((
                            Node {
                                width: Val::Percent(100.0),
                                padding: UiRect::axes(Val::Px(8.0), Val::Px(6.0)),
                                justify_content: JustifyContent::Center,
                                border_radius: BorderRadius::all(Val::Px(6.0)),
                                ..default()
                            },
                            BackgroundColor(INFO_BG),
                            Pickable::IGNORE,
                        ))
                        .with_child(ink(
                            spell.effect_text(),
                            theme::size(win_w, S, XS),
                            TEXT_DARK,
                        ));
                        if spell.is_quick() || blocked.is_some() {
                            tip.spawn((
                                Node {
                                    flex_direction: FlexDirection::Row,
                                    flex_wrap: FlexWrap::Wrap,
                                    justify_content: JustifyContent::Center,
                                    column_gap: Val::Px(6.0),
                                    row_gap: Val::Px(4.0),
                                    ..default()
                                },
                                Pickable::IGNORE,
                            ))
                            .with_children(|chips| {
                                if spell.is_quick() {
                                    chips.spawn(tag_chip(GOLD)).with_child(label(
                                        "Quick: free action",
                                        XS,
                                        TEXT,
                                    ));
                                }
                                if let Some(reason) = blocked {
                                    chips.spawn(tag_chip(RED)).with_child(label(reason, XS, TEXT));
                                }
                            });
                        }
                    });
                });
            }

            // To the right: Discard button above, deck pile below.
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(if is_compact { 6.0 } else { 24.0 }),
                    bottom: Val::Px(16.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::FlexEnd,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(12.0),
                    ..default()
                },
                GlobalZIndex(15),
            ))
            .with_children(|sidebar| {
                let text_color = if can_discard { TEXT } else { TEXT.with_alpha(0.4) };
                sidebar
                    .spawn((
                        DiscardButton,
                        ButtonDisabled(!can_discard),
                        button(RED, Val::Px(discard_btn_w), 40.0),
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
                            Pickable::IGNORE,
                        ));
                        btn.spawn(label_nowrap(
                            format!("{discards_left}/{}", tc_core::MAX_DISCARDS),
                            theme::size(win_w, S, XS),
                            text_color,
                        ));
                    });

                // Deck pile: two stacked card backs with the count on top.
                let (pile_w, pile_h) = if is_compact { (38.0, 52.0) } else { (46.0, 62.0) };
                let back = |z: i32, dx: f32, rot: f32| {
                    (
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(dx),
                            top: Val::Px(dx),
                            width: Val::Px(pile_w),
                            height: Val::Px(pile_h),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(2.0)),
                            border_radius: BorderRadius::all(Val::Px(6.0)),
                            ..default()
                        },
                        BackgroundColor(RED),
                        BorderColor::all(theme::CARD_FACE),
                        BoxShadow(vec![theme::lip(Color::BLACK.with_alpha(0.4), 3.0)]),
                        UiTransform { rotation: Rot2::degrees(rot), ..default() },
                        ZIndex(z),
                    )
                };
                sidebar
                    .spawn(Node { width: Val::Px(pile_w + 4.0), height: Val::Px(pile_h + 4.0), ..default() })
                    .with_children(|pile| {
                        pile.spawn(back(0, 4.0, 4.0));
                        pile.spawn(back(1, 0.0, 0.0)).with_child(label_nowrap(
                            deck_len.to_string(),
                            theme::size(win_w, M, S),
                            TEXT,
                        ));
                    });
            });
        });
}

fn handle_hand_card_interaction(
    mut card_query: Query<
        (&Interaction, &mut CardMotion, &mut ZIndex, &mut Outline, &HandCard),
        (With<Button>, Or<(Changed<Interaction>, Added<HandCard>)>),
    >,
    mut tooltips: Query<(&mut Node, &HandTooltip)>,
    mut state: ResMut<GameState>,
) {
    for (interaction, mut motion, mut z_index, mut outline, card) in &mut card_query {
        if *interaction == Interaction::Pressed {
            state.arm_slot(card.slot_idx);
        }
        let is_armed = state.armed_slot == Some(card.slot_idx);
        let active = !card.is_used && (is_armed || *interaction != Interaction::None);
        motion.active = active;
        *z_index = if active { ZIndex(10) } else { ZIndex(card.slot_idx as i32 + 1) };
        outline.color = if is_armed { GOLD } else { Color::NONE };
        for (mut node, tip) in &mut tooltips {
            if tip.0 == card.slot_idx {
                node.display = if active { Display::Flex } else { Display::None };
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
    mut card_query: Query<(&Interaction, &mut CardMotion, &mut Outline, &DraftCard), Changed<Interaction>>,
    mut pick_writer: MessageWriter<PickCard>,
) {
    for (interaction, mut motion, mut outline, card) in &mut card_query {
        let active = *interaction != Interaction::None;
        motion.active = active;
        outline.color = if active { GOLD } else { Color::NONE };
        if *interaction == Interaction::Pressed {
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
struct PrepareReset;

#[derive(Component)]
struct PrepareStart;

/// What the title panel was last built from; any change rebuilds it.
type TitleKey = (bool, bool, Option<DeckSlot>, Vec<SpellId>, (u32, u32), bool, bool, Option<u32>);

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
    format!("{}: {}{quick}", spell_name(spell), spell.effect_text())
}

/// The Prepare screen's detail line when no card is hovered.
fn prepare_hint(held: Option<SpellId>, has_reserve: bool, held_in_deck: bool) -> String {
    match held {
        Some(spell) if held_in_deck => {
            format!("Tap a reserve card to put it in place of {}.", spell_name(spell))
        }
        Some(spell) => format!("Tap a deck card to swap it out for {}.", spell_name(spell)),
        None if has_reserve => {
            "Shuffled each match. Tap a deck card, then a reserve card, to swap them.".into()
        }
        None => "Shuffled each match. Win spell cards to add them to your reserve.".into(),
    }
}

#[allow(clippy::too_many_arguments)]
fn sync_title_menu(
    mut commands: Commands,
    title_menu: Res<TitleMenu>,
    run: Res<Run>,
    atlas: Res<Atlas>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut last: Local<Option<TitleKey>>,
    overlay_query: Query<Entity, With<TitleOverlay>>,
) {
    let (win_w, win_h) = window.iter().next().map(|w| (w.width(), w.height())).unwrap_or((800.0, 600.0));
    let deck = if title_menu.prepare { run.profile.deck() } else { Vec::new() };
    let key: TitleKey = (
        title_menu.open,
        title_menu.prepare,
        title_menu.held,
        deck,
        (win_w as u32, win_h as u32),
        title_menu.confirm,
        run.in_match,
        run.profile.run.map(|r| r.stage),
    );
    if last.as_ref() == Some(&key) {
        return;
    }
    // Pop in only when the screen changes, not on every deck edit.
    let pop = last.as_ref().is_none_or(|prev| prev.0 != key.0 || prev.1 != key.1);
    *last = Some(key);

    for e in &overlay_query {
        commands.entity(e).despawn();
    }
    if !title_menu.open {
        return;
    }

    let compact = win_w < 500.0;
    let panel_padding = if compact { 12.0 } else { 20.0 };
    commands.spawn((TitleOverlay, GlobalZIndex(200), scrim())).with_children(|overlay| {
        let mut panel_cmd = overlay.spawn(panel(
            PanelKind::Outer,
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(if compact { 6.0 } else { 10.0 }),
                padding: UiRect::all(Val::Px(panel_padding)),
                max_width: Val::Percent(100.0),
                max_height: Val::Percent(100.0),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ));
        if pop {
            panel_cmd.insert(PopIn::new(0.0, 0.22));
        }
        panel_cmd.with_children(|parent| {
            if title_menu.prepare {
                spawn_prepare(parent, &run.profile.stage(), &run, title_menu.held, &atlas, win_w);
            } else {
                spawn_main_menu(parent, &run, title_menu.confirm, win_w, pop);
            }
        });
    });
}

fn spawn_main_menu(parent: &mut ChildSpawnerCommands, run: &Run, confirm: bool, win_w: f32, pop: bool) {
    let compact = win_w < 500.0;
    let btn_w = if compact { fit_width(300.0, win_w) - 24.0 } else { 300.0 };
    let btn_h = if compact { 56.0 } else { 64.0 };
    let btn_px = theme::size(win_w, M, S);

    parent.spawn(title_bar(RED, RED.darker(0.18))).with_child(label_nowrap(
        "TERRAIN CHESS",
        theme::size(win_w, XL, L),
        TEXT,
    ));
    let items = run.profile.owned.len();
    let stats = if run.profile.best_stage > 0 {
        format!("Best: stage {}   Items: {items}", run.profile.best_stage)
    } else {
        format!("Items: {items}")
    };
    parent.spawn(label_nowrap(stats, S, TEXT_DIM));

    let stage = run.profile.stage();
    let mut buttons: Vec<(MenuChoice, Color, String)> = Vec::new();
    if run.in_match {
        buttons.push((MenuChoice::Resume, ORANGE, "Resume".into()));
        let abandon = if confirm { "Tap again to abandon" } else { "Abandon run" };
        buttons.push((MenuChoice::Abandon, if confirm { RED } else { SLATE_LIGHT }, abandon.into()));
    } else if run.profile.run.is_some() {
        buttons.push((MenuChoice::Continue, ORANGE, String::new()));
        let new_run = if confirm { "Tap again: new run" } else { "New run" };
        buttons.push((MenuChoice::NewRun, if confirm { RED } else { SLATE_LIGHT }, new_run.into()));
    } else {
        buttons.push((MenuChoice::NewRun, ORANGE, "Start run".into()));
    }

    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(12.0),
            align_items: AlignItems::Center,
            margin: UiRect::top(Val::Px(6.0)),
            ..default()
        })
        .with_children(|col| {
            for (i, (choice, tone, text)) in buttons.into_iter().enumerate() {
                let mut btn = col.spawn((choice, button(tone, Val::Px(btn_w), btn_h)));
                if pop {
                    btn.insert(PopIn::new(0.03 * i as f32, 0.22));
                }
                if choice != MenuChoice::Continue {
                    btn.with_child(label_nowrap(text, btn_px, TEXT));
                    continue;
                }
                // Continue: the stage number chip, its name and board size.
                btn.entry::<Node>().and_modify(|mut n| {
                    n.justify_content = JustifyContent::FlexStart;
                    n.padding = UiRect::horizontal(Val::Px(8.0));
                });
                btn.with_children(|b| {
                    b.spawn(number_chip(ORANGE.darker(0.25), if compact { 30.0 } else { 36.0 }))
                        .with_child(label_nowrap(stage.number.to_string(), theme::size(win_w, M, S), TEXT));
                    b.spawn((
                        Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::FlexStart,
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .with_children(|t| {
                        t.spawn(label_nowrap(
                            format!("Continue: {}", stage.name()),
                            if compact { XS } else { S },
                            TEXT,
                        ));
                        t.spawn(label_nowrap(format!("{}×{}", stage.size(), stage.size()), XS, TEXT));
                    });
                });
            }
        });
}

/// The Prepare screen: stage info, the 15-card deck, the reserve, and Start.
fn spawn_prepare(
    parent: &mut ChildSpawnerCommands,
    stage: &tc_run::Stage,
    run: &Run,
    held: Option<DeckSlot>,
    atlas: &Atlas,
    win_w: f32,
) {
    let compact = win_w < 500.0;
    let panel_padding = if compact { 12.0 } else { 20.0 };
    let avail = win_w.min(760.0) - 2.0 * OVERLAY_GUTTER - 2.0 * panel_padding;
    let gap = if compact { 4.0 } else { 8.0 };
    let tile_w = ((avail - 4.0 * gap) / 5.0).floor().clamp(62.0, 112.0);
    let grid_w = 5.0 * tile_w + 4.0 * gap;
    let deck = run.profile.deck();
    let reserve = run.profile.deck_reserve();
    let small = if compact { XS } else { S };

    parent.spawn(title_bar(RED, RED.darker(0.18))).with_children(|bar| {
        bar.spawn(number_chip(RED.darker(0.25), if compact { 26.0 } else { 36.0 })).with_child(label_nowrap(
            stage.number.to_string(),
            theme::size(win_w, M, S),
            TEXT,
        ));
        bar.spawn(label(stage.name(), theme::size(win_w, M, S), TEXT));
    });
    let [you, enemy] = stage.piece_counts();
    let size = stage.size();
    let mut header = format!("{size}×{size}, {you} vs {enemy} pcs. Deck: {} cards", deck.len());
    if !reserve.is_empty() {
        header += &format!(", {} in reserve", reserve.len());
    }
    parent.spawn(label(header, small, TEXT));
    let seed = run.profile.run.map_or(0, |r| r.match_seed());
    let items = run.profile.stage_setup(stage, seed).enemy_items.len();
    let mut threat = format!("Enemy: AI {}/7", stage.ai_level());
    if items > 0 {
        threat += &format!(", {items} items");
    }
    if stage.reinforcements() > 0 {
        threat += &format!(", +{} reinforcements", stage.reinforcements());
    }
    parent.spawn(label(threat, XS, RED.lighter(0.1)));

    let grid = Node {
        width: Val::Px(grid_w),
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        justify_content: JustifyContent::Center,
        column_gap: Val::Px(gap),
        row_gap: Val::Px(gap),
        ..default()
    };
    let outline = |slot: DeckSlot| Outline {
        width: Val::Px(3.0),
        offset: Val::Px(2.0),
        color: if held == Some(slot) { GOLD } else { Color::NONE },
    };

    parent.spawn(grid.clone()).with_children(|grid| {
        for (i, &spell) in deck.iter().enumerate() {
            let slot = DeckSlot::Deck(i);
            grid.spawn((
                Button,
                Interaction::default(),
                PrepareCard { slot, spell },
                CardMotion::new(4.0, 1.05, false, 0.0),
                RelativeCursorPosition::default(),
                card_root(
                    Node {
                        width: Val::Px(tile_w),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: if compact {
                            UiRect::axes(Val::Px(1.0), Val::Px(3.0))
                        } else {
                            UiRect::all(Val::Px(4.0))
                        },
                        row_gap: Val::Px(3.0),
                        ..default()
                    },
                    6.0,
                ),
            ))
            .insert(outline(slot))
            .with_children(|tile| {
                // Older art is 60×54, newer art 30×26 drawn at 2×.
                let art = format!("art_{}", spell_item_id(spell));
                let (art_px, _) = integer_scaled_size(atlas, &art, 54.0);
                tile.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        justify_content: JustifyContent::Center,
                        border_radius: BorderRadius::all(Val::Px(4.0)),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BackgroundColor(spell_accent(spell).darker(0.22)),
                    Pickable::IGNORE,
                ))
                .with_child((
                    Node { width: Val::Px(art_px.x), height: Val::Px(art_px.y), ..default() },
                    ImageNode { image: atlas.image.clone(), rect: Some(atlas.rect(&art)), ..default() },
                    Pickable::IGNORE,
                ));
                tile.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        min_height: Val::Px(if compact { 34.0 } else { 26.0 }),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border_radius: BorderRadius::all(Val::Px(4.0)),
                        ..default()
                    },
                    BackgroundColor(SLATE_DARK),
                    Pickable::IGNORE,
                ))
                .with_child(label(spell_name(spell), XS, TEXT));
                if spell.is_quick() {
                    spawn_bolt(tile, atlas);
                }
            });
        }
    });

    parent
        .spawn(panel(
            PanelKind::Inner,
            Node {
                width: Val::Px(grid_w),
                // Room for two lines, so hovering a card with a long effect doesn't resize the panel.
                min_height: Val::Px((small * 2.5 + 8.0).ceil()),
                padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .with_child((
            PrepareDetail,
            label(
                prepare_hint(
                    held_spell(held, &deck, &reserve),
                    !reserve.is_empty(),
                    matches!(held, Some(DeckSlot::Deck(_))),
                ),
                small,
                TEXT,
            ),
        ));

    if !reserve.is_empty() {
        // Phones get 3 wider chips per row so long names like "Featherfall" fit.
        let chip_w = if compact { ((grid_w - 2.0 * gap) / 3.0).floor() } else { tile_w };
        parent.spawn(label_nowrap("Reserve", small, TEXT_DIM));
        parent.spawn(grid).with_children(|grid| {
            for (r, &spell) in reserve.iter().enumerate() {
                let slot = DeckSlot::Reserve(r);
                grid.spawn((
                    Button,
                    Interaction::default(),
                    PrepareCard { slot, spell },
                    CardMotion::new(3.0, 1.05, false, 0.0),
                    Node {
                        width: Val::Px(chip_w),
                        min_height: Val::Px(30.0),
                        column_gap: Val::Px(2.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        padding: UiRect::axes(Val::Px(4.0), Val::Px(3.0)),
                        border_radius: BorderRadius::all(Val::Px(6.0)),
                        ..default()
                    },
                    BackgroundColor(SLATE_LIGHT),
                    outline(slot),
                ))
                .with_children(|chip| {
                    chip.spawn(label(spell_name(spell), XS, TEXT));
                    // Phone chips are too narrow for a long name plus the bolt; hover still says Quick.
                    if spell.is_quick() && !compact {
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

    // The panel's 1 px border eats into `avail`.
    let btn_w = if compact { ((avail - 28.0) / 3.0).floor() } else { 150.0 };
    let btn_h = if compact { 44.0 } else { 48.0 };
    let btn_px = theme::size(win_w, M, S);
    let reset_disabled = run.profile.deck.is_empty();
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(12.0),
            row_gap: Val::Px(10.0),
            justify_content: JustifyContent::Center,
            margin: UiRect::top(Val::Px(4.0)),
            ..default()
        })
        .with_children(|row| {
            row.spawn((PrepareBack, button(SLATE_LIGHT, Val::Px(btn_w), btn_h)))
                .with_child(label_nowrap("Back", btn_px, TEXT));
            row.spawn((
                PrepareReset,
                ButtonDisabled(reset_disabled),
                button(SLATE_LIGHT, Val::Px(btn_w), btn_h),
            ))
            .with_child(label_nowrap("Reset", btn_px, TEXT));
            row.spawn((PrepareStart, button(ORANGE, Val::Px(btn_w), btn_h)))
                .with_child(label_nowrap("Start", btn_px, TEXT));
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

fn handle_main_menu_buttons(
    button_query: Query<(&Interaction, &MenuChoice), (Changed<Interaction>, With<Button>)>,
    run: Res<Run>,
    mut title_menu: ResMut<TitleMenu>,
    mut commands: MessageWriter<RunCommand>,
) {
    for (interaction, choice) in &button_query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match choice {
            MenuChoice::Resume => title_menu.close(),
            MenuChoice::Continue => title_menu.show(true),
            // Starting the first run, or confirming on the second press.
            MenuChoice::NewRun if run.profile.run.is_none() || title_menu.confirm => {
                commands.write(RunCommand::New);
            }
            MenuChoice::Abandon if title_menu.confirm => {
                commands.write(RunCommand::Abandon);
            }
            MenuChoice::NewRun | MenuChoice::Abandon => title_menu.confirm = true,
        }
    }
}

/// Deck tiles: tap to pick up / swap, hover to lift the tile and describe the card.
fn handle_prepare_cards(
    mut cards: Query<(&Interaction, &PrepareCard, &mut CardMotion), Changed<Interaction>>,
    mut detail: Query<&mut Text, With<PrepareDetail>>,
    title_menu: Res<TitleMenu>,
    run: Res<Run>,
    mut edits: MessageWriter<DeckEdit>,
) {
    for (interaction, card, mut motion) in &mut cards {
        motion.active = *interaction == Interaction::Hovered;
        let text = match interaction {
            Interaction::Pressed => {
                edits.write(DeckEdit::Tap(card.slot));
                continue;
            }
            Interaction::Hovered => card_detail(card.spell),
            Interaction::None => {
                let held = held_spell(title_menu.held, &run.profile.deck(), &run.profile.deck_reserve());
                prepare_hint(
                    held,
                    !run.profile.deck_reserve().is_empty(),
                    matches!(title_menu.held, Some(DeckSlot::Deck(_))),
                )
            }
        };
        for mut t in &mut detail {
            t.0 = text.clone();
        }
    }
}

fn handle_prepare_buttons(
    back: Query<&Interaction, (Changed<Interaction>, With<PrepareBack>)>,
    reset: Query<(&Interaction, &ButtonDisabled), (Changed<Interaction>, With<PrepareReset>)>,
    start: Query<&Interaction, (Changed<Interaction>, With<PrepareStart>)>,
    mut title_menu: ResMut<TitleMenu>,
    mut edits: MessageWriter<DeckEdit>,
    mut commands: MessageWriter<RunCommand>,
) {
    if back.iter().any(|i| *i == Interaction::Pressed) {
        title_menu.show(false);
    }
    if reset.iter().any(|(i, d)| *i == Interaction::Pressed && !d.0) {
        edits.write(DeckEdit::Reset);
    }
    if start.iter().any(|i| *i == Interaction::Pressed) && title_menu.prepare {
        commands.write(RunCommand::Start);
    }
}

fn handle_menu_button(
    button_query: Query<&Interaction, (Changed<Interaction>, With<MenuButton>)>,
    run: Res<Run>,
    mut title_menu: ResMut<TitleMenu>,
) {
    for interaction in &button_query {
        if *interaction == Interaction::Pressed {
            if !title_menu.open {
                title_menu.show(false);
            } else if run.menu_closable() {
                title_menu.close();
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_result_buttons(
    claim_query: Query<&Interaction, (Changed<Interaction>, With<ClaimButton>)>,
    next_query: Query<&Interaction, (Changed<Interaction>, With<NextStageButton>)>,
    new_query: Query<&Interaction, (Changed<Interaction>, With<NewRunButton>)>,
    menu_query: Query<&Interaction, (Changed<Interaction>, With<MenuOpenButton>)>,
    keys: Res<ButtonInput<KeyCode>>,
    mut run: ResMut<Run>,
    mut title_menu: ResMut<TitleMenu>,
    mut commands: MessageWriter<RunCommand>,
) {
    let RunPhase::Result { won } = run.phase else {
        return;
    };
    // Enter presses the strip's orange button.
    let enter = !title_menu.open && keys.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter]);
    if won
        && (claim_query.iter().any(|i| *i == Interaction::Pressed)
            || next_query.iter().any(|i| *i == Interaction::Pressed)
            || enter)
    {
        match run.pending_draft.take() {
            Some(draft) => run.phase = RunPhase::Draft(draft),
            None => {
                run.phase = RunPhase::Playing;
                title_menu.show(true);
            }
        }
    }
    if !won && (new_query.iter().any(|i| *i == Interaction::Pressed) || enter) {
        commands.write(RunCommand::New);
    }
    if menu_query.iter().any(|i| *i == Interaction::Pressed) {
        title_menu.show(false);
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
    mut parts: Query<(Option<&mut BackgroundColor>, Option<&mut TextColor>, Option<&mut UiTransform>)>,
) {
    if let Some(msg) = state.toast.take() {
        for (e, _) in &toasts {
            commands.entity(e).despawn();
        }
        let (w, h) = window.iter().next().map(|w| (w.width(), w.height())).unwrap_or((1280.0, 800.0));
        let text = commands
            .spawn((
                Text::new(msg),
                TextFont { font_size: FontSize::Px(theme::size(w, S, XS)), ..default() },
                TextColor(TEXT.with_alpha(0.0)),
                TextLayout { justify: Justify::Center, linebreak: LineBreak::WordBoundary },
                Pickable::IGNORE,
            ))
            .id();
        let pill = commands
            .spawn((
                Node {
                    max_width: Val::Vw(90.0),
                    padding: UiRect::axes(Val::Px(16.0), Val::Px(6.0)),
                    border_radius: BorderRadius::MAX,
                    ..default()
                },
                BackgroundColor(SLATE_DARK.with_alpha(0.0)),
                UiTransform::default(),
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
        // Fade and pop in over 0.15 s, hold, fade out over the last 0.4 s.
        let a = (t / 0.15).min(1.0).min((TOAST_SECS - t) / 0.4);
        if let Ok((Some(mut bg), _, Some(mut tf))) = parts.get_mut(toast.pill) {
            bg.0.set_alpha(0.92 * a);
            tf.scale = Vec2::splat(0.9 + 0.1 * (t / 0.15).min(1.0));
        }
        if let Ok((_, Some(mut color), _)) = parts.get_mut(toast.text) {
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
                animate_end_title,
                sync_title_menu,
                update_draft_card_sizes,
                handle_card_interaction,
                handle_result_buttons,
                handle_main_menu_buttons,
                handle_prepare_cards,
                handle_prepare_buttons,
                handle_menu_button,
                handle_hand_card_interaction,
                handle_discard_button_interaction,
                theme::update_button_visuals,
                sync_hand_bar,
                sync_toast,
            )
                .chain()
                .run_if(in_state(AppState::Ready)),
        );
    }
}
