use tc_ai::{Limits, evaluate, search_blocking};
use tc_core::movegen::Ctx;
use tc_core::worldgen::{GenParams, generate};
use tc_core::{Match, Piece, Position, Rules, Sq};

/// Swap colours and flip ranks; the side to move swaps too.
fn mirror(pos: &Position) -> Position {
    let mut m = Position::empty(pos.size);
    for (sq, p) in pos.pieces() {
        m.set(Sq::new(sq.x, pos.size - 1 - sq.y), Some(Piece::new(p.kind, p.side.opposite())));
    }
    m.side_to_move = pos.side_to_move.opposite();
    m
}

#[test]
fn evaluation_is_colour_symmetric_on_mirrored_boards() {
    for seed in 0..20u64 {
        let (terrain, _) = generate(seed * 31, &GenParams::for_floor(8, 3));
        let rules = Rules::standard(8);
        let mut game = Match::new(terrain.clone(), rules.clone(), Position::start(8));
        // Walk a few pseudo-random plies to get unbalanced positions.
        for i in 0..12 {
            let moves = game.legal_moves();
            if moves.is_empty() {
                break;
            }
            game.play(moves[(seed as usize * 7 + i * 13) % moves.len()]).unwrap();
            let ctx = Ctx { terrain: &terrain, rules: &rules };
            let a = evaluate(&ctx, &game.pos);
            let b = evaluate(&ctx, &mirror(&game.pos));
            assert_eq!(a, b, "seed {seed} ply {i}");
        }
    }
}

#[test]
fn search_is_colour_symmetric() {
    for seed in 0..4u64 {
        let (terrain, _) = generate(seed * 31, &GenParams::for_floor(8, 3));
        let rules = Rules::standard(8);
        let mut game = Match::new(terrain.clone(), rules.clone(), Position::start(8));
        for i in 0..6 {
            let moves = game.legal_moves();
            game.play(moves[(seed as usize * 7 + i * 13) % moves.len()]).unwrap();
        }
        let flipped = Match::new(terrain, rules, mirror(&game.pos));
        let a = search_blocking(&game, Limits::depth(3, f64::INFINITY), &|| 0.0).1;
        let b = search_blocking(&flipped, Limits::depth(3, f64::INFINITY), &|| 0.0).1;
        assert_eq!(a.score, b.score, "seed {seed}");
        let (ma, mb) = (a.best.unwrap(), b.best.unwrap());
        assert_eq!((ma.from.x, ma.to.x), (mb.from.x, mb.to.x), "seed {seed}");
    }
}
