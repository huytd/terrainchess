use tc_core::worldgen::{GenParams, Reject, generate, generate_unchecked, validate};
use tc_core::{Position, Rules, Sq, Terrain, Tile, TileKind};

#[test]
fn same_seed_same_board() {
    let p = GenParams::for_floor(8, 3);
    assert_eq!(generate(42, &p), generate(42, &p));
    assert_ne!(generate_unchecked(1, &p), generate_unchecked(2, &p));
}

#[test]
fn boards_are_mirrored_valid_and_have_flat_home_rows() {
    for size in [8u8, 16, 32] {
        for seed in 0..12 {
            for floor in [0, 7] {
                let (t, used) = generate(seed * 1000, &GenParams::for_floor(size, floor));
                assert!(used >= seed * 1000);
                assert_ne!(t, Terrain::flat(size), "size {size} seed {seed} fell back to flat");
                for y in 0..size {
                    for x in 0..size {
                        assert_eq!(t.get(Sq::new(x, y)), t.get(Sq::new(x, size - 1 - y)));
                    }
                }
                for y in 0..Position::home_rows(size) {
                    for x in 0..size {
                        assert_eq!(*t.get(Sq::new(x, y)), Tile::flat(1));
                    }
                }
                validate(&t, &Rules::standard(size), &Position::start(size)).unwrap();
            }
        }
    }
}

#[test]
fn generated_boards_have_varied_terrain() {
    let (t, _) = generate(7, &GenParams::for_floor(16, 4));
    let heights: std::collections::BTreeSet<u8> = t.tiles.iter().map(|t| t.height).collect();
    assert!(heights.len() >= 3, "{heights:?}");
    assert!(t.tiles.iter().any(|t| t.is_water()));
    assert!(t.tiles.iter().any(|t| matches!(t.feature, tc_core::Feature::Cave(_))));
}

#[test]
fn validator_rejects_a_sealed_board() {
    let mut t = Terrain::flat(8);
    for x in 0..8 {
        t.get_mut(Sq::new(x, 3)).kind = TileKind::DeepWater;
        t.get_mut(Sq::new(x, 4)).kind = TileKind::DeepWater;
    }
    let err = validate(&t, &Rules::standard(8), &Position::start(8)).unwrap_err();
    assert!(matches!(err, Reject::Unreachable(_, _)), "{err:?}");
}

#[test]
fn big_armies_start_legal() {
    for size in [16u8, 32] {
        let pos = Position::start(size);
        assert_eq!(pos.pieces().filter(|(_, p)| p.kind == tc_core::PieceKind::King).count(), 2);
        let m = tc_core::Match::new(Terrain::flat(size), Rules::standard(size), pos);
        assert!(!m.legal_moves().is_empty());
        assert!(m.outcome().is_none());
    }
}
