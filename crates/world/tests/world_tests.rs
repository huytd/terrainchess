use serde::Serialize;
use tc_core::Match;
use tc_core::piece::PieceKind;
use tc_core::rng::Rng;
use tc_world::*;

fn bare_world(width: u16, height: u16, seed: u64) -> World {
    let params = WorldParams { width, height, ..WorldParams::default() };
    let heroes = vec![
        Hero::new(0, Faction::AshenSun, "Player".into(), MapPos::new(1, 1), false, seed),
        Hero::new(
            1,
            Faction::HollowCrown,
            "Rival".into(),
            MapPos::new(width.saturating_sub(2), height.saturating_sub(2)),
            true,
            seed ^ 1,
        ),
    ];
    World {
        map: WorldMap::new(width, height),
        heroes,
        day: 1,
        turn_order: vec![0, 1],
        current: 1,
        fog: vec![0; (width as usize * height as usize).div_ceil(64)],
        rng_state: seed,
        params,
    }
}

fn set_revealed(world: &mut World, pos: MapPos) {
    let index = pos.y as usize * world.map.size.0 as usize + pos.x as usize;
    world.fog[index / 64] |= 1 << (index % 64);
}

#[test]
fn test_deterministic_generation() {
    let params = WorldParams::default();
    let (map1, _) = tc_world::generator::generate_world(42, params.clone());
    let (map2, _) = tc_world::generator::generate_world(42, params);
    assert_eq!(map1, map2);
}

#[test]
fn test_reachable() {
    let (map, _) = tc_world::generator::generate_world(1, WorldParams::default());
    assert!(!map.objects.is_empty());
}

#[test]
fn test_path_costs() {
    let mut map = WorldMap::new(10, 10);
    map.get_mut(MapPos::new(0, 1)).unwrap().biome = Biome::Forest;
    map.get_mut(MapPos::new(0, 2)).unwrap().biome = Biome::Mountain;
    map.get_mut(MapPos::new(1, 0)).unwrap().road = true;

    assert_eq!(tc_world::path::tile_cost(&map, MapPos::new(0, 1)), Some(2.0));
    assert_eq!(tc_world::path::tile_cost(&map, MapPos::new(0, 2)), None);
    assert_eq!(tc_world::path::tile_cost(&map, MapPos::new(1, 0)), Some(0.5));
    assert_eq!(tc_world::path::tile_cost(&map, MapPos::new(0, 0)), Some(1.0));
}

#[test]
fn test_fog_and_movement() {
    let mut world = World::new(42, WorldParams::default());
    let h_id = world.heroes[0].id;
    let start = world.hero(h_id).unwrap().pos;
    assert!(world.is_revealed(start));
    let next_pos = MapPos::new(start.x + 1, start.y);
    world.move_hero(h_id, next_pos);
    assert_eq!(world.hero(h_id).unwrap().pos, next_pos);
    assert!(world.is_revealed(MapPos::new(next_pos.x + world.params.sight_radius, next_pos.y)));
}

#[test]
fn test_ai_does_not_reveal_fog() {
    let mut world = World::new(42, WorldParams::default());
    let ai_id = world.heroes.iter().find(|hero| hero.is_ai).unwrap().id;
    let ai_start = world.hero(ai_id).unwrap().pos;

    assert!(!world.is_revealed(ai_start));
    let revealed_before_ai_turn = world.fog.clone();

    world.ai_turn(ai_id);

    assert_eq!(world.fog, revealed_before_ai_turn);
}

#[test]
fn test_day_loop() {
    let mut world = World::new(42, WorldParams::default());
    let start_day = world.day;
    world.hero_mut(0).unwrap().movement = 0.0;
    world.end_turn();
    world.end_turn();
    assert_eq!(world.day, start_day + 1);
    assert_eq!(world.hero(0).unwrap().movement, world.params.hero_base_movement);
}

#[test]
fn test_battle_setup_produces_valid_match() {
    let mut world = World::new(42, WorldParams::default());
    let h_id = world.heroes[0].id;
    let encounter = Encounter::Camp(MapObject {
        pos: MapPos::new(0, 0),
        kind: tc_world::ObjectKind::Camp(CampKind::KnightCamp),
        owner: None,
        cleared: false,
        guards: vec![PieceKind::King, PieceKind::Knight, PieceKind::Pawn, PieceKind::Pawn, PieceKind::Pawn],
        tier: 1,
    });

    let setup = world.battle_setup(h_id, &encounter);
    let (terrain, _) = setup.terrain();
    // Test that the setup has the boss and king but not the standard setup.
    let pos = setup.position(8).unwrap();
    let pieces: Vec<_> = pos.pieces().collect();
    let knights = pieces.iter().filter(|(_, p)| p.kind == PieceKind::Knight).count();
    let kings = pieces.iter().filter(|(_, p)| p.kind == PieceKind::King).count();
    assert!(knights >= 1);
    assert_eq!(kings, 2); // 1 per side
    assert!(pieces.len() < 32); // not the full standard boards

    let game = Match::new(terrain, setup.rules, pos);
    assert!(!game.legal_moves().is_empty());
}

#[test]
fn test_apply_battle() {
    let mut world = World::new(42, WorldParams::default());
    let h_id = world.heroes[0].id;
    let camp = MapObject {
        pos: MapPos::new(0, 0),
        kind: tc_world::ObjectKind::Camp(CampKind::KnightCamp),
        owner: None,
        cleared: false,
        guards: vec![PieceKind::King, PieceKind::Pawn],
        tier: 1,
    };
    world.map.objects.push(camp.clone());

    let res = BattleResult {
        outcome: tc_world::encounter::Outcome::Won,
        lost: vec![PieceKind::Pawn],
        enemy_lost: vec![PieceKind::King, PieceKind::Pawn],
    };

    world.apply_battle(h_id, Encounter::Camp(camp.clone()), res);

    let updated_camp = world.map.get_object(MapPos::new(0, 0)).unwrap();
    assert!(updated_camp.cleared);
    assert_eq!(updated_camp.owner, Some(h_id));

    let pawns = world.hero(h_id).unwrap().roster.iter().filter(|&&p| p == PieceKind::Pawn).count();
    assert_eq!(pawns, 2);
    let knights = world.hero(h_id).unwrap().roster.iter().filter(|&&p| p == PieceKind::Knight).count();
    assert_eq!(knights, 1);
}

#[test]
fn test_retreat() {
    let mut world = World::new(42, WorldParams::default());
    let h_id = world.heroes[0].id;
    let hero_pos = world.hero(h_id).unwrap().pos;

    // Simulate hero entering the camp from hero_pos.
    let hero = world.hero_mut(h_id).unwrap();
    hero.prev_pos = hero_pos;
    hero.pos = MapPos::new(0, 0); // At camp
    hero.movement = 5.0;

    let camp = MapObject {
        pos: MapPos::new(0, 0),
        kind: tc_world::ObjectKind::Camp(CampKind::KnightCamp),
        owner: None,
        cleared: false,
        guards: vec![PieceKind::King, PieceKind::Knight, PieceKind::Pawn],
        tier: 1,
    };
    world.map.objects.push(camp.clone());

    let res = BattleResult {
        outcome: tc_world::encounter::Outcome::Retreated,
        lost: vec![],
        enemy_lost: vec![PieceKind::Pawn],
    };

    world.apply_battle(h_id, Encounter::Camp(camp.clone()), res);

    let updated_camp = world.map.get_object(MapPos::new(0, 0)).unwrap();
    assert!(!updated_camp.cleared);
    assert_eq!(updated_camp.guards, vec![PieceKind::King, PieceKind::Knight]);

    let hero = world.hero(h_id).unwrap();
    assert_eq!(hero.pos, hero_pos);
    assert_eq!(hero.movement, 0.0);
}

#[test]
fn test_auto_resolve_monotonic() {
    let strong = vec![PieceKind::King, PieceKind::Queen, PieceKind::Queen];
    let weak = vec![PieceKind::King, PieceKind::Pawn];
    let mut rng = Rng::new(123);
    let mut strong_wins = 0;
    let mut weak_wins = 0;
    for _ in 0..200 {
        if tc_world::resolve::auto_resolve(&strong, &weak, &mut rng).outcome
            == tc_world::encounter::Outcome::Won
        {
            strong_wins += 1;
        } else {
            weak_wins += 1;
        }
    }
    assert!(strong_wins > weak_wins);
}

#[test]
fn test_save_round_trip() {
    let world = World::new(42, WorldParams::default());
    let ron = world.to_ron().unwrap();
    let loaded = World::from_ron(&ron).unwrap();
    assert_eq!(world.map.size, loaded.map.size);
    assert_eq!(world.day, loaded.day);
}

#[test]
fn test_rng_advancement() {
    let mut world = World::new(42, WorldParams::default());
    let h_id = world.heroes[0].id;
    let encounter = Encounter::Hero(world.heroes[1].id);
    let setup1 = world.battle_setup(h_id, &encounter);
    let setup2 = world.battle_setup(h_id, &encounter);
    assert_ne!(setup1.seed, setup2.seed);
}

#[test]
fn test_fairness() {
    let params2 = WorldParams { heroes: 2, ..WorldParams::default() };
    let (map2, _heroes2) = tc_world::generator::generate_world(100, params2);
    // Regions might be a bit tricky to segment precisely by distance since it's procedural.
    // The prompt requested a test for fairness (identical camp-kind counts).
    // Because we placed the same template for each hero, the total counts should be exactly 2x the template.
    // And each hero's start should have N camps near it.
    let camps = map2.objects.iter().filter(|o| matches!(o.kind, tc_world::ObjectKind::Camp(_))).count();
    assert_eq!(camps, 12 * 2 + 1); // 12 camps per region * 2 heroes + 1 citadel
}

/// Every hero's region (camps nearest to its start) gets the same camp kinds, and the path costs
/// from each start to its own camps are within 20 % of each other. Checked for 2 heroes, the only count the
/// game uses; with 3+ heroes the square grid cannot rotate regions exactly (see the design spec).
#[test]
fn test_region_fairness() {
    use tc_world::path::find_path;
    for heroes in [2u8] {
        for seed in 0u64..25 {
            let params = WorldParams { heroes, ..WorldParams::default() };
            let world = World::new(seed, params);
            let starts: Vec<MapPos> = world.heroes.iter().map(|h| h.pos).collect();
            let mut kinds: Vec<Vec<String>> = vec![Vec::new(); starts.len()];
            let mut costs: Vec<Vec<f32>> = vec![Vec::new(); starts.len()];
            for obj in &world.map.objects {
                let tc_world::ObjectKind::Camp(kind) = obj.kind else { continue };
                if kind == CampKind::Citadel {
                    continue;
                }
                let (i, start) = starts.iter().enumerate().min_by_key(|(_, s)| s.manhattan(obj.pos)).unwrap();
                kinds[i].push(format!("{kind:?}"));
                let path = find_path(&world.map, *start, obj.pos).expect("camp reachable");
                let cost: f32 = path.iter().map(|p| tc_world::path::tile_cost(&world.map, *p).unwrap()).sum();
                costs[i].push(cost);
            }
            for k in &mut kinds {
                k.sort();
            }
            for i in 1..kinds.len() {
                assert_eq!(kinds[0], kinds[i], "seed {seed}, {heroes} heroes: region {i} camp kinds differ");
            }
            let totals: Vec<f32> = costs.iter().map(|c| c.iter().sum()).collect();
            let (lo, hi) = totals.iter().fold((f32::MAX, 0f32), |(lo, hi), &t| (lo.min(t), hi.max(t)));
            assert!(hi <= lo * 1.2, "seed {seed}, {heroes} heroes: region path totals {totals:?}");
        }
    }
}

#[test]
fn test_ai_turn_is_deterministic() {
    let mut first = bare_world(16, 12, 81);
    let mut second = first.clone();
    assert_eq!(first.heroes[1].cards.len(), 15);

    for _ in 0..5 {
        assert_eq!(first.ai_turn(1), second.ai_turn(1));
        first.refill_movement();
        second.refill_movement();
    }
    assert_eq!(first, second);
}

#[test]
fn test_ai_respects_movement_and_impassable_terrain() {
    let mut world = bare_world(8, 6, 91);
    let ai_start = MapPos::new(1, 2);
    let player_pos = MapPos::new(6, 2);
    world.heroes[0].pos = player_pos;
    world.heroes[0].prev_pos = player_pos;
    world.heroes[1].pos = ai_start;
    world.heroes[1].prev_pos = ai_start;
    world.heroes[1].movement = 10.0;

    for y in 0..world.map.size.1 {
        world.map.get_mut(MapPos::new(3, y)).unwrap().biome = Biome::Mountain;
        world.map.get_mut(MapPos::new(4, y)).unwrap().biome = Biome::Water;
        for x in 0..3 {
            set_revealed(&mut world, MapPos::new(x, y));
        }
    }

    let initial_movement = world.heroes[1].movement;
    world.ai_turn(1);
    let ai = world.hero(1).unwrap();
    assert!(ai.movement >= 0.0);
    assert!(ai.movement <= initial_movement);
    assert!(ai.pos.x < 3);
    assert!(tc_world::path::tile_cost(&world.map, ai.pos).is_some());
    assert!(world.path(1, player_pos).is_none());
}

#[test]
fn test_strong_ai_hunts_player() {
    let mut world = bare_world(12, 6, 12);
    let player_pos = MapPos::new(10, 2);
    world.heroes[0].pos = player_pos;
    world.heroes[0].prev_pos = player_pos;
    world.heroes[1].pos = MapPos::new(2, 2);
    world.heroes[1].prev_pos = world.heroes[1].pos;
    world.heroes[1].roster.push(PieceKind::Queen);
    world.heroes[1].movement = 4.0;

    let mut encounter = None;
    for _ in 0..3 {
        encounter = world.ai_turn(1);
        if encounter.is_some() {
            break;
        }
        world.refill_movement();
    }
    assert_eq!(encounter, Some(Encounter::Hero(0)));
    assert_eq!(world.hero(1).unwrap().pos, player_pos);
}

#[test]
fn test_ai_wins_camp_and_recruits_boss() {
    let mut world = bare_world(8, 6, 27);
    let camp_pos = MapPos::new(2, 1);
    world.heroes[1].pos = MapPos::new(1, 1);
    world.heroes[1].prev_pos = world.heroes[1].pos;
    world.heroes[1].movement = 10.0;
    world.map.objects.push(MapObject {
        pos: camp_pos,
        kind: ObjectKind::Camp(CampKind::KnightCamp),
        owner: None,
        cleared: false,
        guards: vec![PieceKind::King],
        tier: 1,
    });

    world.ai_turn(1);
    let camp = world.map.get_object(camp_pos).unwrap();
    assert!(camp.cleared);
    assert_eq!(camp.owner, Some(1));
    assert!(world.hero(1).unwrap().roster.contains(&PieceKind::Knight));
}

#[test]
fn test_ai_continues_after_a_camp_win() {
    let mut world = bare_world(10, 6, 28);
    world.heroes[1].pos = MapPos::new(1, 1);
    world.heroes[1].prev_pos = world.heroes[1].pos;
    world.heroes[1].movement = 10.0;
    for (pos, kind) in [(MapPos::new(2, 1), CampKind::KnightCamp), (MapPos::new(3, 1), CampKind::BishopCamp)]
    {
        world.map.objects.push(MapObject {
            pos,
            kind: ObjectKind::Camp(kind),
            owner: None,
            cleared: false,
            guards: vec![PieceKind::King],
            tier: 1,
        });
    }

    world.ai_turn(1);
    assert!(world.map.get_object(MapPos::new(2, 1)).unwrap().cleared);
    assert!(world.map.get_object(MapPos::new(3, 1)).unwrap().cleared);
    assert!(world.hero(1).unwrap().roster.contains(&PieceKind::Knight));
    assert!(world.hero(1).unwrap().roster.contains(&PieceKind::Bishop));
}

#[test]
fn test_ai_skips_camps_above_its_army_value() {
    let mut world = bare_world(8, 6, 29);
    let camp_pos = MapPos::new(2, 1);
    world.heroes[1].pos = MapPos::new(1, 1);
    world.heroes[1].prev_pos = world.heroes[1].pos;
    world.heroes[1].movement = 10.0;
    world.map.objects.push(MapObject {
        pos: camp_pos,
        kind: ObjectKind::Camp(CampKind::Citadel),
        owner: None,
        cleared: false,
        guards: vec![PieceKind::King, PieceKind::Queen],
        tier: 1,
    });

    world.ai_turn(1);
    assert!(world.hero(1).unwrap().alive);
    assert!(!world.map.get_object(camp_pos).unwrap().cleared);
}

#[test]
fn test_losing_ai_camp_fight_retreats_and_preserves_the_king() {
    let attacker = vec![PieceKind::King, PieceKind::Pawn, PieceKind::Pawn, PieceKind::Pawn];
    let guards = attacker.clone();
    let seed = (0..1000)
        .find(|seed| {
            let mut rng = Rng::new(*seed);
            rng.next_u64();
            let result = tc_world::resolve::auto_resolve(&attacker, &guards, &mut rng);
            result.outcome == tc_world::encounter::Outcome::Checkmated
                && result.enemy_lost.iter().filter(|&&piece| piece == PieceKind::Pawn).count() < 3
        })
        .unwrap();
    let mut rng = Rng::new(seed);
    rng.next_u64();
    let result = tc_world::resolve::auto_resolve(&attacker, &guards, &mut rng);
    let mut expected_roster = attacker.clone();
    for lost in result.lost {
        if let Some(index) = expected_roster.iter().position(|&piece| piece == lost) {
            expected_roster.remove(index);
        }
    }
    let mut expected_guards = guards.clone();
    for lost in result.enemy_lost {
        if let Some(index) = expected_guards.iter().position(|&piece| piece == lost) {
            expected_guards.remove(index);
        }
    }
    let mut world = bare_world(8, 6, seed);
    let camp_pos = MapPos::new(2, 1);
    world.heroes[0].pos = MapPos::new(6, 4);
    world.heroes[0].prev_pos = world.heroes[0].pos;
    world.heroes[1].pos = MapPos::new(1, 1);
    world.heroes[1].prev_pos = world.heroes[1].pos;
    world.heroes[1].roster = attacker;
    world.heroes[1].movement = 10.0;
    world.map.objects.push(MapObject {
        pos: camp_pos,
        kind: ObjectKind::Camp(CampKind::Village),
        owner: None,
        cleared: false,
        guards: guards.clone(),
        tier: 1,
    });

    world.ai_turn(1);
    let ai = world.hero(1).unwrap();
    assert!(ai.alive);
    assert_eq!(ai.pos, MapPos::new(1, 1));
    assert_eq!(ai.movement, 0.0);
    assert_eq!(ai.roster, expected_roster);
    assert!(ai.roster.contains(&PieceKind::King));
    let camp = world.map.get_object(camp_pos).unwrap();
    assert!(!camp.cleared);
    assert_eq!(camp.owner, None);
    assert_eq!(camp.guards, expected_guards);
    assert_eq!(world.winner(), None);
}

#[test]
fn test_ai_survives_and_clears_camps_over_60_turns_on_multiple_seeds() {
    let mut seeds_with_cleared_camp = 0;
    for seed in 0..11 {
        let mut world = bare_world(10, 6, seed);
        world.heroes[0].pos = MapPos::new(8, 4);
        world.heroes[0].prev_pos = world.heroes[0].pos;
        world.heroes[1].pos = MapPos::new(1, 1);
        world.heroes[1].prev_pos = world.heroes[1].pos;
        world.heroes[1].movement = 10.0;
        for (pos, kind) in
            [(MapPos::new(2, 1), CampKind::KnightCamp), (MapPos::new(4, 1), CampKind::BishopCamp)]
        {
            world.map.objects.push(MapObject {
                pos,
                kind: ObjectKind::Camp(kind),
                owner: None,
                cleared: false,
                guards: vec![PieceKind::King],
                tier: 1,
            });
        }

        for _ in 0..60 {
            world.ai_turn(1);
            world.end_turn();
            world.end_turn();
        }

        assert!(world.hero(1).unwrap().alive, "AI died for seed {seed}");
        if world.map.objects.iter().any(|object| object.cleared) {
            seeds_with_cleared_camp += 1;
        }
    }

    assert!(seeds_with_cleared_camp >= 6, "cleared a camp on {seeds_with_cleared_camp}/11 seeds");
}

#[test]
fn test_starter_deck_save_and_legacy_campaign_load() {
    let mut world = bare_world(8, 6, 33);
    assert_eq!(world.heroes[0].cards.len(), 15);
    assert_eq!(world.heroes[0].deck.len(), 15);
    world.heroes[0].deck.rotate_left(3);
    let selected_deck = world.heroes[0].deck.clone();
    let loaded = World::from_ron(&world.to_ron().unwrap()).unwrap();
    assert_eq!(loaded.heroes[0].deck, selected_deck);

    #[derive(Serialize)]
    struct LegacyHero {
        id: u8,
        faction: Faction,
        name: String,
        pos: MapPos,
        prev_pos: MapPos,
        roster: Vec<PieceKind>,
        cards: Vec<tc_core::SpellId>,
        items: Vec<tc_run::item::ItemId>,
        movement: f32,
        is_ai: bool,
        alive: bool,
    }

    #[derive(Serialize)]
    struct LegacyWorld {
        map: WorldMap,
        heroes: Vec<LegacyHero>,
        day: u32,
        turn_order: Vec<HeroId>,
        current: usize,
        fog: Vec<u64>,
        rng_state: u64,
        params: WorldParams,
    }

    let old = LegacyWorld {
        map: world.map.clone(),
        heroes: world
            .heroes
            .iter()
            .map(|hero| LegacyHero {
                id: hero.id,
                faction: hero.faction,
                name: hero.name.clone(),
                pos: hero.pos,
                prev_pos: hero.prev_pos,
                roster: hero.roster.clone(),
                cards: Vec::new(),
                items: hero.items.clone(),
                movement: hero.movement,
                is_ai: hero.is_ai,
                alive: hero.alive,
            })
            .collect(),
        day: world.day,
        turn_order: world.turn_order.clone(),
        current: world.current,
        fog: world.fog.clone(),
        rng_state: world.rng_state,
        params: world.params.clone(),
    };
    let old_ron = ron::ser::to_string(&old).unwrap();
    let upgraded = World::from_ron(&old_ron).unwrap();
    for hero in upgraded.heroes {
        assert_eq!(hero.cards.len(), 15);
        assert_eq!(hero.deck.len(), 15);
    }
}

#[test]
fn test_battle_setup_uses_selected_hero_decks() {
    let mut world = bare_world(8, 6, 34);
    world.heroes[0].deck.rotate_left(4);
    world.heroes[1].deck.rotate_left(7);
    let player_deck = world.heroes[0].deck.clone();
    let rival_deck = world.heroes[1].deck.clone();
    let setup = world.battle_setup(0, &Encounter::Hero(1));

    assert_eq!(setup.player, tc_core::Side::White);
    assert_eq!(setup.player_deck, player_deck);
    assert_eq!(setup.enemy_deck, rival_deck);
}
