//! Rejects unfair or broken boards: every piece type must be able to reach the
//! enemy half, and nobody may start in check or without moves.

use std::collections::VecDeque;

use crate::board::Sq;
use crate::movegen::Ctx;
use crate::piece::{PieceKind, Side};
use crate::position::Position;
use crate::rules::Rules;
use crate::terrain::Terrain;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reject {
    /// Pieces of this type can never reach the opponent's half (a king here means sealed in).
    Unreachable(Side, PieceKind),
    StartsInCheck(Side),
    NoMoves,
    StartOnBlockedTile(Sq),
}

pub fn validate(terrain: &Terrain, rules: &Rules, pos: &Position) -> Result<(), Reject> {
    let ctx = Ctx { terrain, rules };
    let size = terrain.size;
    for (sq, _) in pos.pieces() {
        if terrain.get(sq).is_blocked() {
            return Err(Reject::StartOnBlockedTile(sq));
        }
    }
    for side in Side::BOTH {
        for kind in PieceKind::ALL {
            let starts: Vec<Sq> =
                pos.pieces().filter(|(_, p)| p.side == side && p.kind == kind).map(|(s, _)| s).collect();
            if starts.is_empty() {
                continue;
            }
            let piece = crate::piece::Piece::new(kind, side);
            let mut seen = vec![false; size as usize * size as usize];
            let mut queue: VecDeque<Sq> = starts.into_iter().collect();
            let mut reached = false;
            while let Some(sq) = queue.pop_front() {
                if std::mem::replace(&mut seen[sq.index(size)], true) {
                    continue;
                }
                if side.relative_rank(sq.y, size) >= size / 2 {
                    reached = true;
                    break;
                }
                queue.extend(ctx.empty_board_targets(sq, piece));
            }
            if !reached {
                return Err(Reject::Unreachable(side, kind));
            }
        }
        if ctx.in_check(pos, side) {
            return Err(Reject::StartsInCheck(side));
        }
    }
    if ctx.legal_moves(pos).is_empty() {
        return Err(Reject::NoMoves);
    }
    Ok(())
}
