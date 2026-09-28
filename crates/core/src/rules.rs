//! Match state: rules configuration, legal play, and end-of-game detection.

use serde::{Deserialize, Serialize};

use crate::movegen::{Ctx, Move};
use crate::piece::{MoveProfile, Piece, PieceKind, Side};
use crate::position::Position;
use crate::terrain::Terrain;

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
}

impl Match {
    pub fn new(terrain: Terrain, rules: Rules, pos: Position) -> Self {
        assert_eq!(terrain.size, pos.size);
        let history = vec![pos.hash()];
        Match { terrain, rules, pos, history, moves: Vec::new() }
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

    /// Play a move if it is legal.
    pub fn play(&mut self, mv: Move) -> Result<(), String> {
        if !self.legal_moves().contains(&mv) {
            return Err(format!("illegal move {}", mv.uci()));
        }
        self.pos.make_move(mv);
        self.history.push(self.pos.hash());
        self.moves.push(mv);
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
