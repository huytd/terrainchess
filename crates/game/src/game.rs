//! Match state shared by the view, input and HUD.

use bevy::prelude::*;
use tc_core::worldgen::{GenParams, generate};
use tc_core::{Match, Move, Outcome, Position, Rules, Side, SpellCast, SpellId, Sq};
use tc_run::MatchSetup;

#[derive(Clone, Debug, PartialEq)]
pub enum GameEvent {
    Moved { to: Sq, landed_height_change: bool },
    Captured { at: Sq, by_side: Side },
    Cast { spell: SpellId, squares: Vec<Sq> },
    Promoted { at: Sq },
    Check { king: Sq },
    Pickup { at: Sq },
    Selected { sq: Sq },
    Cleared { at: Sq },
}

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
    /// Events that occurred during the latest move or spell cast.
    pub events: Vec<GameEvent>,
    /// Enemy enhancement item IDs active in this match.
    pub enemy_items: Vec<String>,
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
        game.veteran = setup.veteran;
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
            events: Vec::new(),
            enemy_items: setup.enemy_items.clone(),
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
            events: Vec::new(),
            enemy_items: Vec::new(),
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
        if spell == SpellId::Rewind {
            self.cast(SpellCast::Rewind);
            return;
        }
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
            SpellId::Bridge => targets
                .into_iter()
                .filter_map(|c| match c {
                    SpellCast::Bridge(sq) => Some(sq),
                    _ => None,
                })
                .collect(),
            SpellId::DigTunnel => {
                if let Some(first) = self.swap_first {
                    let mut seconds = Vec::new();
                    for c in targets {
                        if let SpellCast::DigTunnel(a, b) = c {
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
                        if let SpellCast::DigTunnel(a, b) = c {
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
        let mut squares = match cast {
            SpellCast::RaiseEarth(sq)
            | SpellCast::LowerEarth(sq)
            | SpellCast::Shield(sq)
            | SpellCast::Bridge(sq) => vec![sq],
            SpellCast::Swap(a, b) | SpellCast::DigTunnel(a, b) => vec![a, b],
            SpellCast::Freeze(sq) => {
                let size = before.terrain.size;
                let mut frozen = Vec::new();
                for dy in -1..=1i8 {
                    for dx in -1..=1i8 {
                        if let Some(n) = sq.offset(dx, dy, size)
                            && before.terrain.get(n).is_water()
                        {
                            frozen.push(n);
                        }
                    }
                }
                frozen
            }
            SpellCast::Rewind => Vec::new(),
        };

        if self.game.cast(cast).is_ok() {
            if spell == SpellId::Rewind {
                for sq in tc_core::board::squares(self.size) {
                    if self.game.pos.get(sq).is_some() && self.game.pos.get(sq) != before.pos.get(sq) {
                        squares.push(sq);
                    }
                }
            }
            self.undo.push(before.clone());
            self.outcome = self.game.outcome();
            if matches!(
                spell,
                SpellId::RaiseEarth
                    | SpellId::LowerEarth
                    | SpellId::Freeze
                    | SpellId::Bridge
                    | SpellId::DigTunnel
                    | SpellId::Rewind
            ) || before.terrain != self.game.terrain
            {
                self.terrain_dirty = true;
            }
            self.pieces_dirty = true;
            self.events.push(GameEvent::Cast { spell, squares });
            if self.game.in_check()
                && let Some(king) = self.game.pos.king(self.game.pos.side_to_move)
            {
                self.events.push(GameEvent::Check { king });
            }
        }
        self.disarm();
    }

    pub fn play(&mut self, mv: Move) {
        let before = self.game.clone();
        let moving_side = before.pos.side_to_move;
        let from_height = before.terrain.height(mv.from);
        let capture_info = if let Some(target) = before.pos.get(mv.to) {
            if target.side != moving_side { Some((mv.to, moving_side)) } else { None }
        } else if mv.kind == tc_core::MoveKind::EnPassant {
            Some((Sq::new(mv.to.x, mv.from.y), moving_side))
        } else {
            None
        };
        let pickup_at = before.pickups.iter().find(|(sq, _)| *sq == mv.to).map(|(sq, _)| *sq);

        if self.game.play(mv).is_ok() {
            self.undo.push(before.clone());
            self.outcome = self.game.outcome();
            self.animate = if mv.kind == tc_core::MoveKind::Clear { None } else { Some(mv) };

            if before.terrain != self.game.terrain || mv.kind == tc_core::MoveKind::Clear {
                self.terrain_dirty = true;
            }

            if mv.kind == tc_core::MoveKind::Clear {
                self.events.push(GameEvent::Cleared { at: mv.to });
            } else {
                if let Some((at, by_side)) = capture_info {
                    self.events.push(GameEvent::Captured { at, by_side });
                }

                let to_height = self.game.terrain.height(mv.to);
                let landed_height_change = from_height != to_height;
                self.events.push(GameEvent::Moved { to: mv.to, landed_height_change });

                if let Some(at) = pickup_at {
                    self.events.push(GameEvent::Pickup { at });
                }

                if mv.promotion.is_some() {
                    self.events.push(GameEvent::Promoted { at: mv.to });
                }
            }

            if self.game.in_check()
                && let Some(king) = self.game.pos.king(self.game.pos.side_to_move)
            {
                self.events.push(GameEvent::Check { king });
            }
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
        self.events.clear();
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
