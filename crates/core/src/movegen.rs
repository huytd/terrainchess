//! Terrain-aware move generation (PLAN.md §4). Check detection reuses the same
//! step rules, so terrain that blocks a move also blocks the matching attack.
//! Captures only go level or downhill: a piece on higher ground is safe from anything
//! below it, though the lower piece may still climb onto that square once it is empty.

use crate::board::{DIAG, KING, KNIGHT, ORTHO, Sq};
use crate::piece::{MoveProfile, Piece, PieceKind, Side};
use crate::position::{KINGSIDE, Position, QUEENSIDE};
use crate::rules::Rules;
use crate::terrain::{Terrain, TileKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MoveKind {
    Normal,
    DoublePush,
    EnPassant,
    Castle {
        rook_from: Sq,
        rook_to: Sq,
    },
    /// Tunnel hop from one cave entrance to a linked one.
    Cave,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Move {
    pub from: Sq,
    pub to: Sq,
    pub kind: MoveKind,
    pub promotion: Option<PieceKind>,
}

impl Move {
    pub fn new(from: Sq, to: Sq, kind: MoveKind) -> Self {
        Move { from, to, kind, promotion: None }
    }

    /// Long algebraic form, e.g. `e2e4`, `e7e8q`.
    pub fn uci(&self) -> String {
        let promo = self.promotion.map(|k| k.letter().to_string()).unwrap_or_default();
        format!("{}{}{}", self.from.name(), self.to.name(), promo)
    }
}

/// The static parts of a match that move generation reads.
#[derive(Clone, Copy)]
pub struct Ctx<'a> {
    pub terrain: &'a Terrain,
    pub rules: &'a Rules,
}

impl Ctx<'_> {
    fn size(&self) -> u8 {
        self.terrain.size
    }

    fn up(&self, from: Sq, to: Sq) -> i16 {
        self.terrain.height(to) as i16 - self.terrain.height(from) as i16
    }

    fn in_shallow_water(&self, sq: Sq) -> bool {
        self.terrain.get(sq).kind == TileKind::ShallowWater
    }

    /// The piece may stand on `sq` at all.
    fn can_stand(&self, prof: &MoveProfile, sq: Sq) -> bool {
        let tile = self.terrain.get(sq);
        !tile.is_blocked() && (prof.deep_water || tile.kind != TileKind::DeepWater)
    }

    /// A single step between neighbouring tiles: climbing is limited, drops are free.
    fn can_step(&self, prof: &MoveProfile, from: Sq, to: Sq) -> bool {
        self.can_stand(prof, to) && self.up(from, to) <= prof.max_climb as i16
    }

    /// A slide that just stepped `from` → `to` cannot continue past `to`.
    fn slide_ends(&self, prof: &MoveProfile, from: Sq, to: Sq) -> bool {
        let up = self.up(from, to);
        (prof.uphill_ends_slide && up >= 1) || -up > prof.max_slide_drop as i16 || self.in_shallow_water(to)
    }

    /// Captures can't go uphill: `from` must be at least as high as the victim's square.
    fn can_capture_on(&self, from: Sq, victim: Sq) -> bool {
        self.terrain.height(victim) <= self.terrain.height(from)
    }

    fn can_land_jump(&self, prof: &MoveProfile, from: Sq, to: Sq) -> bool {
        self.can_stand(prof, to) && self.up(from, to).unsigned_abs() <= prof.jump_max_dh as u16
    }

    /// Walk a ray, calling `f` for each reachable square. The ray stops at the first
    /// occupied square (which is still reported) or where terrain ends it.
    fn ray(
        &self,
        pos: &Position,
        prof: &MoveProfile,
        from: Sq,
        dir: (i8, i8),
        f: &mut impl FnMut(Sq, bool) -> bool,
    ) -> bool {
        // A piece standing in shallow water can't move 2+ squares.
        let max_steps = if self.in_shallow_water(from) { 1 } else { usize::MAX };
        let mut cur = from;
        for _ in 0..max_steps {
            let Some(next) = cur.offset(dir.0, dir.1, self.size()) else { break };
            if !self.can_step(prof, cur, next) {
                break;
            }
            if f(next, false) {
                return true;
            }
            if prof.cave_slide && pos.get(next).is_none() {
                for exit in self.terrain.cave_exits(next) {
                    if self.can_stand(prof, exit) && f(exit, true) {
                        return true;
                    }
                }
            }
            if pos.get(next).is_some() || self.slide_ends(prof, cur, next) {
                break;
            }
            cur = next;
        }
        false
    }

    /// Extended attack generation that indicates whether a target was reached via cave hop.
    pub fn for_each_attack_ext(
        &self,
        pos: &Position,
        from: Sq,
        piece: Piece,
        f: &mut impl FnMut(Sq, bool) -> bool,
    ) -> bool {
        let size = self.size();
        let prof = self.rules.profile(piece);
        let slide_dirs: &[(i8, i8)] = match piece.kind {
            PieceKind::Pawn => {
                for dx in [-1, 1] {
                    if let Some(to) = from.offset(dx, piece.side.forward(), size)
                        && self.can_step(prof, from, to)
                        && f(to, false)
                    {
                        return true;
                    }
                }
                &[]
            }
            PieceKind::Knight => {
                if !self.in_shallow_water(from) {
                    for (dx, dy) in KNIGHT {
                        if let Some(to) = from.offset(dx, dy, size)
                            && self.can_land_jump(prof, from, to)
                            && f(to, false)
                        {
                            return true;
                        }
                    }
                }
                &[]
            }
            PieceKind::King => {
                for (dx, dy) in KING {
                    if let Some(to) = from.offset(dx, dy, size)
                        && self.can_step(prof, from, to)
                        && f(to, false)
                    {
                        return true;
                    }
                }
                &[]
            }
            PieceKind::Bishop => &DIAG,
            PieceKind::Rook => &ORTHO,
            PieceKind::Queen => &KING,
        };
        for &dir in slide_dirs {
            if self.ray(pos, prof, from, dir, f) {
                return true;
            }
        }
        // One tunnel hop, for any piece type.
        for exit in self.terrain.cave_exits(from) {
            if self.can_stand(prof, exit) && f(exit, true) {
                return true;
            }
        }
        false
    }

    /// Every square `piece` at `from` reaches by its movement pattern: it moves to these
    /// when empty and captures on them when they hold an enemy no higher than `from`.
    /// Pawn pushes are not included. `f` returns true to stop early.
    pub fn for_each_attack(
        &self,
        pos: &Position,
        from: Sq,
        piece: Piece,
        f: &mut impl FnMut(Sq) -> bool,
    ) -> bool {
        self.for_each_attack_ext(pos, from, piece, &mut |sq, _| f(sq))
    }

    pub fn is_attacked(&self, pos: &Position, target: Sq, by: Side) -> bool {
        if pos.shield.is_some_and(|(sq, side)| sq == target && side != by) {
            return false;
        }
        pos.pieces().filter(|(_, p)| p.side == by).any(|(from, p)| {
            self.can_capture_on(from, target) && self.for_each_attack(pos, from, p, &mut |sq| sq == target)
        })
    }

    pub fn in_check(&self, pos: &Position, side: Side) -> bool {
        pos.king(side).is_some_and(|k| self.is_attacked(pos, k, side.opposite()))
    }

    /// Moves that obey piece and terrain rules but may leave the king in check.
    pub fn pseudo_legal(&self, pos: &Position, out: &mut Vec<Move>) {
        let side = pos.side_to_move;
        for (from, piece) in pos.pieces().filter(|(_, p)| p.side == side) {
            if piece.kind == PieceKind::Pawn {
                self.pawn_moves(pos, from, piece, out);
                continue;
            }
            self.for_each_attack_ext(pos, from, piece, &mut |to, is_cave| {
                let open = match pos.get(to) {
                    None => true,
                    Some(p) => {
                        p.side != side
                            && self.can_capture_on(from, to)
                            && !pos.shield.is_some_and(|(sq, s)| sq == to && s != side)
                    }
                };
                if open {
                    let is_direct_cave = self.terrain.cave_exits(from).any(|e| e == to);
                    let kind = if is_cave || is_direct_cave { MoveKind::Cave } else { MoveKind::Normal };
                    if let Some(existing) = out.iter_mut().find(|m| m.from == from && m.to == to) {
                        if kind == MoveKind::Cave {
                            existing.kind = MoveKind::Cave;
                        }
                    } else {
                        out.push(Move::new(from, to, kind));
                    }
                }
                false
            });
            if piece.kind == PieceKind::King {
                self.castling_moves(pos, from, out);
            }
        }
    }

    fn pawn_moves(&self, pos: &Position, from: Sq, piece: Piece, out: &mut Vec<Move>) {
        let size = self.size();
        let side = piece.side;
        let prof = self.rules.profile(piece);
        let fwd = side.forward();
        let push = |mv: Move, out: &mut Vec<Move>| {
            if side.relative_rank(mv.to.y, size) == size - 1 {
                for kind in PieceKind::PROMOTIONS {
                    out.push(Move { promotion: Some(kind), ..mv });
                }
            } else {
                out.push(mv);
            }
        };

        if let Some(one) = from.offset(0, fwd, size)
            && pos.get(one).is_none()
            && self.can_step(prof, from, one)
        {
            push(Move::new(from, one, MoveKind::Normal), out);
            // The double step can't climb: neither tile may be higher than the start.
            let rank = side.relative_rank(from.y, size);
            let h = self.terrain.height(from);
            if let Some(two) = one.offset(0, fwd, size)
                && rank >= 1
                && rank <= self.rules.double_step_max_rank
                && !self.in_shallow_water(from)
                && !self.slide_ends(prof, from, one)
                && pos.get(two).is_none()
                && self.can_step(prof, one, two)
                && self.terrain.height(two) <= h
            {
                push(Move::new(from, two, MoveKind::DoublePush), out);
            }
        }
        for dx in [-1, 1] {
            let Some(to) = from.offset(dx, fwd, size) else { continue };
            if !self.can_step(prof, from, to) {
                continue;
            }
            if pos.get(to).is_some_and(|p| p.side != side) {
                if self.can_capture_on(from, to) && !pos.shield.is_some_and(|(sq, s)| sq == to && s != side) {
                    push(Move::new(from, to, MoveKind::Normal), out);
                }
            } else if pos.en_passant == Some(to) && self.can_capture_on(from, Sq::new(to.x, from.y)) {
                let victim_sq = Sq::new(to.x, from.y);
                if !pos.shield.is_some_and(|(sq, s)| (sq == to || sq == victim_sq) && s != side) {
                    push(Move::new(from, to, MoveKind::EnPassant), out);
                }
            }
        }
        for exit in self.terrain.cave_exits(from) {
            if self.can_stand(prof, exit)
                && pos.get(exit).is_none_or(|p| {
                    p.side != side
                        && self.can_capture_on(from, exit)
                        && !pos.shield.is_some_and(|(sq, s)| sq == exit && s != side)
                })
            {
                push(Move::new(from, exit, MoveKind::Cave), out);
            }
        }
    }

    /// King moves two squares toward an unmoved rook. The squares between must be empty
    /// and flat, and the king may not be in, pass through, or land in check.
    fn castling_moves(&self, pos: &Position, king: Sq, out: &mut Vec<Move>) {
        let side = pos.side_to_move;
        let enemy = side.opposite();
        for wing in [KINGSIDE, QUEENSIDE] {
            let Some(rook) = pos.castling[side.index()][wing] else { continue };
            if rook.y != king.y || pos.get(rook) != Some(Piece::new(PieceKind::Rook, side)) {
                continue;
            }
            let dir: i8 = if rook.x > king.x { 1 } else { -1 };
            let Some(king_to) = king.offset(2 * dir, 0, self.size()) else { continue };
            let rook_to = king.offset(dir, 0, self.size()).unwrap();
            let (lo, hi) = (king.x.min(rook.x), king.x.max(rook.x));
            let h = self.terrain.height(king);
            let lane_ok = (lo..=hi).all(|x| {
                let sq = Sq::new(x, king.y);
                let tile = self.terrain.get(sq);
                (sq == king || sq == rook || pos.get(sq).is_none())
                    && tile.height == h
                    && !tile.is_blocked()
                    && !tile.is_water()
            });
            // With a far rook the king could still move past it; keep both on the lane.
            let reaches = (king_to.x as i16 - king.x as i16).abs() <= (rook.x as i16 - king.x as i16).abs();
            if !lane_ok || !reaches {
                continue;
            }
            if [king, rook_to, king_to].iter().any(|&sq| self.is_attacked(pos, sq, enemy)) {
                continue;
            }
            out.push(Move::new(king, king_to, MoveKind::Castle { rook_from: rook, rook_to }));
        }
    }

    pub fn legal_moves(&self, pos: &Position) -> Vec<Move> {
        let mut pseudo = Vec::with_capacity(64);
        self.pseudo_legal(pos, &mut pseudo);
        let side = pos.side_to_move;
        pseudo.retain(|&mv| {
            let mut next = pos.clone();
            next.make_move(mv);
            !self.in_check(&next, side)
        });
        pseudo
    }

    /// Count leaf nodes of the legal move tree; the standard movegen correctness test.
    pub fn perft(&self, pos: &Position, depth: u32) -> u64 {
        if depth == 0 {
            return 1;
        }
        let moves = self.legal_moves(pos);
        if depth == 1 {
            return moves.len() as u64;
        }
        moves
            .into_iter()
            .map(|mv| {
                let mut next = pos.clone();
                next.make_move(mv);
                self.perft(&next, depth - 1)
            })
            .sum()
    }

    /// Squares a lone piece could reach in one move on an otherwise empty board,
    /// used by the generator's validator.
    pub fn empty_board_targets(&self, from: Sq, piece: Piece) -> Vec<Sq> {
        let mut pos = Position::empty(self.size());
        pos.set(from, Some(piece));
        pos.side_to_move = piece.side;
        let mut out = Vec::new();
        self.pseudo_legal(&pos, &mut out);
        let mut targets: Vec<Sq> = out.into_iter().map(|m| m.to).collect();
        if piece.kind == PieceKind::Pawn {
            // Diagonals count too: a capture may open them.
            self.for_each_attack(&pos, from, piece, &mut |sq| {
                targets.push(sq);
                false
            });
        }
        targets.sort();
        targets.dedup();
        targets
    }
}
