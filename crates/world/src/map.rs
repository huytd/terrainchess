use crate::hero::HeroId;
use serde::{Deserialize, Serialize};
use tc_core::piece::PieceKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MapPos {
    pub x: u16,
    pub y: u16,
}

impl MapPos {
    pub fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }
    pub fn manhattan(self, other: Self) -> u16 {
        self.x.abs_diff(other.x) + self.y.abs_diff(other.y)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Biome {
    Grass,
    Forest,
    Hills,
    Mountain,
    Water,
    Coast,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapTile {
    pub biome: Biome,
    pub road: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CampKind {
    Village,
    KnightCamp,
    BishopCamp,
    Fortress,
    Citadel,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectKind {
    Camp(CampKind),
    Chest,
    Shrine,
    Signpost,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapObject {
    pub pos: MapPos,
    pub kind: ObjectKind,
    pub owner: Option<HeroId>,
    pub cleared: bool,
    pub guards: Vec<PieceKind>,
    pub tier: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldMap {
    pub size: (u16, u16),
    pub tiles: Vec<MapTile>,
    pub objects: Vec<MapObject>,
}

impl WorldMap {
    pub fn new(width: u16, height: u16) -> Self {
        WorldMap {
            size: (width, height),
            tiles: vec![MapTile { biome: Biome::Grass, road: false }; width as usize * height as usize],
            objects: Vec::new(),
        }
    }

    pub fn get(&self, pos: MapPos) -> Option<MapTile> {
        if pos.x < self.size.0 && pos.y < self.size.1 {
            Some(self.tiles[pos.y as usize * self.size.0 as usize + pos.x as usize])
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, pos: MapPos) -> Option<&mut MapTile> {
        if pos.x < self.size.0 && pos.y < self.size.1 {
            Some(&mut self.tiles[pos.y as usize * self.size.0 as usize + pos.x as usize])
        } else {
            None
        }
    }

    pub fn get_object(&self, pos: MapPos) -> Option<&MapObject> {
        self.objects.iter().find(|o| o.pos == pos)
    }

    pub fn get_object_mut(&mut self, pos: MapPos) -> Option<&mut MapObject> {
        self.objects.iter_mut().find(|o| o.pos == pos)
    }
}
