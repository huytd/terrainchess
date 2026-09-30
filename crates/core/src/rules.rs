//! Match state: rules configuration, legal play, and end-of-game detection.

use serde::{Deserialize, Serialize};

use crate::board::{Sq, squares};
use crate::movegen::{Ctx, Move, MoveKind};
use crate::piece::{MoveProfile, Piece, PieceKind, Side};
use crate::position::{Position, TimedCurse};
use crate::spell::{SpellCast, SpellHand, SpellId};
use crate::terrain::{Feature, MAX_HEIGHT, Obstacle, Terrain, TileKind};

/// Pickups that appear on the board and can be collected by moving pieces.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Pickup {
    RunItem,
}

/// A timed effect on a terrain tile that reverts after a number of plies.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TimedEffect {
    pub sq: Sq,
    pub original_kind: TileKind,
    pub remaining_plies: u8,
}

/// A timed cave link that reverts after a number of plies.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TimedCave {
    pub a: Sq,
    pub b: Sq,
    pub link: u8,
    pub remaining_plies: u8,
}

/// Plies a Curse lasts: the victim side's next 2 turns (one ply per move).
pub const CURSE_PLIES: u8 = 4;
/// Fallback ply timer for Featherfall (also expires on its side's next move).
pub const FEATHERFALL_PLIES: u8 = 2;

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

    pub fn profile_mut(&mut self, piece: Piece) -> &mut MoveProfile {
        &mut self.profiles[piece.side.index()][piece.kind.index()]
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
    pub hands: [SpellHand; 2],
    pub timed_effects: Vec<TimedEffect>,
    pub timed_caves: Vec<TimedCave>,
    pub pickups: Vec<(Sq, Pickup)>,
    pub run_items_collected: [u8; 2],
    pub veteran: [bool; 2],
    pub snapshots: Vec<Match>,
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
            hands: [SpellHand::default(), SpellHand::default()],
            timed_effects: Vec::new(),
            timed_caves: Vec::new(),
            pickups: Vec::new(),
            run_items_collected: [0, 0],
            veteran: [false, false],
            snapshots: Vec::new(),
        }
    }

    pub fn set_veteran(&mut self, side: Side, active: bool) {
        self.veteran[side.index()] = active;
    }

    pub fn set_deck(&mut self, side: Side, cards: Vec<SpellId>) {
        self.hands[side.index()] = SpellHand::new(cards);
    }

    pub fn hand(&self, side: Side) -> &SpellHand {
        &self.hands[side.index()]
    }

    pub fn hand_mut(&mut self, side: Side) -> &mut SpellHand {
        &mut self.hands[side.index()]
    }

    pub fn deck_len(&self, side: Side) -> usize {
        self.hands[side.index()].deck.len()
    }

    pub fn discards_left(&self, side: Side) -> u8 {
        self.hands[side.index()].discards_left
    }

    pub fn can_discard(&self, side: Side, slot: usize) -> bool {
        if slot >= 3 {
            return false;
        }
        let hand = &self.hands[side.index()];
        hand.discards_left > 0 && !hand.deck.is_empty() && hand.hand[slot].is_some() && !hand.used[slot]
    }

    pub fn discard(&mut self, side: Side, slot: usize) -> Result<(), String> {
        if slot >= 3 {
            return Err("slot index out of bounds".to_string());
        }
        let hand = &self.hands[side.index()];
        if hand.discards_left == 0 {
            return Err("no discards remaining this match".to_string());
        }
        if hand.deck.is_empty() {
            return Err("cannot discard when deck is empty".to_string());
        }
        if hand.hand[slot].is_none() {
            return Err("cannot discard an empty slot".to_string());
        }
        if hand.used[slot] {
            return Err("cannot discard a card that has already been used".to_string());
        }

        let hand = &mut self.hands[side.index()];
        hand.discards_left -= 1;
        let card = hand.hand[slot].take().unwrap();
        hand.discarded.push(card);
        let new_card = hand.deck.remove(0);
        hand.hand[slot] = Some(new_card);
        hand.used[slot] = false;
        Ok(())
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

    /// Every capture the side opposing `victim_side` could make right now onto a
    /// `victim_side` piece, as (attacker, victim) square pairs. Runs pseudo-legal
    /// generation on a hypothetical position with the attacker to move, so it respects
    /// the real terrain rules (high ground, shields, cliffs, caves) without needing to
    /// check whether the attacker's own king would end up in check. Used to draw threat
    /// arrows, not to decide legality.
    pub fn threats(&self, victim_side: Side) -> Vec<(Sq, Sq)> {
        let attacker_side = victim_side.opposite();
        let mut pos = self.pos.clone();
        if pos.side_to_move != attacker_side {
            // An en-passant square set for the other side to move doesn't carry over.
            pos.en_passant = None;
        }
        pos.side_to_move = attacker_side;
        let mut moves = Vec::with_capacity(64);
        self.ctx().pseudo_legal(&pos, &mut moves);
        let mut pairs: Vec<(Sq, Sq)> = Vec::new();
        for mv in moves {
            let victim_sq = match mv.kind {
                MoveKind::EnPassant => Sq::new(mv.to.x, mv.from.y),
                _ => mv.to,
            };
            if pos.get(victim_sq).is_some_and(|p| p.side == victim_side)
                && !pairs.contains(&(mv.from, victim_sq))
            {
                pairs.push((mv.from, victim_sq));
            }
        }
        pairs
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

        for cave in &mut self.timed_caves {
            cave.remaining_plies = cave.remaining_plies.saturating_sub(1);
            if cave.remaining_plies == 0 {
                if self.terrain.get(cave.a).feature == Feature::Cave(cave.link) {
                    self.terrain.get_mut(cave.a).feature = Feature::None;
                }
                if self.terrain.get(cave.b).feature == Feature::Cave(cave.link) {
                    self.terrain.get_mut(cave.b).feature = Feature::None;
                }
            }
        }
        self.timed_caves.retain(|c| c.remaining_plies > 0);

        if let Some(f) = &mut self.pos.featherfall {
            f.remaining_plies = f.remaining_plies.saturating_sub(1);
            if f.remaining_plies == 0 {
                self.pos.featherfall = None;
            }
        }
        for curse in &mut self.pos.curses {
            curse.remaining_plies = curse.remaining_plies.saturating_sub(1);
        }
        self.pos.curses.retain(|c| c.remaining_plies > 0);
    }

    /// Play a move if it is legal.
    pub fn play(&mut self, mv: Move) -> Result<(), String> {
        if !self.legal_moves().contains(&mv) {
            return Err(format!("illegal move {}", mv.uci()));
        }
        let side = self.pos.side_to_move;

        // Record snapshot before the move (capped at 4)
        let mut snap = self.clone();
        snap.snapshots.clear();
        if self.snapshots.len() == 4 {
            self.snapshots.remove(0);
        }
        self.snapshots.push(snap);

        let moving_side = side;
        let victim_side = moving_side.opposite();
        let victim_info = match mv.kind {
            MoveKind::EnPassant => {
                let victim_sq = Sq::new(mv.to.x, mv.from.y);
                self.pos.get(victim_sq).map(|p| (victim_sq, p))
            }
            MoveKind::Clear => None,
            _ => self.pos.get(mv.to).map(|p| (mv.to, p)),
        };

        let mut veteran_pushed = None;
        if let Some((victim_sq, victim_piece)) = victim_info
            && victim_piece.side == victim_side
            && victim_piece.kind != PieceKind::King
            && self.veteran[victim_side.index()]
        {
            self.veteran[victim_side.index()] = false;
            let push_dy = -victim_side.forward();
            if let Some(push_sq) = victim_sq.offset(0, push_dy, self.terrain.size) {
                let is_empty = push_sq == mv.from || self.pos.get(push_sq).is_none();
                let tile = self.terrain.get(push_sq);
                let prof = self.rules.profile(victim_piece);
                let is_standable =
                    !tile.is_blocked() && (prof.deep_water || tile.kind != TileKind::DeepWater);
                if is_empty && is_standable {
                    veteran_pushed = Some((push_sq, victim_piece));
                }
            }
        }

        if mv.kind == MoveKind::Clear {
            self.terrain.get_mut(mv.to).feature = Feature::None;
        }

        self.pos.make_move(mv);
        if let Some((push_sq, victim_piece)) = veteran_pushed {
            self.pos.set(push_sq, Some(victim_piece));
        }

        // Collect pickup if the move ended on a pickup square
        if mv.kind != MoveKind::Clear
            && let Some(idx) = self.pickups.iter().position(|(sq, _)| *sq == mv.to)
        {
            let (_, pickup) = self.pickups.swap_remove(idx);
            match pickup {
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

    /// Every legal cast for the side to move, if it has an unused card in hand.
    pub fn cast_targets(&self, spell: SpellId) -> Vec<SpellCast> {
        let side = self.pos.side_to_move;
        let hand = &self.hands[side.index()];
        let has_unused = (0..3).any(|i| hand.hand[i] == Some(spell) && !hand.used[i]);
        if !has_unused || !spell.is_castable() {
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
            SpellId::Bridge => {
                for sq in squares(size) {
                    let tile = self.terrain.get(sq);
                    if !matches!(tile.kind, TileKind::ShallowWater | TileKind::DeepWater | TileKind::Void) {
                        continue;
                    }
                    let is_adjacent = crate::board::ORTHO.iter().any(|&(dx, dy)| {
                        sq.offset(dx, dy, size)
                            .is_some_and(|adj| self.pos.get(adj).is_some_and(|p| p.side == side))
                    });
                    if !is_adjacent {
                        continue;
                    }
                    let mut clone = self.clone();
                    clone.terrain.get_mut(sq).kind = TileKind::Bridge;
                    if !clone.in_check() {
                        out.push(SpellCast::Bridge(sq));
                    }
                }
            }
            SpellId::DigTunnel => {
                let friendly_sqs: Vec<Sq> =
                    squares(size).filter(|&s| self.pos.get(s).is_some_and(|p| p.side == side)).collect();
                let candidates: Vec<Sq> = squares(size)
                    .filter(|&sq| {
                        if self.pos.get(sq).is_some() {
                            return false;
                        }
                        let tile = self.terrain.get(sq);
                        if tile.is_blocked()
                            || tile.kind == TileKind::DeepWater
                            || tile.feature != Feature::None
                        {
                            return false;
                        }
                        friendly_sqs.iter().any(|&p_sq| {
                            (sq.x as i16 - p_sq.x as i16).abs().max((sq.y as i16 - p_sq.y as i16).abs()) <= 2
                        })
                    })
                    .collect();

                for i in 0..candidates.len() {
                    for j in (i + 1)..candidates.len() {
                        let a = candidates[i];
                        let b = candidates[j];
                        let dist = (a.x as i16 - b.x as i16).abs().max((a.y as i16 - b.y as i16).abs());
                        if dist < 3 {
                            continue;
                        }
                        let link = (0..=u8::MAX)
                            .find(|&cand| {
                                !squares(size).any(|s| {
                                    matches!(self.terrain.get(s).feature, Feature::Cave(id) if id == cand)
                                })
                            })
                            .unwrap_or(255);
                        let mut clone = self.clone();
                        clone.terrain.get_mut(a).feature = Feature::Cave(link);
                        clone.terrain.get_mut(b).feature = Feature::Cave(link);
                        if !clone.in_check() {
                            out.push(SpellCast::DigTunnel(a, b));
                        }
                    }
                }
            }
            SpellId::Rewind => {
                if self.snapshots.len() >= 2 && self.moves.len() >= 2 {
                    out.push(SpellCast::Rewind);
                }
            }
            SpellId::Smite => {
                for sq in squares(size) {
                    if !matches!(
                        self.terrain.get(sq).feature,
                        Feature::Obstacle(Obstacle::Rock) | Feature::Obstacle(Obstacle::Tree)
                    ) {
                        continue;
                    }
                    let mut clone = self.clone();
                    clone.terrain.get_mut(sq).feature = Feature::None;
                    if !clone.in_check() {
                        out.push(SpellCast::Smite(sq));
                    }
                }
            }
            SpellId::Evaporate => {
                for sq in squares(size) {
                    let tile = self.terrain.get(sq);
                    if !matches!(tile.kind, TileKind::ShallowWater | TileKind::Ice) {
                        continue;
                    }
                    if tile.feature != Feature::None || self.pos.get(sq).is_some() {
                        continue;
                    }
                    let mut clone = self.clone();
                    clone.terrain.get_mut(sq).kind = TileKind::Sand;
                    if !clone.in_check() {
                        out.push(SpellCast::Evaporate(sq));
                    }
                }
            }
            SpellId::Flood => {
                let home = Position::home_rows(size);
                for sq in squares(size) {
                    let tile = self.terrain.get(sq);
                    if !matches!(tile.kind, TileKind::Grass | TileKind::Sand) {
                        continue;
                    }
                    if tile.feature != Feature::None || self.pos.get(sq).is_some() {
                        continue;
                    }
                    if sq.y < home || sq.y >= size - home {
                        continue;
                    }
                    let mut clone = self.clone();
                    clone.terrain.get_mut(sq).kind = TileKind::ShallowWater;
                    if !clone.in_check() {
                        out.push(SpellCast::Flood(sq));
                    }
                }
            }
            SpellId::Featherfall => {
                for sq in squares(size) {
                    if self.pos.get(sq).is_some_and(|p| p.side == side) {
                        out.push(SpellCast::Featherfall(sq));
                    }
                }
            }
            SpellId::Curse => {
                for sq in squares(size) {
                    if self.pos.get(sq).is_some_and(|p| p.side != side && p.kind != PieceKind::King)
                        && !self.pos.curses.iter().any(|c| c.sq == sq)
                    {
                        out.push(SpellCast::Curse(sq));
                    }
                }
            }
            SpellId::Sprout => {
                let home = Position::home_rows(size);
                for sq in squares(size) {
                    let tile = self.terrain.get(sq);
                    if tile.kind != TileKind::Grass || tile.feature != Feature::None {
                        continue;
                    }
                    if self.pos.get(sq).is_some() {
                        continue;
                    }
                    if sq.y < home || sq.y >= size - home {
                        continue;
                    }
                    let mut clone = self.clone();
                    clone.terrain.get_mut(sq).feature = Feature::Obstacle(Obstacle::Tree);
                    if !clone.in_check() {
                        out.push(SpellCast::Sprout(sq));
                    }
                }
            }
            SpellId::Blink => {
                for (from, piece) in
                    self.pos.pieces().filter(|(_, p)| p.side == side && p.kind != PieceKind::King)
                {
                    let prof = self.rules.profile(piece);
                    for dy in -2..=2i8 {
                        for dx in -2..=2i8 {
                            if dx == 0 && dy == 0 {
                                continue;
                            }
                            let Some(to) = from.offset(dx, dy, size) else { continue };
                            if self.pos.get(to).is_some() {
                                continue;
                            }
                            let tile = self.terrain.get(to);
                            if tile.is_blocked() {
                                continue;
                            }
                            if !prof.deep_water && tile.kind == TileKind::DeepWater {
                                continue;
                            }
                            if self.pos.shield.is_some_and(|(s, sd)| s == to && sd != side) {
                                continue;
                            }
                            let mut clone = self.clone();
                            clone.pos.set(from, None);
                            clone.pos.set(to, Some(piece));
                            if !clone.in_check() {
                                out.push(SpellCast::Blink(from, to));
                            }
                        }
                    }
                }
            }
            SpellId::Insight => {
                if !self.hands[side.index()].deck.is_empty() {
                    out.push(SpellCast::Insight);
                }
            }
        }

        out
    }

    /// Cast a spell: checks legality, spends one card from an unused hand slot,
    /// applies the effect, and ends the turn unless the spell is quick.
    pub fn cast(&mut self, cast: SpellCast) -> Result<(), String> {
        let spell = cast.spell_id();
        let side = self.pos.side_to_move;
        let hand = &self.hands[side.index()];
        let has_unused = (0..3).any(|i| hand.hand[i] == Some(spell) && !hand.used[i]);
        if !has_unused {
            return Err(format!("spell {:?} is not in an unused hand slot", spell));
        }
        let canonical_cast = match cast {
            SpellCast::Swap(a, b) if a > b => SpellCast::Swap(b, a),
            SpellCast::DigTunnel(a, b) if a > b => SpellCast::DigTunnel(b, a),
            other => other,
        };
        if !self.cast_targets(spell).contains(&canonical_cast) {
            return Err(format!("illegal cast {:?}", cast));
        }

        // Record snapshot before turn-ending cast
        if !cast.is_quick() && !matches!(cast, SpellCast::Rewind) {
            let mut snap = self.clone();
            snap.snapshots.clear();
            if self.snapshots.len() == 4 {
                self.snapshots.remove(0);
            }
            self.snapshots.push(snap);
        }

        // Mark that slot used (the card is spent)
        let hand = &mut self.hands[side.index()];
        let slot_idx =
            (0..3).find(|&i| hand.hand[i] == Some(spell) && !hand.used[i]).expect("slot was verified above");
        hand.used[slot_idx] = true;

        // Drawing: as soon as every non-empty slot of the hand is used, the hand is cleared
        // and up to 3 new cards are drawn from the front of the deck.
        let all_used = (0..3).all(|i| hand.hand[i].is_none() || hand.used[i]);
        if all_used {
            hand.draw();
        }

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
            SpellCast::Bridge(sq) => {
                let tile = self.terrain.get_mut(sq);
                tile.kind = TileKind::Bridge;
            }
            SpellCast::DigTunnel(a, b) => {
                let link = (0..=u8::MAX)
                    .find(|&cand| {
                        !squares(self.terrain.size)
                            .any(|s| matches!(self.terrain.get(s).feature, Feature::Cave(id) if id == cand))
                    })
                    .unwrap_or(255);
                self.terrain.get_mut(a).feature = Feature::Cave(link);
                self.terrain.get_mut(b).feature = Feature::Cave(link);
                self.timed_caves.push(TimedCave { a, b, link, remaining_plies: 8 });
            }
            SpellCast::Shield(sq) => {
                self.pos.shield = Some((sq, side));
            }
            SpellCast::Swap(a, b) => {
                let p_a = self.pos.get(a);
                let p_b = self.pos.get(b);
                self.pos.set(a, p_b);
                self.pos.set(b, p_a);
                // Piece-bound effects follow the swapped pieces.
                for c in &mut self.pos.curses {
                    if c.sq == a {
                        c.sq = b;
                    } else if c.sq == b {
                        c.sq = a;
                    }
                }
                if let Some(mut f) = self.pos.featherfall {
                    if f.sq == a {
                        f.sq = b;
                    } else if f.sq == b {
                        f.sq = a;
                    }
                    self.pos.featherfall = Some(f);
                }
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
            SpellCast::Rewind => {
                if self.snapshots.len() < 2 || self.moves.len() < 2 {
                    return Err("fewer than two plies of history to rewind".to_string());
                }
                let target_idx = self.snapshots.len() - 2;
                let mut target = self.snapshots[target_idx].clone();
                let remaining_snapshots: Vec<Match> = self.snapshots[..target_idx].to_vec();
                target.snapshots = remaining_snapshots;
                let updated_hand = self.hands[side.index()].clone();
                target.hands[side.index()] = updated_hand;
                *self = target;
            }
            SpellCast::Smite(sq) => {
                self.terrain.get_mut(sq).feature = Feature::None;
            }
            SpellCast::Evaporate(sq) => {
                self.terrain.get_mut(sq).kind = TileKind::Sand;
            }
            SpellCast::Flood(sq) => {
                self.terrain.get_mut(sq).kind = TileKind::ShallowWater;
            }
            SpellCast::Featherfall(sq) => {
                self.pos.featherfall = Some(TimedCurse { sq, side, remaining_plies: FEATHERFALL_PLIES });
            }
            SpellCast::Curse(sq) => {
                // Refresh any existing curse on that square.
                self.pos.curses.retain(|c| c.sq != sq);
                self.pos.curses.push(TimedCurse { sq, side: side.opposite(), remaining_plies: CURSE_PLIES });
            }
            SpellCast::Sprout(sq) => {
                self.terrain.get_mut(sq).feature = Feature::Obstacle(Obstacle::Tree);
            }
            SpellCast::Blink(from, to) => {
                let piece = self.pos.get(from);
                self.pos.set(from, None);
                self.pos.set(to, piece);
                for rights in &mut self.pos.castling {
                    for r in rights.iter_mut() {
                        if *r == Some(from) {
                            *r = None;
                        }
                    }
                }
                // Piece-bound effects follow the teleported piece.
                for c in &mut self.pos.curses {
                    if c.sq == from {
                        c.sq = to;
                    }
                }
                if let Some(mut f) = self.pos.featherfall
                    && f.sq == from
                {
                    f.sq = to;
                    self.pos.featherfall = Some(f);
                }
            }
            SpellCast::Insight => {
                // Refill spent or empty hand slots (including the just-cast Insight),
                // up to 2 cards and up to the 3-card hand limit.
                let hand = &mut self.hands[side.index()];
                for _ in 0..2 {
                    if hand.deck.is_empty() {
                        break;
                    }
                    let Some(idx) = (0..3).find(|&i| hand.hand[i].is_none() || hand.used[i]) else {
                        break;
                    };
                    let card = hand.deck.remove(0);
                    hand.hand[idx] = Some(card);
                    hand.used[idx] = false;
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

            for cave in &mut self.timed_caves {
                if !matches!(cast, SpellCast::DigTunnel(_, _)) || cave.remaining_plies < 8 {
                    cave.remaining_plies = cave.remaining_plies.saturating_sub(1);
                    if cave.remaining_plies == 0 {
                        if self.terrain.get(cave.a).feature == Feature::Cave(cave.link) {
                            self.terrain.get_mut(cave.a).feature = Feature::None;
                        }
                        if self.terrain.get(cave.b).feature == Feature::Cave(cave.link) {
                            self.terrain.get_mut(cave.b).feature = Feature::None;
                        }
                    }
                }
            }
            self.timed_caves.retain(|c| c.remaining_plies > 0);

            // Piece-bound timers tick down once per ply; a freshly cast Curse
            // keeps its full duration like a freshly cast Freeze.
            if let Some(f) = &mut self.pos.featherfall {
                f.remaining_plies = f.remaining_plies.saturating_sub(1);
                if f.remaining_plies == 0 {
                    self.pos.featherfall = None;
                }
            }
            for curse in &mut self.pos.curses {
                if !matches!(cast, SpellCast::Curse(_)) || curse.remaining_plies < CURSE_PLIES {
                    curse.remaining_plies = curse.remaining_plies.saturating_sub(1);
                }
            }
            self.pos.curses.retain(|c| c.remaining_plies > 0);

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
