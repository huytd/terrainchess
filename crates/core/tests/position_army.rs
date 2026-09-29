use tc_core::piece::PieceKind;
use tc_core::position::Position;

#[test]
fn test_from_army() {
    let standard = vec![
        PieceKind::King,
        PieceKind::Queen,
        PieceKind::Rook,
        PieceKind::Rook,
        PieceKind::Knight,
        PieceKind::Knight,
        PieceKind::Bishop,
        PieceKind::Bishop,
        PieceKind::Pawn,
        PieceKind::Pawn,
        PieceKind::Pawn,
        PieceKind::Pawn,
        PieceKind::Pawn,
        PieceKind::Pawn,
        PieceKind::Pawn,
        PieceKind::Pawn,
    ];
    let pos_start = Position::start(8);
    let pos_army = Position::from_army(8, &standard, &standard).unwrap();

    assert_eq!(pos_army.pieces().collect::<Vec<_>>(), pos_start.pieces().collect::<Vec<_>>());
}
