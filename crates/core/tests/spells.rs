//! Tests for spells and pickups in tc_core (specs/game-design.md §4).

use tc_core::{
    CastBlock, Feature, Match, Move, MoveKind, Obstacle, Pickup, PieceKind, Position, Rules, Side, SpellCast,
    SpellId, Sq, TargetBlock, Terrain, TileKind,
};

fn sq(s: &str) -> Sq {
    Sq::parse(s).unwrap()
}

fn game_with_fen(fen: &str) -> Match {
    Match::new(Terrain::flat(8), Rules::standard(8), Position::from_fen(fen).unwrap())
}

#[test]
fn all_spells_are_castable() {
    assert!(SpellId::Bridge.is_castable());
    assert!(SpellId::DigTunnel.is_castable());
    assert!(SpellId::Rewind.is_castable());
    assert!(SpellId::RaiseEarth.is_castable());
    assert!(SpellId::LowerEarth.is_castable());
    assert!(SpellId::Freeze.is_castable());
    assert!(SpellId::Shield.is_castable());
    assert!(SpellId::Swap.is_castable());
    assert!(SpellId::Smite.is_castable());
    assert!(SpellId::Evaporate.is_castable());
    assert!(SpellId::Flood.is_castable());
    assert!(SpellId::Featherfall.is_castable());
    assert!(SpellId::Curse.is_castable());
    assert!(SpellId::Sprout.is_castable());
    assert!(SpellId::Blink.is_castable());
    assert!(SpellId::Insight.is_castable());
}

#[test]
fn cannot_cast_without_cards_in_hand() {
    let mut game = game_with_fen("7k/8/8/8/8/8/8/7K w - - 0 1");
    assert!(game.cast_targets(SpellId::RaiseEarth).is_empty());
    assert!(game.cast(SpellCast::RaiseEarth(sq("d4"))).is_err());
}

#[test]
fn raise_earth_legality_effect_and_turn_ending() {
    let fen = "7k/8/8/8/8/8/8/R6K w - - 0 1";
    let mut game = game_with_fen(fen);
    game.set_deck(Side::White, vec![SpellId::RaiseEarth, SpellId::LowerEarth]);

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
    assert!(!game.hand(Side::White).used[0]);

    game.cast(SpellCast::RaiseEarth(sq("d4"))).expect("cast should succeed");
    assert_eq!(game.terrain.height(sq("d4")), 1);
    assert!(game.hand(Side::White).hand[0].is_none(), "the cast card left the hand");
    assert_eq!(game.pos.side_to_move, Side::Black, "turn-ending spell flips side");

    // Clamping: raising height 3 stays at 3
    let mut game2 = game_with_fen(fen);
    game2.terrain.get_mut(sq("e4")).height = 3;
    game2.set_deck(Side::White, vec![SpellId::RaiseEarth]);
    game2.cast(SpellCast::RaiseEarth(sq("e4"))).unwrap();
    assert_eq!(game2.terrain.height(sq("e4")), 3);
}

#[test]
fn lower_earth_legality_and_effect() {
    let fen = "7k/8/8/8/8/8/8/7K w - - 0 1";
    let mut game = game_with_fen(fen);
    game.terrain.get_mut(sq("d4")).height = 2;
    game.set_deck(Side::White, vec![SpellId::LowerEarth]);

    game.cast(SpellCast::LowerEarth(sq("d4"))).expect("lower earth should succeed");
    assert_eq!(game.terrain.height(sq("d4")), 1);
    assert_eq!(game.pos.side_to_move, Side::Black);

    // Min 0
    let mut game2 = game_with_fen(fen);
    game2.terrain.get_mut(sq("d4")).height = 0;
    game2.set_deck(Side::White, vec![SpellId::LowerEarth]);
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

    game.set_deck(Side::White, vec![SpellId::LowerEarth]);
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
    game.set_deck(Side::White, vec![SpellId::Freeze]);

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

    game.set_deck(Side::White, vec![SpellId::Shield]);

    // Shield can only target caster's own pieces
    let targets = game.cast_targets(SpellId::Shield);
    assert_eq!(targets, vec![SpellCast::Shield(sq("e1"))]);

    // Cast shield on e1 (White king)
    game.cast(SpellCast::Shield(sq("e1"))).expect("shield cast should succeed");
    assert_eq!(game.pos.shield, Some((sq("e1"), Side::White)));
    assert_eq!(game.pos.side_to_move, Side::White, "quick spell does NOT end turn");
    assert_eq!(game.hand(Side::White).hand[0], None, "used only card, redrawn empty hand");

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
    game.set_deck(Side::White, vec![SpellId::Shield]);

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
    game.set_deck(Side::White, vec![SpellId::Swap]);

    // Swapping e1 (king) and a1 (rook) would put the king on a1 (safe) and rook on e1:
    let targets = game.cast_targets(SpellId::Swap);
    assert!(targets.contains(&SpellCast::Swap(sq("a1"), sq("e1"))));

    // What if swapping puts the king on d1 (in check)?
    // Add a White bishop at d1.
    let mut game2 = game_with_fen("3r3k/8/8/8/8/8/8/3BK3 w - - 0 1");
    game2.set_deck(Side::White, vec![SpellId::Swap]);
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
}

#[test]
fn smite_destroys_rock_or_tree_obstacles() {
    let mut game = game_with_fen("7k/8/8/8/8/8/8/7K w - - 0 1");
    game.terrain.get_mut(sq("d4")).feature = Feature::Obstacle(Obstacle::Rock);
    game.terrain.get_mut(sq("e4")).feature = Feature::Obstacle(Obstacle::Tree);
    game.set_deck(Side::White, vec![SpellId::Smite]);

    let targets = game.cast_targets(SpellId::Smite);
    assert!(targets.contains(&SpellCast::Smite(sq("d4"))), "rock is a valid target");
    assert!(targets.contains(&SpellCast::Smite(sq("e4"))), "tree is a valid target");
    assert!(!targets.contains(&SpellCast::Smite(sq("d5"))), "plain tile is not a target");
    assert!(!targets.contains(&SpellCast::Smite(sq("h1"))), "king square without obstacle is not a target");

    game.cast(SpellCast::Smite(sq("d4"))).expect("smite should succeed");
    assert_eq!(game.terrain.get(sq("d4")).feature, Feature::None);
    assert_eq!(game.pos.side_to_move, Side::Black, "smite ends the turn");
}

#[test]
fn evaporate_turns_empty_water_or_ice_into_sand() {
    // White rook on f4 blocks evaporating that square.
    let mut game = game_with_fen("7k/8/8/8/5R2/8/8/7K w - - 0 1");
    game.terrain.get_mut(sq("d4")).kind = TileKind::ShallowWater;
    game.terrain.get_mut(sq("e4")).kind = TileKind::Ice;
    game.terrain.get_mut(sq("f4")).kind = TileKind::ShallowWater;
    game.set_deck(Side::White, vec![SpellId::Evaporate]);

    let targets = game.cast_targets(SpellId::Evaporate);
    assert!(targets.contains(&SpellCast::Evaporate(sq("d4"))), "shallow water is valid");
    assert!(targets.contains(&SpellCast::Evaporate(sq("e4"))), "ice is valid");
    assert!(!targets.contains(&SpellCast::Evaporate(sq("f4"))), "occupied square is not valid");
    assert!(!targets.contains(&SpellCast::Evaporate(sq("d5"))), "grass is not valid");

    game.cast(SpellCast::Evaporate(sq("d4"))).expect("evaporate should succeed");
    assert_eq!(game.terrain.get(sq("d4")).kind, TileKind::Sand);
    assert_eq!(game.pos.side_to_move, Side::Black, "evaporate ends the turn");
}

#[test]
fn flood_turns_empty_grass_or_sand_into_water_outside_home_rows() {
    // White pawn on d5 blocks flooding that square.
    let mut game = game_with_fen("7k/8/8/3P4/8/8/8/7K w - - 0 1");
    game.terrain.get_mut(sq("e4")).kind = TileKind::Sand;
    game.terrain.get_mut(sq("f4")).kind = TileKind::DeepWater;
    game.set_deck(Side::White, vec![SpellId::Flood]);

    let targets = game.cast_targets(SpellId::Flood);
    assert!(targets.contains(&SpellCast::Flood(sq("d4"))), "grass is valid");
    assert!(targets.contains(&SpellCast::Flood(sq("e4"))), "sand is valid");
    assert!(!targets.contains(&SpellCast::Flood(sq("f4"))), "deep water is not valid");
    assert!(!targets.contains(&SpellCast::Flood(sq("d5"))), "occupied square is not valid");
    assert!(!targets.contains(&SpellCast::Flood(sq("a1"))), "white home row is not valid");
    assert!(!targets.contains(&SpellCast::Flood(sq("a8"))), "black home row is not valid");

    game.cast(SpellCast::Flood(sq("d4"))).expect("flood should succeed");
    assert_eq!(game.terrain.get(sq("d4")).kind, TileKind::ShallowWater);
    assert_eq!(game.pos.side_to_move, Side::Black, "flood ends the turn");
}

#[test]
fn featherfall_ignores_climb_then_expires_on_next_move() {
    let mut game = game_with_fen("7k/8/8/8/3P4/8/8/6K1 w - - 0 1");
    game.terrain.get_mut(sq("d5")).height = 2;
    game.set_deck(Side::White, vec![SpellId::Featherfall]);

    assert!(
        !game.legal_moves().iter().any(|m| m.from == sq("d4") && m.to == sq("d5")),
        "climbing 2 is illegal without featherfall"
    );

    let targets = game.cast_targets(SpellId::Featherfall);
    assert!(targets.contains(&SpellCast::Featherfall(sq("d4"))), "own pawn is a valid target");
    assert!(!targets.contains(&SpellCast::Featherfall(sq("h8"))), "enemy king is not a target");

    game.cast(SpellCast::Featherfall(sq("d4"))).expect("featherfall should succeed");
    assert_eq!(game.pos.side_to_move, Side::White, "featherfall is quick");
    assert!(
        game.legal_moves().iter().any(|m| m.from == sq("d4") && m.to == sq("d5")),
        "feathered pawn can climb"
    );

    // Moving any of our pieces consumes the effect.
    game.play(Move::new(sq("g1"), sq("h1"), MoveKind::Normal)).unwrap();
    assert_eq!(game.pos.featherfall, None, "featherfall expires after our next move");
    assert!(
        !game.legal_moves().iter().any(|m| m.from == sq("d4") && m.to == sq("d5")),
        "climb is illegal again next turn"
    );
}

#[test]
fn featherfall_expires_after_two_plies_without_moving() {
    let mut game = game_with_fen("7k/8/8/8/3P4/8/8/6K1 w - - 0 1");
    game.set_deck(Side::White, vec![SpellId::Featherfall, SpellId::RaiseEarth]);
    game.set_deck(Side::Black, vec![SpellId::RaiseEarth]);

    game.cast(SpellCast::Featherfall(sq("d4"))).unwrap();
    assert!(game.pos.featherfall.is_some());

    // Ply 1: White casts a turn-ending spell instead of moving.
    game.cast(SpellCast::RaiseEarth(sq("d4"))).unwrap();
    assert!(game.pos.featherfall.is_some(), "one ply is not enough to expire");
    assert_eq!(game.pos.side_to_move, Side::Black);

    // Ply 2: Black casts too, and the effect times out.
    game.cast(SpellCast::RaiseEarth(sq("d4"))).unwrap();
    assert_eq!(game.pos.featherfall, None, "featherfall expires after 2 plies");
}

#[test]
fn curse_blocks_victim_for_two_turns_then_expires() {
    let mut game = game_with_fen("7k/8/8/2p5/3P4/8/8/7K w - - 0 1");
    game.set_deck(Side::White, vec![SpellId::Curse]);

    let targets = game.cast_targets(SpellId::Curse);
    assert!(targets.contains(&SpellCast::Curse(sq("c5"))), "enemy pawn is a valid target");
    assert!(!targets.contains(&SpellCast::Curse(sq("h8"))), "enemy king is immune");
    assert!(!targets.contains(&SpellCast::Curse(sq("d4"))), "own pieces are not targets");

    game.cast(SpellCast::Curse(sq("c5"))).expect("curse should succeed");
    assert_eq!(game.pos.side_to_move, Side::Black, "curse ends the turn");
    assert_eq!(game.pos.curses.len(), 1);

    // Black's first turn: the pawn is frozen.
    assert!(
        !game.legal_moves().iter().any(|m| m.from == sq("c5")),
        "cursed pawn cannot move on its first turn"
    );
    game.play(Move::new(sq("h8"), sq("g8"), MoveKind::Normal)).unwrap();
    game.play(Move::new(sq("h1"), sq("g1"), MoveKind::Normal)).unwrap();

    // Black's second turn: still frozen.
    assert_eq!(game.pos.side_to_move, Side::Black);
    assert!(
        !game.legal_moves().iter().any(|m| m.from == sq("c5")),
        "cursed pawn cannot move on its second turn"
    );
    game.play(Move::new(sq("g8"), sq("h8"), MoveKind::Normal)).unwrap();
    game.play(Move::new(sq("g1"), sq("h1"), MoveKind::Normal)).unwrap();

    // Afterwards the curse lifts.
    assert!(game.pos.curses.is_empty(), "curse expires after two turns");
    assert_eq!(game.pos.side_to_move, Side::Black);
    assert!(
        game.legal_moves().iter().any(|m| m.from == sq("c5")),
        "pawn can move again once the curse lifts"
    );
}

#[test]
fn sprout_plants_tree_on_empty_grass_outside_home_rows() {
    // White pawn on d5 blocks sprouting that square.
    let mut game = game_with_fen("7k/8/8/3P4/8/8/8/7K w - - 0 1");
    game.terrain.get_mut(sq("e4")).kind = TileKind::Sand;
    game.set_deck(Side::White, vec![SpellId::Sprout]);

    let targets = game.cast_targets(SpellId::Sprout);
    assert!(targets.contains(&SpellCast::Sprout(sq("d4"))), "empty grass is valid");
    assert!(!targets.contains(&SpellCast::Sprout(sq("e4"))), "sand is not valid");
    assert!(!targets.contains(&SpellCast::Sprout(sq("d5"))), "occupied square is not valid");
    assert!(!targets.contains(&SpellCast::Sprout(sq("a1"))), "home row is not valid");

    game.cast(SpellCast::Sprout(sq("d4"))).expect("sprout should succeed");
    assert_eq!(game.terrain.get(sq("d4")).feature, Feature::Obstacle(Obstacle::Tree));
    assert_eq!(game.pos.side_to_move, Side::Black, "sprout ends the turn");
}

#[test]
fn blink_teleports_within_two_squares() {
    let mut game = game_with_fen("7k/8/8/8/8/8/8/R3K3 w - - 0 1");
    game.set_deck(Side::White, vec![SpellId::Blink]);

    let targets = game.cast_targets(SpellId::Blink);
    assert!(targets.contains(&SpellCast::Blink(sq("a1"), sq("c1"))), "two squares away is valid");
    assert!(targets.contains(&SpellCast::Blink(sq("a1"), sq("b2"))), "diagonal is valid");
    assert!(
        !targets.iter().any(|c| matches!(c, SpellCast::Blink(from, _) if *from == sq("e1"))),
        "the king cannot blink"
    );
    assert!(!targets.contains(&SpellCast::Blink(sq("a1"), sq("d1"))), "three squares is too far");
    assert!(!targets.contains(&SpellCast::Blink(sq("a1"), sq("e1"))), "occupied square is not valid");

    game.cast(SpellCast::Blink(sq("a1"), sq("c1"))).expect("blink should succeed");
    assert_eq!(game.pos.get(sq("c1")).unwrap().kind, PieceKind::Rook);
    assert!(game.pos.get(sq("a1")).is_none());
    assert_eq!(game.pos.side_to_move, Side::Black, "blink ends the turn");
}

#[test]
fn blink_refused_when_king_stays_in_check() {
    // White king e1 is in check from the black rook on e8.
    let mut game = game_with_fen("4r2k/8/8/8/8/8/8/4K1N1 w - - 0 1");
    assert!(game.in_check());
    game.set_deck(Side::White, vec![SpellId::Blink]);

    let targets = game.cast_targets(SpellId::Blink);
    assert!(targets.contains(&SpellCast::Blink(sq("g1"), sq("e2"))), "blocking the file is legal");
    assert!(!targets.contains(&SpellCast::Blink(sq("g1"), sq("f3"))), "leaving the king in check is illegal");
    assert!(game.cast(SpellCast::Blink(sq("g1"), sq("f3"))).is_err());

    game.cast(SpellCast::Blink(sq("g1"), sq("e2"))).unwrap();
    assert!(!game.in_check(), "blocking blink removes the check");
}

#[test]
fn insight_draws_two_cards_without_ending_turn() {
    let mut game = game_with_fen("7k/8/8/8/8/8/8/7K w - - 0 1");
    game.set_deck(
        Side::White,
        vec![SpellId::Shield, SpellId::Insight, SpellId::RaiseEarth, SpellId::LowerEarth, SpellId::Swap],
    );
    // Hand is [Shield, Insight, RaiseEarth], deck is [LowerEarth, Swap].
    game.cast(SpellCast::Shield(sq("h1"))).expect("shield first to spend a slot");

    assert_eq!(game.cast_targets(SpellId::Insight), vec![SpellCast::Insight]);
    game.cast(SpellCast::Insight).expect("insight should succeed");
    assert_eq!(game.pos.side_to_move, Side::White, "insight is quick");
    // Two spent slots refilled from the deck, up to the 3-card hand limit.
    assert_eq!(
        game.hand(Side::White).hand,
        [Some(SpellId::LowerEarth), Some(SpellId::Swap), Some(SpellId::RaiseEarth)]
    );
    assert_eq!(game.hand(Side::White).used, [false, false, false]);
    assert_eq!(game.deck_len(Side::White), 0);
}

#[test]
fn insight_needs_cards_in_deck() {
    let mut game = game_with_fen("7k/8/8/8/8/8/8/7K w - - 0 1");
    game.set_deck(Side::White, vec![SpellId::Insight]);
    assert!(game.cast_targets(SpellId::Insight).is_empty());
    assert!(game.cast(SpellCast::Insight).is_err());
}

#[test]
fn pickups_collected_by_moving_and_capturing() {
    // White knight at b1, Black pawn at c2
    let fen = "7k/8/8/8/8/8/2p5/1N5K w - - 0 1";
    let mut game = game_with_fen(fen);
    game.set_pickups(vec![(sq("a3"), Pickup::RunItem), (sq("c2"), Pickup::RunItem)]);

    assert_eq!(game.run_items_collected[Side::White.index()], 0);

    // 1. Moving onto empty square a3 collects RunItem
    game.play(Move::new(sq("b1"), sq("a3"), MoveKind::Normal)).unwrap();
    assert_eq!(game.run_items_collected[Side::White.index()], 1);
    assert_eq!(game.pickups.len(), 1, "collected pickup removed");

    // Black makes dummy move
    game.play(Move::new(sq("h8"), sq("g8"), MoveKind::Normal)).unwrap();

    // 2. Capturing on c2 collects RunItem
    game.play(Move::new(sq("a3"), sq("c2"), MoveKind::Normal)).unwrap();
    assert_eq!(game.run_items_collected[Side::White.index()], 2);
    assert!(game.pickups.is_empty(), "all pickups collected");
}

#[test]
fn bridge_legality_and_effect() {
    let fen = "7k/8/8/8/8/8/4P3/4K3 w - - 0 1";
    let mut game = game_with_fen(fen);
    game.set_deck(Side::White, vec![SpellId::Bridge]);

    // e2 pawn:
    // e3 is north (orthogonal). Let's set it to ShallowWater.
    game.terrain.get_mut(sq("e3")).kind = TileKind::ShallowWater;
    // d2 is west (orthogonal). Let's set it to Void.
    game.terrain.get_mut(sq("d2")).kind = TileKind::Void;
    // f3 is northeast (diagonal). Let's set it to ShallowWater.
    game.terrain.get_mut(sq("f3")).kind = TileKind::ShallowWater;
    // b5 is far away ShallowWater.
    game.terrain.get_mut(sq("b5")).kind = TileKind::ShallowWater;

    let targets = game.cast_targets(SpellId::Bridge);
    assert!(targets.contains(&SpellCast::Bridge(sq("e3"))), "orthogonal shallow water adjacent to pawn");
    assert!(targets.contains(&SpellCast::Bridge(sq("d2"))), "orthogonal void adjacent to pawn");
    assert!(!targets.contains(&SpellCast::Bridge(sq("f3"))), "diagonal water is not allowed");
    assert!(!targets.contains(&SpellCast::Bridge(sq("b5"))), "distant water is not allowed");

    // Cast Bridge on e3:
    game.cast(SpellCast::Bridge(sq("e3"))).expect("cast bridge on e3");
    assert_eq!(game.terrain.get(sq("e3")).kind, TileKind::Bridge);
    assert!(!game.terrain.get(sq("e3")).is_water());
    assert!(!game.terrain.get(sq("e3")).is_blocked());
    assert_eq!(game.pos.side_to_move, Side::Black, "bridge ends turn");
}

#[test]
fn dig_tunnel_legality_and_effect() {
    let fen = "7k/8/8/8/8/8/8/4K3 w - - 0 1";
    let mut game = game_with_fen(fen);
    game.set_deck(Side::White, vec![SpellId::DigTunnel]);

    // Chebyshev dist <= 2 from e1 (4, 0):
    // c1 is (2, 0) -> dist 2.
    // g1 is (6, 0) -> dist 2.
    // c1 to g1 chebyshev dist is 4 >= 3.
    let targets = game.cast_targets(SpellId::DigTunnel);
    assert!(targets.contains(&SpellCast::DigTunnel(sq("c1"), sq("g1"))));

    // Squares too close (< 3 apart): e1 to d2 dist 1, c1 to d1 dist 1
    assert!(!targets.contains(&SpellCast::DigTunnel(sq("c1"), sq("d1"))));

    // Cast dig tunnel on c1 and g1:
    game.cast(SpellCast::DigTunnel(sq("c1"), sq("g1"))).expect("cast dig tunnel");
    assert_eq!(game.pos.side_to_move, Side::Black, "dig tunnel ends turn");

    // Check cave features
    let c1_feat = game.terrain.get(sq("c1")).feature;
    let g1_feat = game.terrain.get(sq("g1")).feature;
    match (c1_feat, g1_feat) {
        (Feature::Cave(link1), Feature::Cave(link2)) => {
            assert_eq!(link1, link2, "caves must share the same link id");
        }
        _ => panic!("both squares must have cave features"),
    }
    assert_eq!(game.timed_caves.len(), 1);

    // Play 8 plies (moves) to expire the timed cave
    for _ in 0..8 {
        let mv = game.legal_moves()[0];
        game.play(mv).unwrap();
    }

    // Now timed caves should have expired (reverted to Feature::None)
    assert_eq!(game.terrain.get(sq("c1")).feature, Feature::None);
    assert_eq!(game.terrain.get(sq("g1")).feature, Feature::None);
    assert!(game.timed_caves.is_empty());
}

#[test]
fn rewind_undoes_move_pair_and_spends_charge() {
    let fen = "7k/8/8/8/8/8/8/R3K3 w - - 0 1";
    let mut game = game_with_fen(fen);
    game.set_deck(Side::White, vec![SpellId::Rewind, SpellId::RaiseEarth]);

    // With 0 moves, rewind is illegal
    assert!(game.cast_targets(SpellId::Rewind).is_empty());
    assert!(game.cast(SpellCast::Rewind).is_err());

    // Ply 1: White plays Ra1 -> a2
    game.play(Move::new(sq("a1"), sq("a2"), MoveKind::Normal)).unwrap();
    // With 1 move, rewind is still illegal
    assert!(game.cast_targets(SpellId::Rewind).is_empty());
    assert!(game.cast(SpellCast::Rewind).is_err());

    // Ply 2: Black plays Kh8 -> g8
    game.play(Move::new(sq("h8"), sq("g8"), MoveKind::Normal)).unwrap();

    // Now 2 plies have been played! It is White's turn again.
    assert_eq!(game.pos.side_to_move, Side::White);
    let targets = game.cast_targets(SpellId::Rewind);
    assert_eq!(targets, vec![SpellCast::Rewind]);

    // Cast Rewind:
    game.cast(SpellCast::Rewind).expect("rewind cast succeeds");
    // Undoes the full pair: White rook back at a1, Black king back at h8!
    assert!(game.pos.get(sq("a1")).is_some());
    assert_eq!(game.pos.get(sq("a1")).unwrap().kind, PieceKind::Rook);
    assert!(game.pos.get(sq("a2")).is_none());
    assert_eq!(game.pos.get(sq("h8")).unwrap().kind, PieceKind::King);
    assert!(game.pos.get(sq("g8")).is_none());

    // The Rewind card left the hand
    assert!(game.hand(Side::White).hand[0].is_none());
    // Does NOT end turn: it is White's turn again
    assert_eq!(game.pos.side_to_move, Side::White);
}

#[test]
fn drawing_happens_once_the_hand_is_empty() {
    let fen = "7k/8/8/8/8/8/8/7K w - - 0 1";
    let mut game = game_with_fen(fen);

    // Deck of 5 cards: [Shield, Shield, Shield, RaiseEarth, LowerEarth]
    let deck =
        vec![SpellId::Shield, SpellId::Shield, SpellId::Shield, SpellId::RaiseEarth, SpellId::LowerEarth];
    game.set_deck(Side::White, deck);

    // Initial hand draws 3 cards from front:
    let hand = game.hand(Side::White);
    assert_eq!(hand.hand, [Some(SpellId::Shield), Some(SpellId::Shield), Some(SpellId::Shield)]);
    assert_eq!(hand.used, [false, false, false]);
    assert_eq!(game.deck_len(Side::White), 2);

    // Cast 1st Shield on h1 (quick spell, does not end turn)
    game.cast(SpellCast::Shield(sq("h1"))).unwrap();
    let hand = game.hand(Side::White);
    assert_eq!(hand.hand, [None, Some(SpellId::Shield), Some(SpellId::Shield)], "slot 0 emptied");
    assert_eq!(game.deck_len(Side::White), 2, "deck still has 2 cards");

    // Cast 2nd Shield on h1
    game.cast(SpellCast::Shield(sq("h1"))).unwrap();
    let hand = game.hand(Side::White);
    assert_eq!(hand.hand, [None, None, Some(SpellId::Shield)], "slots 0 and 1 emptied");

    // Cast 3rd Shield on h1: the hand is now empty, so up to 3 cards are drawn.
    game.cast(SpellCast::Shield(sq("h1"))).unwrap();
    let hand = game.hand(Side::White);
    assert_eq!(
        hand.hand,
        [Some(SpellId::RaiseEarth), Some(SpellId::LowerEarth), None],
        "remaining 2 cards drawn, 3rd slot stays None"
    );
    assert_eq!(hand.used, [false, false, false], "new hand has unused slots");
    assert_eq!(game.deck_len(Side::White), 0, "deck is now empty");
}

#[test]
fn discard_rules() {
    let fen = "7k/8/8/8/8/8/8/7K w - - 0 1";
    let mut game = game_with_fen(fen);

    // Initial state: hand is empty, cannot discard any slot
    for slot in 0..3 {
        assert!(!game.can_discard(Side::White, slot));
        assert!(game.discard(Side::White, slot).is_err());
    }

    // Give White a deck with 8 cards:
    // Hand draws first 3: [RaiseEarth, LowerEarth, Shield]
    // Remaining in deck: [Swap, Freeze, Bridge, DigTunnel, Rewind]
    let deck = vec![
        SpellId::RaiseEarth,
        SpellId::LowerEarth,
        SpellId::Shield,
        SpellId::Swap,
        SpellId::Freeze,
        SpellId::Bridge,
        SpellId::DigTunnel,
        SpellId::Rewind,
    ];
    game.set_deck(Side::White, deck);
    assert_eq!(game.discards_left(Side::White), 5);
    assert_eq!(game.deck_len(Side::White), 5);
    assert_eq!(
        game.hand(Side::White).hand,
        [Some(SpellId::RaiseEarth), Some(SpellId::LowerEarth), Some(SpellId::Shield)]
    );

    let hash_before = game.pos.hash();
    let turn_before = game.pos.side_to_move;

    // 1. Discarding a slot replaces only that card and puts it in the discard pile
    assert!(game.can_discard(Side::White, 1));
    game.discard(Side::White, 1).expect("discarding slot 1 should succeed");

    assert_eq!(
        game.hand(Side::White).discarded,
        vec![SpellId::LowerEarth],
        "discarded card placed in discard pile"
    );
    assert_eq!(
        game.hand(Side::White).hand,
        [Some(SpellId::RaiseEarth), Some(SpellId::Swap), Some(SpellId::Shield)],
        "only slot 1 replaced, drawn from top of deck"
    );
    assert_eq!(game.deck_len(Side::White), 4);
    assert_eq!(game.discards_left(Side::White), 4);

    // Does not end turn and does not change position hash:
    assert_eq!(game.pos.side_to_move, turn_before);
    assert_eq!(game.pos.hash(), hash_before);

    // 2. A cast card leaves the hand, so its empty slot can't be discarded
    // Cast quick spell Shield (slot 2)
    game.cast(SpellCast::Shield(sq("h1"))).unwrap();
    assert!(game.hand(Side::White).hand[2].is_none(), "slot 2 is now empty");
    assert!(!game.can_discard(Side::White, 2), "empty slot cannot be discarded");
    assert!(game.discard(Side::White, 2).is_err(), "empty slot discard returns error");

    // The other cards can still be discarded:
    assert!(game.can_discard(Side::White, 0));
    assert!(game.can_discard(Side::White, 1));

    // 3. Discard limit: each side may discard at most 5 times per match (the 6th discard fails)
    // Currently discards_left == 4. We will perform 4 more discards:
    game.discard(Side::White, 0).expect("discard #2 should succeed");
    assert_eq!(game.discards_left(Side::White), 3);
    assert_eq!(game.hand(Side::White).hand[0], Some(SpellId::Freeze));

    game.discard(Side::White, 0).expect("discard #3 should succeed");
    assert_eq!(game.discards_left(Side::White), 2);
    assert_eq!(game.hand(Side::White).hand[0], Some(SpellId::Bridge));

    game.discard(Side::White, 0).expect("discard #4 should succeed");
    assert_eq!(game.discards_left(Side::White), 1);
    assert_eq!(game.hand(Side::White).hand[0], Some(SpellId::DigTunnel));

    game.discard(Side::White, 0).expect("discard #5 should succeed");
    assert_eq!(game.discards_left(Side::White), 0);
    assert_eq!(game.hand(Side::White).hand[0], Some(SpellId::Rewind));

    // 6th discard fails because discards_left == 0
    assert!(!game.can_discard(Side::White, 0));
    assert!(game.discard(Side::White, 0).is_err());

    // 4. Empty deck forbids discarding
    let mut game2 = game_with_fen(fen);
    game2.set_deck(Side::White, vec![SpellId::RaiseEarth, SpellId::LowerEarth, SpellId::Shield]);
    assert_eq!(game2.deck_len(Side::White), 0);
    assert_eq!(game2.discards_left(Side::White), 5);
    assert!(!game2.can_discard(Side::White, 0));
    assert!(game2.discard(Side::White, 0).is_err());

    // 5. Counter resets for a new match
    let mut new_game = game_with_fen(fen);
    new_game.set_deck(
        Side::White,
        vec![SpellId::RaiseEarth, SpellId::LowerEarth, SpellId::Shield, SpellId::Swap],
    );
    assert_eq!(new_game.discards_left(Side::White), 5);
}

#[test]
fn discarding_last_card_draws_full_hand() {
    let fen = "7k/8/8/8/8/8/8/7K w - - 0 1";
    let mut game = game_with_fen(fen);

    let deck = vec![
        SpellId::RaiseEarth,
        SpellId::LowerEarth,
        SpellId::Shield,
        SpellId::Swap,
        SpellId::Freeze,
        SpellId::Bridge,
        SpellId::DigTunnel,
        SpellId::Rewind,
    ];
    game.set_deck(Side::White, deck);

    // 1. With 2 cards in hand, discarding one still refills only that slot (other slot unchanged).
    game.hands[Side::White.index()].hand = [Some(SpellId::RaiseEarth), Some(SpellId::LowerEarth), None];
    let deck_len_before = game.deck_len(Side::White);
    game.discard(Side::White, 0).expect("discarding slot 0 should succeed");
    assert_eq!(
        game.hand(Side::White).hand,
        [Some(SpellId::Swap), Some(SpellId::LowerEarth), None],
        "only slot 0 refilled with next card from deck, slot 1 unchanged, slot 2 still None"
    );
    assert_eq!(game.deck_len(Side::White), deck_len_before - 1);

    // 2. With only 1 card in hand (only slot 1 holds a card and slots 0 and 2 are None),
    // discarding it draws a fresh hand of up to 3 cards.
    game.hands[Side::White.index()].hand = [None, Some(SpellId::LowerEarth), None];
    let deck_len_before = game.deck_len(Side::White);
    game.discard(Side::White, 1).expect("discarding last card should succeed");

    let hand = game.hand(Side::White);
    assert!(hand.hand.iter().all(Option::is_some), "all 3 slots are Some");
    assert_eq!(hand.hand, [Some(SpellId::Freeze), Some(SpellId::Bridge), Some(SpellId::DigTunnel)]);
    assert_eq!(hand.used, [false; 3], "used == [false; 3]");
    assert_eq!(game.deck_len(Side::White), deck_len_before - 3, "deck shrank by 3");
}

#[test]
fn spell_hand_serde_default_discards_left() {
    let ron_str = "(deck: [], hand: (None, None, None), used: (false, false, false), discarded: [])";
    let hand: tc_core::SpellHand = ron::from_str(ron_str).expect("should deserialize old SpellHand");
    assert_eq!(hand.discards_left, 5);
}

#[test]
fn tunneler_bishops_slide_through_caves() {
    // White bishop at c1. Cave entrance at e3 linked to cave exit at b6.
    let fen = "7k/8/8/8/8/8/8/2B1K3 w - - 0 1";
    let mut game = game_with_fen(fen);
    game.terrain.get_mut(sq("e3")).feature = Feature::Cave(1);
    game.terrain.get_mut(sq("b6")).feature = Feature::Cave(1);

    // Standard bishop without cave_slide cannot reach b6
    let moves = game.legal_moves();
    assert!(!moves.iter().any(|m| m.to == sq("b6")));

    // Enable cave_slide for White's bishop
    let white_bishop = tc_core::Piece::new(PieceKind::Bishop, Side::White);
    game.rules.profile_mut(white_bishop).cave_slide = true;

    let moves2 = game.legal_moves();
    let cave_move = moves2.iter().find(|m| m.from == sq("c1") && m.to == sq("b6"));
    assert!(cave_move.is_some(), "tunneler bishop can slide into e3 and emerge at b6");
    assert_eq!(cave_move.unwrap().kind, MoveKind::Cave);

    // Play the cave move: bishop ends up at b6
    game.play(*cave_move.unwrap()).unwrap();
    assert_eq!(game.pos.get(sq("b6")).unwrap().kind, PieceKind::Bishop);
    assert!(game.pos.get(sq("c1")).is_none());
}

#[test]
fn veteran_pushes_piece_toward_back_rank_once() {
    // White pawn at d4, Black rook at d8.
    let fen = "3r3k/8/8/8/3P4/8/8/4K3 w - - 0 1";
    let mut game = game_with_fen(fen);
    game.set_veteran(Side::White, true);

    // White plays dummy move Ke1 -> f1
    game.play(Move::new(sq("e1"), sq("f1"), MoveKind::Normal)).unwrap();

    // Black rook captures d8 -> d4
    game.play(Move::new(sq("d8"), sq("d4"), MoveKind::Normal)).unwrap();

    // Veteran activates! White pawn on d4 is pushed toward its back rank (d3),
    // and Black rook lands on d4.
    assert_eq!(game.pos.get(sq("d4")).unwrap().kind, PieceKind::Rook);
    assert_eq!(game.pos.get(sq("d4")).unwrap().side, Side::Black);
    assert_eq!(game.pos.get(sq("d3")).unwrap().kind, PieceKind::Pawn);
    assert_eq!(game.pos.get(sq("d3")).unwrap().side, Side::White);
    // Veteran flag is cleared
    assert!(!game.veteran[Side::White.index()]);

    // If Black rook captures d3 next, pawn is captured normally
    game.play(Move::new(sq("f1"), sq("e1"), MoveKind::Normal)).unwrap();
    game.play(Move::new(sq("d4"), sq("d3"), MoveKind::Normal)).unwrap();
    assert_eq!(game.pos.get(sq("d3")).unwrap().kind, PieceKind::Rook);
    assert_eq!(game.pos.get(sq("d3")).unwrap().side, Side::Black);
}

#[test]
fn cast_block_explains_unusable_cards() {
    let mut game = game_with_fen("4k3/8/8/8/8/8/P7/4K3 w - - 0 1");
    assert_eq!(game.cast_block(SpellId::Freeze), Some(CastBlock::CardUsed));
    game.set_deck(Side::White, vec![SpellId::Freeze, SpellId::Rewind, SpellId::RaiseEarth]);
    assert_eq!(game.cast_block(SpellId::Freeze), Some(CastBlock::NoTargets(SpellId::Freeze)));
    assert_eq!(game.cast_block(SpellId::Rewind), Some(CastBlock::TooEarly));
    assert_eq!(game.cast_block(SpellId::RaiseEarth), None);
}

#[test]
fn target_block_explains_wrong_squares() {
    let mut game = game_with_fen("4k3/8/8/8/8/8/P7/4K3 w - - 0 1");
    game.set_deck(Side::White, vec![SpellId::RaiseEarth, SpellId::Curse, SpellId::Shield]);
    assert_eq!(game.target_block(SpellId::RaiseEarth, sq("e1")), Some(TargetBlock::KingSquare));
    assert_eq!(game.target_block(SpellId::RaiseEarth, sq("d4")), None);
    assert_eq!(game.target_block(SpellId::Shield, sq("d4")), Some(TargetBlock::NotATarget(SpellId::Shield)));
    assert_eq!(game.cast_block(SpellId::Curse), Some(CastBlock::NoTargets(SpellId::Curse)));
}

#[test]
fn swap_onto_last_rank_promotes_pawn() {
    let mut game = game_with_fen("1R2k3/8/8/8/8/8/1P6/4K3 w - - 0 1");
    game.set_deck(Side::White, vec![SpellId::Swap, SpellId::Swap, SpellId::Swap]);
    game.cast(SpellCast::Swap(sq("b2"), sq("b8"))).unwrap();
    assert_eq!(game.pos.get(sq("b8")).unwrap().kind, PieceKind::Queen);
    assert_eq!(game.pos.get(sq("b2")).unwrap().kind, PieceKind::Rook);
}

#[test]
fn reshaping_the_board_is_not_a_repetition() {
    let mut game = game_with_fen("4k3/8/8/8/8/8/P7/4K3 w - - 0 1");
    let deck = vec![SpellId::RaiseEarth; 12];
    game.set_deck(Side::White, deck.clone());
    game.set_deck(Side::Black, deck);
    // Both sides only raise earth; the pieces never move, but the board keeps changing.
    for sq_name in ["c3", "f6", "d3", "e6", "c4", "f5"] {
        game.cast(SpellCast::RaiseEarth(sq(sq_name))).unwrap();
    }
    assert_eq!(game.outcome(), None);
}
