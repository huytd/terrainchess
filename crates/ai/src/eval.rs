//! Static evaluation from the side to move's point of view, in centipawns.
//! Terrain matters: mobility counts terrain-legal attacks, high ground is worth
//! something, and cliffs around a king shelter it.

use tc_core::board::KING;
use tc_core::movegen::Ctx;
use tc_core::terrain::TileKind;
use tc_core::{PieceKind, Position};

pub fn piece_value(kind: PieceKind) -> i32 {
    match kind {
        PieceKind::Pawn => 100,
        PieceKind::Knight => 310,
        PieceKind::Bishop => 320,
        PieceKind::Rook => 500,
        PieceKind::Queen => 900,
        PieceKind::King => 0,
    }
}

const TEMPO: i32 = 10;

pub fn evaluate(ctx: &Ctx, pos: &Position) -> i32 {
    let t = ctx.terrain;
    let size = t.size;
    let center = (size as i32 - 1) as f32 / 2.0;
    let scale = 8.0 / size as f32;
    let mut score = [0i32; 2];

    for (sq, piece) in pos.pieces() {
        let s = &mut score[piece.side.index()];
        let h = t.height(sq) as i32;
        *s += piece_value(piece.kind);

        let dist = (sq.x as f32 - center).abs().max((sq.y as f32 - center).abs());
        let central = ((center - dist) * scale * 2.0) as i32; // 0 at the edge, ~7 in the middle
        match piece.kind {
            PieceKind::Pawn => {
                let rank = piece.side.relative_rank(sq.y, size) as f32;
                *s += (rank * rank * scale * scale * 1.5) as i32 + central;
            }
            PieceKind::Knight | PieceKind::Bishop => *s += central * 3,
            PieceKind::King => {
                // Stay home early; shelter behind own pieces and ground enemies can't climb.
                for (dx, dy) in KING {
                    let Some(n) = sq.offset(dx, dy, size) else {
                        *s += 3;
                        continue;
                    };
                    let tile = t.get(n);
                    if pos.get(n).is_some_and(|p| p.side == piece.side) {
                        *s += 6;
                    } else if tile.is_blocked()
                        || tile.kind == TileKind::DeepWater
                        || (h - tile.height as i32) >= 2
                    {
                        *s += 5;
                    }
                }
                continue;
            }
            _ => {}
        }
        // High ground: harder to attack from below, and it sees further downhill.
        *s += h * 6;
        if t.get(sq).kind == TileKind::ShallowWater {
            *s -= 12;
        }

        // Mobility: terrain-legal squares this piece attacks.
        if piece.kind != PieceKind::Pawn {
            let weight = match piece.kind {
                PieceKind::Knight | PieceKind::Bishop => 4,
                _ => 2,
            };
            let mut n = 0;
            ctx.for_each_attack(pos, sq, piece, &mut |to| {
                if pos.get(to).is_none_or(|p| p.side != piece.side) {
                    n += 1;
                }
                false
            });
            *s += n * weight;
        }
    }

    let us = pos.side_to_move;
    let rel = score[us.index()] - score[us.opposite().index()];
    rel + TEMPO
}
