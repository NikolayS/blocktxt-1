use blocktxt::game::piece::{spawn, PieceKind, Rotation};

#[test]
fn o_spawns_at_cols_5_and_6() {
    let piece = spawn(PieceKind::O);
    assert!(matches!(piece.rotation, Rotation::Zero));

    // O piece cells: (5,22),(6,22),(5,23),(6,23)
    let cells = piece.cells();
    let cols: Vec<i32> = cells.iter().map(|&(c, _)| c).collect();
    assert!(cols.contains(&5), "O should occupy col 5; cells={cells:?}");
    assert!(cols.contains(&6), "O should occupy col 6; cells={cells:?}");
    // Must not occupy cols outside 5..=6
    for &(c, _) in &cells {
        assert!((5..=6).contains(&c), "O cell col {c} out of range 5..=6");
    }
}

#[test]
fn jlstz_spawn_bounding_box_is_cols_4_to_7() {
    for kind in [
        PieceKind::J,
        PieceKind::L,
        PieceKind::S,
        PieceKind::T,
        PieceKind::Z,
    ] {
        let piece = spawn(kind);
        let cells = piece.cells();
        let min_col = cells.iter().map(|&(c, _)| c).min().unwrap();
        let max_col = cells.iter().map(|&(c, _)| c).max().unwrap();
        assert!(
            min_col >= 4,
            "{kind:?} min col {min_col} should be >= 4; cells={cells:?}"
        );
        assert!(
            max_col <= 7,
            "{kind:?} max col {max_col} should be <= 7; cells={cells:?}"
        );
    }
}

#[test]
fn i_spawn_bounding_box_is_cols_4_to_7() {
    let piece = spawn(PieceKind::I);
    let cells = piece.cells();
    let min_col = cells.iter().map(|&(c, _)| c).min().unwrap();
    let max_col = cells.iter().map(|&(c, _)| c).max().unwrap();
    assert!(
        min_col >= 4,
        "I min col {min_col} should be >= 4; cells={cells:?}"
    );
    assert!(
        max_col <= 7,
        "I max col {max_col} should be <= 7; cells={cells:?}"
    );
}
