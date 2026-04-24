//! Snapshot tests using ratatui TestBackend + insta.
//!
//! For first run: `INSTA_UPDATE=always cargo test --test render_snapshots`
//! Then review with `cargo insta review` or accept inline.

use std::time::{Duration, Instant};

use ratatui::backend::TestBackend;
use ratatui::Terminal;

use blocktxt::clock::{Clock, FakeClock};
use blocktxt::game::board::{COLS, TOTAL_ROWS};
use blocktxt::game::piece::{Piece, PieceKind, Rotation};
use blocktxt::game::state::GameState;
use blocktxt::persistence::{HighScore, HighScoreStore};
use blocktxt::render::theme::Palette;
use blocktxt::render::{board_view, hud, Theme};
use blocktxt::Input;

// ── board/view sizing constants (originality-pass dimensions) ─────────────────

/// Board-view width in terminal columns: 12 cells × 2 chars + 2 border = 26.
const BOARD_W: u16 = 26;
/// Board-view height in terminal rows: 24 visible rows + 2 border = 26.
const BOARD_H: u16 = 26;

// ── helpers ───────────────────────────────────────────────────────────────────

fn fake_state() -> GameState {
    let clock = Box::new(FakeClock::new(Instant::now()));
    let mut gs = GameState::new(42, clock);
    // Transition to Playing so render tests see a normal in-game state.
    gs.step(std::time::Duration::ZERO, &[blocktxt::Input::StartGame]);
    gs
}

/// Render a terminal buffer to a multiline string (one char per cell).
fn buf_to_string(terminal: &Terminal<TestBackend>) -> String {
    let buf = terminal.backend().buffer().clone();
    let lines: Vec<String> = (0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| {
                    let cell = &buf[(x, y)];
                    cell.symbol().chars().next().unwrap_or(' ')
                })
                .collect()
        })
        .collect();
    lines.join("\n")
}

// ── existing snapshots (preserved, not churned) ───────────────────────────────

/// Render the HUD panel to a TestBackend and snapshot it.
#[test]
fn snapshot_hud_empty_state() {
    let state = fake_state();
    let theme = Theme::monochrome();

    let backend = TestBackend::new(24, 20);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let area = f.area();
            hud::draw(f, area, &state, &theme);
        })
        .unwrap();

    insta::assert_snapshot!("hud_empty_state", buf_to_string(&terminal));
}

/// Render the HUD in paused state and snapshot it.
#[test]
fn snapshot_hud_paused() {
    let mut state = fake_state();
    // Pause the game.
    state.step(std::time::Duration::ZERO, &[Input::Pause]);

    let theme = Theme::monochrome();

    let backend = TestBackend::new(24, 20);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let area = f.area();
            hud::draw(f, area, &state, &theme);
        })
        .unwrap();

    insta::assert_snapshot!("hud_paused", buf_to_string(&terminal));
}

/// Render the full board_view on an empty board.
#[test]
fn snapshot_board_view_empty() {
    let state = fake_state();
    let theme = Theme::monochrome();

    let backend = TestBackend::new(BOARD_W, BOARD_H);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let area = f.area();
            board_view::draw(f, area, &state, &theme);
        })
        .unwrap();

    insta::assert_snapshot!("board_view_empty", buf_to_string(&terminal));
}

// ── new snapshots (#30) ───────────────────────────────────────────────────────

/// HUD with a non-trivial score, level, and line count.
#[test]
fn snapshot_hud_with_score() {
    let clock = Box::new(FakeClock::new(Instant::now()));
    let mut state = GameState::new(42, clock);
    // Inject stats directly via public fields.
    state.score = 100_000;
    // Also set the rollup display to the same value so the snapshot is stable.
    state.score_display.current = 100_000;
    state.score_display.target = 100_000;
    state.level = 5;
    state.lines_cleared = 40;

    let theme = Theme::monochrome();
    let backend = TestBackend::new(24, 20);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            hud::draw(f, f.area(), &state, &theme);
        })
        .unwrap();

    insta::assert_snapshot!("hud_with_score", buf_to_string(&terminal));
}

/// Board view with a locked piece stack and the active piece + ghost overlay.
#[test]
fn snapshot_board_view_with_active_piece_and_ghost() {
    let clock = Box::new(FakeClock::new(Instant::now()));
    let mut state = GameState::new(42, clock);

    // Fill the bottom 3 rows of the visible area with O pieces
    // to give the ghost something to land on.
    for row in (TOTAL_ROWS - 3)..TOTAL_ROWS {
        for col in 0..COLS {
            state.board.set(col, row, PieceKind::O);
        }
    }

    // Place the active piece near the top-middle of the visible area.
    state.active = Some(Piece {
        kind: PieceKind::T,
        rotation: Rotation::Zero,
        origin: (4, 26),
    });

    let theme = Theme::monochrome();
    let backend = TestBackend::new(BOARD_W, BOARD_H);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            board_view::draw(f, f.area(), &state, &theme);
        })
        .unwrap();

    insta::assert_snapshot!(
        "board_view_with_active_piece_and_ghost",
        buf_to_string(&terminal)
    );
}

/// Game-over overlay with a regular (non-best) score.
#[test]
fn snapshot_game_over_overlay_regular() {
    let clock = Box::new(FakeClock::new(Instant::now()));
    let mut state = GameState::new(42, clock);
    state.score = 500;

    // Force game-over phase via hard-drops until game ends.
    for _ in 0..600 {
        state.step(Duration::ZERO, &[Input::HardDrop]);
        if matches!(state.phase, blocktxt::Phase::GameOver { .. }) {
            break;
        }
    }

    // Clear the zoom animation so the snapshot captures the final full-size
    // overlay (not the animated intermediate state).
    state.gameover_zoom = None;

    // Build a store where the existing best (10_000) beats our score (500).
    let mut store = HighScoreStore::new();
    store.insert(HighScore {
        name: "prev".into(),
        score: 10_000,
        level: 3,
        lines: 20,
        ts: 0,
    });

    let theme = Theme::monochrome();
    let backend = TestBackend::new(24, 20);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            hud::draw_with_scores(f, f.area(), &state, &theme, Some(&store));
        })
        .unwrap();

    insta::assert_snapshot!("game_over_overlay_regular", buf_to_string(&terminal));
}

/// Game-over overlay with a new-best score: highlighted + "NEW BEST!" banner.
#[test]
fn snapshot_game_over_overlay_new_best() {
    let clock = Box::new(FakeClock::new(Instant::now()));
    let mut state = GameState::new(42, clock);

    // Force game-over.
    for _ in 0..600 {
        state.step(Duration::ZERO, &[Input::HardDrop]);
        if matches!(state.phase, blocktxt::Phase::GameOver { .. }) {
            break;
        }
    }

    // Clear the zoom animation so the snapshot captures the final full-size
    // overlay (not the animated intermediate state).
    state.gameover_zoom = None;

    // Build an empty store so every score is a new best.
    let store = HighScoreStore::new();

    let theme = Theme::monochrome();
    let backend = TestBackend::new(24, 20);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            hud::draw_with_scores(f, f.area(), &state, &theme, Some(&store));
        })
        .unwrap();

    insta::assert_snapshot!("game_over_overlay_new_best", buf_to_string(&terminal));
}

/// HUD rendered in monochrome mode via NO_COLOR env var (no_color flag path).
///
/// Uses `Theme::detect(true, ...)` to simulate `--no-color` flag so the test
/// is env-var-free and therefore safe for parallel execution.
#[test]
fn snapshot_hud_no_color_mode() {
    let state = fake_state();
    // Simulate NO_COLOR by passing no_color_flag=true; avoids env mutation.
    let theme = Theme::detect(true, Palette::default());

    let backend = TestBackend::new(24, 20);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            hud::draw(f, f.area(), &state, &theme);
        })
        .unwrap();

    insta::assert_snapshot!("hud_no_color_mode", buf_to_string(&terminal));
}

/// Board view with a locked stack at the bottom and no active piece.
#[test]
fn snapshot_board_view_with_locked_stack() {
    let clock = Box::new(FakeClock::new(Instant::now()));
    let mut state = GameState::new(42, clock);

    // Fill the bottom 5 visible rows with alternating kinds for visual variety.
    for row in (TOTAL_ROWS - 5)..TOTAL_ROWS {
        for col in 0..COLS {
            let kind = if col % 2 == 0 {
                PieceKind::I
            } else {
                PieceKind::O
            };
            state.board.set(col, row, kind);
        }
    }

    // No active piece — game-over state just before the overlay.
    state.active = None;

    let theme = Theme::monochrome();
    let backend = TestBackend::new(BOARD_W, BOARD_H);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            board_view::draw(f, f.area(), &state, &theme);
        })
        .unwrap();

    insta::assert_snapshot!("board_view_with_locked_stack", buf_to_string(&terminal));
}

/// Board view during phase 1 (flash) of the line-clear animation.
#[test]
fn snapshot_line_clear_flash_frame() {
    use blocktxt::{LineClearAnim, LineClearPhase};

    let clock = FakeClock::new(Instant::now());
    let mut state = GameState::new(42, Box::new(clock.clone()));

    // Fill the bottom row (last visible row) completely.
    let bottom = TOTAL_ROWS - 1;
    for col in 0..COLS {
        state.board.set(col, bottom, PieceKind::I);
    }

    // Inject a LineClearAnim in Flash phase (t=0, well within flash window).
    state.line_clear_anim = Some(LineClearAnim {
        rows: vec![bottom],
        started_at: clock.now(),
        phase: LineClearPhase::Flash,
        board_snapshot: state.board.clone(),
        pending_count: 1,
        pending_level_before: 1,
        pending_b2b_active: false,
    });

    let theme = Theme::monochrome();
    let backend = TestBackend::new(BOARD_W, BOARD_H);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            board_view::draw(f, f.area(), &state, &theme);
        })
        .unwrap();

    insta::assert_snapshot!("line_clear_flash_frame", buf_to_string(&terminal));
}

/// Board view captured mid wipe-outward phase — the originality-pass
/// line-clear animation should show cells near the center "wiped" while the
/// outer cells are still visible (dimmed).
#[test]
fn snapshot_line_clear_wipe_midframe() {
    use blocktxt::game::state::{ANIM_FLASH_MS, ANIM_WIPE_MS};
    use blocktxt::{LineClearAnim, LineClearPhase};

    let clock = FakeClock::new(Instant::now());
    // Anchor the animation's started_at in the past so elapsed at render
    // time is mid-wipe (flash over + halfway through the wipe window).
    let anim_started = clock.now() - Duration::from_millis(ANIM_FLASH_MS + ANIM_WIPE_MS / 2);

    let mut state = GameState::new(42, Box::new(clock.clone()));

    let bottom = TOTAL_ROWS - 1;
    for col in 0..COLS {
        state.board.set(col, bottom, PieceKind::I);
    }

    state.line_clear_anim = Some(LineClearAnim {
        rows: vec![bottom],
        started_at: anim_started,
        phase: LineClearPhase::WipeOutward,
        board_snapshot: state.board.clone(),
        pending_count: 1,
        pending_level_before: 1,
        pending_b2b_active: false,
    });

    let theme = Theme::monochrome();
    let backend = TestBackend::new(BOARD_W, BOARD_H);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            board_view::draw(f, f.area(), &state, &theme);
        })
        .unwrap();

    insta::assert_snapshot!("line_clear_wipe_midframe", buf_to_string(&terminal));
}

/// Board view rendered with Catppuccin Mocha palette via explicit Palette arg.
///
/// Pins the non-default path so a regression in palette routing is caught.
#[test]
fn board_view_catppuccin_via_arg() {
    let clock = Box::new(FakeClock::new(std::time::Instant::now()));
    let mut state = GameState::new(42, clock);

    for row in (TOTAL_ROWS - 3)..TOTAL_ROWS {
        for col in 0..COLS {
            state.board.set(col, row, PieceKind::S);
        }
    }

    let theme = Theme::truecolor(Palette::CatppuccinMocha);

    let backend = TestBackend::new(BOARD_W, BOARD_H);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            board_view::draw(f, f.area(), &state, &theme);
        })
        .unwrap();

    insta::assert_snapshot!("board_view_catppuccin_via_arg", buf_to_string(&terminal));
}

// ── hold box snapshots (#62) ──────────────────────────────────────────────────

/// HUD with a piece in the hold slot (not locked).
#[test]
fn snapshot_hud_hold_occupied() {
    let clock = Box::new(FakeClock::new(Instant::now()));
    let mut state = GameState::new(42, clock);
    state.step(Duration::ZERO, &[blocktxt::Input::StartGame]);

    // Inject a T-piece into the hold slot (unlocked state).
    state.hold = Some(PieceKind::T);
    state.hold_used_this_cycle = false;

    let theme = Theme::monochrome();
    let backend = TestBackend::new(24, 22);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            hud::draw(f, f.area(), &state, &theme);
        })
        .unwrap();

    insta::assert_snapshot!("hud_hold_occupied", buf_to_string(&terminal));
}

/// HUD with an empty hold slot.
#[test]
fn snapshot_hud_hold_empty() {
    let clock = Box::new(FakeClock::new(Instant::now()));
    let mut state = GameState::new(42, clock);
    state.step(Duration::ZERO, &[blocktxt::Input::StartGame]);

    assert!(state.hold.is_none());

    let theme = Theme::monochrome();
    let backend = TestBackend::new(24, 22);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            hud::draw(f, f.area(), &state, &theme);
        })
        .unwrap();

    insta::assert_snapshot!("hud_hold_empty", buf_to_string(&terminal));
}

/// HUD with hold slot occupied AND locked (hold_used_this_cycle=true).
#[test]
fn snapshot_hud_hold_locked() {
    let clock = Box::new(FakeClock::new(Instant::now()));
    let mut state = GameState::new(42, clock);
    state.step(Duration::ZERO, &[blocktxt::Input::StartGame]);

    // Inject a locked hold state.
    state.hold = Some(PieceKind::I);
    state.hold_used_this_cycle = true;

    let theme = Theme::monochrome();
    let backend = TestBackend::new(24, 22);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            hud::draw(f, f.area(), &state, &theme);
        })
        .unwrap();

    insta::assert_snapshot!("hud_hold_locked", buf_to_string(&terminal));
}

// ── new palette snapshots (#61) ───────────────────────────────────────────────

/// Board view rendered with Gruvbox Dark palette.
#[test]
fn board_view_gruvbox() {
    let clock = Box::new(FakeClock::new(std::time::Instant::now()));
    let mut state = GameState::new(42, clock);

    for row in (TOTAL_ROWS - 3)..TOTAL_ROWS {
        for col in 0..COLS {
            state.board.set(col, row, PieceKind::S);
        }
    }

    let theme = Theme::truecolor(Palette::GruvboxDark);

    let backend = TestBackend::new(BOARD_W, BOARD_H);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            board_view::draw(f, f.area(), &state, &theme);
        })
        .unwrap();

    insta::assert_snapshot!("board_view_gruvbox", buf_to_string(&terminal));
}

/// Board view rendered with Nord palette.
#[test]
fn board_view_nord() {
    let clock = Box::new(FakeClock::new(std::time::Instant::now()));
    let mut state = GameState::new(42, clock);

    for row in (TOTAL_ROWS - 3)..TOTAL_ROWS {
        for col in 0..COLS {
            state.board.set(col, row, PieceKind::S);
        }
    }

    let theme = Theme::truecolor(Palette::Nord);

    let backend = TestBackend::new(BOARD_W, BOARD_H);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            board_view::draw(f, f.area(), &state, &theme);
        })
        .unwrap();

    insta::assert_snapshot!("board_view_nord", buf_to_string(&terminal));
}

/// Board view rendered with Dracula palette.
#[test]
fn board_view_dracula() {
    let clock = Box::new(FakeClock::new(std::time::Instant::now()));
    let mut state = GameState::new(42, clock);

    for row in (TOTAL_ROWS - 3)..TOTAL_ROWS {
        for col in 0..COLS {
            state.board.set(col, row, PieceKind::S);
        }
    }

    let theme = Theme::truecolor(Palette::Dracula);

    let backend = TestBackend::new(BOARD_W, BOARD_H);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            board_view::draw(f, f.area(), &state, &theme);
        })
        .unwrap();

    insta::assert_snapshot!("board_view_dracula", buf_to_string(&terminal));
}
