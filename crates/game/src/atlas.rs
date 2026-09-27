//! The sprite atlas built by `tools/process_sprites.py`.

use std::collections::HashMap;

use bevy::prelude::*;
use bevy::sprite::Anchor;
use serde::Deserialize;

/// Embedded so the web build doesn't need an extra fetch.
const MANIFEST: &str = include_str!("../../../assets/atlas.ron");

#[derive(Deserialize)]
struct Manifest {
    sprites: HashMap<String, SpriteRect>,
}

#[derive(Deserialize, Clone, Copy)]
struct SpriteRect {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    /// Pivot in image space: (0, 0) top-left, (1, 1) bottom-right.
    anchor: (f32, f32),
}

#[derive(Resource)]
pub struct Atlas {
    image: Handle<Image>,
    rects: HashMap<String, SpriteRect>,
}

impl Atlas {
    /// A sprite and its pivot. Unknown names panic: the manifest is generated.
    pub fn sprite(&self, name: &str) -> (Sprite, Anchor) {
        let r = self.rects.get(name).unwrap_or_else(|| panic!("no sprite {name} in atlas.ron"));
        let sprite = Sprite {
            image: self.image.clone(),
            rect: Some(Rect::new(r.x as f32, r.y as f32, (r.x + r.w) as f32, (r.y + r.h) as f32)),
            ..default()
        };
        (sprite, Anchor(Vec2::new(r.anchor.0 - 0.5, 0.5 - r.anchor.1)))
    }
}

pub struct AtlasPlugin;

impl Plugin for AtlasPlugin {
    fn build(&self, app: &mut App) {
        let manifest: Manifest = ron::from_str(MANIFEST).expect("assets/atlas.ron is invalid");
        let image = app.world().resource::<AssetServer>().load("atlas.png");
        app.insert_resource(Atlas { image, rects: manifest.sprites });
    }
}
