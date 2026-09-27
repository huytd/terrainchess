//! One fixture per terrain rule in PLAN.md §4.

use tc_core::movegen::Ctx;
use tc_core::{Feature, Move, MoveKind, Position, Rules, Sq, Terrain, TileKind};

struct Fixture {
    terrain: Terrain,
    rules: Rules,
    pos: Position,
}

impl Fixture {
    fn new(fen: &str) -> Self {
        Fixture { terrain: Terrain::flat(8), rules: Rules::standard(8), pos: Position::from_fen(fen).unwrap() }
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
    let f = Fixture::new("7k/8/8/8/8/3p4/4P3/7K w - - 0 1").h(&["d3"], 2);
    assert!(!f.can("e2", "d3"));
    let f = Fixture::new("7k/8/8/8/8/3p4/4P3/7K w - - 0 1").h(&["d3"], 1);
    assert!(f.can("e2", "d3"));
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
