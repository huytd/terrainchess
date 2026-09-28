//! Spell definitions and casting actions (PLAN.md §6).

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
    /// Spells that can currently be cast in the game.
    /// Bridge, DigTunnel and Rewind are reserved for future phases.
    pub fn is_castable(&self) -> bool {
        matches!(
            self,
            SpellId::RaiseEarth | SpellId::LowerEarth | SpellId::Freeze | SpellId::Shield | SpellId::Swap
        )
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
    Shield(Sq),
    Swap(Sq, Sq),
}

impl SpellCast {
    pub fn spell_id(&self) -> SpellId {
        match self {
            SpellCast::RaiseEarth(_) => SpellId::RaiseEarth,
            SpellCast::LowerEarth(_) => SpellId::LowerEarth,
            SpellCast::Freeze(_) => SpellId::Freeze,
            SpellCast::Shield(_) => SpellId::Shield,
            SpellCast::Swap(_, _) => SpellId::Swap,
        }
    }

    /// Whether this cast is a quick spell that does not end the turn.
    pub fn is_quick(&self) -> bool {
        self.spell_id().is_quick()
    }
}
