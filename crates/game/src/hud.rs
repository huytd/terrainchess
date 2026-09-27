//! Placeholder HUD: turn/status line, controls help, and the promotion picker.
//! The styled parchment UI comes with the art pass (M7).

use bevy::prelude::*;
use tc_core::{DrawReason, Outcome, PieceKind, Side};

use crate::ai::Thinker;
use crate::game::GameState;

#[derive(Component)]
struct StatusText;

#[derive(Component)]
struct PromotionPanel;

#[derive(Component)]
struct PromotionButton(PieceKind);

const INK: Color = Color::srgb(0.97, 0.93, 0.82);
const PANEL: Color = Color::srgba(0.10, 0.11, 0.17, 0.85);

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
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(12.0),
            bottom: Val::Px(12.0),
            padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
            ..default()
        },
        BackgroundColor(PANEL),
        children![(
            Text::new(
                "Click: select / move   Right-drag, WASD: pan   Wheel: zoom   Alt: heights\n\
                 U: undo   N: new board   1 / 2 / 3: 8x8 / 16x16 / 32x32\n\
                 H: AI / hotseat   F: swap sides with the AI   - / =: AI level",
            ),
            TextFont { font_size: 13.0.into(), ..default() },
            TextColor(INK.with_alpha(0.8)),
        )],
    ));
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
                            BackgroundColor(Color::srgb(0.27, 0.22, 0.18)),
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
            Interaction::Hovered => bg.0 = Color::srgb(0.45, 0.35, 0.22),
            Interaction::None => bg.0 = Color::srgb(0.27, 0.22, 0.18),
        }
    }
}

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, (promotion_clicks, show_promotion, update_status).chain());
    }
}
