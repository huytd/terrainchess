//! Roguelike run state, match setup, reward drafting, and save/load (specs/game-design.md §1, §4).

use serde::{Deserialize, Serialize};
use tc_core::Pickup;
use tc_core::board::Sq;
use tc_core::piece::Side;
use tc_core::position::Position;
use tc_core::rng::Rng;
use tc_core::rules::Rules;
use tc_core::spell::SpellId;
use tc_core::terrain::{Feature, Terrain, TileKind};
use tc_core::worldgen::GenParams;

use crate::item::{ItemId, ItemKind, Rarity, RelicEffect, catalog, find_item};

/// Number of normal floors before the boss.
pub const FLOORS: u8 = 7;
/// Floor index of the final boss match.
pub const BOSS_FLOOR: u8 = 7;
/// Total number of matches in a victorious run (floors 0 through 7).
pub const TOTAL_FLOORS: u8 = 8;

/// Outcome of a completed run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RunOutcome {
    Won,
    Lost,
}

/// Errors that can occur during run operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunError {
    /// Attempted to pick an item that was not offered in the draft.
    NotInDraft(String),
    /// Attempted to pick when no draft has been rolled.
    NoDraftAvailable,
}

impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunError::NotInDraft(id) => write!(f, "item '{id}' was not in the last draft"),
            RunError::NoDraftAvailable => write!(f, "no draft is currently available to pick from"),
        }
    }
}

impl std::error::Error for RunError {}

/// Configuration and rules for an individual match in the run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MatchSetup {
    pub seed: u64,
    pub r#gen: GenParams,
    pub rules: Rules,
    pub ai_level: u8,
    pub player: Side,
    pub relics: Vec<RelicEffect>,
    pub pickups: Vec<(Sq, Pickup)>,
    pub player_deck: Vec<SpellId>,
    pub enemy_deck: Vec<SpellId>,
    pub enemy_items: Vec<ItemId>,
    pub veteran: [bool; 2],
    #[serde(default)]
    pub armies: Option<[Vec<tc_core::piece::PieceKind>; 2]>,
}

impl MatchSetup {
    pub fn position(&self, size: u8) -> Result<Position, String> {
        if let Some(ref armies) = self.armies {
            Position::from_army(size, &armies[0], &armies[1])
        } else {
            Ok(Position::start(size))
        }
    }
    /// Generate terrain for this match and apply relic modifications.
    pub fn terrain(&self) -> (Terrain, u64) {
        let pos = self.position(self.r#gen.size).unwrap_or_else(|_| Position::start(self.r#gen.size));
        let (mut terrain, actual_seed) = tc_core::worldgen::generate_for(self.seed, &self.r#gen, &pos);
        if self.relics.contains(&RelicEffect::TectonicPact) {
            let home = Position::home_rows(self.r#gen.size);
            let y_range = match self.player {
                Side::White => 0..home,
                Side::Black => (self.r#gen.size - home)..self.r#gen.size,
            };
            for y in y_range {
                for x in 0..self.r#gen.size {
                    let tile = terrain.get_mut(Sq::new(x, y));
                    tile.height = (tile.height + 1).min(tc_core::terrain::MAX_HEIGHT);
                }
            }
        }
        if self.relics.contains(&RelicEffect::TideCharm) {
            let mid = self.r#gen.size / 2;
            let y_range = match self.player {
                Side::White => 0..mid,
                Side::Black => mid..self.r#gen.size,
            };
            for y in y_range {
                for x in 0..self.r#gen.size {
                    let tile = terrain.get_mut(Sq::new(x, y));
                    if tile.kind == TileKind::ShallowWater {
                        tile.kind = TileKind::Sand;
                    }
                }
            }
        }
        (terrain, actual_seed)
    }
}

/// The persistent state of a roguelike run (specs/game-design.md §1, §4).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunState {
    /// Initial seed for the entire run.
    pub seed: u64,
    /// Board grid size (e.g. 8 for 8×8).
    pub size: u8,
    /// Current floor index (0..=7).
    pub floor: u8,
    /// IDs of all items owned by the player.
    pub owned: Vec<String>,
    /// Persistent RNG state for drafting and procedural rolls.
    pub rng: u64,
    /// Final outcome of the run once finished.
    pub outcome: Option<RunOutcome>,
    /// The most recent draft offered to the player.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_draft: Option<[ItemId; 3]>,
    /// Extra draft picks remaining from collected run items.
    #[serde(default)]
    pub bonus_picks: u8,
}

impl RunState {
    /// Create a fresh run at floor 0.
    pub fn new(seed: u64, size: u8) -> Self {
        RunState {
            seed,
            size,
            floor: 0,
            owned: Vec::new(),
            rng: seed,
            outcome: None,
            last_draft: None,
            bonus_picks: 0,
        }
    }

    /// Derive the deterministic match seed from run seed and floor.
    pub fn match_seed(run_seed: u64, floor: u8) -> u64 {
        tc_core::rng::mix(
            run_seed
                ^ ((floor as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)).wrapping_add(0xD1B5_4A32_D192_ED03),
        )
    }

    /// Build the match configuration for the current floor.
    pub fn match_setup(&self) -> MatchSetup {
        let seed = Self::match_seed(self.seed, self.floor);
        let mut r#gen = GenParams::for_floor(self.size, self.floor);

        let mut relics = Vec::new();
        for id in &self.owned {
            if let Some(crate::item::Item { kind: ItemKind::Relic { effect }, .. }) = find_item(id) {
                relics.push(*effect);
            }
        }

        if relics.contains(&RelicEffect::CalmTerrain) {
            r#gen.roughness = (r#gen.roughness - 0.15).max(0.0);
        }

        let mut rules = Rules::standard(self.size);
        let player = Side::White;

        for id in &self.owned {
            if let Some(crate::item::Item { kind: ItemKind::Enhancement { piece, delta }, .. }) =
                find_item(id)
            {
                let profile = &mut rules.profiles[player.index()][piece.index()];
                if let Some(d) = delta.max_climb {
                    profile.max_climb = (profile.max_climb as i8 + d).clamp(0, 3) as u8;
                }
                if let Some(d) = delta.jump_max_dh {
                    profile.jump_max_dh = (profile.jump_max_dh as i8 + d).clamp(0, 3) as u8;
                }
                if let Some(d) = delta.max_slide_drop {
                    profile.max_slide_drop = (profile.max_slide_drop as i8 + d).clamp(0, 3) as u8;
                }
                if let Some(b) = delta.deep_water {
                    profile.deep_water = b;
                }
                if let Some(b) = delta.uphill_ends_slide {
                    profile.uphill_ends_slide = b;
                }
                if let Some(b) = delta.cave_slide {
                    profile.cave_slide = b;
                }
            }
        }

        let mut veteran = [false; 2];
        if self.owned.iter().any(|id| id == "veteran") {
            veteran[player.index()] = true;
        }

        let mut enemy_items = Vec::new();
        let enemy_count = if self.floor >= 3 { ((self.floor - 3) / 2 + 1) as usize } else { 0 };
        if enemy_count > 0 {
            let mut available_enhancements: Vec<&'static crate::item::Item> =
                catalog().iter().filter(|item| matches!(item.kind, ItemKind::Enhancement { .. })).collect();
            available_enhancements.sort_by_key(|i| &i.id);
            let mut enemy_rng = Rng::new(seed ^ 0x454E_454D_595F_454E);
            for _ in 0..enemy_count {
                if available_enhancements.is_empty() {
                    break;
                }
                let idx = enemy_rng.below(available_enhancements.len() as u32) as usize;
                let item = available_enhancements.swap_remove(idx);
                enemy_items.push(item.id.clone());
                if let ItemKind::Enhancement { piece, delta } = &item.kind {
                    let profile = &mut rules.profiles[Side::Black.index()][piece.index()];
                    if let Some(d) = delta.max_climb {
                        profile.max_climb = (profile.max_climb as i8 + d).clamp(0, 3) as u8;
                    }
                    if let Some(d) = delta.jump_max_dh {
                        profile.jump_max_dh = (profile.jump_max_dh as i8 + d).clamp(0, 3) as u8;
                    }
                    if let Some(d) = delta.max_slide_drop {
                        profile.max_slide_drop = (profile.max_slide_drop as i8 + d).clamp(0, 3) as u8;
                    }
                    if let Some(b) = delta.deep_water {
                        profile.deep_water = b;
                    }
                    if let Some(b) = delta.uphill_ends_slide {
                        profile.uphill_ends_slide = b;
                    }
                    if let Some(b) = delta.cave_slide {
                        profile.cave_slide = b;
                    }
                }
            }
        }

        let mut player_deck = Vec::with_capacity(15);
        for id in &self.owned {
            if let Some(crate::item::Item { kind: ItemKind::Spell { spell, charges: c, .. }, .. }) =
                find_item(id)
            {
                for _ in 0..*c {
                    if player_deck.len() < 15 {
                        player_deck.push(*spell);
                    }
                }
            }
        }

        const FILLER: [SpellId; 5] =
            [SpellId::RaiseEarth, SpellId::LowerEarth, SpellId::Shield, SpellId::Swap, SpellId::Freeze];
        let mut filler_idx = 0;
        while player_deck.len() < 15 {
            player_deck.push(FILLER[filler_idx % FILLER.len()]);
            filler_idx += 1;
        }

        let mut player_rng = Rng::new(seed);
        for i in (1..player_deck.len()).rev() {
            let j = player_rng.below((i + 1) as u32) as usize;
            player_deck.swap(i, j);
        }

        const ALL_CASTABLE: [SpellId; 8] = [
            SpellId::RaiseEarth,
            SpellId::LowerEarth,
            SpellId::Freeze,
            SpellId::Bridge,
            SpellId::DigTunnel,
            SpellId::Shield,
            SpellId::Swap,
            SpellId::Rewind,
        ];
        let mut enemy_deck_rng = Rng::new(seed ^ 0x454E_454D_595F_4445);
        let mut enemy_deck = Vec::with_capacity(15);
        for _ in 0..enemy_items.len() {
            if enemy_deck.len() < 15 {
                let spell = ALL_CASTABLE[enemy_deck_rng.below(ALL_CASTABLE.len() as u32) as usize];
                enemy_deck.push(spell);
            }
        }
        let mut enemy_filler_idx = 0;
        while enemy_deck.len() < 15 {
            enemy_deck.push(FILLER[enemy_filler_idx % FILLER.len()]);
            enemy_filler_idx += 1;
        }
        for i in (1..enemy_deck.len()).rev() {
            let j = enemy_deck_rng.below((i + 1) as u32) as usize;
            enemy_deck.swap(i, j);
        }

        let ai_level = (self.floor + 1).min(7);

        let setup_temp = MatchSetup {
            seed,
            r#gen: r#gen.clone(),
            rules: rules.clone(),
            ai_level,
            player,
            relics: relics.clone(),
            pickups: Vec::new(),
            player_deck,
            enemy_deck,
            enemy_items,
            veteran,
            armies: None,
        };
        let (terrain, actual_seed) = setup_temp.terrain();
        let pos = setup_temp.position(self.size).unwrap_or_else(|_| Position::start(self.size));
        let cartographer = relics.contains(&RelicEffect::Cartographer);
        let pickups = place_pickups(actual_seed, self.floor, self.size, &terrain, player, cartographer, &pos);

        MatchSetup { pickups, ..setup_temp }
    }

    /// Record the outcome of the current match.
    pub fn record_result(&mut self, won: bool, run_items_collected: u8) {
        if !won {
            self.outcome = Some(RunOutcome::Lost);
        } else if self.floor >= BOSS_FLOOR {
            self.outcome = Some(RunOutcome::Won);
        } else {
            self.floor += 1;
            self.bonus_picks = self.bonus_picks.saturating_add(run_items_collected);
        }
    }

    /// Roll three distinct items the player does not own yet.
    pub fn draft(&mut self) -> [ItemId; 3] {
        let selected = roll_draft(&mut self.rng, &self.owned, self.floor, 3);
        assert_eq!(selected.len(), 3, "not enough unowned items available to draft");
        let draft = [selected[0].clone(), selected[1].clone(), selected[2].clone()];
        self.last_draft = Some(draft.clone());
        draft
    }

    /// Add a chosen item to owned items if it was part of the last draft.
    pub fn pick(&mut self, id: &str) -> Result<(), RunError> {
        let Some(ref draft) = self.last_draft else {
            return Err(RunError::NoDraftAvailable);
        };
        if !draft.iter().any(|d| d == id) {
            return Err(RunError::NotInDraft(id.to_string()));
        }
        self.owned.push(id.to_string());
        self.last_draft = None;
        Ok(())
    }

    /// Serialize run state to a RON formatted string.
    pub fn to_ron(&self) -> Result<String, ron::Error> {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
    }

    /// Deserialize run state from a RON formatted string.
    pub fn from_ron(s: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str(s)
    }
}

/// Roll up to `count` distinct unowned items matching rarity tier rules.
pub(crate) fn roll_draft(rng_state: &mut u64, owned: &[ItemId], floor: u8, count: usize) -> Vec<ItemId> {
    let mut rng = Rng::new(*rng_state);
    let mut selected: Vec<ItemId> = Vec::with_capacity(count);

    for _ in 0..count {
        let available: Vec<&'static crate::item::Item> = catalog()
            .iter()
            .filter(|item| {
                !owned.contains(&item.id)
                    && !selected.contains(&item.id)
                    && (floor >= 2 || item.rarity != Rarity::Rare)
            })
            .collect();

        if available.is_empty() {
            break;
        }

        let common: Vec<&'static crate::item::Item> =
            available.iter().copied().filter(|i| i.rarity == Rarity::Common).collect();
        let uncommon: Vec<&'static crate::item::Item> =
            available.iter().copied().filter(|i| i.rarity == Rarity::Uncommon).collect();
        let rare: Vec<&'static crate::item::Item> =
            available.iter().copied().filter(|i| i.rarity == Rarity::Rare).collect();

        let w_common = if common.is_empty() { 0 } else { 60 };
        let w_uncommon = if uncommon.is_empty() { 0 } else { 30 };
        let w_rare = if rare.is_empty() { 0 } else { 10 };
        let total_w = w_common + w_uncommon + w_rare;

        if total_w == 0 {
            break;
        }

        let roll = rng.below(total_w);
        let pool = if roll < w_common {
            &common
        } else if roll < w_common + w_uncommon {
            &uncommon
        } else {
            &rare
        };

        let chosen_idx = rng.below(pool.len() as u32) as usize;
        selected.push(pool[chosen_idx].id.clone());
    }

    *rng_state = rng.state();
    selected
}

/// Place pickups deterministically on valid board squares favoring high ground,
/// cave entrances and the enemy half.
pub(crate) fn place_pickups(
    actual_seed: u64,
    floor: u8,
    size: u8,
    terrain: &Terrain,
    player: Side,
    cartographer: bool,
    start_pos: &Position,
) -> Vec<(Sq, Pickup)> {
    let base_count = match size {
        8 => 2,
        16 => 4,
        32 => 8,
        s => (s / 4).max(2) as usize,
    };
    let target_count = if cartographer { base_count + 1 } else { base_count };

    let home = Position::home_rows(size);

    // Candidates: empty non-home squares that a piece can stand on
    let mut candidates: Vec<(Sq, u32)> = Vec::new();
    for sq in tc_core::board::squares(size) {
        if sq.y < home || sq.y >= size - home {
            continue;
        }
        if start_pos.get(sq).is_some() {
            continue;
        }
        let tile = terrain.get(sq);
        if tile.is_blocked() || tile.kind == TileKind::DeepWater {
            continue;
        }

        // Preference weighting:
        // Highest squares: tile.height * 3
        // Cave entrances: +4
        // Enemy half: +3
        let mut weight = 1 + tile.height as u32 * 3;
        if matches!(tile.feature, Feature::Cave(_)) {
            weight += 4;
        }
        let enemy_half = match player {
            Side::White => sq.y >= size / 2,
            Side::Black => sq.y < size / 2,
        };
        if enemy_half {
            weight += 3;
        }

        candidates.push((sq, weight));
    }

    let mut rng = Rng::new(actual_seed ^ 0x5049_434B_5550_5321);
    let mut selected_squares: Vec<Sq> = Vec::with_capacity(target_count);

    while selected_squares.len() < target_count && !candidates.is_empty() {
        let total_weight: u32 = candidates.iter().map(|(_, w)| *w).sum();
        if total_weight == 0 {
            break;
        }
        let roll = rng.below(total_weight);
        let mut running = 0;
        let mut chosen_idx = 0;
        for (i, (_, w)) in candidates.iter().enumerate() {
            running += *w;
            if roll < running {
                chosen_idx = i;
                break;
            }
        }
        let (sq, _) = candidates.swap_remove(chosen_idx);
        selected_squares.push(sq);
    }

    let mut run_item_indices = Vec::new();
    if cartographer && !selected_squares.is_empty() {
        let idx = rng.below(selected_squares.len() as u32) as usize;
        run_item_indices.push(idx);
    }
    let has_run_item = floor >= 2 && rng.below(100) < 30 && selected_squares.len() > run_item_indices.len();
    if has_run_item {
        let remaining: Vec<usize> =
            (0..selected_squares.len()).filter(|i| !run_item_indices.contains(i)).collect();
        let idx = remaining[rng.below(remaining.len() as u32) as usize];
        run_item_indices.push(idx);
    }

    let mut pickups = Vec::new();
    for (i, sq) in selected_squares.into_iter().enumerate() {
        if run_item_indices.contains(&i) {
            pickups.push((sq, Pickup::RunItem));
        }
    }

    pickups
}
