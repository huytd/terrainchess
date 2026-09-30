//! Terrain Chess: Bevy front end (native + web).

// Bevy query filters are verbose by nature.
#![allow(clippy::type_complexity)]

mod ai;
mod announce;
mod atlas;
mod board_view;
mod fx;
mod game;
mod hud;
mod input;
mod loading;
mod particles;
mod run;
mod save;
mod scenery;
mod sfx;
mod theme;
mod ui_fx;

use bevy::asset::{AssetId, AssetMetaCheck};
use bevy::prelude::*;
use bevy::text::Font;

/// m6x11plus by Daniel Linssen, the only UI font; crisp at multiples of 9 px (see `theme`).
const M6X11_FONT: &[u8] = include_bytes!("../../../assets/fonts/m6x11plus.ttf");

fn main() {
    let mut app = App::new();
    app.add_plugins(
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
        announce::AnnouncePlugin,
        ui_fx::UiFxPlugin,
        particles::ParticlesPlugin,
    ));

    let m6x11 = Font::from_bytes(M6X11_FONT.to_vec());
    app.world_mut()
        .resource_mut::<Assets<Font>>()
        .insert(AssetId::default(), m6x11)
        .expect("insert default font");

    app.run();
}
