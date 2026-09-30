//! Persistent cross-level player state.

use serde::{Deserialize, Serialize};
use tc_core::Position;
use tc_core::spell::SpellId;

use crate::deck;
use crate::item::{ItemId, RelicEffect};
use crate::level::Stage;
use crate::run::{MatchSetup, RunError, RunState, place_pickups, roll_draft, roll_enemy_deck};

/// The run in progress. Everything else about a stage comes from `Stage::new(stage)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Endless {
    pub seed: u64,
    /// The stage to play next (1-based).
    pub stage: u32,
}

impl Endless {
    /// The board seed for the current stage: a reload gives the same board.
    pub fn match_seed(&self) -> u64 {
        tc_core::rng::mix(self.seed ^ (self.stage as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    pub rng: u64,
    pub owned: Vec<ItemId>,
    #[serde(default)]
    pub last_draft: Option<Vec<ItemId>>,
    /// The chosen deck cards; empty until the player changes it. Reconciled with owned cards on
    /// use and shuffled every match, so the order carries no meaning.
    #[serde(default)]
    pub deck: Vec<SpellId>,
    /// The endless run in progress, if any.
    #[serde(default)]
    pub run: Option<Endless>,
    /// Highest stage ever cleared.
    #[serde(default)]
    pub best_stage: u32,
}

impl Profile {
    pub fn new(seed: u64) -> Self {
        Profile { rng: seed, owned: Vec::new(), last_draft: None, deck: Vec::new(), run: None, best_stage: 0 }
    }

    /// Start a new endless run at stage 1. Owned items and the deck carry over.
    pub fn new_run(&mut self, seed: u64) {
        self.run = Some(Endless { seed, stage: 1 });
        self.last_draft = None;
    }

    /// The stage to play next (stage 1 when no run is going).
    pub fn stage(&self) -> Stage {
        Stage::new(self.run.map_or(1, |r| r.stage))
    }

    /// Everything needed to play `stage`: a temporary RunState at `floor = stage.difficulty()`
    /// supplies relics, enhancements, enemy items, AI level and terrain; then the stage's armies,
    /// the player's shuffled deck and the stage's extra enemy spells go on top.
    pub fn stage_setup(&self, stage: &Stage, match_seed: u64) -> MatchSetup {
        let size = stage.size();
        let floor = stage.difficulty();
        let temp_run = RunState {
            seed: match_seed,
            size,
            floor,
            owned: self.owned.clone(),
            rng: match_seed,
            outcome: None,
            bonus_picks: 0,
            last_draft: None,
        };
        let mut setup = temp_run.match_setup();
        setup.player_deck = self.deck();
        deck::shuffle(&mut setup.player_deck, setup.seed);
        setup.enemy_deck = roll_enemy_deck(setup.seed, setup.enemy_items.len() + stage.extra_enemy_spells());
        setup.armies = stage.armies();
        if setup.armies.is_some() {
            let (terrain, actual_seed) = setup.terrain();
            let pos = setup.position(size).unwrap_or_else(|_| Position::start(size));
            let cartographer = setup.relics.contains(&RelicEffect::Cartographer);
            setup.pickups =
                place_pickups(actual_seed, floor, size, &terrain, setup.player, cartographer, &pos);
        }
        setup
    }

    /// Every card the player can put in the deck (owned spell charges, padded with filler).
    pub fn deck_pool(&self) -> Vec<SpellId> {
        deck::card_pool(&self.owned)
    }

    /// The 15-card deck in draw order: the saved arrangement reconciled with owned cards.
    pub fn deck(&self) -> Vec<SpellId> {
        deck::reconcile(&self.deck, &self.deck_pool())
    }

    /// Pool cards that aren't in the deck.
    pub fn deck_reserve(&self) -> Vec<SpellId> {
        deck::reserve(&self.deck(), &self.deck_pool())
    }

    /// Put reserve card `r` at deck position `i`; the card that was there joins the reserve.
    pub fn swap_with_reserve(&mut self, i: usize, r: usize) {
        let mut cards = self.deck();
        let reserve = deck::reserve(&cards, &self.deck_pool());
        if i >= cards.len() || r >= reserve.len() {
            return;
        }
        cards[i] = reserve[r];
        self.deck = cards;
    }

    /// Back to the default deck.
    pub fn reset_deck(&mut self) {
        self.deck.clear();
    }

    /// The current stage was won: record it, move the run on, and roll a draft of up to 3
    /// unowned items (empty once everything is owned).
    pub fn win_stage(&mut self) -> Vec<ItemId> {
        let stage = self.stage();
        self.best_stage = self.best_stage.max(stage.number);
        if let Some(run) = &mut self.run {
            run.stage += 1;
        }
        let draft = roll_draft(&mut self.rng, &self.owned, stage.difficulty(), 3);
        self.last_draft = Some(draft.clone());
        draft
    }

    /// The run is over; returns the stage it ended on. Items and the deck are kept.
    pub fn end_run(&mut self) -> u32 {
        let stage = self.stage().number;
        self.run = None;
        self.last_draft = None;
        stage
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
        let level = Stage::new(1);
        let profile = spell_profile();

        // Same seed => same order (deterministic).
        let first = profile.stage_setup(&level, 42).player_deck;
        let second = profile.stage_setup(&level, 42).player_deck;
        assert_eq!(first, second);
        assert_eq!(first.len(), 15);

        // Different seeds => different orders for most seed pairs.
        let mut differed = 0;
        for i in 0..20u64 {
            let a = profile.stage_setup(&level, i).player_deck;
            let b = profile.stage_setup(&level, 1000 + i).player_deck;
            if a != b {
                differed += 1;
            }
        }
        assert!(differed >= 15, "only {differed}/20 seed pairs differed");
    }
}
