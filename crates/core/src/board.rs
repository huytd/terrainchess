//! Grid coordinates. `x` is the file (0 = a), `y` is the rank (0 = White's back rank).

use serde::{Deserialize, Serialize};

/// Supported board sizes (specs/game-design.md §3).
pub const SIZES: [u8; 3] = [8, 16, 32];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Sq {
    pub x: u8,
    pub y: u8,
}

impl Sq {
    pub const fn new(x: u8, y: u8) -> Self {
        Sq { x, y }
    }

    pub fn index(self, size: u8) -> usize {
        self.y as usize * size as usize + self.x as usize
    }

    pub fn from_index(i: usize, size: u8) -> Self {
        Sq::new((i % size as usize) as u8, (i / size as usize) as u8)
    }

    pub fn offset(self, dx: i8, dy: i8, size: u8) -> Option<Sq> {
        let x = self.x as i16 + dx as i16;
        let y = self.y as i16 + dy as i16;
        (0..size as i16).contains(&x).then_some(())?;
        (0..size as i16).contains(&y).then_some(())?;
        Some(Sq::new(x as u8, y as u8))
    }

    /// Algebraic name such as `e4`. Files past `z` are not needed below 32×32.
    pub fn name(self) -> String {
        format!("{}{}", (b'a' + self.x) as char, self.y as u32 + 1)
    }

    pub fn parse(s: &str) -> Option<Sq> {
        let mut chars = s.chars();
        let f = chars.next()?;
        if !f.is_ascii_lowercase() {
            return None;
        }
        let r: u8 = chars.as_str().parse().ok()?;
        (r >= 1).then(|| Sq::new(f as u8 - b'a', r - 1))
    }
}

pub fn squares(size: u8) -> impl Iterator<Item = Sq> {
    (0..size as usize * size as usize).map(move |i| Sq::from_index(i, size))
}

pub const ORTHO: [(i8, i8); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
pub const DIAG: [(i8, i8); 4] = [(1, 1), (-1, 1), (1, -1), (-1, -1)];
pub const KING: [(i8, i8); 8] = [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, 1), (1, -1), (-1, -1)];
pub const KNIGHT: [(i8, i8); 8] = [(1, 2), (2, 1), (2, -1), (1, -2), (-1, -2), (-2, -1), (-2, 1), (-1, 2)];
