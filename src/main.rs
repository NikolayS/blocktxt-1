mod cli;
mod signals;
mod terminal;

use std::io;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crossterm::event;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use terminal::{install_panic_hook, TerminalGuard};

use crossterm::event::{Event as CEvent, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;

use blocktxt::clock::RealClock;
use blocktxt::game::state::{GameState, Phase};
use blocktxt::input::{InputTranslator, KittySupport};
use blocktxt::persistence::{self, HighScore, HighScoreStore};
use blocktxt::render::theme::{Palette, OVERLAY, TEXT};
use blocktxt::render::{self, title as title_screen, Theme};
use blocktxt::{Event as GameEvent, Input};

// ── Title-phase input helpers ─────────────────────────────────────────────────

/// Translate a raw event on the Title screen.
///
/// - `q` / Ctrl-C / Ctrl-D → quit.
/// - `r` → sets `reset_scores_pressed` flag (caller handles phase change).
/// - Any other printable / arrow / space / enter → `Input::StartGame`.
fn translate_title_input(
    ev: &CEvent,
    _now: Instant,
    reset_scores_pressed: &mut bool,
) -> (Vec<Input>, bool) {
    let mut inputs = Vec::new();
    let mut quit = false;

    if let CEvent::Key(key) = ev {
        if key.kind == KeyEventKind::Release {
            return (inputs, quit);
        }
        // Quit keys.
        if matches!(key.code, KeyCode::Char('q'))
            || (key.modifiers.contains(KeyModifiers::CONTROL)
                && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('d')))
        {
            quit = true;
            return (inputs, quit);
        }
        // Reset scores.
        if key.code == KeyCode::Char('r') {
            *reset_scores_pressed = true;
            return (inputs, quit);
        }
        // Everything else starts the game.
        inputs.push(Input::StartGame);
    }
    (inputs, quit)
}

/// Translate a raw event in the ConfirmResetScores overlay.
///
/// - `y` → `ConfirmYes`.
/// - `n` / Escape → `ConfirmNo`.
/// - `q` / Ctrl-C / Ctrl-D → quit.
fn translate_confirm_input(ev: &CEvent, confirm_yes: &mut bool) -> (Vec<Input>, bool) {
    let mut inputs = Vec::new();
    let mut quit = false;

    if let CEvent::Key(key) = ev {
        if key.kind == KeyEventKind::Release {
            return (inputs, quit);
        }
        if matches!(key.code, KeyCode::Char('q'))
            || (key.modifiers.contains(KeyModifiers::CONTROL)
                && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('d')))
        {
            quit = true;
            return (inputs, quit);
        }
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                *confirm_yes = true;
                inputs.push(Input::ConfirmYes);
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                inputs.push(Input::ConfirmNo);
            }
            _ => {}
        }
    }
    (inputs, quit)
}

// ── Confirm-reset overlay ─────────────────────────────────────────────────────

/// Draw the "reset scores?" confirmation modal on top of the title screen.
fn draw_confirm_reset_overlay(frame: &mut Frame, area: Rect, _theme: &Theme) {
    let overlay_w: u16 = 21;
    let overlay_h: u16 = 7;
    let x = area.x + area.width.saturating_sub(overlay_w) / 2;
    let y = area.y + area.height.saturating_sub(overlay_h) / 2;
    let overlay_area = Rect::new(x, y, overlay_w.min(area.width), overlay_h.min(area.height));

    let content = Text::from(vec![
        Line::from(""),
        Line::from(Span::styled("  y  yes", Style::default().fg(TEXT))),
        Line::from(Span::styled("  n  no / back", Style::default().fg(TEXT))),
        Line::from(""),
    ]);

    frame.render_widget(Clear, overlay_area);
    frame.render_widget(
        Paragraph::new(content)
            .block(
                Block::default()
                    .title("─ reset scores? ─")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(OVERLAY)),
            )
            .alignment(Alignment::Left),
        overlay_area,
    );
}

fn main() -> anyhow::Result<()> {
    // Step 1: Parse CLI args (clap derive).
    let args = cli::parse();

    // Step 2: Handle --reset-scores in cooked mode before any raw mode.
    if args.reset_scores {
        cli::handle_reset_scores(&args)?;
        return Ok(());
    }

    // Parse --theme early so invalid values exit before entering raw mode.
    let palette: Palette = match args.theme.parse() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("blocktxt: {e}");
            std::process::exit(2);
        }
    };

    // Step 3: Load persistence BEFORE entering raw mode so any warning goes
    //         to stderr while the terminal is still in cooked mode.
    let persist_dir_result = persistence::init_data_dir();
    // Keep a copy of the path (if Ok) for use in the game loop.
    let persist_dir_path = persist_dir_result.as_ref().ok().cloned();
    let (mut store, persist_err) = HighScoreStore::new_with_fallback(persist_dir_result);

    if let Some(ref err) = persist_err {
        eprintln!(
            "blocktxt: warning: persistence unavailable ({err}); \
             scores will not be saved this session."
        );
    }

    // Step 4: Install signal flags BEFORE guard enter so Ctrl-C works if
    //         guard setup fails partway through.
    // Step 5: Install the panic hook BEFORE guard setup so the terminal is
    //         restored even if setup panics.
    install_panic_hook();

    #[cfg(unix)]
    let flags = signals::unix::install()?;

    // Step 6: Probe kitty protocol (quick 50 ms probe before entering raw mode).
    //
    // Risk note (P-4): if the process panics between writing the probe query
    // bytes (`CSI > 1u`) and a terminal that does not understand them, some
    // stray characters could in principle remain in the terminal.  In
    // practice the panic hook installed above calls `restore_raw()` which
    // writes a known ANSI reset sequence to fd 2, so the terminal ends up
    // cleanly restored regardless of probe response state.
    let kitty = InputTranslator::probe_kitty(Duration::from_millis(50));

    // Step 7: Enter TUI (raw mode + alternate screen + hide cursor).
    let mut guard = TerminalGuard::enter()?;

    // Step 8: --crash-for-test panics after guard entry (used by PTY tests).
    if args.crash_for_test {
        panic!("--crash-for-test: intentional panic after guard entry");
    }

    // Step 9: Run game loop.
    run_loop(
        &mut guard,
        &flags,
        &args,
        &mut store,
        persist_dir_path.as_deref(),
        kitty,
        palette,
    )?;

    Ok(())
}

#[cfg(unix)]
fn run_loop(
    guard: &mut TerminalGuard,
    flags: &signals::unix::Flags,
    args: &cli::Args,
    store: &mut HighScoreStore,
    persist_dir: Option<&std::path::Path>,
    kitty: KittySupport,
    palette: Palette,
) -> anyhow::Result<()> {
    use nix::sys::signal::{self, Signal};

    // Build ratatui terminal on top of the existing crossterm raw-mode/alt-screen
    // that TerminalGuard already set up.
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    // Game state + theme.
    let seed = args.seed.unwrap_or_else(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64 ^ d.as_secs())
            .unwrap_or(42)
    });
    let clock = Box::new(RealClock);
    let mut state = GameState::new(seed, clock);
    let theme = Theme::detect(args.no_color, palette);

    // DAS/ARR input translator.
    let mut translator = InputTranslator::new(kitty);

    // Frame cadence: 16 ms ceiling (≈ 60 fps).
    const FRAME_DT: Duration = Duration::from_millis(16);
    // Event poll: 8 ms per SPEC §4.
    const POLL_DT: Duration = Duration::from_millis(8);

    let mut last_frame = Instant::now();

    loop {
        // --- 1. Signal flags (ordered per SPEC §3) ---

        if flags.shutdown.swap(false, Ordering::Relaxed) {
            break;
        }

        if flags.tstp_pending.swap(false, Ordering::Relaxed) {
            guard.restore();
            let _ = signal::raise(Signal::SIGSTOP);
            guard.re_enter()?;
            flags.cont_pending.store(false, Ordering::Relaxed);
            flags.winch_pending.store(true, Ordering::Relaxed);
            terminal.clear()?;
            last_frame = Instant::now();
            continue;
        }

        if flags.cont_pending.swap(false, Ordering::Relaxed) {
            guard.re_enter()?;
            flags.winch_pending.store(true, Ordering::Relaxed);
        }

        if flags.winch_pending.swap(false, Ordering::Relaxed) {
            terminal.autoresize()?;
        }

        // --- 2. Collect inputs ---
        let now = Instant::now();
        let mut inputs: Vec<Input> = Vec::new();
        let mut quit_requested = false;
        // Track whether 'r' was pressed (for reset-scores on title).
        let mut reset_scores_pressed = false;
        // Track ConfirmYes (y) pressed in ConfirmResetScores phase.
        let mut confirm_yes_pressed = false;

        // Poll for events (8 ms timeout).
        if event::poll(POLL_DT)? {
            let ev = event::read()?;
            // Phase-aware input translation.
            let (translated, quit) = match &state.phase {
                Phase::Title => translate_title_input(&ev, now, &mut reset_scores_pressed),
                Phase::ConfirmResetScores => translate_confirm_input(&ev, &mut confirm_yes_pressed),
                _ => {
                    let (evs, q) = translator.translate_event(&ev, now);
                    (evs, q)
                }
            };
            inputs.extend(translated);
            if quit {
                quit_requested = true;
            }
        }

        // Emit any DAS/ARR ticks (only meaningful in Playing / Paused).
        if matches!(state.phase, Phase::Playing | Phase::Paused) {
            translator.tick(now, &mut inputs);
        }

        if quit_requested {
            break;
        }

        // --- 3. Step game ---
        let dt = now.duration_since(last_frame).min(FRAME_DT * 2);

        // If 'r' on Title → enter ConfirmResetScores.
        if reset_scores_pressed && matches!(state.phase, Phase::Title) {
            state.phase = Phase::ConfirmResetScores;
        } else {
            let game_events = state.step(dt, &inputs);

            // Handle emitted events — save score on game-over.
            for ev in game_events {
                if let GameEvent::GameOver(_) = ev {
                    let ts = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    let hs = HighScore {
                        name: "player".to_owned(),
                        score: state.score,
                        level: state.level,
                        lines: state.lines_cleared,
                        ts,
                    };
                    let new_best = store.insert(hs);
                    if new_best {
                        eprintln!(
                            "blocktxt: new personal best: {} points at \
                             level {}.",
                            state.score, state.level
                        );
                    }
                    if let Some(dir) = persist_dir {
                        if let Err(e) = persistence::save(store, dir) {
                            eprintln!("blocktxt: warning: could not save score: {e}");
                        }
                    }
                }
            }

            // If ConfirmYes was pressed and phase just returned to Title,
            // clear the in-memory store and persist the empty store.
            if confirm_yes_pressed && matches!(state.phase, Phase::Title) {
                *store = HighScoreStore::new();
                if let Some(dir) = persist_dir {
                    if let Err(e) = persistence::save(store, dir) {
                        eprintln!(
                            "blocktxt: warning: could not save cleared \
                             scores: {e}"
                        );
                    }
                }
            }
        }

        last_frame = now;

        // --- 4. Draw ---
        let store_ref: &HighScoreStore = store;
        match &state.phase {
            Phase::Title => {
                terminal.draw(|f| {
                    let area = f.area();
                    title_screen::draw(f, area, Some(store_ref), &theme);
                })?;
            }
            Phase::ConfirmResetScores => {
                terminal.draw(|f| {
                    let area = f.area();
                    title_screen::draw(f, area, Some(store_ref), &theme);
                    draw_confirm_reset_overlay(f, area, &theme);
                })?;
            }
            _ => {
                terminal
                    .draw(|f| render::render_with_scores(f, &state, &theme, Some(store_ref)))?;
            }
        }

        // --- 5. Sleep remainder of frame budget ---
        let elapsed = last_frame.elapsed();
        if elapsed < FRAME_DT {
            std::thread::sleep(FRAME_DT - elapsed);
        }
    }

    Ok(())
}

// Non-unix stub so the crate still compiles on Windows (WSL takes the unix path).
#[cfg(not(unix))]
fn run_loop(
    _guard: &mut TerminalGuard,
    _flags: &(),
    _args: &cli::Args,
    _store: &mut HighScoreStore,
    _persist_dir: Option<&std::path::Path>,
    _kitty: KittySupport,
    _palette: Palette,
) -> anyhow::Result<()> {
    Ok(())
}
