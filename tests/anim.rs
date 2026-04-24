//! Unit tests for the line-clear animation phase transitions.
//!
//! All timing is deterministic via `FakeClock::advance`.

use std::time::{Duration, Instant};

use blocktxt::clock::{Clock, FakeClock};
use blocktxt::game::board::{Board, COLS, TOTAL_ROWS};
use blocktxt::game::piece::PieceKind;
use blocktxt::game::state::{
    GameState, Input, LineClearAnim, LineClearPhase, ANIM_FLASH_MS, ANIM_TOTAL_MS, ANIM_WIPE_MS,
};
use blocktxt::Event;

// ── helpers ───────────────────────────────────────────────────────────────────

fn make_game(seed: u64) -> (GameState, FakeClock) {
    let origin = Instant::now();
    let clock = FakeClock::new(origin);
    let mut gs = GameState::new(seed, Box::new(clock.clone()));
    // Transition to Playing so animation tests can inject state directly.
    gs.step(std::time::Duration::ZERO, &[Input::StartGame]);
    (gs, clock)
}

/// Fill all columns of `row` (absolute board row) with O pieces.
fn fill_row(board: &mut Board, row: usize) {
    for col in 0..COLS {
        board.set(col, row, PieceKind::O);
    }
}

// ── constants ─────────────────────────────────────────────────────────────────

#[test]
fn anim_constants_match_spec() {
    assert_eq!(ANIM_FLASH_MS, 50, "flash phase must be 50 ms");
    assert_eq!(ANIM_WIPE_MS, 200, "wipe phase must be 200 ms");
    assert_eq!(ANIM_TOTAL_MS, 250, "total animation budget must be 250 ms");
}

// ── phase machine via FakeClock ───────────────────────────────────────────────

/// Injecting a LineClearAnim in Flash phase and advancing 20 ms leaves it
/// in Flash (< 50 ms threshold).
#[test]
fn anim_stays_flash_before_flash_boundary() {
    let (mut gs, clock) = make_game(42);

    let bottom = TOTAL_ROWS - 1;
    fill_row(&mut gs.board, bottom);
    gs.line_clear_anim = Some(LineClearAnim {
        rows: vec![bottom],
        started_at: clock.now(),
        phase: LineClearPhase::Flash,
        board_snapshot: gs.board.clone(),
        pending_count: 1,
        pending_level_before: 1,
        pending_b2b_active: false,
    });

    // Advance 20 ms — still in flash.
    clock.advance(Duration::from_millis(20));
    gs.step(Duration::from_millis(20), &[]);

    let phase = gs
        .line_clear_anim
        .as_ref()
        .expect("animation should still be active")
        .phase
        .clone();
    assert_eq!(phase, LineClearPhase::Flash);
}

/// Advancing past the flash boundary transitions Flash → WipeOutward.
#[test]
fn anim_transitions_to_wipe_at_flash_boundary() {
    let (mut gs, clock) = make_game(42);

    let bottom = TOTAL_ROWS - 1;
    fill_row(&mut gs.board, bottom);
    gs.line_clear_anim = Some(LineClearAnim {
        rows: vec![bottom],
        started_at: clock.now(),
        phase: LineClearPhase::Flash,
        board_snapshot: gs.board.clone(),
        pending_count: 1,
        pending_level_before: 1,
        pending_b2b_active: false,
    });

    // Advance past the flash boundary — should flip to WipeOutward.
    clock.advance(Duration::from_millis(ANIM_FLASH_MS));
    gs.step(Duration::from_millis(ANIM_FLASH_MS), &[]);

    let phase = gs
        .line_clear_anim
        .as_ref()
        .expect("animation should still be active after flash phase")
        .phase
        .clone();
    assert_eq!(phase, LineClearPhase::WipeOutward);
}

/// Advancing the full animation budget completes the animation: the board
/// row is cleared, and a `LinesCleared` event is emitted.
#[test]
fn anim_finishes_and_clears_board() {
    let (mut gs, clock) = make_game(42);

    let bottom = TOTAL_ROWS - 1;
    fill_row(&mut gs.board, bottom);
    let board_snap = gs.board.clone();

    gs.line_clear_anim = Some(LineClearAnim {
        rows: vec![bottom],
        started_at: clock.now(),
        phase: LineClearPhase::Flash,
        board_snapshot: board_snap,
        pending_count: 1,
        pending_level_before: 1,
        pending_b2b_active: false,
    });

    clock.advance(Duration::from_millis(ANIM_TOTAL_MS));
    let events = gs.step(Duration::from_millis(ANIM_TOTAL_MS), &[]);

    assert!(
        gs.line_clear_anim.is_none(),
        "animation must be cleared after total budget"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::LinesCleared { count: 1, .. })),
        "LinesCleared(1) event must be emitted on animation finish"
    );
    assert!(
        (0..COLS).all(|c| gs.board.cell_kind(c, bottom).is_none()),
        "cleared row must be empty after animation"
    );
}

/// During the animation (< total), gravity/lock are suspended and no new
/// piece is spawned prematurely.
#[test]
fn anim_suspends_gravity_during_play() {
    let (mut gs, clock) = make_game(42);

    let bottom = TOTAL_ROWS - 1;
    fill_row(&mut gs.board, bottom);
    // Remove active piece so we can test board-only state cleanly.
    gs.active = None;

    gs.line_clear_anim = Some(LineClearAnim {
        rows: vec![bottom],
        started_at: clock.now(),
        phase: LineClearPhase::Flash,
        board_snapshot: gs.board.clone(),
        pending_count: 1,
        pending_level_before: 1,
        pending_b2b_active: false,
    });

    // Advance mid-wipe but before completion.
    let midway = Duration::from_millis(ANIM_FLASH_MS + ANIM_WIPE_MS / 2);
    clock.advance(midway);
    gs.step(midway, &[]);

    // Bottom row must still be full (board not yet cleared).
    assert!(
        (0..COLS).all(|c| gs.board.cell_kind(c, bottom).is_some()),
        "row must remain intact while animating"
    );
    assert!(gs.line_clear_anim.is_some());
}

/// Inputs are accepted non-blocking during animation (no panic / hang).
#[test]
fn anim_accepts_inputs_without_blocking() {
    let (mut gs, clock) = make_game(42);

    let bottom = TOTAL_ROWS - 1;
    fill_row(&mut gs.board, bottom);
    gs.line_clear_anim = Some(LineClearAnim {
        rows: vec![bottom],
        started_at: clock.now(),
        phase: LineClearPhase::Flash,
        board_snapshot: gs.board.clone(),
        pending_count: 1,
        pending_level_before: 1,
        pending_b2b_active: false,
    });

    // Should not panic or deadlock even with movement inputs mid-animation.
    clock.advance(Duration::from_millis(20));
    gs.step(
        Duration::from_millis(20),
        &[Input::MoveLeft, Input::RotateCw],
    );
    assert!(gs.line_clear_anim.is_some());
}

// ── GameOver new-best logic ───────────────────────────────────────────────────

/// `is_new_best(score, None)` → false (no store).
#[test]
fn new_best_none_store_returns_false() {
    assert!(!blocktxt::render::hud::is_new_best(999_999, None));
}

/// `is_new_best(score, Some(empty_store))` → true (any score beats nothing).
#[test]
fn new_best_empty_store_returns_true() {
    let store = blocktxt::persistence::HighScoreStore::new();
    assert!(blocktxt::render::hud::is_new_best(1, Some(&store)));
}

/// `is_new_best` returns false when score ≤ existing best.
#[test]
fn new_best_below_existing_returns_false() {
    use blocktxt::persistence::HighScore;
    let mut store = blocktxt::persistence::HighScoreStore::new();
    store.insert(HighScore {
        name: "prev".into(),
        score: 10_000,
        level: 1,
        lines: 5,
        ts: 0,
    });
    assert!(!blocktxt::render::hud::is_new_best(9_999, Some(&store)));
    assert!(!blocktxt::render::hud::is_new_best(10_000, Some(&store)));
}

/// `is_new_best` returns true when score beats existing best.
#[test]
fn new_best_above_existing_returns_true() {
    use blocktxt::persistence::HighScore;
    let mut store = blocktxt::persistence::HighScoreStore::new();
    store.insert(HighScore {
        name: "prev".into(),
        score: 10_000,
        level: 1,
        lines: 5,
        ts: 0,
    });
    assert!(blocktxt::render::hud::is_new_best(10_001, Some(&store)));
}
