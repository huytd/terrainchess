//! One fixture per terrain rule in PLAN.md §4.

use tc_core::movegen::Ctx;
use tc_core::{
    Feature, Match, Move, MoveKind, Obstacle, PieceKind, Position, Rules, Side, Sq, Terrain, TileKind,
};

struct Fixture {
    terrain: Terrain,
    rules: Rules,
    pos: Position,
}

impl Fixture {
    fn new(fen: &str) -> Self {
        Fixture {
            terrain: Terrain::flat(8),
            rules: Rules::standard(8),
            pos: Position::from_fen(fen).unwrap(),
        }
    }

    fn h(mut self, squares: &[&str], height: u8) -> Self {
        for s in squares {
            self.terrain.get_mut(sq(s)).height = height;
        }
        self
    }

    fn kind(mut self, squares: &[&str], kind: TileKind) -> Self {
        for s in squares {
            self.terrain.get_mut(sq(s)).kind = kind;
        }
        self
    }

    fn cave(mut self, squares: &[&str], link: u8) -> Self {
        for s in squares {
            self.terrain.get_mut(sq(s)).feature = Feature::Cave(link);
        }
        self
    }

    fn obstacle(mut self, squares: &[&str], obstacle: Obstacle) -> Self {
        for s in squares {
            self.terrain.get_mut(sq(s)).feature = Feature::Obstacle(obstacle);
        }
        self
    }

    fn ctx(&self) -> Ctx<'_> {
        Ctx { terrain: &self.terrain, rules: &self.rules }
    }

    /// Destination names of legal moves from `from`, sorted.
    fn targets(&self, from: &str) -> Vec<String> {
        let mut v: Vec<String> = self
            .ctx()
            .legal_moves(&self.pos)
            .into_iter()
            .filter(|m| m.from == sq(from))
            .map(|m| m.to.name())
            .collect();
        v.sort();
        v.dedup();
        v
    }

    fn can(&self, from: &str, to: &str) -> bool {
        self.targets(from).contains(&to.to_string())
    }

    fn match_(&self) -> Match {
        Match::new(self.terrain.clone(), self.rules.clone(), self.pos.clone())
    }
}

fn sq(s: &str) -> Sq {
    Sq::parse(s).unwrap()
}

// Kings tucked in the corners so they don't get in the way.
const ROOK_A1: &str = "7k/8/8/8/8/8/8/R6K w - - 0 1";

#[test]
fn climb_of_two_is_a_cliff() {
    let f = Fixture::new(ROOK_A1).h(&["a2"], 2);
    assert!(!f.can("a1", "a2"));
    assert!(!f.can("a1", "a8"));
}

#[test]
fn uphill_step_ends_a_slide() {
    let f = Fixture::new(ROOK_A1).h(&["a3", "a4", "a5", "a6", "a7", "a8"], 1);
    assert!(f.can("a1", "a2"));
    assert!(f.can("a1", "a3"));
    assert!(!f.can("a1", "a4"), "the ray stops on the tile it climbed onto");
}

#[test]
fn long_drop_ends_a_slide_but_short_drop_does_not() {
    let f = Fixture::new(ROOK_A1).h(&["a1", "a2"], 3).h(&["a3"], 1);
    assert!(f.can("a1", "a3"));
    assert!(!f.can("a1", "a4"), "a drop of 2 stops the slide");

    let f = Fixture::new(ROOK_A1).h(&["a1", "a2"], 1);
    assert!(f.can("a1", "a8"), "a drop of 1 keeps sliding");
}

#[test]
fn knight_lands_only_within_two_levels_and_never_in_deep_water() {
    let f = Fixture::new("7k/8/8/8/8/8/8/1N5K w - - 0 1").h(&["c3"], 3).h(&["a3"], 2);
    assert!(!f.can("b1", "c3"));
    assert!(f.can("b1", "a3"));

    let f = Fixture::new("7k/8/8/8/8/8/8/1N5K w - - 0 1").kind(&["d2"], TileKind::DeepWater);
    assert!(!f.can("b1", "d2"));
}

#[test]
fn knight_ignores_terrain_in_between() {
    let f = Fixture::new("7k/8/8/8/8/8/8/1N5K w - - 0 1").h(&["b2", "c2", "b3"], 3);
    assert!(f.can("b1", "c3"));
}

#[test]
fn shallow_water_ends_a_slide_and_slows_the_next_move() {
    let f = Fixture::new(ROOK_A1).kind(&["a4"], TileKind::ShallowWater);
    assert!(f.can("a1", "a4"));
    assert!(!f.can("a1", "a5"));

    let f = Fixture::new(ROOK_A1).kind(&["a1"], TileKind::ShallowWater);
    assert_eq!(f.targets("a1"), ["a2", "b1"]);

    let f = Fixture::new("7k/8/8/8/8/8/8/1N5K w - - 0 1").kind(&["b1"], TileKind::ShallowWater);
    assert!(f.targets("b1").is_empty(), "a knight's jump is 2+ squares");
}

#[test]
fn deep_water_is_impassable() {
    let f = Fixture::new(ROOK_A1).kind(&["a3"], TileKind::DeepWater);
    assert_eq!(f.targets("a1").iter().filter(|t| t.starts_with('a')).count(), 1);
    let f = Fixture::new("7k/8/8/8/8/8/8/7K w - - 0 1").kind(&["g1", "g2", "h2"], TileKind::DeepWater);
    assert!(f.targets("h1").is_empty());
}

#[test]
fn king_cannot_climb_two() {
    let f = Fixture::new("7k/8/8/8/8/8/8/7K w - - 0 1").h(&["g1", "g2", "h2"], 2);
    assert!(f.targets("h1").is_empty());
    let f = Fixture::new("7k/8/8/8/8/8/8/7K w - - 0 1").h(&["g1"], 1);
    assert!(f.can("h1", "g1"));
}

#[test]
fn cave_hop_moves_and_captures_through_the_tunnel() {
    let f = Fixture::new("7k/8/8/8/3p4/8/8/B6K w - - 0 1").cave(&["a1", "d4"], 0);
    let moves = f.ctx().legal_moves(&f.pos);
    let hop = moves.iter().find(|m| m.from == sq("a1") && m.to == sq("d4")).unwrap();
    assert_eq!(hop.kind, MoveKind::Cave, "the capture reaches d4 through the tunnel");

    // A pawn may use a cave too, even sideways.
    let f = Fixture::new("7k/8/8/8/8/8/P7/7K w - - 0 1").cave(&["a2", "f2"], 3);
    assert!(f.can("a2", "f2"));

    // Own piece on the far entrance blocks the hop.
    let f = Fixture::new("7k/8/8/8/8/8/P4P2/7K w - - 0 1").cave(&["a2", "f2"], 3);
    assert!(!f.can("a2", "f2"));
}

#[test]
fn pawn_double_step_cannot_climb() {
    let f = Fixture::new("7k/8/8/8/8/8/4P3/7K w - - 0 1").h(&["e4"], 1);
    assert_eq!(f.targets("e2"), ["e3"]);
    let f = Fixture::new("7k/8/8/8/8/8/4P3/7K w - - 0 1").h(&["e3"], 1);
    assert_eq!(f.targets("e2"), ["e3"]);
    let f = Fixture::new("7k/8/8/8/8/8/4P3/7K w - - 0 1").h(&["e2"], 1);
    assert_eq!(f.targets("e2"), ["e3", "e4"]);
}

#[test]
fn high_ground_cannot_be_captured_from_below() {
    // Pawn: even one level up is out of reach; level or downhill is fine.
    let f = Fixture::new("7k/8/8/8/8/3p4/4P3/7K w - - 0 1").h(&["d3"], 1);
    assert!(!f.can("e2", "d3"));
    let f = Fixture::new("7k/8/8/8/8/3p4/4P3/7K w - - 0 1").h(&["d3", "e2"], 1);
    assert!(f.can("e2", "d3"));
    let f = Fixture::new("7k/8/8/8/8/3p4/4P3/7K w - - 0 1").h(&["e2"], 1);
    assert!(f.can("e2", "d3"), "capturing downhill is allowed");

    // Rook: may climb onto an empty higher square but not capture on one.
    let f = Fixture::new("7k/8/8/8/8/8/8/R2p3K w - - 0 1").h(&["b1"], 1);
    assert!(f.can("a1", "b1"));
    let f = Fixture::new("7k/8/8/8/8/8/8/Rp5K w - - 0 1").h(&["b1"], 1);
    assert!(!f.can("a1", "b1"));

    // Knight jumps can't capture uphill either.
    let f = Fixture::new("7k/8/8/8/8/8/2p5/N6K w - - 0 1").h(&["c2"], 1);
    assert!(!f.can("a1", "c2"));

    // En passant: the victim's square counts.
    let f = Fixture::new("7k/8/8/3pP3/8/8/8/7K w - d6 0 1").h(&["d5"], 1);
    assert!(!f.can("e5", "d6"));
    let f = Fixture::new("7k/8/8/3pP3/8/8/8/7K w - d6 0 1");
    assert!(f.can("e5", "d6"));

    // Cave hops can't capture onto higher ground.
    let f = Fixture::new("7k/8/8/8/3p4/8/8/B6K w - - 0 1").cave(&["a1", "d4"], 0).h(&["d4"], 1);
    assert!(!f.can("a1", "d4"));
}

#[test]
fn high_ground_king_is_not_in_check_from_below() {
    let fen = "4r2k/8/8/8/8/8/8/4K3 w - - 0 1";
    let f = Fixture::new(fen).h(&["e1"], 1);
    assert!(!f.ctx().in_check(&f.pos, tc_core::Side::White), "the rook is below the king");
    let f = Fixture::new(fen).h(&["e8"], 1);
    assert!(f.ctx().in_check(&f.pos, tc_core::Side::White), "attacking downhill still checks");
}

#[test]
fn terrain_blocks_check() {
    let fen = "4r2k/8/8/8/8/8/8/4K3 w - - 0 1";
    let f = Fixture::new(fen);
    assert!(f.ctx().in_check(&f.pos, tc_core::Side::White));
    let f = Fixture::new(fen).h(&["e4"], 2);
    assert!(!f.ctx().in_check(&f.pos, tc_core::Side::White), "the cliff at e4 stops the rook's ray");
    let f = Fixture::new(fen).h(&["e2", "e3", "e4", "e5", "e6", "e7", "e8"], 2);
    assert!(f.ctx().in_check(&f.pos, tc_core::Side::White), "a drop only ends the ray where it lands");
    let f = Fixture::new(fen).h(&["e1"], 2);
    assert!(!f.ctx().in_check(&f.pos, tc_core::Side::White), "attacking up a cliff is blocked");
}

#[test]
fn castling_needs_flat_clear_squares() {
    let fen = "4k3/8/8/8/8/8/8/R3K2R w KQ - 0 1";
    let castles = |f: &Fixture| -> Vec<String> {
        f.ctx()
            .legal_moves(&f.pos)
            .into_iter()
            .filter(|m| matches!(m.kind, MoveKind::Castle { .. }))
            .map(|m: Move| m.to.name())
            .collect()
    };
    assert_eq!(castles(&Fixture::new(fen)), ["g1", "c1"]);
    assert_eq!(castles(&Fixture::new(fen).h(&["b1"], 1)), ["g1"]);
    assert_eq!(castles(&Fixture::new(fen).kind(&["f1"], TileKind::ShallowWater)), ["c1"]);
}

#[test]
fn pawn_clears_rock_diagonally_ahead_only() {
    let f = Fixture::new("7k/8/8/8/8/8/4P3/7K w - - 0 1")
        .obstacle(&["e3"], Obstacle::Rock)
        .obstacle(&["d3"], Obstacle::Rock)
        .obstacle(&["f3"], Obstacle::Tree);
    let moves = f.ctx().legal_moves(&f.pos);
    let clears: Vec<String> = moves
        .into_iter()
        .filter(|m| m.from == sq("e2") && m.kind == MoveKind::Clear)
        .map(|m| m.uci())
        .collect();
    assert!(!clears.contains(&"e2e3x".to_string()), "straight-ahead obstacle gives no Clear move");
    assert!(clears.contains(&"e2d3x".to_string()), "left diagonal obstacle gives Clear move");
    assert!(clears.contains(&"e2f3x".to_string()), "right diagonal obstacle gives Clear move");
    assert_eq!(clears.len(), 2);
}

#[test]
fn cannot_clear_behind_or_sideways() {
    let f = Fixture::new("7k/8/8/8/8/4P3/8/7K w - - 0 1").obstacle(&["e2", "d3", "f3"], Obstacle::Rock);
    let moves = f.ctx().legal_moves(&f.pos);
    let clears: Vec<Move> =
        moves.into_iter().filter(|m| m.from == sq("e3") && m.kind == MoveKind::Clear).collect();
    assert!(clears.is_empty(), "cannot clear behind or sideways: {clears:?}");

    let f = Fixture::new("7k/8/4p3/8/8/8/8/7K b - - 0 1").obstacle(&["e7", "d6", "f6"], Obstacle::Rock);
    let moves = f.ctx().legal_moves(&f.pos);
    let clears: Vec<Move> =
        moves.into_iter().filter(|m| m.from == sq("e6") && m.kind == MoveKind::Clear).collect();
    assert!(clears.is_empty(), "black pawn cannot clear behind or sideways: {clears:?}");
}

#[test]
fn non_pawns_cannot_clear() {
    let f = Fixture::new("7k/8/8/3NBR2/3b1q2/3r1k2/8/7K w - - 0 1").obstacle(&["d4"], Obstacle::Rock);
    let moves = f.ctx().legal_moves(&f.pos);
    assert!(!moves.iter().any(|m| m.kind == MoveKind::Clear));
}

#[test]
fn pawn_clear_sacrifices_pawn_and_removes_obstacle() {
    let mut terrain = Terrain::flat(8);
    terrain.get_mut(sq("d3")).feature = Feature::Obstacle(Obstacle::Rock);
    let rules = Rules::standard(8);
    let pos = Position::from_fen("7k/8/8/8/8/8/4P3/4R2K w - - 0 1").unwrap();
    let mut m = Match::new(terrain, rules, pos);

    // Obstacle is present on d3
    assert!(!m.legal_moves().iter().any(|mv| mv.to == sq("d3") && mv.kind == MoveKind::Normal));

    // Clear the rock by sacrificing pawn at e2
    let clear_move = Move::new(sq("e2"), sq("d3"), MoveKind::Clear);
    assert_eq!(clear_move.uci(), "e2d3x");
    m.play(clear_move).unwrap();

    // After playing a Clear, the pawn square is empty and the obstacle is gone
    assert_eq!(m.pos.get(sq("e2")), None);
    assert_eq!(m.terrain.get(sq("d3")).feature, Feature::None);
    assert_eq!(m.pos.halfmove_clock, 0);

    // Black makes a king move
    m.play(Move::new(sq("h8"), sq("g8"), MoveKind::Normal)).unwrap();

    // Now e1 rook can walk to e2 (pawn square is empty)
    let rook_move = Move::new(sq("e1"), sq("e2"), MoveKind::Normal);
    assert!(m.legal_moves().contains(&rook_move));
    m.play(rook_move).unwrap();
    assert_eq!(m.pos.get(sq("e2")).map(|p| p.kind), Some(PieceKind::Rook));
}

#[test]
fn clearing_is_illegal_when_it_leaves_king_in_check() {
    let f = Fixture::new("7k/8/8/4r3/8/8/3P4/4K3 w - - 0 1").obstacle(&["e3"], Obstacle::Rock);
    let moves = f.ctx().legal_moves(&f.pos);
    let clear_e3 = Move::new(sq("d2"), sq("e3"), MoveKind::Clear);
    assert!(!moves.contains(&clear_e3), "clearing e3 would open e-file and expose king to check");

    let f = Fixture::new("7k/8/8/r7/8/8/3P4/4K3 w - - 0 1").obstacle(&["e3"], Obstacle::Rock);
    let moves = f.ctx().legal_moves(&f.pos);
    assert!(moves.contains(&clear_e3), "clearing e3 is legal when not exposing king to check");
}

#[test]
fn pinned_pawn_cannot_clear_if_that_exposes_king() {
    // White king at e1, white pawn at e2, black rook at e8.
    // Obstacle at d3. Pawn is pinned on the e-file.
    let f = Fixture::new("4r2k/8/8/8/8/8/4P3/4K3 w - - 0 1").obstacle(&["d3"], Obstacle::Rock);
    let moves = f.ctx().legal_moves(&f.pos);
    let clear_d3 = Move::new(sq("e2"), sq("d3"), MoveKind::Clear);
    assert!(
        !moves.contains(&clear_d3),
        "pinned pawn cannot sacrifice itself to clear if exposing king to check"
    );

    // If rook is on a different file, clearing d3 is legal
    let f = Fixture::new("r6k/8/8/8/8/8/4P3/4K3 w - - 0 1").obstacle(&["d3"], Obstacle::Rock);
    let moves = f.ctx().legal_moves(&f.pos);
    assert!(moves.contains(&clear_d3), "unpinned pawn can sacrifice itself to clear");
}

#[test]
fn rook_attacking_a_pawn_on_flat_ground_is_a_threat() {
    let f = Fixture::new("7k/8/8/8/8/8/4p3/4R2K w - - 0 1");
    let threats = f.match_().threats(Side::Black);
    assert!(threats.contains(&(sq("e1"), sq("e2"))));
}

#[test]
fn threat_onto_higher_ground_is_not_reported() {
    // Captures can't go uphill even one level, though the rook could still step there.
    let f = Fixture::new("7k/8/8/8/8/8/4p3/4R2K w - - 0 1").h(&["e2"], 1);
    let threats = f.match_().threats(Side::Black);
    assert!(!threats.contains(&(sq("e1"), sq("e2"))));
}

#[test]
fn shielded_piece_is_not_reported_as_a_threat() {
    let f = Fixture::new("7k/8/8/8/8/8/4p3/4R2K w - - 0 1");
    let mut m = f.match_();
    m.pos.shield = Some((sq("e2"), Side::Black));
    assert!(!m.threats(Side::Black).contains(&(sq("e1"), sq("e2"))));
}

#[test]
fn knight_fork_reports_both_pairs() {
    let f = Fixture::new("k7/8/2p3p1/4N3/8/8/8/7K w - - 0 1");
    let mut threats = f.match_().threats(Side::Black);
    threats.sort();
    assert_eq!(threats, [(sq("e5"), sq("c6")), (sq("e5"), sq("g6"))]);
}
