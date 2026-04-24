//! Unit tests for `render::helpers` — pure functions only.

use std::collections::VecDeque;

use blocktxt::game::board::{Board, COLS, TOTAL_ROWS};
use blocktxt::game::piece::{Piece, PieceKind, Rotation};
use blocktxt::render::helpers::{
    format_level, format_lines, format_score, ghost_y, next_preview_glyphs,
};
use blocktxt::render::theme::Theme;

// ── ghost_y ──────────────────────────────────────────────────────────────────

/// I-piece at its spawn position on an empty 12×48 board should land with
/// its cells resting on the last visible row (row TOTAL_ROWS - 1 = 47).
///
/// I piece in Zero rotation: offsets are (0,1),(1,1),(2,1),(3,1).
/// origin=(4, 22) → body cells at row 23.
/// `is_occupied` returns true for row TOTAL_ROWS.
/// Body bottom = origin.1 + 1. Drop stops when origin.1 + 2 == TOTAL_ROWS.
/// Therefore ghost_y = TOTAL_ROWS - 2.
#[test]
fn ghost_y_open_column_lands_on_floor() {
    let board = Board::empty();
    let piece = Piece {
        kind: PieceKind::I,
        rotation: Rotation::Zero,
        origin: (4, 22),
    };
    let g = ghost_y(&board, &piece);
    let expected = (TOTAL_ROWS as i32) - 2;
    assert_eq!(
        g, expected,
        "I piece should land with origin row {expected} on empty board (got {g})"
    );
}

#[test]
fn ghost_y_stops_on_stack() {
    let mut board = Board::empty();
    // Place a blocker at row 40, filling every column.
    let blocker_row: usize = 40;
    for col in 0..COLS {
        board.set(col, blocker_row, PieceKind::O);
    }
    // I piece, origin (4, 22): body cells at origin.1+1.
    // Piece can drop while origin.1 + 2 < blocker_row.
    let piece = Piece {
        kind: PieceKind::I,
        rotation: Rotation::Zero,
        origin: (4, 22),
    };
    let g = ghost_y(&board, &piece);
    let expected = (blocker_row as i32) - 2;
    assert_eq!(
        g, expected,
        "I piece should stop above blocker at row {blocker_row}"
    );
}

// ── format_score ─────────────────────────────────────────────────────────────

#[test]
fn format_score_zero_pad() {
    let s = format_score(0);
    assert_eq!(s, "        0", "zero should be right-aligned to 9 chars");
}

#[test]
fn format_score_thousands_separator() {
    let s = format_score(1_234_567);
    assert_eq!(s, "1 234 567");
}

#[test]
fn format_score_small() {
    let s = format_score(42);
    assert_eq!(s, "       42");
}

// ── format_lines / format_level ───────────────────────────────────────────────

#[test]
fn format_lines_width() {
    assert_eq!(format_lines(0).len(), 6);
    assert_eq!(format_lines(999_999).len(), 6);
}

#[test]
fn format_level_width() {
    assert_eq!(format_level(1).len(), 3);
    assert_eq!(format_level(99).len(), 3);
}

// ── next_preview_glyphs ───────────────────────────────────────────────────────

#[test]
fn next_preview_glyphs_maps_kind_to_char() {
    let theme = Theme::monochrome();
    let mut queue: VecDeque<PieceKind> = VecDeque::new();
    queue.push_back(PieceKind::I);
    queue.push_back(PieceKind::O);
    queue.push_back(PieceKind::T);

    let pairs: Vec<(PieceKind, char)> = next_preview_glyphs(&queue, &theme).collect();
    assert_eq!(pairs.len(), 3);
    assert_eq!(pairs[0], (PieceKind::I, 'I'));
    assert_eq!(pairs[1], (PieceKind::O, 'O'));
    assert_eq!(pairs[2], (PieceKind::T, 'T'));
}
