//! Behavioural tests for the SPEC §1a originality-pass presentation:
//! single-piece letter-box preview, floor-line ghost, and dimmed locked
//! cells. Each test pins a specific deviation from canonical falling-block
//! presentation convention.

use std::time::Instant;

use ratatui::backend::TestBackend;
use ratatui::style::Color;
use ratatui::Terminal;

use blocktxt::clock::FakeClock;
use blocktxt::game::board::{BUFFER_ROWS, TOTAL_ROWS};
use blocktxt::game::piece::{Piece, PieceKind, Rotation};
use blocktxt::game::state::GameState;
use blocktxt::render::board_view::{self, dim_color, FILLED, GHOST_LINE, LOCKED_CELL_DIM};
use blocktxt::render::{hud, Theme};
use blocktxt::Input;

// ── helpers ───────────────────────────────────────────────────────────────────

/// Board-view dimensions (12 cells × 2 chars + 2 border = 26; 24 rows + 2).
const BOARD_W: u16 = 26;
const BOARD_H: u16 = 26;

fn make_playing_state() -> GameState {
    let clock = Box::new(FakeClock::new(Instant::now()));
    let mut gs = GameState::new(42, clock);
    gs.step(std::time::Duration::ZERO, &[Input::StartGame]);
    gs
}

fn render_board(state: &GameState, theme: &Theme) -> (Terminal<TestBackend>, String) {
    let backend = TestBackend::new(BOARD_W, BOARD_H);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            board_view::draw(f, f.area(), state, theme);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let out: String = (0..buf.area.height)
        .map(|y| {
            let row: String = (0..buf.area.width)
                .map(|x| {
                    let cell = &buf[(x, y)];
                    cell.symbol().chars().next().unwrap_or(' ')
                })
                .collect();
            row
        })
        .collect::<Vec<_>>()
        .join("\n");
    (terminal, out)
}

// ── 1. Next preview: ONE piece, rendered as a centered letter ────────────────

/// `peek_next_kind()` returns a single `PieceKind` (not a multi-piece queue).
#[test]
fn next_preview_shows_only_one_piece() {
    let gs = make_playing_state();
    let peeked = gs
        .peek_next_kind()
        .expect("peek_next_kind must return Some once the round has started");
    // Smoke assertion: the peeked kind is one of the seven legal kinds.
    let valid = matches!(
        peeked,
        PieceKind::I
            | PieceKind::O
            | PieceKind::T
            | PieceKind::S
            | PieceKind::Z
            | PieceKind::J
            | PieceKind::L
    );
    assert!(
        valid,
        "unexpected piece kind from peek_next_kind: {peeked:?}"
    );
}

/// The rendered HUD shows the next piece as its ASCII letter, not as a
/// miniature playfield of cells.
#[test]
fn next_preview_renders_as_centered_letter_box() {
    let mut gs = make_playing_state();
    // Pin the preview to a known letter so the assertion is deterministic.
    gs.next_queue.clear();
    gs.next_queue.push_back(PieceKind::T);
    gs.next_queue.push_back(PieceKind::L); // trailing — should NOT render

    let theme = Theme::monochrome();
    let backend = TestBackend::new(24, 22);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            hud::draw(f, f.area(), &gs, &theme);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let mut flat = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            let cell = &buf[(x, y)];
            flat.push(cell.symbol().chars().next().unwrap_or(' '));
        }
    }

    // Single-letter preview: exactly one 'T' should appear in the next-panel
    // area. The peeked kind is T; the trailing L must not be rendered.
    assert!(
        flat.contains('T'),
        "HUD should render the 'T' letter preview"
    );
    assert!(
        !flat.contains('L'),
        "HUD should NOT render the second queued piece (single-lookahead preview)"
    );
    // And the preview must NOT look like a miniature playfield of filled
    // cells — no `▰▰` glyphs should appear anywhere in the HUD region.
    assert!(
        !flat.contains('▰'),
        "HUD next-preview must not render miniature cell shapes"
    );
}

// ── 2. Ghost: single horizontal floor line, not a piece-shaped outline ──────

/// When the active piece is well above the floor, the ghost renders as the
/// floor-line glyph at the landing row — not as a piece-shaped outline.
#[test]
fn ghost_renders_as_floor_line() {
    let clock = Box::new(FakeClock::new(Instant::now()));
    let mut gs = GameState::new(42, clock);
    gs.step(std::time::Duration::ZERO, &[Input::StartGame]);

    // Park the active T-piece near the top of the visible area so the ghost
    // has several rows of clearance below it.
    gs.active = Some(Piece {
        kind: PieceKind::T,
        rotation: Rotation::Zero,
        origin: (4, (BUFFER_ROWS as i32) + 2),
    });

    let theme = Theme::monochrome();
    let (_terminal, out) = render_board(&gs, &theme);

    // The floor-line glyph must appear somewhere below the active piece.
    assert!(
        out.contains(GHOST_LINE.chars().next().unwrap()),
        "ghost floor-line glyph {GHOST_LINE:?} must be present in the board view"
    );

    // The legacy piece-shaped ghost glyph (`░`) must NOT appear anywhere —
    // that was the old Tetris-style ghost.
    assert!(
        !out.contains('░'),
        "old piece-shaped ghost glyph must not render in the originality pass"
    );
}

/// When the piece is already on the floor (ghost_y == origin.1) the ghost
/// must NOT render — otherwise the floor-line would either be drawn under
/// the piece or fall outside the playfield. This is the edge case the floor
/// line needs to silently handle.
#[test]
fn ghost_does_not_render_when_piece_is_already_on_floor() {
    let clock = Box::new(FakeClock::new(Instant::now()));
    let mut gs = GameState::new(42, clock);
    gs.step(std::time::Duration::ZERO, &[Input::StartGame]);

    // Hard-drop the active piece so it lands on the floor of the empty
    // playfield. The spawn-fade plus the piece itself are still drawn, but
    // the ghost should be suppressed — the piece IS on its landing row.
    gs.step(std::time::Duration::ZERO, &[Input::HardDrop]);

    // The hard-drop locks the piece, so state.active has advanced to the
    // next piece spawn. To exercise the "active piece already at the floor"
    // case directly, place a fresh piece with origin at its ghost_y.
    let mut piece = Piece {
        kind: PieceKind::T,
        rotation: Rotation::Zero,
        origin: (4, (BUFFER_ROWS as i32) + 2),
    };
    // Manually drop it until its ghost_y == origin.1.
    let ghost = blocktxt::render::helpers::ghost_y(&gs.board, &piece);
    piece.origin.1 = ghost;
    gs.active = Some(piece);

    let theme = Theme::monochrome();
    let (_terminal, out) = render_board(&gs, &theme);

    // When grounded, no floor-line should be drawn — the piece's own body
    // already occupies the landing row.
    assert!(
        !out.contains(GHOST_LINE.chars().next().unwrap()),
        "floor-line ghost must not render when the piece is already landed"
    );
}

// ── 3. Locked cells render dimmer than the active piece ─────────────────────

/// Render a board with one locked T cell and verify its effective fg color
/// is `dim_color(kind_color, LOCKED_CELL_DIM)`, not the full piece color.
#[test]
fn locked_pieces_render_dimmer_than_active() {
    use ratatui::style::Color as TuiColor;

    let clock = Box::new(FakeClock::new(Instant::now()));
    let mut gs = GameState::new(42, clock);
    gs.step(std::time::Duration::ZERO, &[Input::StartGame]);

    // Plant a single locked T cell in a visible, distinctive spot.
    let locked_col: usize = 0;
    let locked_row: usize = TOTAL_ROWS - 1;
    gs.board.set(locked_col, locked_row, PieceKind::T);

    // Hide the active piece so only the locked cell and HUD matter.
    gs.active = None;

    // Use a color theme so `dim_color` actually has RGB to dim.
    let theme = Theme::truecolor(blocktxt::render::theme::Palette::TokyoNight);
    let backend = TestBackend::new(BOARD_W, BOARD_H);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            board_view::draw(f, f.area(), &gs, &theme);
        })
        .unwrap();

    // The locked T cell sits on the last visible row, col 0 — that's
    // terminal coord (inner.x + 0, inner.y + VISIBLE_ROWS - 1) = (1, 24).
    let buf = terminal.backend().buffer().clone();
    let cell = &buf[(1_u16, 24_u16)];
    let rendered_fg = cell.style().fg.unwrap_or(TuiColor::Reset);

    let expected = dim_color(theme.color(PieceKind::T), LOCKED_CELL_DIM);
    let full_color = theme.color(PieceKind::T);

    assert_eq!(
        rendered_fg, expected,
        "locked cell fg must equal dim_color(kind_color, {LOCKED_CELL_DIM})"
    );
    assert_ne!(
        rendered_fg, full_color,
        "locked cell fg must differ from the full active-piece color"
    );

    // Also assert the glyph is still the thematic filled block — only the
    // color, not the shape, changes on lock.
    assert_eq!(
        cell.symbol(),
        &FILLED[0..FILLED.chars().next().unwrap().len_utf8()],
        "locked cell glyph must stay the originality-pass filled block"
    );

    // Sanity: `dim_color` at 0.6 really produces a darker shade.
    if let (TuiColor::Rgb(r0, g0, b0), TuiColor::Rgb(r1, g1, b1)) = (full_color, expected) {
        assert!(r1 < r0 || g1 < g0 || b1 < b0, "dimmed color must be darker");
    } else {
        panic!("truecolor palette should yield Color::Rgb");
    }
}

/// `LOCKED_CELL_DIM` is the documented 0.6 constant — pin it so accidental
/// edits flip the trade-dress safety posture.
#[test]
fn locked_cell_dim_constant_is_60_percent() {
    assert!(
        (LOCKED_CELL_DIM - 0.6).abs() < f32::EPSILON,
        "LOCKED_CELL_DIM must stay at 0.6 per SPEC §1a"
    );
}

/// `dim_color` is a pure function — same input produces same output, and
/// non-RGB colors pass through unchanged.
#[test]
fn dim_color_scales_rgb_channels() {
    let c = Color::Rgb(200, 100, 50);
    let d = dim_color(c, 0.5);
    assert_eq!(d, Color::Rgb(100, 50, 25));
    // Non-RGB pass-through.
    assert_eq!(dim_color(Color::Reset, 0.5), Color::Reset);
}
