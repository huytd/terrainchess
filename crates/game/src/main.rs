//! Terrain Chess: Bevy front end (native + web).

// Bevy query filters are verbose by nature.
#![allow(clippy::type_complexity)]

mod ai;
mod atlas;
mod board_view;
mod game;
mod hud;
mod input;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()).set(WindowPlugin {
            primary_window: Some(Window {
                title: "Terrain Chess".into(),
                canvas: Some("#bevy".into()),
                fit_canvas_to_parent: true,
                prevent_default_event_handling: true,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb_u8(0x1A, 0x1C, 0x2C)))
        .add_plugins((
            atlas::AtlasPlugin,
            ai::AiPlugin,
            game::GamePlugin,
            board_view::BoardViewPlugin,
            input::InputPlugin,
            hud::HudPlugin,
        ))
        .run();
}
