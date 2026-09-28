//! Item data models and catalog loader (PLAN.md §2, §6).

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};
use tc_core::PieceKind;

/// Rarity tier for item drafting and rewards.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
}

/// Changes applied to a piece's [`tc_core::MoveProfile`].
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileDelta {
    /// Delta added to max climb height.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_climb: Option<i8>,
    /// Whether entering deep water is permitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deep_water: Option<bool>,
    /// Delta added to knight jump height difference limit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jump_max_dh: Option<i8>,
    /// Whether moving uphill terminates a slide.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uphill_ends_slide: Option<bool>,
    /// Delta added to max slide drop before stopping.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_slide_drop: Option<i8>,
}

/// Passive global relic effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelicEffect {
    /// Player's home ranks start 1 level higher.
    TectonicPact,
    /// Floors generate with roughness reduced by 0.15.
    CalmTerrain,
}

/// Active spells available in a run.
pub use tc_core::SpellId;

/// The specific behavior and data for an item.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemKind {
    Enhancement { piece: PieceKind, delta: ProfileDelta },
    Relic { effect: RelicEffect },
    Spell { spell: SpellId, charges: u8, quick: bool },
}

/// String identifier for an item.
pub type ItemId = String;

/// An item reward that can be drafted and owned during a run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    pub id: ItemId,
    pub name: String,
    pub description: String,
    pub rarity: Rarity,
    pub kind: ItemKind,
}

const ENHANCEMENTS_RON: &str = include_str!("../../../assets/items/enhancements.ron");
const RELICS_RON: &str = include_str!("../../../assets/items/relics.ron");
const SPELLS_RON: &str = include_str!("../../../assets/items/spells.ron");

/// Parse items from a RON formatted string.
pub fn parse_items_from_ron(ron_str: &str) -> Result<Vec<Item>, ron::error::SpannedError> {
    ron::from_str(ron_str)
}

/// Load and parse all embedded item catalog files.
pub fn load_all_items() -> Result<Vec<Item>, ron::error::SpannedError> {
    let mut items = Vec::new();
    let enhancements: Vec<Item> = parse_items_from_ron(ENHANCEMENTS_RON)?;
    let relics: Vec<Item> = parse_items_from_ron(RELICS_RON)?;
    let spells: Vec<Item> = parse_items_from_ron(SPELLS_RON)?;
    items.extend(enhancements);
    items.extend(relics);
    items.extend(spells);
    Ok(items)
}

static CATALOG: LazyLock<Vec<Item>> = LazyLock::new(|| {
    // Invariant: embedded RON data files are validated by tests at build time.
    load_all_items().expect("embedded item RON assets must be valid")
});

/// Global catalog of all available items.
pub fn catalog() -> &'static [Item] {
    &CATALOG
}

/// Look up an item in the catalog by its ID.
pub fn find_item(id: &str) -> Option<&'static Item> {
    catalog().iter().find(|i| i.id == id)
}
