//! Persistent cross-level player state.

use serde::{Deserialize, Serialize};
use tc_core::Position;

use crate::item::{ItemId, RelicEffect};
use crate::level::Level;
use crate::run::{MatchSetup, RunError, RunState, place_pickups, roll_draft};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    pub rng: u64,
    pub owned: Vec<ItemId>,
    pub cleared: Vec<u8>,
    #[serde(default)]
    pub last_draft: Option<Vec<ItemId>>,
}

impl Profile {
    pub fn new(seed: u64) -> Self {
        Profile { rng: seed, owned: Vec::new(), cleared: Vec::new(), last_draft: None }
    }

    /// Everything needed to play `level`: builds a temporary RunState { seed: match_seed, size: level.size,
    /// floor: level.difficulty, owned: self.owned.clone(), .. } and calls its match_setup() (so relics,
    /// enhancements, spells, enemy items, AI level and pickups all work as before), then sets
    /// `armies = level.army.map(|a| [a.to_vec(), a.to_vec()])`.
    pub fn match_setup(&self, level: &Level, match_seed: u64) -> MatchSetup {
        let temp_run = RunState {
            seed: match_seed,
            size: level.size,
            floor: level.difficulty,
            owned: self.owned.clone(),
            rng: match_seed,
            outcome: None,
            bonus_picks: 0,
            last_draft: None,
        };
        let mut setup = temp_run.match_setup();
        setup.armies = level.army.map(|a| [a.to_vec(), a.to_vec()]);
        if setup.armies.is_some() {
            let (terrain, actual_seed) = setup.terrain();
            let pos = setup.position(level.size).unwrap_or_else(|_| Position::start(level.size));
            let cartographer = setup.relics.contains(&RelicEffect::Cartographer);
            setup.pickups = place_pickups(
                actual_seed,
                level.difficulty,
                level.size,
                &terrain,
                setup.player,
                cartographer,
                &pos,
            );
        }
        setup
    }

    /// Mark cleared (no duplicates) and roll a draft: up to 3 distinct unowned items (same rarity rules as
    /// RunState::draft with floor = level.difficulty). Must NOT panic when fewer than 3 unowned items remain —
    /// return fewer (possibly empty). Stores it in last_draft.
    pub fn record_win(&mut self, level: &Level) -> Vec<ItemId> {
        if !self.cleared.contains(&level.id) {
            self.cleared.push(level.id);
        }
        let draft = roll_draft(&mut self.rng, &self.owned, level.difficulty, 3);
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

    /// Serialize profile to a RON formatted string.
    pub fn to_ron(&self) -> Result<String, ron::Error> {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
    }

    /// Deserialize profile from a RON formatted string.
    pub fn from_ron(s: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spell_profile() -> Profile {
        let mut profile = Profile::new(0);
        profile.owned = vec![
            "raise_earth".into(),
            "lower_earth".into(),
            "freeze".into(),
            "bridge".into(),
            "smite".into(),
            "shield".into(),
        ];
        profile
    }

    #[test]
    fn match_setup_shuffles_player_deck_per_seed() {
        let level = crate::level(1).unwrap();
        let profile = spell_profile();

        // Same seed => same order (deterministic).
        let first = profile.match_setup(level, 42).player_deck;
        let second = profile.match_setup(level, 42).player_deck;
        assert_eq!(first, second);
        assert_eq!(first.len(), 15);

        // Different seeds => different orders for most seed pairs.
        let mut differed = 0;
        for i in 0..20u64 {
            let a = profile.match_setup(level, i).player_deck;
            let b = profile.match_setup(level, 1000 + i).player_deck;
            if a != b {
                differed += 1;
            }
        }
        assert!(differed >= 15, "only {differed}/20 seed pairs differed");
    }
}
