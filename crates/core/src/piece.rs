//! Pieces and their data-driven movement profiles (PLAN.md §2, §4).

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    /// The Ashen Sun (holy kingdom). Moves first.
    White,
    /// The Hollow Crown (undead court).
    Black,
}

impl Side {
    pub const BOTH: [Side; 2] = [Side::White, Side::Black];

    pub fn opposite(self) -> Side {
        match self {
            Side::White => Side::Black,
            Side::Black => Side::White,
        }
    }

    pub fn index(self) -> usize {
        self as usize
    }

    /// Rank direction pawns move in.
    pub fn forward(self) -> i8 {
        match self {
            Side::White => 1,
            Side::Black => -1,
        }
    }

    /// Rank counted from this side's own back rank.
    pub fn relative_rank(self, y: u8, size: u8) -> u8 {
        match self {
            Side::White => y,
            Side::Black => size - 1 - y,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PieceKind {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl PieceKind {
    pub const ALL: [PieceKind; 6] = [
        PieceKind::Pawn,
        PieceKind::Knight,
        PieceKind::Bishop,
        PieceKind::Rook,
        PieceKind::Queen,
        PieceKind::King,
    ];
    pub const PROMOTIONS: [PieceKind; 4] = [
        PieceKind::Queen,
        PieceKind::Rook,
        PieceKind::Bishop,
        PieceKind::Knight,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    /// Lowercase FEN letter.
    pub fn letter(self) -> char {
        match self {
            PieceKind::Pawn => 'p',
            PieceKind::Knight => 'n',
            PieceKind::Bishop => 'b',
            PieceKind::Rook => 'r',
            PieceKind::Queen => 'q',
            PieceKind::King => 'k',
        }
    }

    pub fn from_letter(c: char) -> Option<PieceKind> {
        Some(match c.to_ascii_lowercase() {
            'p' => PieceKind::Pawn,
            'n' => PieceKind::Knight,
            'b' => PieceKind::Bishop,
            'r' => PieceKind::Rook,
            'q' => PieceKind::Queen,
            'k' => PieceKind::King,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Piece {
    pub kind: PieceKind,
    pub side: Side,
}

impl Piece {
    pub const fn new(kind: PieceKind, side: Side) -> Self {
        Piece { kind, side }
    }

    pub fn fen_char(self) -> char {
        let c = self.kind.letter();
        match self.side {
            Side::White => c.to_ascii_uppercase(),
            Side::Black => c,
        }
    }
}

/// How a piece type interacts with terrain. Most upgrades just change these numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoveProfile {
    /// Highest upward step onto a neighbouring tile. Anything taller is a cliff.
    pub max_climb: u8,
    /// A slide stops on a tile reached by dropping more than this.
    pub max_slide_drop: u8,
    /// Any upward step ends a slide ("hard going uphill").
    pub uphill_ends_slide: bool,
    /// May enter deep water.
    pub deep_water: bool,
    /// Jumps (knight) may only land where `|Δh|` is at most this.
    pub jump_max_dh: u8,
}

impl MoveProfile {
    pub const fn standard() -> Self {
        MoveProfile {
            max_climb: 1,
            max_slide_drop: 1,
            uphill_ends_slide: true,
            deep_water: false,
            jump_max_dh: 2,
        }
    }
}

impl Default for MoveProfile {
    fn default() -> Self {
        Self::standard()
    }
}
