//! The sprite atlas built by `tools/process_sprites.py`.

use std::collections::HashMap;

use bevy::prelude::*;
use serde::Deserialize;

/// Embedded so the web build doesn't need an extra fetch.
const MANIFEST: &str = include_str!("../../../assets/atlas.ron");

#[derive(Deserialize)]
struct Manifest {
    size: (u32, u32),
    sprites: HashMap<String, SpriteRect>,
}

#[derive(Deserialize, Clone, Copy)]
struct SpriteRect {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

#[derive(Resource)]
pub struct Atlas {
    pub image: Handle<Image>,
    size: Vec2,
    rects: HashMap<String, SpriteRect>,
}

/// Where a sprite sits in the atlas, in texture coordinates (0..1, y down).
#[derive(Clone, Copy)]
pub struct Uv {
    pub min: Vec2,
    pub max: Vec2,
}

impl Uv {
    /// Texture coordinate of a point given as fractions across (u) and down (v) the sprite.
    pub fn at(&self, u: f32, v: f32) -> [f32; 2] {
        [self.min.x + (self.max.x - self.min.x) * u, self.min.y + (self.max.y - self.min.y) * v]
    }
}

impl Atlas {
    fn sprite_rect(&self, name: &str) -> SpriteRect {
        *self.rects.get(name).unwrap_or_else(|| panic!("no sprite {name} in atlas.ron"))
    }

    /// Pixel rect of a sprite for UI `ImageNode { image, rect, .. }`.
    pub fn rect(&self, name: &str) -> Rect {
        let r = self.sprite_rect(name);
        Rect::new(r.x as f32, r.y as f32, (r.x + r.w) as f32, (r.y + r.h) as f32)
    }

    /// Texture coordinates of a sprite. Unknown names panic: the manifest is generated.
    pub fn uv(&self, name: &str) -> Uv {
        let r = self.sprite_rect(name);
        // Tile edges use half-texel centers so linear sampling cannot pull in the atlas gutter.
        let inset = if name.starts_with("ow_") && r.w == 32 && r.h == 32 { 0.5 } else { 0.02 };
        Uv {
            min: Vec2::new(r.x as f32 + inset, r.y as f32 + inset) / self.size,
            max: Vec2::new((r.x + r.w) as f32 - inset, (r.y + r.h) as f32 - inset) / self.size,
        }
    }

    /// Size of a sprite in pixels.
    pub fn px(&self, name: &str) -> Vec2 {
        let r = self.sprite_rect(name);
        Vec2::new(r.w as f32, r.h as f32)
    }
}

pub struct AtlasPlugin;

impl Plugin for AtlasPlugin {
    fn build(&self, app: &mut App) {
        let manifest: Manifest = ron::from_str(MANIFEST).expect("assets/atlas.ron is invalid");
        let image = app.world().resource::<AssetServer>().load("atlas.png");
        let size = Vec2::new(manifest.size.0 as f32, manifest.size.1 as f32);
        app.insert_resource(Atlas { image, size, rects: manifest.sprites });
    }
}
