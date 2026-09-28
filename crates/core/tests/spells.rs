//! Tests for spells and pickups in tc_core (PLAN.md §6).

use tc_core::{
    Feature, Match, Move, MoveKind, Obstacle, Pickup, PieceKind, Position, Rules, Side, SpellCast, SpellId,
    Sq, Terrain, TileKind,
};

fn sq(s: &str) -> Sq {
    Sq::parse(s).unwrap()
}

fn game_with_fen(fen: &str) -> Match {
    Match::new(Terrain::flat(8), Rules::standard(8), Position::from_fen(fen).unwrap())
}

#[test]
fn uncastable_spells_return_false_and_no_targets() {
    let mut game = game_with_fen("7k/8/8/8/8/8/8/7K w - - 0 1");
    assert!(!SpellId::Bridge.is_castable());
    assert!(!SpellId::DigTunnel.is_castable());
    assert!(!SpellId::Rewind.is_castable());

    assert!(SpellId::RaiseEarth.is_castable());
    assert!(SpellId::LowerEarth.is_castable());
    assert!(SpellId::Freeze.is_castable());
    assert!(SpellId::Shield.is_castable());
    assert!(SpellId::Swap.is_castable());

    game.set_charges(Side::White, vec![(SpellId::Bridge, 5), (SpellId::DigTunnel, 5), (SpellId::Rewind, 5)]);
    assert!(game.cast_targets(SpellId::Bridge).is_empty());
    assert!(game.cast_targets(SpellId::DigTunnel).is_empty());
    assert!(game.cast_targets(SpellId::Rewind).is_empty());
}

#[test]
fn cannot_cast_without_charges() {
    let mut game = game_with_fen("7k/8/8/8/8/8/8/7K w - - 0 1");
    assert!(game.cast_targets(SpellId::RaiseEarth).is_empty());
    assert!(game.cast(SpellCast::RaiseEarth(sq("d4"))).is_err());
}

#[test]
fn raise_earth_legality_effect_and_turn_ending() {
    let fen = "7k/8/8/8/8/8/8/R6K w - - 0 1";
    let mut game = game_with_fen(fen);
    game.set_charges(Side::White, vec![(SpellId::RaiseEarth, 1)]);

    // Blocked tiles (void or obstacle) or tiles with kings cannot be targeted
    game.terrain.get_mut(sq("b2")).kind = TileKind::Void;
    game.terrain.get_mut(sq("b3")).feature = Feature::Obstacle(Obstacle::Rock);

    let targets = game.cast_targets(SpellId::RaiseEarth);
    assert!(!targets.contains(&SpellCast::RaiseEarth(sq("b2"))));
    assert!(!targets.contains(&SpellCast::RaiseEarth(sq("b3"))));
    assert!(!targets.contains(&SpellCast::RaiseEarth(sq("h1"))), "cannot target friendly king");
    assert!(!targets.contains(&SpellCast::RaiseEarth(sq("h8"))), "cannot target enemy king");
    assert!(targets.contains(&SpellCast::RaiseEarth(sq("a1"))), "non-king piece is allowed");
    assert!(targets.contains(&SpellCast::RaiseEarth(sq("d4"))), "empty square is allowed");

    // Cast on d4: raises height from 0 to 1
    assert_eq!(game.terrain.height(sq("d4")), 0);
    assert_eq!(game.pos.side_to_move, Side::White);
    assert_eq!(game.pos.fullmove, 1);

    game.cast(SpellCast::RaiseEarth(sq("d4"))).expect("cast should succeed");
    assert_eq!(game.terrain.height(sq("d4")), 1);
    assert_eq!(game.charge_count(Side::White, SpellId::RaiseEarth), 0);
    assert_eq!(game.pos.side_to_move, Side::Black, "turn-ending spell flips side");

    // Clamping: raising height 3 stays at 3
    let mut game2 = game_with_fen(fen);
    game2.terrain.get_mut(sq("e4")).height = 3;
    game2.set_charges(Side::White, vec![(SpellId::RaiseEarth, 1)]);
    game2.cast(SpellCast::RaiseEarth(sq("e4"))).unwrap();
    assert_eq!(game2.terrain.height(sq("e4")), 3);
}

#[test]
fn lower_earth_legality_and_effect() {
    let fen = "7k/8/8/8/8/8/8/7K w - - 0 1";
    let mut game = game_with_fen(fen);
    game.terrain.get_mut(sq("d4")).height = 2;
    game.set_charges(Side::White, vec![(SpellId::LowerEarth, 1)]);

    game.cast(SpellCast::LowerEarth(sq("d4"))).expect("lower earth should succeed");
    assert_eq!(game.terrain.height(sq("d4")), 1);
    assert_eq!(game.pos.side_to_move, Side::Black);

    // Min 0
    let mut game2 = game_with_fen(fen);
    game2.terrain.get_mut(sq("d4")).height = 0;
    game2.set_charges(Side::White, vec![(SpellId::LowerEarth, 1)]);
    game2.cast(SpellCast::LowerEarth(sq("d4"))).unwrap();
    assert_eq!(game2.terrain.height(sq("d4")), 0);
}

#[test]
fn earth_spell_refused_if_it_leaves_king_in_check() {
    // White king at e1, Black rook at e8.
    // Hill at e4 of height 1: uphill step ends the rook's slide, blocking check!
    let fen = "4r2k/8/8/8/8/8/8/4K3 w - - 0 1";
    let mut game = game_with_fen(fen);
    game.terrain.get_mut(sq("e4")).height = 1;
    assert!(!game.in_check(), "uphill step onto e4 blocks check");

    game.set_charges(Side::White, vec![(SpellId::LowerEarth, 1)]);
    // Lowering e4 to height 0 flattens the file and exposes White king to check!
    let targets = game.cast_targets(SpellId::LowerEarth);
    assert!(!targets.contains(&SpellCast::LowerEarth(sq("e4"))), "lowering hill leaves king in check");
    assert!(game.cast(SpellCast::LowerEarth(sq("e4"))).is_err());
}

#[test]
fn freeze_legality_effect_and_reverting_after_6_plies() {
    let fen = "7k/8/8/8/8/8/8/R6K w - - 0 1";
    let mut game = game_with_fen(fen);
    game.terrain.get_mut(sq("a2")).kind = TileKind::DeepWater;
    game.terrain.get_mut(sq("b2")).kind = TileKind::ShallowWater;
    game.set_charges(Side::White, vec![(SpellId::Freeze, 1)]);

    // Must have at least one water tile in Chebyshev distance 1
    assert!(game.cast_targets(SpellId::Freeze).contains(&SpellCast::Freeze(sq("a1"))));
    assert!(game.cast_targets(SpellId::Freeze).contains(&SpellCast::Freeze(sq("a2"))));
    assert!(!game.cast_targets(SpellId::Freeze).contains(&SpellCast::Freeze(sq("g7"))));

    // Cast freeze on a1
    game.cast(SpellCast::Freeze(sq("a1"))).expect("freeze cast should succeed");
    assert_eq!(game.terrain.get(sq("a2")).kind, TileKind::Ice);
    assert_eq!(game.terrain.get(sq("b2")).kind, TileKind::Ice);
    assert!(!game.terrain.get(sq("a2")).is_water());
    assert_eq!(game.pos.side_to_move, Side::Black);

    // Ice is walkable like Grass: rook can slide over a2 and b2
    // Let's step 6 plies:
    // Ply 1: Black moves king h8 -> g8
    game.play(Move::new(sq("h8"), sq("g8"), MoveKind::Normal)).unwrap();
    assert_eq!(game.terrain.get(sq("a2")).kind, TileKind::Ice, "ply 1: still ice");

    // Ply 2: White moves rook a1 -> a3 (sliding across ice at a2!)
    assert!(game.legal_moves().iter().any(|m| m.from == sq("a1") && m.to == sq("a3")));
    game.play(Move::new(sq("a1"), sq("a3"), MoveKind::Normal)).unwrap();
    assert_eq!(game.terrain.get(sq("a2")).kind, TileKind::Ice, "ply 2: still ice");

    // Ply 3: Black moves g8 -> h8
    game.play(Move::new(sq("g8"), sq("h8"), MoveKind::Normal)).unwrap();
    assert_eq!(game.terrain.get(sq("a2")).kind, TileKind::Ice, "ply 3: still ice");

    // Ply 4: White moves rook a3 -> a1
    game.play(Move::new(sq("a3"), sq("a1"), MoveKind::Normal)).unwrap();
    assert_eq!(game.terrain.get(sq("a2")).kind, TileKind::Ice, "ply 4: still ice");

    // Ply 5: Black moves h8 -> g8
    game.play(Move::new(sq("h8"), sq("g8"), MoveKind::Normal)).unwrap();
    assert_eq!(game.terrain.get(sq("a2")).kind, TileKind::Ice, "ply 5: still ice");

    // Ply 6: White moves rook a1 -> b1
    game.play(Move::new(sq("a1"), sq("b1"), MoveKind::Normal)).unwrap();
    // Reverts after 6 plies!
    assert_eq!(game.terrain.get(sq("a2")).kind, TileKind::DeepWater, "a2 reverts to original DeepWater");
    assert_eq!(
        game.terrain.get(sq("b2")).kind,
        TileKind::ShallowWater,
        "b2 reverts to original ShallowWater"
    );
}

#[test]
fn shield_is_quick_protects_piece_and_removes_check() {
    // Black rook at e8, White king at e1.
    // White king is in check!
    let fen = "4r2k/8/8/8/8/8/8/4K3 w - - 0 1";
    let mut game = game_with_fen(fen);
    assert!(game.in_check(), "White king starts in check");

    game.set_charges(Side::White, vec![(SpellId::Shield, 1)]);

    // Shield can only target caster's own pieces
    let targets = game.cast_targets(SpellId::Shield);
    assert_eq!(targets, vec![SpellCast::Shield(sq("e1"))]);

    // Cast shield on e1 (White king)
    game.cast(SpellCast::Shield(sq("e1"))).expect("shield cast should succeed");
    assert_eq!(game.pos.shield, Some((sq("e1"), Side::White)));
    assert_eq!(game.pos.side_to_move, Side::White, "quick spell does NOT end turn");
    assert_eq!(game.charge_count(Side::White, SpellId::Shield), 0);

    // Shielded king is NOT in check!
    assert!(!game.in_check(), "shielded king does not count as attacked");

    // White can now make a move on the same turn!
    game.play(Move::new(sq("e1"), sq("d1"), MoveKind::Normal)).expect("White king moves to d1");
    assert_eq!(game.pos.side_to_move, Side::Black);

    // The shield follows nothing: it remains on e1!
    assert_eq!(game.pos.shield, Some((sq("e1"), Side::White)));

    // Black cannot capture onto e1 even if it puts a piece there
    // Black moves rook e8 -> e5
    game.play(Move::new(sq("e8"), sq("e5"), MoveKind::Normal)).unwrap();

    // Now it is White's turn again: the shield is cleared when White's turn begins!
    assert_eq!(game.pos.side_to_move, Side::White);
    assert_eq!(game.pos.shield, None, "shield cleared when shielded side turn begins");
}

#[test]
fn shielded_piece_cannot_be_captured() {
    // White pawn at d4, Black pawn at c5.
    let fen = "7k/8/8/2p5/3P4/8/8/7K w - - 0 1";
    let mut game = game_with_fen(fen);
    game.set_charges(Side::White, vec![(SpellId::Shield, 1)]);

    // White shields d4 pawn
    game.cast(SpellCast::Shield(sq("d4"))).unwrap();
    // White moves king h1 -> g1 to end turn
    game.play(Move::new(sq("h1"), sq("g1"), MoveKind::Normal)).unwrap();

    // Black pawn at c5 would normally capture d4 (c5d4). But d4 is shielded!
    let black_moves = game.legal_moves();
    assert!(
        !black_moves.iter().any(|m| m.from == sq("c5") && m.to == sq("d4")),
        "Black cannot capture onto shielded square d4"
    );
}

#[test]
fn swap_legality_effect_and_check_refusal() {
    // White king at e1, White rook at a1, Black rook at d8.
    // d1 is in check from Black rook!
    let fen = "3r3k/8/8/8/8/8/8/R3K3 w - - 0 1";
    let mut game = game_with_fen(fen);
    game.set_charges(Side::White, vec![(SpellId::Swap, 1)]);

    // Swapping e1 (king) and a1 (rook) would put the king on a1 (safe) and rook on e1:
    let targets = game.cast_targets(SpellId::Swap);
    assert!(targets.contains(&SpellCast::Swap(sq("a1"), sq("e1"))));

    // What if swapping puts the king on d1 (in check)?
    // Add a White bishop at d1.
    let mut game2 = game_with_fen("3r3k/8/8/8/8/8/8/3BK3 w - - 0 1");
    game2.set_charges(Side::White, vec![(SpellId::Swap, 1)]);
    // Swapping d1 and e1 would put king on d1, which is in check by d8 rook!
    let targets2 = game2.cast_targets(SpellId::Swap);
    assert!(
        !targets2.contains(&SpellCast::Swap(sq("d1"), sq("e1"))),
        "swap refused when it leaves king in check"
    );
    assert!(game2.cast(SpellCast::Swap(sq("d1"), sq("e1"))).is_err());

    // Successful swap:
    game.cast(SpellCast::Swap(sq("a1"), sq("e1"))).unwrap();
    assert_eq!(game.pos.get(sq("a1")).unwrap().kind, PieceKind::King);
    assert_eq!(game.pos.get(sq("e1")).unwrap().kind, PieceKind::Rook);
    assert_eq!(game.pos.side_to_move, Side::Black, "swap ends turn");
    assert_eq!(game.charge_count(Side::White, SpellId::Swap), 0);
}

#[test]
fn pickups_collected_by_moving_and_capturing() {
    // White knight at b1, Black pawn at c2
    let fen = "7k/8/8/8/8/8/2p5/1N5K w - - 0 1";
    let mut game = game_with_fen(fen);
    game.set_pickups(vec![(sq("a3"), Pickup::SpellCharge(SpellId::Freeze)), (sq("c2"), Pickup::RunItem)]);

    assert_eq!(game.charge_count(Side::White, SpellId::Freeze), 0);
    assert_eq!(game.run_items_collected[Side::White.index()], 0);

    // 1. Moving onto empty square a3 collects SpellCharge(Freeze)
    game.play(Move::new(sq("b1"), sq("a3"), MoveKind::Normal)).unwrap();
    assert_eq!(game.charge_count(Side::White, SpellId::Freeze), 1);
    assert_eq!(game.pickups.len(), 1, "collected pickup removed");

    // Black makes dummy move
    game.play(Move::new(sq("h8"), sq("g8"), MoveKind::Normal)).unwrap();

    // 2. Capturing on c2 collects RunItem
    game.play(Move::new(sq("a3"), sq("c2"), MoveKind::Normal)).unwrap();
    assert_eq!(game.run_items_collected[Side::White.index()], 1);
    assert!(game.pickups.is_empty(), "all pickups collected");
}
