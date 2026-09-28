//! Terrain Chess: Bevy front end (native + web).

// Bevy query filters are verbose by nature.
#![allow(clippy::type_complexity)]

mod ai;
mod atlas;
mod board_view;
mod fx;
mod game;
mod hud;
mod input;
mod loading;
mod run;
mod save;
mod scenery;
mod sfx;

use bevy::asset::AssetMetaCheck;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                // Assets have no .meta files; don't request them (they 404 on the web).
                .set(AssetPlugin { meta_check: AssetMetaCheck::Never, ..default() })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Terrain Chess".into(),
                        canvas: Some("#bevy".into()),
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: true,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .insert_resource(ClearColor(Color::srgb_u8(0xA9, 0xB8, 0xC4)))
        .add_plugins((
            loading::LoadingPlugin,
            atlas::AtlasPlugin,
            ai::AiPlugin,
            run::RunPlugin,
            game::GamePlugin,
            board_view::BoardViewPlugin,
            scenery::SceneryPlugin,
            input::InputPlugin,
            hud::HudPlugin,
            fx::FxPlugin,
            sfx::SfxPlugin,
        ))
        .run();
}
