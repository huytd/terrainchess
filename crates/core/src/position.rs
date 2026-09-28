//! Piece placement and game state, plus starting armies and FEN (8×8 only).

use crate::board::Sq;
use crate::movegen::{Move, MoveKind};
use crate::piece::{Piece, PieceKind, Side};
use crate::rng::mix;

/// Castling wings, indexing `Position::castling`.
pub const KINGSIDE: usize = 0;
pub const QUEENSIDE: usize = 1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Position {
    pub size: u8,
    pub squares: Vec<Option<Piece>>,
    pub side_to_move: Side,
    /// Per side and wing, the square of a rook that may still castle. Its king is
    /// unmoved while any right remains.
    pub castling: [[Option<Sq>; 2]; 2],
    /// Square a pawn skipped with its last double step.
    pub en_passant: Option<Sq>,
    pub halfmove_clock: u16,
    pub fullmove: u16,
    pub shield: Option<(Sq, Side)>,
    pub cleared: Vec<Sq>,
}

impl Position {
    pub fn empty(size: u8) -> Self {
        Position {
            size,
            squares: vec![None; size as usize * size as usize],
            side_to_move: Side::White,
            castling: [[None; 2]; 2],
            en_passant: None,
            halfmove_clock: 0,
            fullmove: 1,
            shield: None,
            cleared: Vec::new(),
        }
    }

    pub fn get(&self, sq: Sq) -> Option<Piece> {
        self.squares[sq.index(self.size)]
    }

    pub fn set(&mut self, sq: Sq, piece: Option<Piece>) {
        self.squares[sq.index(self.size)] = piece;
    }

    pub fn pieces(&self) -> impl Iterator<Item = (Sq, Piece)> + '_ {
        self.squares.iter().enumerate().filter_map(|(i, p)| p.map(|p| (Sq::from_index(i, self.size), p)))
    }

    pub fn king(&self, side: Side) -> Option<Sq> {
        self.pieces().find(|(_, p)| p.side == side && p.kind == PieceKind::King).map(|(s, _)| s)
    }

    /// Number of flattened home ranks the terrain generator must keep clear.
    pub fn home_rows(size: u8) -> u8 {
        if size <= 8 { 2 } else { 3 }
    }

    /// The starting army for a board size (PLAN.md §5).
    pub fn start(size: u8) -> Self {
        use PieceKind::*;
        const STANDARD: [PieceKind; 8] = [Rook, Knight, Bishop, Queen, King, Bishop, Knight, Rook];
        // 16×16: doubled back rank, one king (4R 4N 4B 2Q 1K + an extra rook).
        const DOUBLED: [PieceKind; 16] = [
            Rook, Knight, Bishop, Rook, Knight, Bishop, Queen, King, Queen, Bishop, Knight, Rook, Bishop,
            Knight, Rook, Rook,
        ];
        let (back, x0): (&[PieceKind], u8) = match size {
            8 => (&STANDARD, 0),
            16 => (&DOUBLED, 0),
            // 32×32 "skirmish": a standard army in a central home zone.
            _ => (&STANDARD, size / 2 - 4),
        };
        let mut pos = Position::empty(size);
        for side in Side::BOTH {
            let (back_y, pawn_y) = match side {
                Side::White => (0, 1),
                Side::Black => (size - 1, size - 2),
            };
            for (i, &kind) in back.iter().enumerate() {
                let x = x0 + i as u8;
                pos.set(Sq::new(x, back_y), Some(Piece::new(kind, side)));
                pos.set(Sq::new(x, pawn_y), Some(Piece::new(Pawn, side)));
            }
            // Castling with the nearest rook on each side of the king.
            let king_x = x0 + back.iter().position(|&k| k == King).unwrap() as u8;
            let rook_at = |x: &u8| back[(*x - x0) as usize] == Rook;
            let s = side.index();
            pos.castling[s][KINGSIDE] =
                (king_x + 1..x0 + back.len() as u8).find(rook_at).map(|x| Sq::new(x, back_y));
            pos.castling[s][QUEENSIDE] = (x0..king_x).rev().find(rook_at).map(|x| Sq::new(x, back_y));
        }
        pos
    }

    /// Parse an 8×8 FEN (castling letters refer to the corner rooks).
    pub fn from_fen(fen: &str) -> Result<Self, String> {
        let parts: Vec<&str> = fen.split_whitespace().collect();
        if parts.len() < 4 {
            return Err("FEN needs at least 4 fields".into());
        }
        let mut pos = Position::empty(8);
        for (row, rank) in parts[0].split('/').enumerate() {
            let y = 7u8.checked_sub(row as u8).ok_or("too many ranks")?;
            let mut x = 0u8;
            for c in rank.chars() {
                if let Some(d) = c.to_digit(10) {
                    x += d as u8;
                } else {
                    let kind = PieceKind::from_letter(c).ok_or(format!("bad piece {c}"))?;
                    let side = if c.is_ascii_uppercase() { Side::White } else { Side::Black };
                    if x >= 8 {
                        return Err("rank too long".into());
                    }
                    pos.set(Sq::new(x, y), Some(Piece::new(kind, side)));
                    x += 1;
                }
            }
        }
        pos.side_to_move = match parts[1] {
            "w" => Side::White,
            "b" => Side::Black,
            s => return Err(format!("bad side {s}")),
        };
        for c in parts[2].chars() {
            match c {
                'K' => pos.castling[0][KINGSIDE] = Some(Sq::new(7, 0)),
                'Q' => pos.castling[0][QUEENSIDE] = Some(Sq::new(0, 0)),
                'k' => pos.castling[1][KINGSIDE] = Some(Sq::new(7, 7)),
                'q' => pos.castling[1][QUEENSIDE] = Some(Sq::new(0, 7)),
                '-' => {}
                _ => return Err(format!("bad castling {c}")),
            }
        }
        pos.en_passant = match parts[3] {
            "-" => None,
            s => Some(Sq::parse(s).ok_or(format!("bad en passant {s}"))?),
        };
        pos.halfmove_clock = parts.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        pos.fullmove = parts.get(5).and_then(|s| s.parse().ok()).unwrap_or(1);
        Ok(pos)
    }

    /// Apply a move. Legality is the move generator's job.
    pub fn make_move(&mut self, mv: Move) {
        let side = self.side_to_move;
        let s = side.index();
        let piece = self.get(mv.from).expect("move from an empty square");

        if mv.kind == MoveKind::Clear {
            self.en_passant = None;
            if !self.cleared.contains(&mv.to) {
                self.cleared.push(mv.to);
                self.cleared.sort();
            }
            self.halfmove_clock += 1;
            if side == Side::Black {
                self.fullmove += 1;
            }
            self.side_to_move = side.opposite();
            if self.shield.is_some_and(|(_, s)| s == self.side_to_move) {
                self.shield = None;
            }
            return;
        }

        let captured = self.get(mv.to);

        self.en_passant = None;
        self.set(mv.from, None);
        match mv.kind {
            MoveKind::EnPassant => self.set(Sq::new(mv.to.x, mv.from.y), None),
            MoveKind::DoublePush => {
                self.en_passant = Some(Sq::new(mv.from.x, (mv.from.y + mv.to.y) / 2));
            }
            MoveKind::Castle { rook_from, rook_to } => {
                let rook = self.get(rook_from);
                self.set(rook_from, None);
                self.set(rook_to, rook);
            }
            MoveKind::Normal | MoveKind::Cave => {}
            MoveKind::Clear => unreachable!(),
        }
        let placed = match mv.promotion {
            Some(kind) => Piece::new(kind, side),
            None => piece,
        };
        self.set(mv.to, Some(placed));

        if piece.kind == PieceKind::King {
            self.castling[s] = [None; 2];
        }
        for rights in &mut self.castling {
            for r in rights.iter_mut() {
                if *r == Some(mv.from) || *r == Some(mv.to) {
                    *r = None;
                }
            }
        }

        let is_capture = captured.is_some() || mv.kind == MoveKind::EnPassant;
        if piece.kind == PieceKind::Pawn || is_capture {
            self.halfmove_clock = 0;
        } else {
            self.halfmove_clock += 1;
        }
        if side == Side::Black {
            self.fullmove += 1;
        }
        self.side_to_move = side.opposite();
        if self.shield.is_some_and(|(_, s)| s == self.side_to_move) {
            self.shield = None;
        }
    }

    /// Zobrist hash for repetition detection and (later) the AI's transposition table.
    pub fn hash(&self) -> u64 {
        let mut h = 0u64;
        for (sq, p) in self.pieces() {
            let key = 1 + (p.side.index() * 6 + p.kind.index()) * 1024 + sq.index(self.size);
            h ^= mix(key as u64);
        }
        if self.side_to_move == Side::Black {
            h ^= mix(20_000);
        }
        for (s, rights) in self.castling.iter().enumerate() {
            for r in rights.iter().flatten() {
                h ^= mix(30_000 + s as u64 * 2048 + r.index(self.size) as u64);
            }
        }
        if let Some(ep) = self.en_passant {
            h ^= mix(40_000 + ep.index(self.size) as u64);
        }
        if let Some((sq, s)) = self.shield {
            h ^= mix(50_000 + s.index() as u64 * 2048 + sq.index(self.size) as u64);
        }
        for sq in &self.cleared {
            h ^= mix(60_000 + sq.index(self.size) as u64);
        }
        h
    }
}
