use crate::map::MapPos;
use serde::{Deserialize, Serialize};
use tc_core::piece::PieceKind;
use tc_core::spell::SpellId;
use tc_run::item::ItemId;

pub type HeroId = u8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Faction {
    AshenSun,
    HollowCrown,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Hero {
    pub id: HeroId,
    pub faction: Faction,
    pub name: String,
    pub pos: MapPos,
    pub prev_pos: MapPos,
    pub roster: Vec<PieceKind>,
    pub cards: Vec<SpellId>,
    pub items: Vec<ItemId>,
    pub movement: f32,
    pub is_ai: bool,
    pub alive: bool,
}

impl Hero {
    pub fn new(id: HeroId, faction: Faction, name: String, pos: MapPos, is_ai: bool) -> Self {
        Hero {
            id,
            faction,
            name,
            pos,
            prev_pos: pos,
            roster: vec![PieceKind::King, PieceKind::Pawn, PieceKind::Pawn, PieceKind::Pawn],
            cards: Vec::new(),
            items: Vec::new(),
            movement: 0.0,
            is_ai,
            alive: true,
        }
    }
}
