//! Integration tests for the endless run's stages and the Profile.

use std::collections::HashSet;

use tc_ai::{Action, Limits, choose_action};
use tc_core::piece::Side;
use tc_core::position::Position;
use tc_core::{Match, Terrain};
use tc_run::{Endless, Profile, STAGES_PER_TIER, Stage, catalog};

#[test]
fn every_stage_builds_a_valid_match() {
    for n in 1..=3 * STAGES_PER_TIER {
        let stage = Stage::new(n);
        let size = stage.size();
        for seed in [1u64, 42] {
            let profile = Profile::new(1);
            let setup = profile.stage_setup(&stage, seed);

            // 1. Position is Ok
            let pos = setup.position(size).expect("position should be valid");
            assert_eq!(pos.size, size);

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
            assert_eq!(white_kings, 1, "Stage {} seed {} white king count", n, seed);
            assert_eq!(black_kings, 1, "Stage {} seed {} black king count", n, seed);

            // 3. Piece counts
            let [you, enemy] = stage.piece_counts();
            assert_eq!(pos.pieces().filter(|(_, p)| p.side == Side::White).count(), you, "stage {n}");
            assert_eq!(pos.pieces().filter(|(_, p)| p.side == Side::Black).count(), enemy, "stage {n}");

            // 4. Terrain generation without panic
            let (terrain, _) = setup.terrain();
            assert_eq!(terrain.size, size);

            // 5. Pickups never on occupied or home-row squares
            let home = Position::home_rows(size);
            for (sq, _) in &setup.pickups {
                assert!(
                    pos.get(*sq).is_none(),
                    "Pickup on square {:?} is occupied by a piece in stage {} seed {}",
                    sq,
                    n,
                    seed
                );
                assert!(
                    sq.y >= home && sq.y < size - home,
                    "Pickup on square {:?} is on home row (home={}) in stage {} seed {}",
                    sq,
                    home,
                    n,
                    seed
                );
            }

            // 6. White has at least one legal move
            let ctx = tc_core::movegen::Ctx { terrain: &terrain, rules: &setup.rules };
            let legal_moves = ctx.legal_moves(&pos);
            assert!(
                !legal_moves.is_empty(),
                "White must have at least one legal move in stage {} seed {}",
                n,
                seed
            );

            // 7. AI returns an action (first tier only: later tiers repeat the boards)
            if stage.tier > 0 || size > 8 {
                continue;
            }
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
    for theme in 0..tc_run::THEMES.len() {
        let stage = Stage::new(1 + (0..theme).map(|t| tc_run::THEMES[t].variants.len() as u32).sum::<u32>());
        let size = stage.size();
        let mut non_flat = 0;
        let flat = Terrain::flat(size);
        for seed in 1..=5 {
            let profile = Profile::new(1);
            let setup = profile.stage_setup(&stage, seed);
            let (terrain, _) = setup.terrain();
            if terrain != flat {
                non_flat += 1;
            }
        }
        assert!(
            non_flat >= 1,
            "Stage {} (size {}) should generate non-flat terrain for at least 1 of 5 seeds (got {})",
            stage.number,
            size,
            non_flat
        );
    }
}

#[test]
fn win_advances_and_records_best() {
    let mut profile = Profile::new(42);
    profile.new_run(7);
    let draft1 = profile.win_stage();
    assert!(!draft1.is_empty() && draft1.len() <= 3);
    assert_eq!(profile.last_draft, Some(draft1.clone()));
    let mut set = HashSet::new();
    for item in &draft1 {
        assert!(set.insert(item.clone()), "draft contains duplicates");
    }
    profile.pick(&draft1[0]).expect("pick should succeed");
    assert!(profile.owned.contains(&draft1[0]));
    assert!(profile.pick(&draft1[0]).is_err());

    profile.win_stage();
    assert_eq!(profile.stage().number, 3);
    assert_eq!(profile.best_stage, 2);
}

#[test]
fn end_run_keeps_items() {
    let mut profile = Profile::new(1);
    profile.owned.push("raise_earth".into());
    profile.new_run(3);
    profile.win_stage();
    assert_eq!(profile.end_run(), 2);
    assert!(profile.run.is_none());
    assert_eq!(profile.owned, vec!["raise_earth".to_string()]);
    assert_eq!(profile.best_stage, 1);
    assert_eq!(profile.stage().number, 1);
}

#[test]
fn stage_board_is_deterministic() {
    let run = Endless { seed: 9, stage: 4 };
    assert_eq!(run.match_seed(), Endless { seed: 9, stage: 4 }.match_seed());
    assert_ne!(run.match_seed(), Endless { seed: 9, stage: 5 }.match_seed());
    let profile = all_spells_profile();
    let stage = Stage::new(4);
    let a = profile.stage_setup(&stage, run.match_seed());
    let b = profile.stage_setup(&stage, run.match_seed());
    assert_eq!(a.terrain(), b.terrain());
    assert_eq!(a.player_deck, b.player_deck);
}

#[test]
fn later_tiers_reinforce_the_enemy() {
    let stage = Stage::new(STAGES_PER_TIER + 1);
    assert_eq!(stage.piece_counts(), [2, 2]);
    let setup = Profile::new(1).stage_setup(&stage, 3);
    let spells = setup.enemy_deck.iter().filter(|s| !tc_run::FILLER.contains(s)).count();
    assert!(setup.enemy_items.len() >= 3);
    assert!(spells >= 1);
}

#[test]
fn win_with_all_items_owned_does_not_panic() {
    let mut profile = Profile::new(12345);
    profile.owned = catalog().iter().map(|i| i.id.clone()).collect();
    profile.new_run(1);
    let draft = profile.win_stage();
    assert!(draft.is_empty(), "draft should be empty when all items are owned");
}

#[test]
fn profile_ron_round_trip() {
    let mut profile = Profile::new(999);
    profile.owned.push("mountaineer_rooks".into());
    profile.owned.push("raise_earth".into());
    profile.new_run(5);
    profile.best_stage = 12;
    profile.last_draft = Some(vec!["tide_charm".into(), "veteran".into()]);

    let ron_str = profile.to_ron().expect("serialize to ron");
    let loaded: Profile = Profile::from_ron(&ron_str).expect("deserialize from ron");
    assert_eq!(profile, loaded);

    // Test with last_draft omitted
    let ron_without_last_draft = "(rng: 100, owned: [\"veteran\"], cleared: [1])";
    let loaded_default: Profile = Profile::from_ron(ron_without_last_draft).expect("deserialize");
    assert_eq!(loaded_default.last_draft, None);
    assert_eq!(loaded_default.run, None);
    assert_eq!(loaded_default.best_stage, 0);
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
    assert!(profile.deck().contains(&reserve[0]));
    assert!(profile.deck_reserve().contains(&old));
    let mut all = profile.deck();
    all.extend(profile.deck_reserve());
    assert_eq!(multiset(&all), multiset(&profile.deck_pool()));

    // Out of range is a no-op.
    let before = profile.deck();
    profile.swap_with_reserve(99, 0);
    profile.swap_with_reserve(0, 99);
    assert_eq!(profile.deck(), before);

    profile.reset_deck();
    assert!(profile.deck.is_empty());
}

#[test]
fn stage_setup_shuffles_chosen_deck() {
    let stage = Stage::new(5);
    let mut profile = all_spells_profile();
    profile.swap_with_reserve(0, 1);
    let dealt = profile.stage_setup(&stage, 5).player_deck;
    assert_eq!(multiset(&dealt), multiset(&profile.deck()));

    // Unchosen profile: exactly the deck RunState always built.
    let fresh = all_spells_profile();
    let run = tc_run::RunState {
        seed: 5,
        size: stage.size(),
        floor: stage.difficulty(),
        owned: fresh.owned.clone(),
        rng: 5,
        outcome: None,
        bonus_picks: 0,
        last_draft: None,
    };
    assert_eq!(fresh.stage_setup(&stage, 5).player_deck, run.match_setup().player_deck);
}

#[test]
fn deck_round_trips_and_old_saves_load() {
    let mut profile = all_spells_profile();
    profile.swap_with_reserve(0, 0);
    let loaded = Profile::from_ron(&profile.to_ron().unwrap()).unwrap();
    assert_eq!(loaded, profile);

    let old = Profile::from_ron("(rng: 1, owned: [], cleared: [])").unwrap();
    assert!(old.deck.is_empty());
    // Saves from when the draw order could be fixed still load.
    let ordered =
        Profile::from_ron("(rng: 1, owned: [], cleared: [], deck: [Swap], shuffle_deck: false)").unwrap();
    assert_eq!(ordered.deck, vec![tc_core::SpellId::Swap]);
}
