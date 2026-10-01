//! Balatro-style UI kit: palette, type scale, panels, buttons and card faces, all drawn in code.

use bevy::prelude::*;
use tc_core::SpellId;

use crate::atlas::Atlas;

pub const RED: Color = Color::srgb_u8(0xFE, 0x5F, 0x55);
pub const BLUE: Color = Color::srgb_u8(0x00, 0x9D, 0xFF);
pub const GREEN: Color = Color::srgb_u8(0x4B, 0xC2, 0x92);
pub const ORANGE: Color = Color::srgb_u8(0xFD, 0xA2, 0x00);
pub const GOLD: Color = Color::srgb_u8(0xEA, 0xC0, 0x58);
pub const PURPLE: Color = Color::srgb_u8(0x88, 0x67, 0xA5);
/// Outer panels.
pub const SLATE: Color = Color::srgb_u8(0x37, 0x42, 0x44);
/// Panel outlines, secondary buttons, chips.
pub const SLATE_LIGHT: Color = Color::srgb_u8(0x4F, 0x63, 0x67);
/// Inner boxes and name plates.
pub const SLATE_DARK: Color = Color::srgb_u8(0x26, 0x2E, 0x30);
/// Disabled buttons.
pub const GREY: Color = Color::srgb_u8(0x5F, 0x73, 0x77);
pub const CARD_FACE: Color = Color::srgb_u8(0xF1, 0xEC, 0xE2);
pub const INFO_BG: Color = Color::WHITE;
pub const TEXT: Color = Color::WHITE;
pub const TEXT_DIM: Color = Color::srgba(1.0, 1.0, 1.0, 0.72);
/// Text on light boxes (card face, tooltip).
pub const TEXT_DARK: Color = Color::srgb_u8(0x4F, 0x63, 0x67);
pub const SCRIM: Color = Color::srgba(0.06, 0.08, 0.09, 0.78);
pub const SHADOW: Color = Color::srgba(0.0, 0.0, 0.0, 0.35);

/// Font sizes: m6x11plus is drawn on an 18 px em, so multiples of 9 px stay on its pixel grid.
pub const XS: f32 = 18.0;
pub const S: f32 = 27.0;
pub const M: f32 = 36.0;
pub const L: f32 = 54.0;
pub const XL: f32 = 72.0;

/// Space kept clear between a modal panel and the screen edge.
pub const OVERLAY_GUTTER: f32 = 16.0;

/// `desktop` on wide windows, `phone` below 500 px.
pub fn size(win_w: f32, desktop: f32, phone: f32) -> f32 {
    if win_w < 500.0 { phone } else { desktop }
}

fn text_shadow(px: f32) -> TextShadow {
    TextShadow {
        offset: Vec2::new(0.0, if px >= L { 3.0 } else { 2.0 }),
        color: Color::srgba(0.0, 0.0, 0.0, 0.5),
    }
}

/// Pixel text with Balatro's hard drop shadow; wraps at word boundaries.
pub fn label(text: impl Into<String>, px: f32, color: Color) -> impl Bundle {
    (
        Text::new(text),
        TextFont { font_size: FontSize::Px(px), ..default() },
        TextColor(color),
        text_shadow(px),
        TextLayout { justify: Justify::Center, linebreak: LineBreak::WordBoundary },
        Pickable::IGNORE,
    )
}

/// `label` on one line.
pub fn label_nowrap(text: impl Into<String>, px: f32, color: Color) -> impl Bundle {
    (
        Text::new(text),
        TextFont { font_size: FontSize::Px(px), ..default() },
        TextColor(color),
        text_shadow(px),
        TextLayout { justify: Justify::Center, linebreak: LineBreak::NoWrap },
        Pickable::IGNORE,
    )
}

/// Text without a shadow, for dark ink on light boxes.
pub fn ink(text: impl Into<String>, px: f32, color: Color) -> impl Bundle {
    (
        Text::new(text),
        TextFont { font_size: FontSize::Px(px), ..default() },
        TextColor(color),
        TextLayout { justify: Justify::Center, linebreak: LineBreak::WordBoundary },
        Pickable::IGNORE,
    )
}

fn hard_shadow(color: Color, x: f32, y: f32) -> ShadowStyle {
    ShadowStyle {
        color,
        x_offset: Val::Px(x),
        y_offset: Val::Px(y),
        spread_radius: Val::ZERO,
        blur_radius: Val::ZERO,
    }
}

pub enum PanelKind {
    /// Slate box with an outline and a hard shadow.
    Outer,
    /// Darker flat box inside a panel.
    Inner,
    /// Rounded dark pill for floating labels.
    Pill,
}

/// A slate box; `node` carries the layout, this fills in the look.
pub fn panel(kind: PanelKind, mut node: Node) -> impl Bundle {
    let (bg, radius, shadow) = match kind {
        PanelKind::Outer => {
            node.border = UiRect::all(Val::Px(1.0));
            (SLATE, 10.0, vec![hard_shadow(SHADOW, 0.0, 6.0)])
        }
        PanelKind::Inner => (SLATE_DARK, 6.0, vec![]),
        PanelKind::Pill => (SLATE_DARK.with_alpha(0.85), 999.0, vec![]),
    };
    node.border_radius = BorderRadius::all(Val::Px(radius));
    (node, BackgroundColor(bg), BorderColor::all(SLATE_LIGHT), BoxShadow(shadow))
}

/// Full-screen dim backdrop that centers its children.
pub fn scrim() -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            padding: UiRect::all(Val::Px(OVERLAY_GUTTER)),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(SCRIM),
    )
}

/// A colored title bar with a darker lip; children go inside (usually a `label`).
pub fn title_bar(fill: Color, lip_color: Color) -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            column_gap: Val::Px(12.0),
            padding: UiRect::axes(Val::Px(24.0), Val::Px(6.0)),
            margin: UiRect::bottom(Val::Px(4.0)),
            max_width: Val::Percent(100.0),
            border_radius: BorderRadius::all(Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(fill),
        BoxShadow(vec![hard_shadow(lip_color, 0.0, 4.0)]),
    )
}

/// A small square number chip (level number, order badge).
pub fn number_chip(fill: Color, side: f32) -> impl Bundle {
    (
        Node {
            min_width: Val::Px(side),
            height: Val::Px(side),
            padding: UiRect::horizontal(Val::Px(4.0)),
            flex_shrink: 0.0,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border_radius: BorderRadius::all(Val::Px(6.0)),
            ..default()
        },
        BackgroundColor(fill),
        BoxShadow(vec![hard_shadow(fill.darker(0.18), 0.0, 3.0)]),
        Pickable::IGNORE,
    )
}

/// A rounded tag, e.g. "Quick"; add a `label` child.
pub fn tag_chip(fill: Color) -> impl Bundle {
    (
        Node {
            padding: UiRect::axes(Val::Px(6.0), Val::Px(1.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border_radius: BorderRadius::MAX,
            ..default()
        },
        BackgroundColor(fill),
        Pickable::IGNORE,
    )
}

/// A button's fill; hover, press and disabled looks derive from it.
#[derive(Component, Clone, Copy)]
pub struct Tone(pub Color);

/// Grey disabled look and dimmed label; toggled by the owner.
#[derive(Component, Default, PartialEq, Eq)]
pub struct ButtonDisabled(pub bool);

pub fn lip(tone: Color, y: f32) -> ShadowStyle {
    hard_shadow(tone.darker(0.18), 0.0, y)
}

/// Chunky flat button with a darker lip underneath.
pub fn button(tone: Color, width: Val, height: f32) -> impl Bundle {
    (
        Button,
        Interaction::default(),
        Tone(tone),
        UiTransform::default(),
        Node {
            width,
            height: Val::Px(height),
            flex_shrink: 0.0,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            column_gap: Val::Px(8.0),
            padding: UiRect::horizontal(Val::Px(10.0)),
            border_radius: BorderRadius::all(Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(tone),
        BoxShadow(vec![lip(tone, 4.0)]),
    )
}

/// Hover lightens and grows a button, press sinks it onto its lip, disabled greys it out.
pub fn update_button_visuals(
    mut q: Query<
        (
            &Interaction,
            &Tone,
            Option<&ButtonDisabled>,
            &mut BackgroundColor,
            &mut BoxShadow,
            &mut UiTransform,
        ),
        Or<(Changed<Interaction>, Changed<ButtonDisabled>, Added<Tone>)>,
    >,
) {
    for (interaction, tone, disabled, mut bg, mut shadow, mut tf) in &mut q {
        let (fill, lip_y, dy, scale) = if disabled.is_some_and(|d| d.0) {
            (GREY, 4.0, 0.0, 1.0)
        } else {
            match interaction {
                Interaction::Pressed => (tone.0, 1.0, 3.0, 1.0),
                Interaction::Hovered => (tone.0.lighter(0.06), 4.0, 0.0, 1.04),
                Interaction::None => (tone.0, 4.0, 0.0, 1.0),
            }
        };
        bg.0 = fill;
        let base = if disabled.is_some_and(|d| d.0) { GREY } else { tone.0 };
        shadow.0 = vec![lip(base, lip_y)];
        tf.translation = Val2::px(0.0, dy);
        tf.scale = Vec2::splat(scale);
    }
}

pub fn spell_item_id(spell: SpellId) -> &'static str {
    match spell {
        SpellId::RaiseEarth => "raise_earth",
        SpellId::LowerEarth => "lower_earth",
        SpellId::Freeze => "freeze",
        SpellId::Bridge => "bridge",
        SpellId::DigTunnel => "dig_tunnel",
        SpellId::Shield => "shield",
        SpellId::Swap => "swap",
        SpellId::Rewind => "rewind",
        SpellId::Smite => "smite",
        SpellId::Evaporate => "evaporate",
        SpellId::Flood => "flood",
        SpellId::Featherfall => "featherfall",
        SpellId::Curse => "curse",
        SpellId::Sprout => "sprout",
        SpellId::Blink => "blink",
        SpellId::Insight => "insight",
    }
}

pub fn spell_name(spell: SpellId) -> &'static str {
    match spell {
        SpellId::RaiseEarth => "Raise Earth",
        SpellId::LowerEarth => "Lower Earth",
        SpellId::Freeze => "Freeze",
        SpellId::Shield => "Shield",
        SpellId::Swap => "Swap",
        SpellId::Bridge => "Bridge",
        SpellId::DigTunnel => "Dig Tunnel",
        SpellId::Rewind => "Rewind",
        SpellId::Smite => "Smite",
        SpellId::Evaporate => "Evaporate",
        SpellId::Flood => "Flood",
        SpellId::Featherfall => "Featherfall",
        SpellId::Curse => "Curse",
        SpellId::Sprout => "Sprout",
        SpellId::Blink => "Blink",
        SpellId::Insight => "Insight",
    }
}

/// Card accent by spell family: earth green, water blue, piece purple, power orange.
pub fn spell_accent(spell: SpellId) -> Color {
    match spell {
        SpellId::RaiseEarth
        | SpellId::LowerEarth
        | SpellId::Bridge
        | SpellId::DigTunnel
        | SpellId::Sprout => GREEN,
        SpellId::Freeze | SpellId::Flood | SpellId::Evaporate => BLUE,
        SpellId::Shield | SpellId::Swap | SpellId::Blink | SpellId::Featherfall | SpellId::Curse => PURPLE,
        SpellId::Smite | SpellId::Rewind | SpellId::Insight => ORANGE,
    }
}

pub fn rarity_color(rarity: tc_run::Rarity) -> Color {
    match rarity {
        tc_run::Rarity::Common => BLUE,
        tc_run::Rarity::Uncommon => GREEN,
        tc_run::Rarity::Rare => RED,
    }
}

pub fn integer_scaled_size(atlas: &Atlas, name: &str, target_h: f32) -> (Vec2, f32) {
    let px = atlas.px(name);
    let scale = (target_h / px.y).round().max(1.0);
    (px * scale, scale)
}

/// The look of a card's root node: light face, rounded corners, hard shadow, hidden outline.
/// `node` carries size and layout.
pub fn card_root(mut node: Node, radius: f32) -> impl Bundle {
    node.border = UiRect::all(Val::Px(2.0));
    node.border_radius = BorderRadius::all(Val::Px(radius));
    (
        node,
        BackgroundColor(CARD_FACE),
        BorderColor::all(CARD_FACE.darker(0.12)),
        BoxShadow(vec![hard_shadow(SHADOW, 3.0, 5.0)]),
        Outline { width: Val::Px(3.0), offset: Val::Px(2.0), color: Color::NONE },
    )
}

/// Card proportions in design units: 3-unit margins around a 49-wide face.
pub const CARD_W_U: f32 = 55.0;
pub const CARD_H_U: f32 = 79.0;

/// Face of a full-size card (55:79): art window, name plate, footer tags.
/// Children are absolutely placed in percent, so the root can be any size with that ratio.
#[allow(clippy::too_many_arguments)]
pub fn spawn_card_face(
    parent: &mut ChildSpawnerCommands,
    atlas: &Atlas,
    art: &str,
    name: &str,
    accent: Color,
    tags: &[(&str, Color)],
    card_w: f32,
    name_px: f32,
) {
    let u = card_w / CARD_W_U;
    let pct = |v: f32, total: f32| Val::Percent(v / total * 100.0);
    // Narrow cards give the name plate room for two lines of the smallest text.
    let (art_h, plate_top, plate_h) = if card_w < 140.0 { (33.0, 38.0, 25.0) } else { (40.0, 46.0, 18.0) };
    let (art_size, _) = integer_scaled_size(atlas, art, (art_h - 4.0) * u);
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: pct(3.0, CARD_W_U),
                top: pct(3.0, CARD_H_U),
                width: pct(49.0, CARD_W_U),
                height: pct(art_h, CARD_H_U),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                overflow: Overflow::clip(),
                border_radius: BorderRadius::all(Val::Px(3.0 * u)),
                ..default()
            },
            BackgroundColor(accent.darker(0.22)),
            Pickable::IGNORE,
        ))
        .with_child((
            Node { width: Val::Px(art_size.x), height: Val::Px(art_size.y), ..default() },
            ImageNode { image: atlas.image.clone(), rect: Some(atlas.rect(art)), ..default() },
            Pickable::IGNORE,
        ));
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: pct(3.0, CARD_W_U),
                top: pct(plate_top, CARD_H_U),
                width: pct(49.0, CARD_W_U),
                height: pct(plate_h, CARD_H_U),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(Val::Px(2.0)),
                border_radius: BorderRadius::all(Val::Px(3.0 * u)),
                ..default()
            },
            BackgroundColor(SLATE_DARK),
            Pickable::IGNORE,
        ))
        .with_child(label(name, name_px, TEXT));
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: pct(3.0, CARD_W_U),
                top: pct(66.0, CARD_H_U),
                width: pct(49.0, CARD_W_U),
                height: pct(10.0, CARD_H_U),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|row| {
            for &(text, color) in tags {
                row.spawn(tag_chip(color)).with_child(label_nowrap(text, XS, TEXT));
            }
        });
}
