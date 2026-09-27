//! Transposition table: remembers searched positions by Zobrist hash.

use tc_core::Move;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Bound {
    Exact,
    /// The true score is at least this (a beta cutoff).
    Lower,
    /// The true score is at most this (nothing beat alpha).
    Upper,
}

#[derive(Clone, Copy)]
pub struct Entry {
    pub key: u64,
    pub mv: Option<Move>,
    pub score: i32,
    pub depth: i8,
    pub bound: Bound,
}

pub struct Tt {
    entries: Vec<Option<Entry>>,
    mask: usize,
}

impl Tt {
    /// `bits` = log2 of the entry count.
    pub fn new(bits: u32) -> Self {
        Tt { entries: vec![None; 1 << bits], mask: (1 << bits) - 1 }
    }

    pub fn probe(&self, key: u64) -> Option<Entry> {
        self.entries[key as usize & self.mask].filter(|e| e.key == key)
    }

    /// Keep the deeper result when two positions share a slot.
    pub fn store(&mut self, entry: Entry) {
        let slot = &mut self.entries[entry.key as usize & self.mask];
        if slot.is_none_or(|old| old.key != entry.key || old.depth <= entry.depth) {
            *slot = Some(entry);
        }
    }
}
