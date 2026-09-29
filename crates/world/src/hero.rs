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
    #[serde(default)]
    pub cards: Vec<SpellId>,
    #[serde(default)]
    pub deck: Vec<SpellId>,
    #[serde(default)]
    pub items: Vec<ItemId>,
    pub movement: f32,
    pub is_ai: bool,
    pub alive: bool,
}

impl Hero {
    pub fn new(id: HeroId, faction: Faction, name: String, pos: MapPos, is_ai: bool, seed: u64) -> Self {
        let cards = tc_core::filler_deck(seed);
        Hero {
            id,
            faction,
            name,
            pos,
            prev_pos: pos,
            roster: vec![PieceKind::King, PieceKind::Pawn, PieceKind::Pawn, PieceKind::Pawn],
            deck: cards.clone(),
            cards,
            items: Vec::new(),
            movement: 0.0,
            is_ai,
            alive: true,
        }
    }

    pub fn normalize_cards(&mut self, seed: u64) {
        if self.cards.len() < 15 {
            let filler = tc_core::filler_deck(seed);
            self.cards.extend(filler.into_iter().take(15 - self.cards.len()));
        }

        if !self.has_valid_deck() {
            self.deck = self.cards.iter().take(15).copied().collect();
        }
    }

    pub fn has_valid_deck(&self) -> bool {
        if self.deck.len() != 15 {
            return false;
        }

        let mut available = self.cards.clone();
        for card in &self.deck {
            let Some(index) = available.iter().position(|candidate| candidate == card) else {
                return false;
            };
            available.remove(index);
        }
        true
    }
}
