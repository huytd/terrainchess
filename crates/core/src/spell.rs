//! Spell definitions and casting actions (specs/game-design.md §4).

use serde::{Deserialize, Serialize};

use crate::board::Sq;

/// Active spells available in Terrain Chess.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpellId {
    RaiseEarth,
    LowerEarth,
    Freeze,
    Bridge,
    DigTunnel,
    Shield,
    Swap,
    Rewind,
}

impl SpellId {
    /// Spells that can currently be cast in the game (all spells are castable).
    pub fn is_castable(&self) -> bool {
        true
    }

    /// Quick spells do not end the caster's turn.
    pub fn is_quick(&self) -> bool {
        matches!(self, SpellId::Shield)
    }
}

/// A specific spell cast with target square(s).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpellCast {
    RaiseEarth(Sq),
    LowerEarth(Sq),
    Freeze(Sq),
    Bridge(Sq),
    DigTunnel(Sq, Sq),
    Shield(Sq),
    Swap(Sq, Sq),
    Rewind,
}

impl SpellCast {
    pub fn spell_id(&self) -> SpellId {
        match self {
            SpellCast::RaiseEarth(_) => SpellId::RaiseEarth,
            SpellCast::LowerEarth(_) => SpellId::LowerEarth,
            SpellCast::Freeze(_) => SpellId::Freeze,
            SpellCast::Bridge(_) => SpellId::Bridge,
            SpellCast::DigTunnel(_, _) => SpellId::DigTunnel,
            SpellCast::Shield(_) => SpellId::Shield,
            SpellCast::Swap(_, _) => SpellId::Swap,
            SpellCast::Rewind => SpellId::Rewind,
        }
    }

    /// Whether this cast is a quick spell that does not end the turn.
    pub fn is_quick(&self) -> bool {
        matches!(self, SpellCast::Shield(_) | SpellCast::Rewind)
    }
}

pub const MAX_DISCARDS: u8 = 5;

fn default_discards_left() -> u8 {
    MAX_DISCARDS
}

/// Spell deck, hand of up to 3 cards, slot usage, and discard pile.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpellHand {
    pub deck: Vec<SpellId>,
    pub hand: [Option<SpellId>; 3],
    pub used: [bool; 3],
    pub discarded: Vec<SpellId>,
    #[serde(default = "default_discards_left")]
    pub discards_left: u8,
}

impl Default for SpellHand {
    fn default() -> Self {
        SpellHand {
            deck: Vec::new(),
            hand: [None; 3],
            used: [false; 3],
            discarded: Vec::new(),
            discards_left: MAX_DISCARDS,
        }
    }
}

impl SpellHand {
    /// Creates a hand from an already shuffled deck, empties the hand and draws up to 3 cards.
    pub fn new(deck: Vec<SpellId>) -> Self {
        let mut sh = SpellHand {
            deck,
            hand: [None; 3],
            used: [false; 3],
            discarded: Vec::new(),
            discards_left: MAX_DISCARDS,
        };
        sh.draw();
        sh
    }

    /// Clears the hand and draws up to 3 cards from the front of the deck.
    pub fn draw(&mut self) {
        self.hand = [None; 3];
        self.used = [false; 3];
        for i in 0..3 {
            if !self.deck.is_empty() {
                self.hand[i] = Some(self.deck.remove(0));
            } else {
                self.hand[i] = None;
            }
        }
    }
}

/// Generates a fresh 15-card filler deck cycling RaiseEarth, LowerEarth, Shield, Swap, Freeze,
/// shuffled deterministically from the given seed.
pub fn filler_deck(seed: u64) -> Vec<SpellId> {
    const FILLER: [SpellId; 5] =
        [SpellId::RaiseEarth, SpellId::LowerEarth, SpellId::Shield, SpellId::Swap, SpellId::Freeze];
    let mut deck = Vec::with_capacity(15);
    for i in 0..15 {
        deck.push(FILLER[i % FILLER.len()]);
    }
    let mut rng = crate::rng::Rng::new(seed);
    for i in (1..deck.len()).rev() {
        let j = rng.below((i + 1) as u32) as usize;
        deck.swap(i, j);
    }
    deck
}
