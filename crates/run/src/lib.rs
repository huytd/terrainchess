//! Roguelike run crate for Terrain Chess (`tc_run`).
//!
//! Provides the run loop logic: run state persistence, reward drafts,
//! piece enhancements, relics, spells, and match configuration.

pub mod deck;
pub mod item;
pub mod level;
pub mod profile;
pub mod run;

pub use deck::{DECK_SIZE, FILLER};
pub use item::{
    Item, ItemId, ItemKind, ProfileDelta, Rarity, RelicEffect, SpellId, catalog, find_item, load_all_items,
    parse_items_from_ron,
};
pub use level::{STAGES_PER_TIER, Stage, THEMES, Theme, Variant, reinforce};
pub use profile::{Endless, Profile};
pub use run::{BOSS_FLOOR, FLOORS, MatchSetup, RunError, RunOutcome, RunState, TOTAL_FLOORS};
pub use tc_core::{Pickup, SpellCast};
