use tc_ai::{Action, Limits, choose_action};
use tc_core::spell::{SpellCast, SpellId};
use tc_core::worldgen::{GenParams, generate};
use tc_core::{Match, Position, Rules, Side, Sq, Terrain};

fn flat(fen: &str) -> Match {
    Match::new(Terrain::flat(8), Rules::standard(8), Position::from_fen(fen).unwrap())
}

#[test]
fn casts_shield_when_queen_attacked_by_pawn_and_has_no_safe_square() {
    // Black queen on a8 is attacked by white pawn on b7 (defended by white king on c6).
    // Friendly pawn on a7 and bishop on b8 prevent the queen from moving.
    // The only move that isn't captured by the pawn is taking on b7, which loses the queen to Kxc6.
    // Casting Shield on the queen protects it from the pawn capture.
    let mut game = flat("qb5k/pPp5/P1K5/8/8/8/8/8 b - - 0 1");
    game.set_charges(Side::Black, vec![(SpellId::Shield, 1)]);

    let action = choose_action(&game, Limits::depth(3, 2000.0));
    assert_eq!(
        action,
        Action::Cast(SpellCast::Shield(Sq::parse("a8").unwrap())),
        "AI should choose to shield its queen instead of losing it"
    );
}

#[test]
fn with_no_charges_it_always_returns_a_move() {
    // 1. Standard start position with no charges
    let start_game = Match::new(Terrain::flat(8), Rules::standard(8), Position::start(8));
    assert!(start_game.charges(Side::White).is_empty());
    let action = choose_action(&start_game, Limits::depth(2, 500.0));
    assert!(matches!(action, Action::Move(_)));

    // 2. Position where queen is attacked, but without charges AI must return a Move
    let mut trapped = flat("qb5k/pPp5/P1K5/8/8/8/8/8 b - - 0 1");
    trapped.set_charges(Side::Black, vec![(SpellId::Shield, 0)]);
    let action = choose_action(&trapped, Limits::depth(2, 500.0));
    assert!(matches!(action, Action::Move(_)));

    // 3. Generated board with explicit empty charges
    let (terrain, _) = generate(8, &GenParams::for_floor(8, 2));
    let mut gen_game = Match::new(terrain, Rules::standard(8), Position::start(8));
    gen_game.set_charges(Side::White, vec![]);
    let action = choose_action(&gen_game, Limits::depth(2, 500.0));
    assert!(matches!(action, Action::Move(_)));
}

#[test]
fn never_returns_an_illegal_cast() {
    let all_spells = [
        (SpellId::RaiseEarth, 2),
        (SpellId::LowerEarth, 2),
        (SpellId::Freeze, 2),
        (SpellId::Bridge, 2),
        (SpellId::DigTunnel, 1),
        (SpellId::Shield, 1),
        (SpellId::Swap, 1),
        (SpellId::Rewind, 1),
    ];

    // Test across several board types and positions
    for seed in [1u64, 42, 999] {
        let (terrain, _) = generate(seed, &GenParams::for_floor(8, 3));
        let mut game = Match::new(terrain, Rules::standard(8), Position::start(8));
        game.set_charges(Side::White, all_spells.to_vec());
        game.set_charges(Side::Black, all_spells.to_vec());

        // Play several plies and verify any cast returned is completely legal
        for _ in 0..6 {
            if game.outcome().is_some() {
                break;
            }
            let action = choose_action(&game, Limits::depth(2, 200.0));
            match action {
                Action::Move(mv) => {
                    assert!(game.legal_moves().contains(&mv), "move {:?} must be legal", mv);
                    game.play(mv).unwrap();
                }
                Action::Cast(cast) => {
                    let legal_targets = game.cast_targets(cast.spell_id());
                    assert!(legal_targets.contains(&cast), "cast {:?} must be in legal cast targets", cast);
                    let mut clone = game.clone();
                    assert!(clone.cast(cast).is_ok(), "cast {:?} must apply successfully", cast);
                    let _ = game.cast(cast);
                }
            }
        }
    }
}
