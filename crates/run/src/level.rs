//! Level ladder definitions for the level select mode.

use tc_core::PieceKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Level {
    pub id: u8,
    pub name: &'static str,
    pub size: u8,
    pub army: Option<&'static [PieceKind]>,
    /// The enemy's army when it differs from the player's (easier early levels).
    pub enemy_army: Option<&'static [PieceKind]>,
    pub difficulty: u8,
}

use PieceKind::*;

// The first two levels are lopsided in the player's favour so they end quickly.
const ARMY_L1: [PieceKind; 2] = [King, Queen];
const ENEMY_L1: [PieceKind; 1] = [King];
const ARMY_L2: [PieceKind; 3] = [King, Rook, Rook];
const ENEMY_L2: [PieceKind; 2] = [King, Pawn];
const ARMY_L3: [PieceKind; 4] = [King, Knight, Pawn, Pawn];
const ARMY_L4: [PieceKind; 5] = [King, Rook, Bishop, Pawn, Pawn];
const ARMY_L5: [PieceKind; 10] = [King, Queen, Rook, Bishop, Knight, Pawn, Pawn, Pawn, Pawn, Pawn];
const ARMY_L6: [PieceKind; 8] = [King, Queen, Rook, Knight, Pawn, Pawn, Pawn, Pawn];
const ARMY_L7: [PieceKind; 12] =
    [King, Queen, Rook, Rook, Bishop, Knight, Pawn, Pawn, Pawn, Pawn, Pawn, Pawn];
const ARMY_L9: [PieceKind; 24] = [
    King, Queen, Queen, Rook, Rook, Rook, Bishop, Bishop, Bishop, Knight, Knight, Knight, Pawn, Pawn, Pawn,
    Pawn, Pawn, Pawn, Pawn, Pawn, Pawn, Pawn, Pawn, Pawn,
];

pub const LEVELS: [Level; 10] = [
    Level {
        id: 1,
        name: "Queen's Hunt",
        size: 4,
        army: Some(&ARMY_L1),
        enemy_army: Some(&ENEMY_L1),
        difficulty: 0,
    },
    Level {
        id: 2,
        name: "Twin Rooks",
        size: 5,
        army: Some(&ARMY_L2),
        enemy_army: Some(&ENEMY_L2),
        difficulty: 0,
    },
    Level { id: 3, name: "Knight's Field", size: 6, army: Some(&ARMY_L3), enemy_army: None, difficulty: 1 },
    Level { id: 4, name: "Five Pieces", size: 6, army: Some(&ARMY_L4), enemy_army: None, difficulty: 2 },
    Level { id: 5, name: "Gardner Mini", size: 5, army: Some(&ARMY_L5), enemy_army: None, difficulty: 3 },
    Level { id: 6, name: "Half Army", size: 7, army: Some(&ARMY_L6), enemy_army: None, difficulty: 4 },
    Level { id: 7, name: "Almost Full", size: 8, army: Some(&ARMY_L7), enemy_army: None, difficulty: 5 },
    Level { id: 8, name: "Full Chess", size: 8, army: None, enemy_army: None, difficulty: 6 },
    Level { id: 9, name: "Big Board", size: 12, army: Some(&ARMY_L9), enemy_army: None, difficulty: 7 },
    Level { id: 10, name: "Grand Battle", size: 16, army: None, enemy_army: None, difficulty: 7 },
];

pub fn level(id: u8) -> Option<&'static Level> {
    LEVELS.iter().find(|l| l.id == id)
}
