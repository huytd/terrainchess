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
    Smite,
    Evaporate,
    Flood,
    Featherfall,
    Curse,
    Sprout,
    Blink,
    Insight,
}

impl SpellId {
    pub const ALL: [SpellId; 16] = [
        SpellId::RaiseEarth,
        SpellId::LowerEarth,
        SpellId::Freeze,
        SpellId::Bridge,
        SpellId::DigTunnel,
        SpellId::Shield,
        SpellId::Swap,
        SpellId::Rewind,
        SpellId::Smite,
        SpellId::Evaporate,
        SpellId::Flood,
        SpellId::Featherfall,
        SpellId::Curse,
        SpellId::Sprout,
        SpellId::Blink,
        SpellId::Insight,
    ];

    /// Spells that can currently be cast in the game (all spells are castable).
    pub fn is_castable(&self) -> bool {
        true
    }

    /// One line on what the spell does, for the cast announcement.
    pub fn effect_text(self) -> &'static str {
        match self {
            SpellId::RaiseEarth => "Raises a square one level",
            SpellId::LowerEarth => "Lowers a square one level",
            SpellId::Freeze => "Turns nearby water to ice for a while",
            SpellId::Bridge => "Builds a bridge over water or a gap",
            SpellId::DigTunnel => "Links two squares with a tunnel for a while",
            SpellId::Shield => "A piece can't be captured until its side's next turn",
            SpellId::Swap => "Two pieces trade places",
            SpellId::Rewind => "The last two turns are undone",
            SpellId::Smite => "Destroys a tree or rock",
            SpellId::Evaporate => "Dries water or ice into sand",
            SpellId::Flood => "Floods a square with shallow water",
            SpellId::Featherfall => "A piece ignores height on its next move",
            SpellId::Curse => "A piece can't move for two turns",
            SpellId::Sprout => "Grows a tree on a square",
            SpellId::Blink => "A piece teleports up to two squares",
            SpellId::Insight => "Draws extra cards",
        }
    }

    /// Why this spell has no target anywhere on the board.
    pub fn no_target_hint(self) -> &'static str {
        match self {
            SpellId::RaiseEarth | SpellId::LowerEarth => "No square can be reshaped right now",
            SpellId::Freeze => "No water to freeze",
            SpellId::Bridge => "No water or gap next to your pieces",
            SpellId::DigTunnel => "No room for a tunnel near your pieces",
            SpellId::Shield | SpellId::Featherfall => "You have no piece to target",
            SpellId::Swap => "You need two pieces to swap",
            SpellId::Rewind => "Rewind needs two turns of history",
            SpellId::Smite => "No trees or rocks to destroy",
            SpellId::Evaporate => "No free shallow water or ice to dry up",
            SpellId::Flood => "No free grass or sand in the middle rows",
            SpellId::Curse => "No enemy piece to curse",
            SpellId::Sprout => "No free grass in the middle rows",
            SpellId::Blink => "None of your pieces has room to blink",
            SpellId::Insight => "Your deck is empty",
        }
    }

    /// What this spell can target, shown when the player picks a wrong square.
    pub fn target_hint(self) -> &'static str {
        match self {
            SpellId::RaiseEarth => "Raise Earth needs an open square",
            SpellId::LowerEarth => "Lower Earth needs an open square",
            SpellId::Freeze => "Freeze needs water next to the square",
            SpellId::Bridge => "Bridge needs water or a gap next to your piece",
            SpellId::DigTunnel => "Tunnels need empty squares near your pieces, 3+ apart",
            SpellId::Shield | SpellId::Featherfall | SpellId::Swap => "Pick one of your own pieces",
            SpellId::Blink => "Pick your piece, then an open square within 2",
            SpellId::Smite => "Smite needs a tree or rock",
            SpellId::Evaporate => "Evaporate needs free shallow water or ice",
            SpellId::Flood => "Flood needs free grass or sand in the middle rows",
            SpellId::Curse => "Pick an enemy piece (not the king)",
            SpellId::Sprout => "Sprout needs free grass in the middle rows",
            SpellId::Rewind | SpellId::Insight => "This spell has no target",
        }
    }

    /// Quick spells do not end the caster's turn.
    pub fn is_quick(&self) -> bool {
        matches!(self, SpellId::Shield | SpellId::Featherfall | SpellId::Insight)
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
    Smite(Sq),
    Evaporate(Sq),
    Flood(Sq),
    Featherfall(Sq),
    Curse(Sq),
    Sprout(Sq),
    Blink(Sq, Sq),
    Insight,
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
            SpellCast::Smite(_) => SpellId::Smite,
            SpellCast::Evaporate(_) => SpellId::Evaporate,
            SpellCast::Flood(_) => SpellId::Flood,
            SpellCast::Featherfall(_) => SpellId::Featherfall,
            SpellCast::Curse(_) => SpellId::Curse,
            SpellCast::Sprout(_) => SpellId::Sprout,
            SpellCast::Blink(_, _) => SpellId::Blink,
            SpellCast::Insight => SpellId::Insight,
        }
    }

    /// Board squares this cast names.
    pub fn squares(&self) -> Vec<Sq> {
        match *self {
            SpellCast::RaiseEarth(a)
            | SpellCast::LowerEarth(a)
            | SpellCast::Freeze(a)
            | SpellCast::Bridge(a)
            | SpellCast::Shield(a)
            | SpellCast::Smite(a)
            | SpellCast::Evaporate(a)
            | SpellCast::Flood(a)
            | SpellCast::Featherfall(a)
            | SpellCast::Curse(a)
            | SpellCast::Sprout(a) => vec![a],
            SpellCast::DigTunnel(a, b) | SpellCast::Swap(a, b) | SpellCast::Blink(a, b) => vec![a, b],
            SpellCast::Rewind | SpellCast::Insight => Vec::new(),
        }
    }

    /// Whether this cast is a quick spell that does not end the turn.
    pub fn is_quick(&self) -> bool {
        matches!(
            self,
            SpellCast::Shield(_) | SpellCast::Rewind | SpellCast::Featherfall(_) | SpellCast::Insight
        )
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
    /// Legacy: cast cards used to stay in the hand marked used; they now leave the hand, so
    /// this stays all false. Kept so the rest of the rules and saved states still line up.
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
