use std::time::Instant;

use tc_ai::{Limits, SearchJob, search_blocking};
use tc_core::worldgen::{GenParams, generate};
use tc_core::{Match, Position, Rules, Sq, Terrain};

fn clock() -> impl Fn() -> f64 {
    let start = Instant::now();
    move || start.elapsed().as_secs_f64() * 1000.0
}

fn flat(fen: &str) -> Match {
    Match::new(Terrain::flat(8), Rules::standard(8), Position::from_fen(fen).unwrap())
}

fn best(game: &Match, depth: u8) -> String {
    search_blocking(game, Limits::depth(depth, 10_000.0), &clock()).0.unwrap().uci()
}

#[test]
fn finds_back_rank_mate() {
    let game = flat("6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1");
    assert_eq!(best(&game, 3), "a1a8");
}

#[test]
fn takes_a_free_queen() {
    let game = flat("4k3/8/8/3q4/8/8/3R4/4K3 w - - 0 1");
    assert_eq!(best(&game, 3), "d2d5");
}

#[test]
fn does_not_grab_a_defended_pawn_with_the_queen() {
    let game = flat("4k3/8/2p5/3p4/8/8/3Q4/4K3 w - - 0 1");
    assert_ne!(best(&game, 4), "d2d5");
}

#[test]
fn a_cliff_makes_the_capture_impossible_so_it_is_not_counted_as_a_threat() {
    // Black's rook on e8 would hang to the rook on e1 on flat ground, but the cliff at
    // e4 blocks the file: the rook can't win it and should not try.
    let mut game = flat("4r1k1/8/8/8/8/8/8/K3R3 w - - 0 1");
    let (mv, _) = search_blocking(&game, Limits::depth(3, 10_000.0), &clock());
    assert_eq!(mv.unwrap().uci(), "e1e8", "flat: the rook trade is available");
    game.terrain.get_mut(Sq::parse("e4").unwrap()).height = 2;
    let (mv, _) = search_blocking(&game, Limits::depth(3, 10_000.0), &clock());
    assert_ne!(mv.unwrap().uci(), "e1e8");
}

#[test]
fn plays_a_legal_move_in_time_on_generated_boards() {
    for size in [8u8, 16, 32] {
        let (terrain, _) = generate(size as u64, &GenParams::for_floor(size, 4));
        let game = Match::new(terrain, Rules::standard(size), Position::start(size));
        let t0 = Instant::now();
        let (mv, info) = search_blocking(&game, Limits::depth(20, 300.0), &clock());
        let ms = t0.elapsed().as_millis();
        assert!(game.legal_moves().contains(&mv.unwrap()), "size {size}");
        assert!(ms < 600, "size {size} took {ms} ms");
        assert!(info.depth >= 1, "size {size}: {info:?}");
    }
}

#[test]
fn search_can_be_sliced() {
    let game = Match::new(Terrain::flat(8), Rules::standard(8), Position::start(8));
    let clock = clock();
    let mut job = SearchJob::new(&game, Limits::depth(4, 5_000.0));
    let mut slices = 0;
    let mv = loop {
        slices += 1;
        if let Some(mv) = job.step(&clock, 1.0) {
            break mv;
        }
    };
    assert!(slices > 1);
    assert!(game.legal_moves().contains(&mv));
    assert_eq!(job.info().depth, 4);
}

#[test]
fn no_moves_means_no_result() {
    let game = flat("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1"); // stalemate
    assert_eq!(search_blocking(&game, Limits::depth(3, 1000.0), &clock()).0, None);
}

#[test]
fn ai_clears_blocker_to_avoid_stalemate() {
    let mut terrain = Terrain::flat(8);
    let rock_sq = Sq::parse("e3").unwrap();
    terrain.get_mut(rock_sq).feature = tc_core::Feature::Obstacle(tc_core::Obstacle::Rock);
    // White king at h1, black king at f2, black rook at g2.
    // White pawn at e2 with rock at e3.
    // White king has no legal moves. Pawn cannot push because e3 is blocked.
    // Clearing the rock at e3 is the ONLY legal move that avoids stalemate.
    let pos = Position::from_fen("8/8/8/8/8/8/4Pkr1/7K w - - 0 1").unwrap();
    let game = Match::new(terrain, Rules::standard(8), pos);

    let legal = game.legal_moves();
    assert_eq!(legal.len(), 1);
    assert_eq!(legal[0].uci(), "e2e3x");

    let (mv, _) = search_blocking(&game, Limits::depth(3, 5000.0), &clock());
    assert_eq!(mv.map(|m| m.uci()), Some("e2e3x".to_string()));
    assert!(game.legal_moves().contains(&mv.unwrap()));
}
