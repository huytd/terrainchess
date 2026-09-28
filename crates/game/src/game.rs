//! Match state shared by the view, input and HUD.

use bevy::prelude::*;
use tc_core::worldgen::{GenParams, generate};
use tc_core::{Match, Move, Outcome, Position, Rules, Side, SpellCast, SpellId, Sq};
use tc_run::MatchSetup;

#[derive(Resource)]
pub struct GameState {
    pub size: u8,
    pub seed: u64,
    pub game: Match,
    /// Earlier states, for undo in hotseat play.
    pub undo: Vec<Match>,
    pub selected: Option<Sq>,
    pub outcome: Option<Outcome>,
    /// Set when the board layout changes (new game), so the view rebuilds terrain.
    pub terrain_dirty: bool,
    /// Set when pieces or selection change, so the view rebuilds pieces/overlays.
    pub pieces_dirty: bool,
    /// The move just played, to animate.
    pub animate: Option<Move>,
    /// Side the AI plays, or `None` for hotseat.
    pub ai_side: Option<Side>,
    /// AI strength as a run floor (0 = gentle, 7 = boss).
    pub ai_level: u8,
    /// Spell armed by clicking its card in the spell bar or pressing 5–9.
    pub armed_spell: Option<SpellId>,
    /// For Swap: first square selected.
    pub swap_first: Option<Sq>,
}

pub const MAX_AI_LEVEL: u8 = 7;

impl GameState {
    /// A new match configured from a roguelike run setup.
    pub fn from_setup(setup: &MatchSetup) -> Self {
        let (terrain, seed) = setup.terrain();
        let size = setup.r#gen.size;
        let mut game = Match::new(terrain, setup.rules.clone(), Position::start(size));
        game.set_pickups(setup.pickups.clone());
        game.set_charges(setup.player, setup.charges.clone());
        GameState {
            size,
            seed,
            game,
            undo: Vec::new(),
            selected: None,
            outcome: None,
            terrain_dirty: true,
            pieces_dirty: true,
            animate: None,
            ai_side: Some(Side::Black),
            ai_level: setup.ai_level,
            armed_spell: None,
            swap_first: None,
        }
    }

    pub fn new(size: u8, seed: u64, ai_side: Option<Side>, ai_level: u8) -> Self {
        let (terrain, seed) = generate(seed, &GenParams::for_floor(size, 2));
        let game = Match::new(terrain, Rules::standard(size), Position::start(size));
        GameState {
            size,
            seed,
            game,
            undo: Vec::new(),
            selected: None,
            outcome: None,
            terrain_dirty: true,
            pieces_dirty: true,
            animate: None,
            ai_side,
            ai_level,
            armed_spell: None,
            swap_first: None,
        }
    }

    /// A fresh board with the same opponent settings.
    #[allow(dead_code)]
    pub fn restart(&self, size: u8, seed: u64) -> Self {
        GameState::new(size, seed, self.ai_side, self.ai_level)
    }

    pub fn ai_to_move(&self) -> bool {
        self.outcome.is_none() && self.ai_side == Some(self.game.pos.side_to_move)
    }

    /// Returns the player's castable spells that have charges remaining.
    pub fn castable_spells(&self) -> Vec<(SpellId, u8)> {
        let side = Side::White;
        let mut list = Vec::new();
        for &(spell, count) in self.game.charges(side) {
            if spell.is_castable() && count > 0 && !list.iter().any(|(s, _)| *s == spell) {
                list.push((spell, count));
            }
        }
        list
    }

    pub fn arm_spell(&mut self, spell: SpellId) {
        self.armed_spell = Some(spell);
        self.swap_first = None;
        self.selected = None;
        self.pieces_dirty = true;
    }

    pub fn disarm(&mut self) {
        self.armed_spell = None;
        self.swap_first = None;
        self.pieces_dirty = true;
    }

    /// Target squares for the currently armed spell.
    pub fn cast_target_squares(&self) -> Vec<Sq> {
        let Some(spell) = self.armed_spell else {
            return Vec::new();
        };
        let targets = self.game.cast_targets(spell);
        match spell {
            SpellId::RaiseEarth => targets
                .into_iter()
                .filter_map(|c| match c {
                    SpellCast::RaiseEarth(sq) => Some(sq),
                    _ => None,
                })
                .collect(),
            SpellId::LowerEarth => targets
                .into_iter()
                .filter_map(|c| match c {
                    SpellCast::LowerEarth(sq) => Some(sq),
                    _ => None,
                })
                .collect(),
            SpellId::Freeze => targets
                .into_iter()
                .filter_map(|c| match c {
                    SpellCast::Freeze(sq) => Some(sq),
                    _ => None,
                })
                .collect(),
            SpellId::Shield => targets
                .into_iter()
                .filter_map(|c| match c {
                    SpellCast::Shield(sq) => Some(sq),
                    _ => None,
                })
                .collect(),
            SpellId::Swap => {
                if let Some(first) = self.swap_first {
                    let mut seconds = Vec::new();
                    for c in targets {
                        if let SpellCast::Swap(a, b) = c {
                            if a == first && !seconds.contains(&b) {
                                seconds.push(b);
                            } else if b == first && !seconds.contains(&a) {
                                seconds.push(a);
                            }
                        }
                    }
                    seconds
                } else {
                    let mut firsts = Vec::new();
                    for c in targets {
                        if let SpellCast::Swap(a, b) = c {
                            if !firsts.contains(&a) {
                                firsts.push(a);
                            }
                            if !firsts.contains(&b) {
                                firsts.push(b);
                            }
                        }
                    }
                    firsts
                }
            }
            _ => Vec::new(),
        }
    }

    pub fn cast(&mut self, cast: SpellCast) {
        let before = self.game.clone();
        let spell = cast.spell_id();
        if self.game.cast(cast).is_ok() {
            self.undo.push(before);
            self.outcome = self.game.outcome();
            if matches!(spell, SpellId::RaiseEarth | SpellId::LowerEarth | SpellId::Freeze) {
                self.terrain_dirty = true;
            }
        }
        self.disarm();
    }

    pub fn play(&mut self, mv: Move) {
        let before = self.game.clone();
        if self.game.play(mv).is_ok() {
            self.undo.push(before);
            self.outcome = self.game.outcome();
            self.animate = Some(mv);
        }
        self.disarm();
        self.selected = None;
        self.pieces_dirty = true;
    }

    /// Take back a move; against the AI, back to the player's last turn.
    pub fn undo(&mut self) {
        while let Some(prev) = self.undo.pop() {
            self.game = prev;
            if self.ai_side != Some(self.game.pos.side_to_move) {
                break;
            }
        }
        self.disarm();
        self.outcome = None;
        self.selected = None;
        self.animate = None;
        self.terrain_dirty = true;
        self.pieces_dirty = true;
    }

    /// Legal moves of the selected piece.
    pub fn selected_moves(&self) -> Vec<Move> {
        match self.selected {
            Some(sq) if self.outcome.is_none() && self.armed_spell.is_none() => {
                self.game.legal_moves().into_iter().filter(|m| m.from == sq).collect()
            }
            _ => Vec::new(),
        }
    }
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, _app: &mut App) {}
}
