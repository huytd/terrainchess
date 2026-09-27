//! Match state shared by the view, input and HUD.

use bevy::prelude::*;
use tc_core::worldgen::{GenParams, generate};
use tc_core::{Match, Move, Outcome, Position, Rules, Side, Sq};

#[derive(Resource)]
pub struct GameState {
    pub size: u8,
    pub seed: u64,
    pub game: Match,
    /// Earlier states, for undo in hotseat play.
    pub undo: Vec<Match>,
    pub selected: Option<Sq>,
    /// A promotion waiting for the player to choose a piece.
    pub pending_promotion: Option<Vec<Move>>,
    pub outcome: Option<Outcome>,
    /// Set when the board layout changes (new game), so the view rebuilds terrain.
    pub terrain_dirty: bool,
    /// Set when pieces or selection change, so the view rebuilds pieces/overlays.
    pub pieces_dirty: bool,
    /// The move just played, to animate.
    pub animate: Option<Move>,
    /// Side the AI plays, or `None` for hotseat.
    pub ai_side: Option<Side>,
    /// AI strength as a run floor (0 = gentle, 7 = boss).
    pub ai_level: u8,
}

pub const MAX_AI_LEVEL: u8 = 7;

impl GameState {
    pub fn new(size: u8, seed: u64, ai_side: Option<Side>, ai_level: u8) -> Self {
        let (terrain, seed) = generate(seed, &GenParams::for_floor(size, 2));
        let game = Match::new(terrain, Rules::standard(size), Position::start(size));
        GameState {
            size,
            seed,
            game,
            undo: Vec::new(),
            selected: None,
            pending_promotion: None,
            outcome: None,
            terrain_dirty: true,
            pieces_dirty: true,
            animate: None,
            ai_side,
            ai_level,
        }
    }

    /// A fresh board with the same opponent settings.
    pub fn restart(&self, size: u8, seed: u64) -> Self {
        GameState::new(size, seed, self.ai_side, self.ai_level)
    }

    pub fn ai_to_move(&self) -> bool {
        self.outcome.is_none() && self.ai_side == Some(self.game.pos.side_to_move)
    }

    pub fn play(&mut self, mv: Move) {
        let before = self.game.clone();
        if self.game.play(mv).is_ok() {
            self.undo.push(before);
            self.outcome = self.game.outcome();
            self.animate = Some(mv);
        }
        self.selected = None;
        self.pending_promotion = None;
        self.pieces_dirty = true;
    }

    /// Take back a move; against the AI, back to the player's last turn.
    pub fn undo(&mut self) {
        while let Some(prev) = self.undo.pop() {
            self.game = prev;
            if self.ai_side != Some(self.game.pos.side_to_move) {
                break;
            }
        }
        self.outcome = None;
        self.selected = None;
        self.pending_promotion = None;
        self.animate = None;
        self.pieces_dirty = true;
    }

    /// Legal moves of the selected piece.
    pub fn selected_moves(&self) -> Vec<Move> {
        match self.selected {
            Some(sq) if self.outcome.is_none() => {
                self.game.legal_moves().into_iter().filter(|m| m.from == sq).collect()
            }
            _ => Vec::new(),
        }
    }
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GameState::new(8, 1, Some(Side::Black), 2));
    }
}
