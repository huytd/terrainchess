//! Placeholder HUD: turn/status line, toolbar, controls help, and the promotion picker.
//! The styled parchment UI comes with the art pass (M7).

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use tc_core::{DrawReason, Outcome, PieceKind, Side};

use crate::ai::Thinker;
use crate::board_view::ShowHeights;
use crate::game::GameState;
use crate::input::Action;

#[derive(Component)]
struct StatusText;

#[derive(Component)]
struct HelpPanel;

/// Toolbar buttons: the keyboard commands, reachable by mouse or touch.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Tool {
    Undo,
    New,
    Size,
    Mode,
    Swap,
    AiDown,
    AiUp,
    Heights,
    TurnLeft,
    TurnRight,
}

#[derive(Component)]
struct ToolLabel(Tool);

#[derive(Component)]
struct PromotionPanel;

#[derive(Component)]
struct PromotionButton(PieceKind);

const INK: Color = Color::srgb(0.97, 0.93, 0.82);
const PANEL: Color = Color::srgba(0.10, 0.11, 0.17, 0.85);
const BUTTON: Color = Color::srgb(0.27, 0.22, 0.18);
const BUTTON_HOVER: Color = Color::srgb(0.45, 0.35, 0.22);
const BUTTON_ON: Color = Color::srgb(0.55, 0.42, 0.18);
/// Below this window width the keyboard help is hidden; the toolbar covers it.
const HELP_MIN_WIDTH: f32 = 820.0;

fn side_name(side: Side) -> &'static str {
    match side {
        Side::White => "Ashen Sun",
        Side::Black => "Hollow Crown",
    }
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(12.0),
            top: Val::Px(12.0),
            padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(PANEL),
        children![(
            StatusText,
            Text::new(""),
            TextFont { font_size: 18.0.into(), ..default() },
            TextColor(INK)
        )],
    ));
    commands.spawn((
        HelpPanel,
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(12.0),
            top: Val::Px(12.0),
            padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
            ..default()
        },
        BackgroundColor(PANEL),
        children![(
            Text::new(
                "Click / tap: select, move   Middle-drag, WASD, touch drag: pan\n\
                 Right-drag, Q / E, two-finger twist: turn   Wheel, pinch: zoom\n\
                 Alt (hold) / T: heights   U: undo\n\
                 N: new board   1 / 2 / 3: 8x8 / 16x16 / 32x32\n\
                 H: AI / hotseat   F: swap sides   - / =: AI level",
            ),
            TextFont { font_size: 13.0.into(), ..default() },
            TextColor(INK.with_alpha(0.8)),
        )],
    ));
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            bottom: Val::Px(12.0),
            padding: UiRect::horizontal(Val::Px(8.0)),
            justify_content: JustifyContent::Center,
            flex_wrap: FlexWrap::Wrap,
            row_gap: Val::Px(6.0),
            column_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|bar| {
            for tool in [
                Tool::Undo,
                Tool::New,
                Tool::Size,
                Tool::Mode,
                Tool::Swap,
                Tool::AiDown,
                Tool::AiUp,
                Tool::Heights,
                Tool::TurnLeft,
                Tool::TurnRight,
            ] {
                bar.spawn((
                    Button,
                    tool,
                    Node {
                        min_width: Val::Px(44.0),
                        min_height: Val::Px(40.0),
                        padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(BUTTON),
                    children![(
                        ToolLabel(tool),
                        Text::new(""),
                        TextFont { font_size: 16.0.into(), ..default() },
                        TextColor(INK)
                    )],
                ));
            }
        });
}

fn tool_clicks(
    state: Res<GameState>,
    mut actions: MessageWriter<Action>,
    buttons: Query<(&Interaction, &Tool), Changed<Interaction>>,
) {
    for (interaction, tool) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        actions.write(match tool {
            Tool::Undo => Action::Undo,
            Tool::New => Action::NewBoard(state.size),
            Tool::Size => Action::NewBoard(match state.size {
                8 => 16,
                16 => 32,
                _ => 8,
            }),
            Tool::Mode => Action::ToggleAi,
            Tool::Swap => Action::SwapSides,
            Tool::AiDown => Action::AiLevel(-1),
            Tool::AiUp => Action::AiLevel(1),
            Tool::Heights => Action::ToggleHeights,
            Tool::TurnLeft => Action::Turn(-1),
            Tool::TurnRight => Action::Turn(1),
        });
    }
}

fn update_toolbar(
    state: Res<GameState>,
    heights: Res<ShowHeights>,
    mut labels: Query<(&ToolLabel, &mut Text)>,
    mut buttons: Query<(&Tool, &Interaction, &mut BackgroundColor)>,
) {
    for (label, mut text) in &mut labels {
        let s = match label.0 {
            Tool::Undo => "Undo".into(),
            Tool::New => "New".into(),
            Tool::Size => format!("{0}x{0}", state.size),
            Tool::Mode => if state.ai_side.is_some() { "vs AI" } else { "Hotseat" }.into(),
            Tool::Swap => "Swap".into(),
            Tool::AiDown => "AI -".into(),
            Tool::AiUp => "AI +".into(),
            Tool::Heights => "Heights".into(),
            Tool::TurnLeft => "<".into(),
            Tool::TurnRight => ">".into(),
        };
        if text.0 != s {
            text.0 = s;
        }
    }
    for (tool, interaction, mut bg) in &mut buttons {
        let color = match interaction {
            Interaction::Hovered | Interaction::Pressed => BUTTON_HOVER,
            Interaction::None if *tool == Tool::Heights && heights.0 => BUTTON_ON,
            Interaction::None => BUTTON,
        };
        if bg.0 != color {
            bg.0 = color;
        }
    }
}

fn fit_help(window: Single<&Window, With<PrimaryWindow>>, mut help: Single<&mut Node, With<HelpPanel>>) {
    let display = if window.width() >= HELP_MIN_WIDTH { Display::Flex } else { Display::None };
    if help.display != display {
        help.display = display;
    }
}

fn update_status(
    state: Res<GameState>,
    thinker: Res<Thinker>,
    mut text: Single<&mut Text, With<StatusText>>,
) {
    if !state.is_changed() && !thinker.is_changed() {
        return;
    }
    let turn = state.game.pos.side_to_move;
    let line = match state.outcome {
        Some(Outcome::Checkmate { winner }) => format!("Checkmate - the {} wins!", side_name(winner)),
        Some(Outcome::Draw(reason)) => format!(
            "Draw by {}",
            match reason {
                DrawReason::Stalemate => "stalemate",
                DrawReason::FiftyMoves => "the fifty-move rule",
                DrawReason::Repetition => "repetition",
                DrawReason::InsufficientMaterial => "insufficient material",
            }
        ),
        None => {
            let check = if state.game.in_check() { " - check!" } else { "" };
            let verb = if state.ai_to_move() && thinker.is_thinking() { "is thinking" } else { "to move" };
            format!("{} {verb}{check}", side_name(turn))
        }
    };
    let mode = match state.ai_side {
        Some(ai) => format!("vs AI ({}) level {}", side_name(ai), state.ai_level),
        None => "hotseat".to_string(),
    };
    text.0 = format!(
        "{line}\n{mode}\n{0}x{0}  seed {1}  move {2}",
        state.size, state.seed, state.game.pos.fullmove
    );
}

fn show_promotion(mut commands: Commands, state: Res<GameState>, panel: Query<Entity, With<PromotionPanel>>) {
    if !state.is_changed() {
        return;
    }
    let open = state.pending_promotion.is_some();
    match (open, panel.single()) {
        (true, Err(_)) => {
            let mut buttons = Vec::new();
            for kind in PieceKind::PROMOTIONS {
                let label = match kind {
                    PieceKind::Queen => "Queen",
                    PieceKind::Rook => "Rook",
                    PieceKind::Bishop => "Bishop",
                    _ => "Knight",
                };
                buttons.push(
                    commands
                        .spawn((
                            Button,
                            PromotionButton(kind),
                            Node { padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)), ..default() },
                            BackgroundColor(BUTTON),
                            children![(
                                Text::new(label),
                                TextFont { font_size: 18.0.into(), ..default() },
                                TextColor(INK)
                            )],
                        ))
                        .id(),
                );
            }
            commands
                .spawn((
                    PromotionPanel,
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                ))
                .with_children(|root| {
                    root.spawn((
                        Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            row_gap: Val::Px(10.0),
                            padding: UiRect::all(Val::Px(16.0)),
                            ..default()
                        },
                        BackgroundColor(PANEL),
                    ))
                    .with_children(|p| {
                        p.spawn((
                            Text::new("Promote to"),
                            TextFont { font_size: 20.0.into(), ..default() },
                            TextColor(INK),
                        ));
                        p.spawn(Node { column_gap: Val::Px(8.0), ..default() }).add_children(&buttons);
                    });
                });
        }
        (false, Ok(e)) => commands.entity(e).despawn(),
        _ => {}
    }
}

fn promotion_clicks(
    mut state: ResMut<GameState>,
    mut buttons: Query<(&Interaction, &PromotionButton, &mut BackgroundColor), Changed<Interaction>>,
) {
    for (interaction, button, mut bg) in &mut buttons {
        match interaction {
            Interaction::Pressed => {
                let chosen = state
                    .pending_promotion
                    .as_ref()
                    .and_then(|moves| moves.iter().find(|m| m.promotion == Some(button.0)).copied());
                if let Some(mv) = chosen {
                    state.play(mv);
                }
            }
            Interaction::Hovered => bg.0 = BUTTON_HOVER,
            Interaction::None => bg.0 = BUTTON,
        }
    }
}

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup).add_systems(
            Update,
            (tool_clicks, update_toolbar, fit_help, promotion_clicks, show_promotion, update_status).chain(),
        );
    }
}
