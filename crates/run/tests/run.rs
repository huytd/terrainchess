//! Integration tests for tc_run (specs/game-design.md §4, specs/project-plan.md §5 and /tmp/m5_phase1.md).

use std::collections::HashSet;

use tc_core::board::Sq;
use tc_core::piece::{Piece, PieceKind, Side};
use tc_core::position::Position;
use tc_core::{Pickup, SpellId, TileKind};
use tc_run::{Rarity, RunError, RunOutcome, RunState, catalog, load_all_items};

#[test]
fn item_files_parse_and_ids_are_unique() {
    let items = load_all_items().expect("item files must parse cleanly");
    assert!(!items.is_empty(), "item catalog should not be empty");

    let mut ids = HashSet::new();
    for item in &items {
        assert!(ids.insert(&item.id), "duplicate item id detected: {}", item.id);
        assert!(!item.name.is_empty(), "item name should not be empty");
        assert!(!item.description.is_empty(), "item description should not be empty");
    }

    // Check specific required items from brief.
    let expected_enhancements = [
        "mountaineer_rooks",
        "amphibious_knights",
        "surefooted_pawns",
        "momentum_bishops",
        "long_jump_knights",
        "daring_queens",
        "tunneler_bishops",
        "veteran",
    ];
    for id in expected_enhancements {
        assert!(ids.contains(&id.to_string()), "missing enhancement item: {id}");
    }

    let expected_relics = ["tectonic_pact", "calm_terrain", "tide_charm", "cartographer"];
    for id in expected_relics {
        assert!(ids.contains(&id.to_string()), "missing relic item: {id}");
    }

    let expected_spells =
        ["raise_earth", "lower_earth", "freeze", "bridge", "dig_tunnel", "shield", "swap", "rewind"];
    for id in expected_spells {
        assert!(ids.contains(&id.to_string()), "missing spell item: {id}");
    }
}

#[test]
fn every_draft_is_three_distinct_unowned_items() {
    for seed in [1, 42, 999, 12345, 987654321] {
        let mut run = RunState::new(seed, 8);

        for floor in 0..5 {
            let draft = run.draft();
            // 3 distinct items.
            assert_ne!(draft[0], draft[1], "draft items 0 and 1 must be distinct on seed {seed}");
            assert_ne!(draft[1], draft[2], "draft items 1 and 2 must be distinct on seed {seed}");
            assert_ne!(draft[0], draft[2], "draft items 0 and 2 must be distinct on seed {seed}");

            // None of the drafted items are already owned.
            for id in &draft {
                assert!(!run.owned.contains(id), "draft offered already owned item '{id}'");
            }

            // Rare items only appear from floor 2 onwards.
            if floor < 2 {
                for id in &draft {
                    let item = catalog().iter().find(|i| &i.id == id).unwrap();
                    assert_ne!(item.rarity, Rarity::Rare, "rare item '{id}' offered before floor 2");
                }
            }

            // Pick one item and advance floor.
            run.pick(&draft[0]).expect("pick should succeed");
            assert!(run.owned.contains(&draft[0]));
            run.record_result(true, 0);
        }
    }
}

#[test]
fn enhancements_change_only_the_player_profile_and_are_clamped() {
    let mut run = RunState::new(42, 8);

    // Initial rules profile check.
    let initial_setup = run.match_setup();
    let white_rook = Piece::new(PieceKind::Rook, Side::White);
    let black_rook = Piece::new(PieceKind::Rook, Side::Black);
    assert_eq!(initial_setup.rules.profile(white_rook).max_climb, 1);
    assert_eq!(initial_setup.rules.profile(black_rook).max_climb, 1);

    // Add mountaineer_rooks (+1 max_climb to rooks).
    run.owned.push("mountaineer_rooks".into());
    let setup = run.match_setup();
    assert_eq!(setup.rules.profile(white_rook).max_climb, 2);
    assert_eq!(setup.rules.profile(black_rook).max_climb, 1, "black rook profile should remain unchanged");

    // Add amphibious_knights (deep_water true for knights).
    let white_knight = Piece::new(PieceKind::Knight, Side::White);
    let black_knight = Piece::new(PieceKind::Knight, Side::Black);
    run.owned.push("amphibious_knights".into());
    let setup = run.match_setup();
    assert!(setup.rules.profile(white_knight).deep_water);
    assert!(!setup.rules.profile(black_knight).deep_water, "black knight deep water should remain false");

    // Add momentum_bishops (uphill_ends_slide false for bishops).
    let white_bishop = Piece::new(PieceKind::Bishop, Side::White);
    let black_bishop = Piece::new(PieceKind::Bishop, Side::Black);
    run.owned.push("momentum_bishops".into());
    let setup = run.match_setup();
    assert!(!setup.rules.profile(white_bishop).uphill_ends_slide);
    assert!(
        setup.rules.profile(black_bishop).uphill_ends_slide,
        "black bishop uphill ends slide should remain true"
    );

    // Add long_jump_knights (+1 jump_max_dh for knights).
    run.owned.push("long_jump_knights".into());
    let setup = run.match_setup();
    assert_eq!(setup.rules.profile(white_knight).jump_max_dh, 3);
    assert_eq!(setup.rules.profile(black_knight).jump_max_dh, 2);

    // Add daring_queens (+1 max_slide_drop for queens).
    let white_queen = Piece::new(PieceKind::Queen, Side::White);
    let black_queen = Piece::new(PieceKind::Queen, Side::Black);
    run.owned.push("daring_queens".into());
    let setup = run.match_setup();
    assert_eq!(setup.rules.profile(white_queen).max_slide_drop, 2);
    assert_eq!(setup.rules.profile(black_queen).max_slide_drop, 1);

    // Test clamping upper bound: 0..=3 range.
    // Standard rook max_climb is 1. Adding mountaineer_rooks 5 times would be 6, clamped to 3.
    run.owned.push("mountaineer_rooks".into());
    run.owned.push("mountaineer_rooks".into());
    run.owned.push("mountaineer_rooks".into());
    run.owned.push("mountaineer_rooks".into());
    run.owned.push("mountaineer_rooks".into());
    let setup = run.match_setup();
    assert_eq!(setup.rules.profile(white_rook).max_climb, 3, "max_climb must clamp to 3");
    assert_eq!(setup.rules.profile(black_rook).max_climb, 1, "black rook profile must remain unaffected");

    // Knight jump clamping: standard 2 + multiple upgrades should clamp to 3.
    run.owned.push("long_jump_knights".into());
    run.owned.push("long_jump_knights".into());
    let setup = run.match_setup();
    assert_eq!(setup.rules.profile(white_knight).jump_max_dh, 3, "jump_max_dh must clamp to 3");

    // Queen slide drop clamping: standard 1 + multiple upgrades should clamp to 3.
    run.owned.push("daring_queens".into());
    run.owned.push("daring_queens".into());
    run.owned.push("daring_queens".into());
    let setup = run.match_setup();
    assert_eq!(setup.rules.profile(white_queen).max_slide_drop, 3, "max_slide_drop must clamp to 3");
}

#[test]
fn tectonic_pact_raises_exactly_the_player_home_ranks() {
    for size in [8u8, 16u8] {
        let run_without_pact = RunState::new(12345, size);
        let setup_without_pact = run_without_pact.match_setup();
        let (terrain_without, seed_without) = setup_without_pact.terrain();

        let mut run_with_pact = RunState::new(12345, size);
        run_with_pact.owned.push("tectonic_pact".into());
        let setup_with_pact = run_with_pact.match_setup();
        let (terrain_with, seed_with) = setup_with_pact.terrain();

        assert_eq!(seed_without, seed_with, "base terrain seeds should match");

        let home_rows = Position::home_rows(size);
        for y in 0..size {
            for x in 0..size {
                let sq = Sq::new(x, y);
                let h_without = terrain_without.height(sq);
                let h_with = terrain_with.height(sq);

                if y < home_rows {
                    // White's home ranks are raised by 1, clamped to max height 3.
                    let expected = (h_without + 1).min(tc_core::terrain::MAX_HEIGHT);
                    assert_eq!(h_with, expected, "sq ({x}, {y}) in player home rank should be raised by 1");
                } else {
                    // Outside player home ranks (including opponent's home ranks), exactly unchanged.
                    assert_eq!(
                        h_with, h_without,
                        "sq ({x}, {y}) outside player home ranks should not be modified"
                    );
                }
            }
        }
    }
}

#[test]
fn calm_terrain_relic_adjusts_generation_roughness() {
    let run_normal = RunState::new(42, 8);
    let setup_normal = run_normal.match_setup();

    let mut run_calm = RunState::new(42, 8);
    run_calm.owned.push("calm_terrain".into());
    let setup_calm = run_calm.match_setup();

    assert!(
        (setup_calm.r#gen.roughness - (setup_normal.r#gen.roughness - 0.15).max(0.0)).abs() < 1e-6,
        "calm terrain should reduce roughness by 0.15"
    );
}

#[test]
fn save_round_trips_and_reloaded_run_rolls_same_draft() {
    let mut run = RunState::new(42, 8);
    run.record_result(true, 0);
    run.record_result(true, 0);
    let d = run.draft();
    run.pick(&d[0]).expect("pick should succeed");

    let ron_str = run.to_ron().expect("serialization to ron should succeed");
    let mut reloaded = RunState::from_ron(&ron_str).expect("deserialization from ron should succeed");

    assert_eq!(run, reloaded, "reloaded run state should match original");

    // Both roll the same draft since rng state is preserved.
    let draft_orig = run.draft();
    let draft_reloaded = reloaded.draft();
    assert_eq!(draft_orig, draft_reloaded, "reloaded run must roll the exact same draft");
    assert_eq!(run.rng, reloaded.rng, "rng state must advance identically on both");

    // Save with active draft also round-trips.
    let ron_with_draft = run.to_ron().unwrap();
    let reloaded_with_draft = RunState::from_ron(&ron_with_draft).unwrap();
    assert_eq!(run, reloaded_with_draft);
    assert_eq!(reloaded_with_draft.last_draft, Some(draft_orig));
}

#[test]
fn winning_eight_matches_sets_won_and_one_loss_sets_lost() {
    let mut run = RunState::new(100, 8);
    assert_eq!(run.floor, 0);
    assert_eq!(run.outcome, None);

    // Win 7 matches through normal floors 0..=6.
    for f in 0..7 {
        let setup = run.match_setup();
        assert_eq!(setup.ai_level, f + 1, "ai level should rise with floor");
        assert_eq!(run.floor, f);
        assert_eq!(run.outcome, None);
        run.record_result(true, 0);
    }

    // Now at boss floor (index 7).
    assert_eq!(run.floor, 7);
    let boss_setup = run.match_setup();
    assert_eq!(boss_setup.ai_level, 7, "boss AI level should be 7");
    assert_eq!(run.outcome, None);

    // Win boss match (8th match total).
    run.record_result(true, 0);
    assert_eq!(run.outcome, Some(RunOutcome::Won), "winning 8 matches sets Won");

    // A fresh run experiencing a loss sets Lost.
    let mut run2 = RunState::new(200, 8);
    run2.record_result(true, 0);
    assert_eq!(run2.floor, 1);
    run2.record_result(false, 0);
    assert_eq!(run2.outcome, Some(RunOutcome::Lost), "one loss sets Lost");
    assert_eq!(run2.floor, 1, "floor should not change after a loss");
}

#[test]
fn same_run_seed_and_floor_give_same_match_seed() {
    let run1 = RunState::new(42, 8);
    let run2 = RunState::new(42, 8);
    assert_eq!(
        run1.match_setup().seed,
        run2.match_setup().seed,
        "identical run seed and floor must produce identical match seed"
    );

    for floor in 0..8 {
        assert_eq!(RunState::match_seed(999, floor), RunState::match_seed(999, floor));
    }

    // Different floor gives different seed.
    assert_ne!(
        RunState::match_seed(999, 0),
        RunState::match_seed(999, 1),
        "different floors should produce different match seeds"
    );

    // Different run seed gives different match seed.
    assert_ne!(
        RunState::match_seed(100, 0),
        RunState::match_seed(200, 0),
        "different run seeds should produce different match seeds"
    );
}

#[test]
fn pick_validates_last_draft() {
    let mut run = RunState::new(555, 8);

    // Cannot pick without drafting.
    assert_eq!(run.pick("mountaineer_rooks"), Err(RunError::NoDraftAvailable));

    let draft = run.draft();

    // Cannot pick item not offered in draft.
    assert_eq!(run.pick("nonexistent_item_id"), Err(RunError::NotInDraft("nonexistent_item_id".into())));

    // Picking a valid offered item succeeds.
    let chosen = &draft[1];
    assert!(run.pick(chosen).is_ok());
    assert!(run.owned.contains(chosen));

    // After picking, the draft is consumed and cannot pick again.
    assert_eq!(run.pick(&draft[0]), Err(RunError::NoDraftAvailable));
}

#[test]
fn pickup_placement_is_deterministic_and_only_on_valid_squares() {
    for size in [8u8, 16, 32] {
        let run = RunState::new(12345, size);
        let setup = run.match_setup();
        // Floor 0 without cartographer places no RunItem pickups (spell charges are no longer placed)
        assert!(setup.pickups.is_empty());

        // Deterministic: second setup with same run state produces identical pickups
        let setup2 = run.match_setup();
        assert_eq!(setup.pickups, setup2.pickups);
    }

    // From floor 2 onwards, RunItem has a 30% chance to spawn for one of the pickups
    let mut found_run_item = false;
    for seed in 0..50u64 {
        let mut run = RunState::new(seed, 8);
        run.floor = 2;
        let setup = run.match_setup();
        let run_items = setup.pickups.iter().filter(|(_, p)| matches!(p, Pickup::RunItem)).count();
        assert!(run_items <= 1, "at most one pickup should be a RunItem");
        if run_items == 1 {
            found_run_item = true;
            let (sq, pickup) = setup.pickups[0];
            assert_eq!(pickup, Pickup::RunItem);
            let (terrain, _) = setup.terrain();
            let home = Position::home_rows(8);
            let start_pos = Position::start(8);
            assert!(sq.y >= home && sq.y < 8 - home, "pickup must be on non-home rows");
            assert!(start_pos.get(sq).is_none(), "pickup must not spawn on starting piece");
            let tile = terrain.get(sq);
            assert!(!tile.is_blocked(), "pickup cannot be on blocked tile");
            assert_ne!(tile.kind, TileKind::DeepWater, "pickup cannot be in deep water");
        }
    }
    assert!(
        found_run_item,
        "over 50 seeds on floor 2, at least one match should place a RunItem (30% chance)"
    );
}

#[test]
fn bonus_picks_from_run_items() {
    let mut run = RunState::new(42, 8);
    assert_eq!(run.bonus_picks, 0);

    // Winning a match with 2 collected run items awards 2 bonus picks
    run.record_result(true, 2);
    assert_eq!(run.bonus_picks, 2);
    assert_eq!(run.floor, 1);

    // Normal draft between floors
    let d0 = run.draft();
    run.pick(&d0[0]).expect("normal draft pick");

    // The game calls draft again while bonus_picks > 0
    let mut bonus_count = 0;
    while run.bonus_picks > 0 {
        run.bonus_picks -= 1;
        let d = run.draft();
        run.pick(&d[0]).expect("bonus draft pick");
        bonus_count += 1;
    }
    assert_eq!(bonus_count, 2);
    assert_eq!(run.bonus_picks, 0);
    assert_eq!(run.owned.len(), 3, "1 normal pick + 2 bonus picks = 3 owned items");
}

#[test]
fn player_deck_from_owned_spell_items() {
    let mut run = RunState::new(99, 8);
    let initial_setup = run.match_setup();
    assert_eq!(initial_setup.player_deck.len(), 15);
    assert_eq!(initial_setup.enemy_deck.len(), 15);

    // Add raise_earth (2 charges in spells.ron) and shield (1 charge)
    run.owned.push("raise_earth".into());
    run.owned.push("shield".into());

    let setup = run.match_setup();
    assert_eq!(setup.player_deck.len(), 15);
    assert_eq!(setup.enemy_deck.len(), 15);

    let count_raise = setup.player_deck.iter().filter(|&&s| s == SpellId::RaiseEarth).count();
    let count_shield = setup.player_deck.iter().filter(|&&s| s == SpellId::Shield).count();
    assert!(count_raise >= 2, "must have at least 2 RaiseEarth from owned item");
    assert!(count_shield >= 1, "must have at least 1 Shield from owned item");
}

#[test]
fn tide_charm_converts_shallow_water_on_player_half_to_sand() {
    let size = 8u8;
    let mut run = RunState::new(42, size);
    let setup_without = run.match_setup();
    let (terrain_without, _) = setup_without.terrain();

    run.owned.push("tide_charm".into());
    let setup_with = run.match_setup();
    let (terrain_with, _) = setup_with.terrain();

    let mid = size / 2;
    for y in 0..size {
        for x in 0..size {
            let sq = Sq::new(x, y);
            let kind_without = terrain_without.get(sq).kind;
            let kind_with = terrain_with.get(sq).kind;

            if y < mid {
                // Player's half
                if kind_without == TileKind::ShallowWater {
                    assert_eq!(kind_with, TileKind::Sand, "shallow water on player half should become sand");
                } else {
                    assert_eq!(kind_with, kind_without);
                }
            } else {
                // Opponent's half
                assert_eq!(kind_with, kind_without, "opponent half should remain untouched");
            }
        }
    }
}

#[test]
fn cartographer_relic_guarantees_extra_run_item_pickup() {
    let size = 8u8;
    // On floor 0 without cartographer: 0 pickups (spell charges no longer placed)
    let run = RunState::new(12345, size);
    let setup = run.match_setup();
    assert_eq!(setup.pickups.len(), 0);

    // On floor 0 with cartographer: 1 pickup (guaranteed RunItem)
    let mut run_carto = RunState::new(12345, size);
    run_carto.owned.push("cartographer".into());
    let setup_carto = run_carto.match_setup();
    assert_eq!(setup_carto.pickups.len(), 1);
    assert!(
        setup_carto.pickups.iter().any(|(_, p)| matches!(p, Pickup::RunItem)),
        "cartographer must guarantee a RunItem pickup from floor 0"
    );
}

#[test]
fn tunneler_bishops_and_veteran_enhancements_applied() {
    let mut run = RunState::new(101, 8);
    let white_bishop = Piece::new(PieceKind::Bishop, Side::White);
    let black_bishop = Piece::new(PieceKind::Bishop, Side::Black);

    let setup0 = run.match_setup();
    assert!(!setup0.rules.profile(white_bishop).cave_slide);
    assert_eq!(setup0.veteran, [false, false]);

    run.owned.push("tunneler_bishops".into());
    run.owned.push("veteran".into());
    let setup1 = run.match_setup();
    assert!(setup1.rules.profile(white_bishop).cave_slide);
    assert!(!setup1.rules.profile(black_bishop).cave_slide);
    assert_eq!(setup1.veteran, [true, false]);
}

#[test]
fn enemy_enhancements_from_floor_3_on() {
    // Floors 0..3: no enemy items
    for floor in 0..3 {
        let mut run = RunState::new(777, 8);
        run.floor = floor;
        let setup = run.match_setup();
        assert!(setup.enemy_items.is_empty(), "floor {floor} should have no enemy items");
    }

    // Floor 3: 1 enemy item
    let mut run3 = RunState::new(777, 8);
    run3.floor = 3;
    let setup3 = run3.match_setup();
    assert_eq!(setup3.enemy_items.len(), 1, "floor 3 should have 1 enemy item");

    // Floor 4: 1 enemy item ((4-3)/2 + 1 = 1)
    let mut run4 = RunState::new(777, 8);
    run4.floor = 4;
    let setup4 = run4.match_setup();
    assert_eq!(setup4.enemy_items.len(), 1, "floor 4 should have 1 enemy item");

    // Floor 5: 2 enemy items ((5-3)/2 + 1 = 2)
    let mut run5 = RunState::new(777, 8);
    run5.floor = 5;
    let setup5 = run5.match_setup();
    assert_eq!(setup5.enemy_items.len(), 2, "floor 5 should have 2 enemy items");

    // Floor 7: 3 enemy items ((7-3)/2 + 1 = 3)
    let mut run7 = RunState::new(777, 8);
    run7.floor = 7;
    let setup7 = run7.match_setup();
    assert_eq!(setup7.enemy_items.len(), 3, "floor 7 should have 3 enemy items");

    // Deterministic from match seed
    let mut run7_b = RunState::new(777, 8);
    run7_b.floor = 7;
    let setup7_b = run7_b.match_setup();
    assert_eq!(setup7.enemy_items, setup7_b.enemy_items, "enemy items must be deterministic");
}

#[test]
fn spell_item_descriptions_and_deck_generation() {
    use tc_run::ItemKind;

    for item in catalog() {
        if let ItemKind::Spell { charges, .. } = item.kind {
            // Older spells use generic deck text; newer spells describe their effect.
            if item.description.starts_with("Adds") {
                let expected = if charges == 1 {
                    "Adds 1 card to your spell deck."
                } else {
                    "Adds 2 cards to your spell deck."
                };
                assert_eq!(item.description, expected, "description mismatch for {}", item.id);
            } else {
                assert!(!item.description.is_empty(), "spell {} needs a description", item.id);
            }
        }
    }

    // Verify 15-card player deck and enemy deck generation across floors
    for floor in 0..8 {
        let mut run = RunState::new(12345, 8);
        run.floor = floor;
        let setup = run.match_setup();
        assert_eq!(setup.player_deck.len(), 15, "player deck must have exactly 15 cards");
        assert_eq!(setup.enemy_deck.len(), 15, "enemy deck must have exactly 15 cards");
    }
}
