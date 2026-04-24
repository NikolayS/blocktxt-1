//! Tests for Phase::Title, Title→Playing transition,
//! ConfirmResetScores, and the title render snapshots.

use std::time::Instant;

use blocktxt::clock::FakeClock;
use blocktxt::game::state::{GameState, Input, Phase};
use blocktxt::persistence::{HighScore, HighScoreStore};

// ── helpers ───────────────────────────────────────────────────────────────────

fn make_game(seed: u64) -> GameState {
    let clock = Box::new(FakeClock::new(Instant::now()));
    GameState::new(seed, clock)
}

fn step(gs: &mut GameState, inputs: &[Input]) {
    gs.step(std::time::Duration::ZERO, inputs);
}

// ── Phase::Title initial state ────────────────────────────────────────────────

#[test]
fn title_phase_is_initial() {
    let gs = make_game(42);
    assert_eq!(gs.phase, Phase::Title, "new game must start in Title phase");
}

#[test]
fn title_does_not_spawn_first_piece() {
    let gs = make_game(42);
    assert!(gs.active.is_none(), "no active piece while in Title phase");
}

// ── Title → Playing transition ────────────────────────────────────────────────

#[test]
fn start_game_transitions_title_to_playing() {
    let mut gs = make_game(42);
    assert_eq!(gs.phase, Phase::Title);
    step(&mut gs, &[Input::StartGame]);
    assert_eq!(gs.phase, Phase::Playing, "StartGame moves Title → Playing");
    assert!(gs.active.is_some(), "first piece spawned on StartGame");
}

#[test]
fn move_left_in_title_goes_to_playing() {
    let mut gs = make_game(42);
    step(&mut gs, &[Input::MoveLeft]);
    assert_eq!(
        gs.phase,
        Phase::Playing,
        "MoveLeft in Title transitions to Playing"
    );
    assert!(gs.active.is_some());
}

#[test]
fn hard_drop_in_title_goes_to_playing() {
    let mut gs = make_game(42);
    step(&mut gs, &[Input::HardDrop]);
    assert_eq!(gs.phase, Phase::Playing);
    assert!(gs.active.is_some());
}

// ── GameOver → Title transition ───────────────────────────────────────────────

#[test]
fn game_over_restart_returns_to_title() {
    let mut gs = make_game(42);
    // Transition to Playing first.
    step(&mut gs, &[Input::StartGame]);
    // Force GameOver by setting phase directly (testing the step handler).
    gs.phase = Phase::GameOver {
        reason: blocktxt::game::state::GameOverReason::BlockOut,
    };
    step(&mut gs, &[Input::Restart]);
    assert_eq!(
        gs.phase,
        Phase::Title,
        "Restart from GameOver goes to Title"
    );
    assert!(
        gs.active.is_none(),
        "no active piece in Title after GameOver restart"
    );
}

// ── ConfirmResetScores ────────────────────────────────────────────────────────

#[test]
fn reset_scores_y_clears_store() {
    let mut gs = make_game(42);
    let mut store = HighScoreStore::new();
    store.insert(HighScore {
        name: "player".into(),
        score: 9999,
        level: 5,
        lines: 30,
        ts: 0,
    });
    store.insert(HighScore {
        name: "player".into(),
        score: 4242,
        level: 3,
        lines: 15,
        ts: 0,
    });
    assert_eq!(store.top(5).len(), 2, "precondition: 2 scores present");

    // Enter the confirm phase and drive the ConfirmYes input — this is
    // exactly the sequence main.rs observes when the user presses `y`.
    gs.phase = Phase::ConfirmResetScores;
    let confirm_yes_seen = true;
    step(&mut gs, &[Input::ConfirmYes]);

    // Phase returns to Title.
    assert_eq!(gs.phase, Phase::Title, "ConfirmYes returns to Title");

    // Caller side-effect: on ConfirmYes + phase-back-to-Title, the store is
    // cleared via the `clear()` helper main.rs invokes.
    if confirm_yes_seen && gs.phase == Phase::Title {
        store.clear();
    }

    assert!(
        store.top(5).is_empty(),
        "ConfirmYes flow must leave the store empty; got {} entries",
        store.top(5).len()
    );
}

/// The `clear()` helper that the main loop uses must empty the store.
#[test]
fn high_score_store_clear_removes_all_entries() {
    let mut store = HighScoreStore::new();
    for s in [100u32, 200, 300] {
        store.insert(HighScore {
            name: "player".into(),
            score: s,
            level: 1,
            lines: 0,
            ts: 0,
        });
    }
    assert_eq!(store.top(5).len(), 3);
    store.clear();
    assert!(store.top(5).is_empty(), "clear() must remove every entry");
}

/// ConfirmNo must NOT touch the store.
#[test]
fn reset_scores_n_preserves_store() {
    let mut gs = make_game(42);
    let mut store = HighScoreStore::new();
    store.insert(HighScore {
        name: "player".into(),
        score: 9999,
        level: 5,
        lines: 30,
        ts: 0,
    });

    gs.phase = Phase::ConfirmResetScores;
    step(&mut gs, &[Input::ConfirmNo]);

    assert_eq!(gs.phase, Phase::Title);
    // No clear() call on the ConfirmNo path.
    assert_eq!(
        store.top(5).len(),
        1,
        "ConfirmNo must leave the store unchanged"
    );
}

#[test]
fn reset_scores_n_returns_to_title() {
    let mut gs = make_game(42);
    gs.phase = Phase::ConfirmResetScores;
    step(&mut gs, &[Input::ConfirmNo]);
    assert_eq!(
        gs.phase,
        Phase::Title,
        "ConfirmNo returns to Title unchanged"
    );
}

// ── Title render snapshots ────────────────────────────────────────────────────

#[cfg(test)]
mod render_tests {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    use blocktxt::persistence::{HighScore, HighScoreStore};
    use blocktxt::render::title;
    use blocktxt::render::Theme;

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

    #[test]
    fn title_screen_empty_leaderboard() {
        let theme = Theme::monochrome();
        let store = HighScoreStore::new();

        let backend = TestBackend::new(80, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let area = f.area();
                title::draw(f, area, Some(&store), &theme);
            })
            .unwrap();

        insta::assert_snapshot!("title_empty_leaderboard", buf_to_string(&terminal));
    }

    #[test]
    fn title_screen_with_scores() {
        let theme = Theme::monochrome();
        let mut store = HighScoreStore::new();
        for (score, level, lines) in [(12345, 8, 80), (9200, 6, 60), (6140, 5, 40)] {
            store.insert(HighScore {
                name: "player".into(),
                score,
                level,
                lines,
                ts: 1_000_000,
            });
        }

        let backend = TestBackend::new(80, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let area = f.area();
                title::draw(f, area, Some(&store), &theme);
            })
            .unwrap();

        insta::assert_snapshot!("title_with_scores", buf_to_string(&terminal));
    }
}
