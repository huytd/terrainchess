//! Terrain Chess rules engine. Pure Rust, no Bevy, so the rules can be tested
//! headlessly and shared with the AI (see PLAN.md §2).

pub mod board;
pub mod movegen;
pub mod piece;
pub mod position;
pub mod rng;
pub mod rules;
pub mod terrain;
pub mod worldgen;

pub use board::Sq;
pub use movegen::{Move, MoveKind};
pub use piece::{MoveProfile, Piece, PieceKind, Side};
pub use position::Position;
pub use rules::{DrawReason, Match, Outcome, Rules};
pub use terrain::{Feature, Obstacle, Terrain, Tile, TileKind};
