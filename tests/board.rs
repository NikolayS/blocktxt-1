use blocktxt::game::board::{Board, BUFFER_ROWS, COLS, TOTAL_ROWS};
use blocktxt::game::piece::PieceKind;

#[test]
fn empty_board_has_no_occupancy_on_visible_rows() {
    let board = Board::empty();
    for row in (BUFFER_ROWS as i32)..(TOTAL_ROWS as i32) {
        for col in 0..(COLS as i32) {
            assert!(
                !board.is_occupied(col, row),
                "expected empty at col={col} row={row}"
            );
        }
    }
}

#[test]
fn set_and_is_occupied_round_trip() {
    let mut board = Board::empty();
    board.set(3, 30, PieceKind::T);
    assert!(board.is_occupied(3, 30));
    assert!(!board.is_occupied(4, 30));
    assert!(!board.is_occupied(3, 31));
}

#[test]
fn out_of_bounds_reads_as_occupied() {
    let board = Board::empty();
    // negative col
    assert!(board.is_occupied(-1, BUFFER_ROWS as i32));
    // negative row
    assert!(board.is_occupied(0, -1));
    // col >= COLS
    assert!(board.is_occupied(COLS as i32, BUFFER_ROWS as i32));
    // row >= TOTAL_ROWS
    assert!(board.is_occupied(0, TOTAL_ROWS as i32));
}

#[test]
fn clear_full_rows_counts_and_shifts_down() {
    let mut board = Board::empty();
    let bottom = TOTAL_ROWS - 1;
    // Fill bottom visible row completely.
    for col in 0..COLS {
        board.set(col, bottom, PieceKind::I);
    }
    // Partially fill the row above.
    board.set(0, bottom - 1, PieceKind::O);

    let cleared = board.clear_full_rows();
    assert_eq!(cleared, 1, "expected 1 cleared row");

    // The partial row should have shifted down by one.
    assert!(
        board.is_occupied(0, bottom as i32),
        "partial row should shift to the bottom"
    );
    assert!(
        !board.is_occupied(0, (bottom - 1) as i32),
        "row above bottom should now be empty"
    );
    assert!(!board.is_occupied(1, bottom as i32));
}

#[test]
fn clear_full_rows_on_empty_board_does_not_mutate() {
    let mut b = Board::empty();
    b.set(3, 5, PieceKind::I);
    b.set(3, 10, PieceKind::T);
    let snapshot_before = b.clone();
    let cleared = b.clear_full_rows();
    assert_eq!(cleared, 0);
    assert_eq!(b, snapshot_before);
}

#[test]
fn clear_full_rows_leaves_partial_row_intact() {
    let mut b = Board::empty();
    for c in 0..(COLS - 1) {
        b.set(c, 30, PieceKind::I);
    }
    let cleared = b.clear_full_rows();
    assert_eq!(cleared, 0);
    for c in 0..(COLS - 1) {
        assert!(b.is_occupied(c as i32, 30));
    }
    assert!(!b.is_occupied((COLS - 1) as i32, 30));
}

#[test]
fn block_out_detected() {
    // Spawn cells for O are cols 5 and 6, rows 22..=23.
    // Fill one of those cells to simulate block-out condition.
    let mut board = Board::empty();
    board.set(5, 22, PieceKind::I);

    let spawn_cells: [(i32, i32); 4] = [(5, 22), (6, 22), (5, 23), (6, 23)];
    let blocked = spawn_cells.iter().any(|&(c, r)| board.is_occupied(c, r));
    assert!(
        blocked,
        "block-out should be detected when spawn cells are occupied"
    );
}
