//! Heightmap terrain (PLAN.md §3). One standable surface per square.

use serde::{Deserialize, Serialize};

use crate::board::{Sq, squares};

pub const MAX_HEIGHT: u8 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TileKind {
    Grass,
    Stone,
    Sand,
    ShallowWater,
    DeepWater,
    Void,
    Ice,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Obstacle {
    Rock,
    Tree,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Feature {
    None,
    /// Entrance to a tunnel; all entrances with the same id are linked.
    Cave(u8),
    Obstacle(Obstacle),
    Pickup(u16),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Tile {
    pub height: u8,
    pub kind: TileKind,
    pub feature: Feature,
}

impl Tile {
    pub const fn flat(height: u8) -> Self {
        Tile { height, kind: TileKind::Grass, feature: Feature::None }
    }

    /// No piece may ever stand here.
    pub fn is_blocked(&self) -> bool {
        self.kind == TileKind::Void || matches!(self.feature, Feature::Obstacle(_))
    }

    pub fn is_water(&self) -> bool {
        matches!(self.kind, TileKind::ShallowWater | TileKind::DeepWater)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Terrain {
    pub size: u8,
    pub tiles: Vec<Tile>,
}

impl Terrain {
    pub fn flat(size: u8) -> Self {
        Terrain { size, tiles: vec![Tile::flat(0); size as usize * size as usize] }
    }

    pub fn get(&self, sq: Sq) -> &Tile {
        &self.tiles[sq.index(self.size)]
    }

    pub fn get_mut(&mut self, sq: Sq) -> &mut Tile {
        &mut self.tiles[sq.index(self.size)]
    }

    pub fn height(&self, sq: Sq) -> u8 {
        self.get(sq).height
    }

    /// The other entrances linked to a cave entrance at `sq` (empty if none).
    pub fn cave_exits(&self, sq: Sq) -> impl Iterator<Item = Sq> + '_ {
        let link = match self.get(sq).feature {
            Feature::Cave(id) => Some(id),
            _ => None,
        };
        squares(self.size).filter(move |&s| {
            s != sq && link.is_some() && self.get(s).feature == Feature::Cave(link.unwrap())
        })
    }
}
