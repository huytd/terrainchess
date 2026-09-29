use crate::hero::HeroId;
use crate::map::MapObject;
use serde::{Deserialize, Serialize};
use tc_core::piece::PieceKind;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Encounter {
    Camp(MapObject),
    Hero(HeroId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    Won,
    Retreated,
    Checkmated,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BattleResult {
    pub outcome: Outcome,
    pub lost: Vec<PieceKind>,
    pub enemy_lost: Vec<PieceKind>,
}
