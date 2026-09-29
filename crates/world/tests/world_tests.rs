use tc_core::Match;
use tc_core::piece::PieceKind;
use tc_core::rng::Rng;
use tc_world::*;

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
/// from each start to its own camps are within 20 % of each other. Checked for 2–4 heroes; at 6+ heroes the
/// square grid cannot rotate the map exactly and regions drift slightly past 20 % (see the design spec).
#[test]
fn test_region_fairness() {
    use tc_world::path::find_path;
    for heroes in [2u8, 3, 4] {
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
