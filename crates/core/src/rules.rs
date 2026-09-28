//! Match state: rules configuration, legal play, and end-of-game detection.

use serde::{Deserialize, Serialize};

use crate::board::{Sq, squares};
use crate::movegen::{Ctx, Move};
use crate::piece::{MoveProfile, Piece, PieceKind, Side};
use crate::position::Position;
use crate::spell::{SpellCast, SpellId};
use crate::terrain::{Feature, MAX_HEIGHT, Terrain, TileKind};

/// Pickups that appear on the board and can be collected by moving pieces.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Pickup {
    SpellCharge(SpellId),
    RunItem,
}

/// A timed effect on a terrain tile that reverts after a number of plies.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TimedEffect {
    pub sq: Sq,
    pub original_kind: TileKind,
    pub remaining_plies: u8,
}

/// Per-side movement profiles. Run upgrades edit these numbers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rules {
    pub profiles: [[MoveProfile; 6]; 2],
    /// Pawns may double-step while on this relative rank or lower.
    pub double_step_max_rank: u8,
}

impl Rules {
    pub fn standard(size: u8) -> Self {
        Rules {
            profiles: [[MoveProfile::standard(); 6]; 2],
            // 16×16 and up: until the pawn passes its 4th rank.
            double_step_max_rank: if size <= 8 { 1 } else { 3 },
        }
    }

    pub fn profile(&self, piece: Piece) -> &MoveProfile {
        &self.profiles[piece.side.index()][piece.kind.index()]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawReason {
    Stalemate,
    FiftyMoves,
    Repetition,
    InsufficientMaterial,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Checkmate { winner: Side },
    Draw(DrawReason),
}

/// One game on one board.
#[derive(Clone, Debug)]
pub struct Match {
    pub terrain: Terrain,
    pub rules: Rules,
    pub pos: Position,
    /// Hashes of every position so far, for repetition.
    history: Vec<u64>,
    pub moves: Vec<Move>,
    pub charges: [Vec<(SpellId, u8)>; 2],
    pub timed_effects: Vec<TimedEffect>,
    pub pickups: Vec<(Sq, Pickup)>,
    pub run_items_collected: [u8; 2],
}

impl Match {
    pub fn new(terrain: Terrain, rules: Rules, pos: Position) -> Self {
        assert_eq!(terrain.size, pos.size);
        let history = vec![pos.hash()];
        Match {
            terrain,
            rules,
            pos,
            history,
            moves: Vec::new(),
            charges: [Vec::new(), Vec::new()],
            timed_effects: Vec::new(),
            pickups: Vec::new(),
            run_items_collected: [0, 0],
        }
    }

    pub fn set_charges(&mut self, side: Side, charges: Vec<(SpellId, u8)>) {
        self.charges[side.index()] = charges;
    }

    pub fn charges(&self, side: Side) -> &[(SpellId, u8)] {
        &self.charges[side.index()]
    }

    pub fn charge_count(&self, side: Side, spell: SpellId) -> u8 {
        self.charges[side.index()].iter().filter(|(s, _)| *s == spell).map(|(_, c)| *c).sum()
    }

    pub fn set_pickups(&mut self, pickups: Vec<(Sq, Pickup)>) {
        self.pickups = pickups;
    }

    pub fn ctx(&self) -> Ctx<'_> {
        Ctx { terrain: &self.terrain, rules: &self.rules }
    }

    pub fn legal_moves(&self) -> Vec<Move> {
        self.ctx().legal_moves(&self.pos)
    }

    /// Hashes of every position so far, oldest first (the current one last).
    pub fn history(&self) -> &[u64] {
        &self.history
    }

    pub fn in_check(&self) -> bool {
        self.ctx().in_check(&self.pos, self.pos.side_to_move)
    }

    /// Ticks down timed effects by one ply, reverting expired effects.
    pub fn tick_timed_effects(&mut self) {
        for effect in &mut self.timed_effects {
            effect.remaining_plies = effect.remaining_plies.saturating_sub(1);
            if effect.remaining_plies == 0 {
                self.terrain.get_mut(effect.sq).kind = effect.original_kind;
            }
        }
        self.timed_effects.retain(|e| e.remaining_plies > 0);
    }

    /// Play a move if it is legal.
    pub fn play(&mut self, mv: Move) -> Result<(), String> {
        if !self.legal_moves().contains(&mv) {
            return Err(format!("illegal move {}", mv.uci()));
        }
        let side = self.pos.side_to_move;
        self.pos.make_move(mv);

        // Collect pickup if the move ended on a pickup square
        if let Some(idx) = self.pickups.iter().position(|(sq, _)| *sq == mv.to) {
            let (_, pickup) = self.pickups.swap_remove(idx);
            match pickup {
                Pickup::SpellCharge(spell) => {
                    if let Some(entry) = self.charges[side.index()].iter_mut().find(|(s, _)| *s == spell) {
                        entry.1 = entry.1.saturating_add(1);
                    } else {
                        self.charges[side.index()].push((spell, 1));
                    }
                }
                Pickup::RunItem => {
                    self.run_items_collected[side.index()] =
                        self.run_items_collected[side.index()].saturating_add(1);
                }
            }
        }

        self.tick_timed_effects();
        self.history.push(self.pos.hash());
        self.moves.push(mv);
        Ok(())
    }

    /// Every legal cast for the side to move, if it has a charge.
    pub fn cast_targets(&self, spell: SpellId) -> Vec<SpellCast> {
        let side = self.pos.side_to_move;
        if self.charge_count(side, spell) == 0 || !spell.is_castable() {
            return Vec::new();
        }

        let mut out = Vec::new();
        let size = self.terrain.size;

        match spell {
            SpellId::RaiseEarth => {
                for sq in squares(size) {
                    let tile = self.terrain.get(sq);
                    if tile.kind == TileKind::Void
                        || matches!(tile.feature, Feature::Obstacle(_))
                        || self.pos.get(sq).is_some_and(|p| p.kind == PieceKind::King)
                    {
                        continue;
                    }
                    let mut clone = self.clone();
                    let t = clone.terrain.get_mut(sq);
                    t.height = (t.height + 1).min(MAX_HEIGHT);
                    if !clone.in_check() {
                        out.push(SpellCast::RaiseEarth(sq));
                    }
                }
            }
            SpellId::LowerEarth => {
                for sq in squares(size) {
                    let tile = self.terrain.get(sq);
                    if tile.kind == TileKind::Void
                        || matches!(tile.feature, Feature::Obstacle(_))
                        || self.pos.get(sq).is_some_and(|p| p.kind == PieceKind::King)
                    {
                        continue;
                    }
                    let mut clone = self.clone();
                    let t = clone.terrain.get_mut(sq);
                    t.height = t.height.saturating_sub(1);
                    if !clone.in_check() {
                        out.push(SpellCast::LowerEarth(sq));
                    }
                }
            }
            SpellId::Freeze => {
                for sq in squares(size) {
                    let mut has_water = false;
                    for dy in -1..=1i8 {
                        for dx in -1..=1i8 {
                            if let Some(n) = sq.offset(dx, dy, size) {
                                let t = self.terrain.get(n);
                                if t.kind == TileKind::ShallowWater || t.kind == TileKind::DeepWater {
                                    has_water = true;
                                    break;
                                }
                            }
                        }
                        if has_water {
                            break;
                        }
                    }
                    if !has_water {
                        continue;
                    }
                    let mut clone = self.clone();
                    for dy in -1..=1i8 {
                        for dx in -1..=1i8 {
                            if let Some(n) = sq.offset(dx, dy, size) {
                                let t = clone.terrain.get_mut(n);
                                if t.kind == TileKind::ShallowWater || t.kind == TileKind::DeepWater {
                                    t.kind = TileKind::Ice;
                                }
                            }
                        }
                    }
                    if !clone.in_check() {
                        out.push(SpellCast::Freeze(sq));
                    }
                }
            }
            SpellId::Shield => {
                for sq in squares(size) {
                    if self.pos.get(sq).is_some_and(|p| p.side == side) {
                        out.push(SpellCast::Shield(sq));
                    }
                }
            }
            SpellId::Swap => {
                let friendly_sqs: Vec<Sq> =
                    squares(size).filter(|&sq| self.pos.get(sq).is_some_and(|p| p.side == side)).collect();
                for i in 0..friendly_sqs.len() {
                    for j in (i + 1)..friendly_sqs.len() {
                        let a = friendly_sqs[i];
                        let b = friendly_sqs[j];
                        let mut clone = self.clone();
                        let p_a = clone.pos.get(a);
                        let p_b = clone.pos.get(b);
                        clone.pos.set(a, p_b);
                        clone.pos.set(b, p_a);
                        if !clone.in_check() {
                            out.push(SpellCast::Swap(a, b));
                        }
                    }
                }
            }
            SpellId::Bridge | SpellId::DigTunnel | SpellId::Rewind => {}
        }

        out
    }

    /// Cast a spell: checks legality, spends one charge, applies the effect,
    /// and ends the turn unless the spell is quick.
    pub fn cast(&mut self, cast: SpellCast) -> Result<(), String> {
        let spell = cast.spell_id();
        let side = self.pos.side_to_move;
        if self.charge_count(side, spell) == 0 {
            return Err(format!("no charges remaining for spell {:?}", spell));
        }
        let canonical_cast = match cast {
            SpellCast::Swap(a, b) if a > b => SpellCast::Swap(b, a),
            other => other,
        };
        if !self.cast_targets(spell).contains(&canonical_cast) {
            return Err(format!("illegal cast {:?}", cast));
        }

        // Spend one charge
        let entry = self.charges[side.index()]
            .iter_mut()
            .find(|(s, c)| *s == spell && *c > 0)
            .expect("charge count was verified above");
        entry.1 -= 1;

        // Apply effect
        match cast {
            SpellCast::RaiseEarth(sq) => {
                let tile = self.terrain.get_mut(sq);
                tile.height = (tile.height + 1).min(MAX_HEIGHT);
            }
            SpellCast::LowerEarth(sq) => {
                let tile = self.terrain.get_mut(sq);
                tile.height = tile.height.saturating_sub(1);
            }
            SpellCast::Freeze(sq) => {
                let size = self.terrain.size;
                for dy in -1..=1i8 {
                    for dx in -1..=1i8 {
                        if let Some(n) = sq.offset(dx, dy, size) {
                            let tile = self.terrain.get_mut(n);
                            if tile.kind == TileKind::ShallowWater || tile.kind == TileKind::DeepWater {
                                self.timed_effects.push(TimedEffect {
                                    sq: n,
                                    original_kind: tile.kind,
                                    remaining_plies: 6,
                                });
                                tile.kind = TileKind::Ice;
                            }
                        }
                    }
                }
            }
            SpellCast::Shield(sq) => {
                self.pos.shield = Some((sq, side));
            }
            SpellCast::Swap(a, b) => {
                let p_a = self.pos.get(a);
                let p_b = self.pos.get(b);
                self.pos.set(a, p_b);
                self.pos.set(b, p_a);
                if p_a.is_some_and(|p| p.kind == PieceKind::King)
                    || p_b.is_some_and(|p| p.kind == PieceKind::King)
                {
                    self.pos.castling[side.index()] = [None; 2];
                }
                for rights in &mut self.pos.castling {
                    for r in rights.iter_mut() {
                        if *r == Some(a) || *r == Some(b) {
                            *r = None;
                        }
                    }
                }
            }
        }

        if !cast.is_quick() {
            // End turn like a move
            self.pos.en_passant = None;
            self.pos.halfmove_clock += 1;
            if side == Side::Black {
                self.pos.fullmove += 1;
            }
            self.pos.side_to_move = side.opposite();
            if self.pos.shield.is_some_and(|(_, s)| s == self.pos.side_to_move) {
                self.pos.shield = None;
            }
            // Existing timed effects tick down once per ply
            for effect in &mut self.timed_effects {
                if !matches!(cast, SpellCast::Freeze(_)) || effect.remaining_plies < 6 {
                    effect.remaining_plies = effect.remaining_plies.saturating_sub(1);
                    if effect.remaining_plies == 0 {
                        self.terrain.get_mut(effect.sq).kind = effect.original_kind;
                    }
                }
            }
            self.timed_effects.retain(|e| e.remaining_plies > 0);

            self.history.push(self.pos.hash());
        }

        Ok(())
    }

    pub fn outcome(&self) -> Option<Outcome> {
        if self.legal_moves().is_empty() {
            return Some(if self.in_check() {
                Outcome::Checkmate { winner: self.pos.side_to_move.opposite() }
            } else {
                Outcome::Draw(DrawReason::Stalemate)
            });
        }
        if self.pos.halfmove_clock >= 100 {
            return Some(Outcome::Draw(DrawReason::FiftyMoves));
        }
        let now = self.pos.hash();
        if self.history.iter().filter(|&&h| h == now).count() >= 3 {
            return Some(Outcome::Draw(DrawReason::Repetition));
        }
        if insufficient_material(&self.pos) {
            return Some(Outcome::Draw(DrawReason::InsufficientMaterial));
        }
        None
    }
}

/// King vs king, or king vs king and a single minor piece.
fn insufficient_material(pos: &Position) -> bool {
    let mut minors = 0;
    for (_, p) in pos.pieces() {
        match p.kind {
            PieceKind::King => {}
            PieceKind::Knight | PieceKind::Bishop => minors += 1,
            _ => return false,
        }
    }
    minors <= 1
}
