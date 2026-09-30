//! Integration tests for Level ladder and Profile (Job 1).

use std::collections::HashSet;

use tc_ai::{Action, Limits, choose_action};
use tc_core::piece::Side;
use tc_core::position::Position;
use tc_core::{Match, Terrain};
use tc_run::{LEVELS, Profile, catalog, level};

#[test]
fn all_ten_levels_defined_and_retrievable() {
    assert_eq!(LEVELS.len(), 10);
    for id in 1..=10 {
        let l = level(id).expect("level should exist");
        assert_eq!(l.id, id);
    }
    assert!(level(0).is_none());
    assert!(level(11).is_none());
}

#[test]
fn test_levels_setup_position_and_ai() {
    for lvl in &LEVELS {
        for seed in [1u64, 42, 100, 2024] {
            let profile = Profile::new(1);
            let setup = profile.match_setup(lvl, seed);

            // 1. Position is Ok
            let pos = setup.position(lvl.size).expect("position should be valid");
            assert_eq!(pos.size, lvl.size);

            // 2. Exactly one King per side
            assert!(pos.king(Side::White).is_some(), "White must have a king");
            assert!(pos.king(Side::Black).is_some(), "Black must have a king");
            let white_kings = pos
                .pieces()
                .filter(|(_, p)| p.side == Side::White && p.kind == tc_core::PieceKind::King)
                .count();
            let black_kings = pos
                .pieces()
                .filter(|(_, p)| p.side == Side::Black && p.kind == tc_core::PieceKind::King)
                .count();
            assert_eq!(white_kings, 1, "Level {} seed {} white king count", lvl.id, seed);
            assert_eq!(black_kings, 1, "Level {} seed {} black king count", lvl.id, seed);

            // 3. Piece counts
            if let Some(army) = lvl.army {
                let white_count = pos.pieces().filter(|(_, p)| p.side == Side::White).count();
                let black_count = pos.pieces().filter(|(_, p)| p.side == Side::Black).count();
                assert_eq!(white_count, army.len(), "Level {} seed {} white pieces count", lvl.id, seed);
                assert_eq!(
                    black_count,
                    lvl.enemy_army.unwrap_or(army).len(),
                    "Level {} seed {} black pieces count",
                    lvl.id,
                    seed
                );
            } else if lvl.size == 8 {
                let white_count = pos.pieces().filter(|(_, p)| p.side == Side::White).count();
                assert_eq!(white_count, 16);
            } else if lvl.size == 16 {
                let white_count = pos.pieces().filter(|(_, p)| p.side == Side::White).count();
                assert_eq!(white_count, 32);
            }

            // 4. Terrain generation without panic
            let (terrain, _) = setup.terrain();
            assert_eq!(terrain.size, lvl.size);

            // 5. Pickups never on occupied or home-row squares
            let home = Position::home_rows(lvl.size);
            for (sq, _) in &setup.pickups {
                assert!(
                    pos.get(*sq).is_none(),
                    "Pickup on square {:?} is occupied by a piece in level {} seed {}",
                    sq,
                    lvl.id,
                    seed
                );
                assert!(
                    sq.y >= home && sq.y < lvl.size - home,
                    "Pickup on square {:?} is on home row (home={}) in level {} seed {}",
                    sq,
                    home,
                    lvl.id,
                    seed
                );
            }

            // 6. White has at least one legal move
            let ctx = tc_core::movegen::Ctx { terrain: &terrain, rules: &setup.rules };
            let legal_moves = ctx.legal_moves(&pos);
            assert!(
                !legal_moves.is_empty(),
                "White must have at least one legal move in level {} seed {}",
                lvl.id,
                seed
            );

            // 7. AI returns an action at AI level 1 within test
            let mut game = Match::new(terrain, setup.rules.clone(), pos);
            game.set_deck(Side::White, setup.player_deck.clone());
            game.set_deck(Side::Black, setup.enemy_deck.clone());
            let action = choose_action(&game, Limits::for_floor(1));
            assert!(
                matches!(action, Action::Move(_) | Action::Cast(_)),
                "AI returned invalid action: {:?}",
                action
            );
        }
    }
}

#[test]
fn terrain_is_non_flat_for_most_seeds_on_all_sizes() {
    for lvl in &LEVELS {
        let mut non_flat = 0;
        let flat = Terrain::flat(lvl.size);
        for seed in 1..=5 {
            let profile = Profile::new(1);
            let setup = profile.match_setup(lvl, seed);
            let (terrain, _) = setup.terrain();
            if terrain != flat {
                non_flat += 1;
            }
        }
        assert!(
            non_flat >= 1,
            "Level {} (size {}) should generate non-flat terrain for at least 1 of 5 seeds (got {})",
            lvl.id,
            lvl.size,
            non_flat
        );
    }
}

#[test]
fn record_win_and_draft_progression() {
    let mut profile = Profile::new(42);
    let lvl1 = level(1).unwrap();

    let draft1 = profile.record_win(lvl1);
    assert!(draft1.len() <= 3);
    assert!(!draft1.is_empty());
    assert_eq!(profile.cleared, vec![1]);
    assert_eq!(profile.last_draft, Some(draft1.clone()));

    // Record win on the same level again does not duplicate cleared
    let draft1_again = profile.record_win(lvl1);
    assert_eq!(profile.cleared, vec![1]);
    assert_eq!(profile.last_draft, Some(draft1_again.clone()));

    // Distinct items in draft
    let mut set = HashSet::new();
    for item in &draft1 {
        assert!(set.insert(item.clone()), "draft contains duplicates");
        assert!(!profile.owned.contains(item), "draft contains already owned item");
    }

    // Pick an item from the latest draft
    let picked = draft1_again[0].clone();
    profile.pick(&picked).expect("pick should succeed");
    assert!(profile.owned.contains(&picked));
    assert_eq!(profile.last_draft, None);

    // Pick again should fail
    assert!(profile.pick(&picked).is_err());
}

#[test]
fn record_win_with_all_items_owned_does_not_panic() {
    let mut profile = Profile::new(12345);
    let all_items: Vec<String> = catalog().iter().map(|i| i.id.clone()).collect();
    profile.owned = all_items;

    let lvl = level(5).unwrap();
    let draft = profile.record_win(lvl);
    assert!(draft.is_empty(), "draft should be empty when all items are owned");
    assert_eq!(profile.last_draft, Some(Vec::new()));
}

#[test]
fn profile_ron_round_trip() {
    let mut profile = Profile::new(999);
    profile.owned.push("mountaineer_rooks".into());
    profile.owned.push("raise_earth".into());
    profile.cleared.push(1);
    profile.cleared.push(2);
    profile.last_draft = Some(vec!["tide_charm".into(), "veteran".into()]);

    let ron_str = profile.to_ron().expect("serialize to ron");
    let loaded: Profile = Profile::from_ron(&ron_str).expect("deserialize from ron");
    assert_eq!(profile, loaded);

    // Test with last_draft omitted
    let ron_without_last_draft = "(rng: 100, owned: [\"veteran\"], cleared: [1])";
    let loaded_default: Profile = Profile::from_ron(ron_without_last_draft).expect("deserialize");
    assert_eq!(loaded_default.last_draft, None);
}

fn all_spells_profile() -> Profile {
    let mut profile = Profile::new(7);
    profile.owned = catalog()
        .iter()
        .filter(|i| matches!(i.kind, tc_run::item::ItemKind::Spell { .. }))
        .map(|i| i.id.clone())
        .collect();
    profile
}

fn multiset(v: &[tc_run::SpellId]) -> Vec<String> {
    let mut s: Vec<String> = v.iter().map(|c| format!("{c:?}")).collect();
    s.sort();
    s
}

#[test]
fn deck_swaps_keep_the_pool() {
    let mut profile = all_spells_profile();
    let reserve = profile.deck_reserve();
    assert!(!reserve.is_empty());
    let old = profile.deck()[2];
    profile.swap_with_reserve(2, 0);
    assert!(profile.shuffle_deck, "reserve swaps don't change shuffling");
    assert_eq!(profile.deck()[2], reserve[0]);
    assert!(profile.deck_reserve().contains(&old));
    let mut all = profile.deck();
    all.extend(profile.deck_reserve());
    assert_eq!(multiset(&all), multiset(&profile.deck_pool()));

    let before = profile.deck();
    profile.swap_deck_cards(0, 4);
    assert!(!profile.shuffle_deck);
    assert_eq!(profile.deck()[0], before[4]);
    assert_eq!(profile.deck()[4], before[0]);

    // Out of range is a no-op.
    let before = profile.deck();
    profile.swap_deck_cards(0, 99);
    profile.swap_with_reserve(99, 0);
    assert_eq!(profile.deck(), before);

    profile.reset_deck();
    assert!(profile.deck.is_empty() && profile.shuffle_deck);
}

#[test]
fn match_setup_uses_arranged_deck() {
    let lvl = level(3).unwrap();
    let mut profile = all_spells_profile();
    profile.swap_with_reserve(0, 1);
    profile.swap_deck_cards(1, 2);
    assert_eq!(profile.match_setup(lvl, 5).player_deck, profile.deck());

    profile.shuffle_deck = true;
    let shuffled = profile.match_setup(lvl, 5).player_deck;
    assert_eq!(multiset(&shuffled), multiset(&profile.deck()));

    // Unarranged profile: exactly the deck RunState always built.
    let fresh = all_spells_profile();
    let run = tc_run::RunState {
        seed: 5,
        size: lvl.size,
        floor: lvl.difficulty,
        owned: fresh.owned.clone(),
        rng: 5,
        outcome: None,
        bonus_picks: 0,
        last_draft: None,
    };
    assert_eq!(fresh.match_setup(lvl, 5).player_deck, run.match_setup().player_deck);
}

#[test]
fn deck_fields_round_trip_and_default() {
    let mut profile = all_spells_profile();
    profile.swap_deck_cards(0, 1);
    let loaded = Profile::from_ron(&profile.to_ron().unwrap()).unwrap();
    assert_eq!(loaded, profile);
    assert!(!loaded.shuffle_deck);

    let old = Profile::from_ron("(rng: 1, owned: [], cleared: [])").unwrap();
    assert!(old.shuffle_deck);
    assert!(old.deck.is_empty());
}
