# tetris-tui-1 — SPEC v0.1

## 1. Goal & why it's needed

**Goal.** Ship a polished, single-player terminal Tetris clone with modern Guideline-compliant mechanics, playable on macOS and Linux terminals (and Windows via WSL), with persistent high scores.

**Why this exists.** Existing TUI Tetris clones in the wild fall into two buckets: (a) toys with non-standard rotation and wrong scoring that feel "off" to anyone who has played real Tetris, and (b) heavy GUI ports that require a windowing stack. There is a genuine niche for a *correct*, lightweight, keyboard-only Tetris that lives in a terminal tab — fast to launch, zero deps beyond a single binary, and faithful enough to Guideline mechanics (SRS rotation, 7-bag, proper scoring) that muscle memory from mainstream Tetris transfers cleanly. This project exists to fill that niche.

**Non-goals (explicit).** No multiplayer, no netcode, no sound, no GUI. These are out of scope per the brief and must not re-enter the design in disguise (e.g. no "stub network module", no audio crate pulled in "for later").

## 2. User stories

1. **Keyboard-native dev on macOS.** As a developer who lives in iTerm2/Ghostty/Alacritty, I want to run `tetris-tui` in a terminal tab and immediately play a full game with arrow-key / WASD / vim-style bindings, so I can take a five-minute break without leaving my terminal workflow. Outcome: the binary launches into a playable board in under 200 ms, and I can clear lines, pause with `p`, and quit with `q` without reading docs.
2. **Returning player chasing a high score.** As a returning player, I want my best score persisted across sessions so I can try to beat it tomorrow. Outcome: after a game-over, my score is written to a local high-score file; next launch shows the top-5 on the title screen, and a new personal best is highlighted on the game-over screen.
3. **Purist who expects real Tetris feel.** As a player who has played Guideline Tetris, I want SRS rotation with standard wall kicks, a 7-bag randomizer, a ghost piece, and a next-piece preview, so the game feels like the Tetris I know. Outcome: T-spin-adjacent kicks behave per the SRS kick table; I never get the same piece four times in a row impossibly; I can see where my piece will land and what's coming next.
4. **Linux power user on a slow SSH session.** As a user playing over SSH from a laptop on hotel Wi-Fi, I want the renderer to only redraw changed cells so input feels responsive even at 30–50 ms RTT. Outcome: no full-screen repaints per frame; inputs register within one game tick; no flicker.
5. **Accessibility-minded player.** As a player on a monochrome or 16-color terminal, I want the game to remain fully playable without 256-color support, using distinct ASCII glyphs per tetromino. Outcome: the game detects terminal color capability and falls back to glyph-based differentiation when color is unavailable or disabled via `NO_COLOR`.

## 3. Architecture

<!-- architecture:begin -->

```text
(architecture not yet specified)
```

<!-- architecture:end -->

**Language & stack.** Rust (stable, MSRV 1.75). Chosen because: (a) single static binary, trivial cross-platform distribution; (b) `ratatui` + `crossterm` is the most mature cross-platform TUI stack today; (c) strong type system makes the state machine (SRS kicks, lock delay, line-clear phases) much easier to get right than in a dynamic language; (d) the project's "veteran TUI game dev" persona maps to the Rust TUI ecosystem.

**Crate dependencies (v0.1, pinned and minimal):**

- `ratatui` — widget/framebuffer layer.
- `crossterm` — cross-platform terminal I/O backend (raw mode, events, color).
- `rand` + `rand_chacha` — seedable RNG for the 7-bag (ChaCha for reproducible seeds in tests).
- `serde` + `serde_json` — high-score persistence.
- `directories` — resolve platform config/data dirs (`~/Library/Application Support/...` on macOS, `$XDG_DATA_HOME` on Linux).
- `anyhow` / `thiserror` — error handling at the boundaries.
- `clap` (derive) — CLI flags (`--seed`, `--config`, `--reset-scores`).

No async runtime. No audio. No networking.

**Module layout (`src/`):**

```
main.rs            // CLI parsing, terminal setup/teardown, top-level loop
app.rs             // AppState enum: Title | Playing | Paused | GameOver; transitions
game/
  mod.rs           // re-exports
  board.rs         // 10x40 playfield (20 visible + buffer), collision, line detection
  piece.rs         // Tetromino enum (I,O,T,S,Z,J,L), shape tables, rotation states
  srs.rs           // SRS rotation + wall-kick tables (JLSTZ table + I table)
  bag.rs           // 7-bag randomizer with seedable RNG
  rules.rs         // gravity curve, lock delay, scoring, level progression
  score.rs         // Score + line-clear classification (single/double/triple/tetris)
input.rs           // Keybind map -> GameAction; DAS/ARR handling
render/
  mod.rs
  board_view.rs    // playfield renderer
  hud.rs           // score, level, lines, next-piece preview, ghost overlay
  theme.rs         // color/glyph lookup; NO_COLOR + capability detection
persistence.rs     // high-score file load/save (atomic write via tempfile+rename)
config.rs          // optional TOML config for keybinds
clock.rs           // monotonic time abstraction (real + fake for tests)
```

**Key abstractions.**

- `GameState` — pure, deterministic struct holding board, active piece, bag, stats, timers. All mutation goes through `GameState::step(dt, &[Input]) -> Vec<Event>`. No I/O, no rendering. This is the testable core.
- `Clock` trait — `now() -> Instant`-like; real impl wraps `std::time::Instant`, fake impl is a manually-advanced counter used in tests.
- `Rng` trait-object (or generic) — real impl is `ChaCha8Rng`, fake/seeded impl gives reproducible bags for tests.
- `Renderer` — takes `&GameState` and draws into a `ratatui::Frame`; no game logic inside.
- `Input` enum — `MoveLeft`, `MoveRight`, `SoftDrop`, `HardDrop`, `RotateCW`, `RotateCCW`, `Hold` (v0.2), `Pause`, `Quit`, `Restart`. Raw key events are translated to `Input` at the boundary.

**Data flow.**

```
terminal keys → crossterm::Event → input::translate → Input
                                                        │
  Clock::now() ──► main loop ──► GameState::step(dt, inputs) ──► Events
                                                        │
                                       Renderer::draw(&GameState) → ratatui Frame → stdout
```

The main loop runs at a fixed ~60 Hz redraw cadence, but game logic is dt-driven (gravity measured in seconds-per-cell from a Guideline-derived level curve), so rendering rate and simulation rate are decoupled.

**Terminal rendering details.**

- Each board cell is drawn as **two characters wide** (e.g. `"[]"` or `"██"`) so blocks look roughly square in a typical terminal font. Playfield area: `20 rows × 10 cols × 2 chars = 20×20` character grid.
- Colors: 8 Guideline piece colors when 256-color is available; fall back to 16-color; fall back to per-piece ASCII glyphs (`I`,`O`,`T`,`S`,`Z`,`J`,`L`) when `NO_COLOR` is set or color is unsupported.
- Only dirty regions are redrawn (`ratatui` handles diffing against its internal buffer).
- On startup: enter alternate screen, enable raw mode, hide cursor. On exit (including panic hook): restore all three. Panic hook is installed before any terminal mutation.

## 4. Implementation details

**Rotation system — SRS.** Implement full Guideline SRS:

- 4 rotation states (0, R, 2, L) per piece.
- JLSTZ share one wall-kick table (5 offsets per transition, 8 transitions).
- I piece has its own kick table.
- O piece does not kick.
- Rotation attempt: test base rotation; if blocked, test kicks in order; first non-colliding offset wins; if all fail, rotation is rejected.
- Kick tables are encoded as `const` arrays in `srs.rs` and unit-tested against the published SRS reference values.

**7-bag randomizer.** Sequence of 7 tetrominoes, shuffled (Fisher–Yates using the injected `Rng`), drained one at a time; when empty, refill + reshuffle. Seedable via `--seed` for tests and for reproducible speedruns.

**Gravity & level curve.** Use the Guideline formula: `gravity_seconds_per_cell = (0.8 − (level−1) × 0.007) ^ (level−1)`, clamped at level 20. Soft drop = 20× gravity (and awards 1 pt/cell). Hard drop = instant (awards 2 pts/cell).

**Lock delay.** 500 ms "extended placement" style: timer starts when piece first touches the stack from above; resets on successful move/rotate up to a cap of 15 resets; piece locks when timer expires or after 15 resets when still grounded. Cap prevents infinite stalling.

**Line clears.** After lock: detect full rows, play a brief clear animation (2 frames of inverted cells, ~100 ms), then collapse rows. Scoring (Guideline, no T-spin in v0.1):

| Clear | Points |
| --- | --- |
| Single | 100 × level |
| Double | 300 × level |
| Triple | 500 × level |
| Tetris | 800 × level |
| Back-to-back Tetris | ×1.5 |

Soft-drop: +1/cell. Hard-drop: +2/cell.

**Level progression.** Every 10 lines → level +1. Max level 20 for gravity; score keeps multiplying past that.

**State transitions.**

```
              start
                │
                ▼
          ┌──────────┐   Enter    ┌──────────┐   p      ┌──────────┐
          │  Title   ├──────────►│ Playing  ├────────►│  Paused  │
          └──────────┘            └────┬─────┘◄────────┴──────────┘
                ▲                      │ top-out   p/Enter
                │ r                    ▼
                │                ┌──────────┐
                └────────────────┤ GameOver │
                                 └──────────┘
```

**Top-out condition.** Piece spawns overlapping stack, OR any part of a locked piece sits entirely above row 20 (visible area).

**Input handling (DAS/ARR).** Delayed Auto Shift and Auto Repeat Rate are modeled explicitly:

- DAS (default 170 ms): hold left/right → wait DAS → then auto-shift.
- ARR (default 50 ms): interval between auto-shifts while held.
- Soft drop repeat uses its own rate (default 30 ms).
- These are configurable in the TOML config (v0.2 surfaces a UI for it).

**Persistence.**

- High scores: JSON array of `{score, lines, level, date}`, top 10 kept. Path: `directories::ProjectDirs::data_dir()/high_scores.json`.
- Writes are atomic: write to `high_scores.json.tmp`, `fsync`, `rename` over the real file. Corrupt file → back up to `high_scores.json.bak`, start fresh, log to stderr.
- Config (optional): `config.toml` in `config_dir()` with `[keybinds]` and `[timing]` sections. Missing file = defaults.

**CLI flags.**

- `--seed <u64>` — deterministic bag for recording/replay.
- `--reset-scores` — wipe high-score file (with confirmation prompt).
- `--config <path>` — override config file location.
- `--no-color` — force monochrome (also honors `NO_COLOR` env var).

**v0.1 scope decisions (resolved from interview).**

- **scope-and-modes:** single-player Marathon only. No Sprint/Ultra modes in v0.1.
- **rotation-and-rules:** full Guideline SRS with kicks; Guideline scoring; lock delay with 15-reset cap; 7-bag.
- **rendering-stack:** Rust + `ratatui` over `crossterm`; double-wide cells; dirty-region diffing via ratatui's buffer.
- **input-and-timing:** single-threaded main loop; `crossterm::event::poll` with short timeout; DAS/ARR handled in-sim with dt.
- **persistence-and-config:** JSON high-scores via `directories` + atomic rename; optional TOML config; `NO_COLOR` respected.

**v0.1 "nice-to-have" cut line.** IN for v0.1: 7-bag, next-piece preview, ghost piece (these are cheap and materially change feel). OUT for v0.1, tracked for v0.2: hold piece, T-spin detection, color themes.

## 5. Tests plan

**Red/green TDD call-outs.** The following components are built **test-first** — failing test committed before implementation:

- `srs.rs` wall-kick tables and rotation resolution.
- `bag.rs` 7-bag invariants (every 7 consecutive pieces contain all 7 types).
- `rules.rs` scoring for single/double/triple/tetris and back-to-back multiplier.
- `rules.rs` gravity formula values at levels 1, 5, 10, 15, 20.
- `rules.rs` lock-delay 15-reset cap.
- `board.rs` line detection + row collapse.
- `persistence.rs` atomic write + corrupt-file recovery.

UI / rendering code is NOT TDD'd — it's developed against a hand-rolled `Renderer::draw` that writes into an in-memory `Buffer`, then visually verified.

**CI tests (must pass on every PR):**

1. **Unit tests** — `cargo test` across all modules. Fast, deterministic, seeded RNG.
2. **SRS conformance suite** — golden test of every kick from every rotation state, asserted against an embedded table derived from the published SRS spec.
3. **Bag invariant property test** — `proptest` generating 1000 seeds, drawing 700 pieces each, asserting every 7-window is a permutation of `IOTSZJL`.
4. **Scoring table test** — parameterized test covering the 4 clear types × 5 levels × with/without B2B.
5. **Snapshot render tests** — render `GameState` fixtures into a ratatui `Buffer`, compare against committed `.snap` files using `insta`. Covers: empty board, mid-game, game-over overlay, pause overlay, no-color fallback.
6. **Integration test: headless game loop** — construct `GameState` with fake `Clock` and fake `Rng`, feed a scripted input sequence, assert final score/lines/level. Guards against regressions in `step()`.
7. **Persistence round-trip test** — write high scores, read back, assert equality; separately, write then corrupt the file, assert graceful recovery + backup.
8. **Cross-platform CI** — GitHub Actions matrix: `ubuntu-latest` + `macos-latest`, stable toolchain, `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`.
9. **Panic-safety test** — force a panic mid-game in a subprocess, assert terminal state is restored (via a smoke script that checks raw mode is off after exit).

**Manual test plan (mirrors user stories).** Each user story in §2 has a one-paragraph manual walkthrough in `docs/manual-test.md` — run before tagging a release.

## 6. Team

Veteran experts to hire for v0.1:

- **Veteran Rust systems engineer (1)** — owns build, cross-platform concerns, CI matrix, `directories`/persistence, panic safety.
- **Veteran TUI / ratatui specialist (1)** — owns `render/`, double-wide cell layout, color/glyph fallback, snapshot tests.
- **Veteran game-logic engineer w/ Tetris domain knowledge (1)** — owns `srs.rs`, `bag.rs`, `rules.rs`, scoring, lock-delay, level curve. This is the hardest-to-get-right role.
- **Veteran QA / test engineer (1)** — owns property tests, SRS conformance suite, snapshot fixtures, manual test doc.

Total: 4 engineers. Roles are distinct enough to parallelize cleanly after the first sprint.

## 7. Implementation plan

Four sprints of ~1 week each. Sprint 1 is mostly sequential (scaffolding); sprints 2–3 parallelize across the four engineers; sprint 4 converges.

### Sprint 1 — Foundation (week 1, mostly serial)

- **Rust systems eng:** repo scaffold, `cargo` workspace layout, CI matrix (Ubuntu + macOS), `rustfmt`/`clippy` gates, panic hook + terminal teardown, CLI skeleton with `clap`. Ships a binary that opens + closes a blank ratatui screen cleanly.
- **TUI specialist:** draw an empty 10×20 playfield with border and placeholder HUD; double-wide cell layout verified on multiple terminals (iTerm2, Alacritty, Ghostty, gnome-terminal).
- **Game-logic eng:** `board.rs` + `piece.rs` data structures; shape tables for all 7 tetrominoes in 4 rotation states; collision primitive. TDD.
- **QA eng:** set up `insta` snapshot infra, `proptest` dependency, first failing SRS kick test (red).

Exit criteria: `cargo test` passes; binary opens/closes cleanly on both OSes; empty board renders.

### Sprint 2 — Core mechanics (week 2, parallel)

- **Game-logic eng:** implement `srs.rs` (turn QA's red tests green), `bag.rs` with 7-bag invariant, `rules.rs` gravity curve + lock delay + scoring. All TDD.
- **TUI specialist:** wire `Renderer` to real `GameState`; implement ghost piece and next-piece preview; NO_COLOR / glyph fallback path.
- **Rust systems eng:** `persistence.rs` with atomic write + corruption recovery; `config.rs` TOML loader with defaults; `clock.rs` abstraction.
- **QA eng:** SRS conformance golden suite; bag invariant property test; scoring parameterized test; first snapshot fixtures (empty / mid-game / game-over).

Exit criteria: a human can play a full game end-to-end in a dev build; all core TDD tests green; snapshot tests cover the three main screens.

### Sprint 3 — Polish & edge cases (week 3, parallel)

- **Game-logic eng:** soft-drop/hard-drop scoring, back-to-back Tetris multiplier, top-out detection, level progression wiring.
- **TUI specialist:** line-clear animation (2-frame invert), pause overlay, game-over overlay with new-best-score highlight, title screen with top-5 high scores.
- **Rust systems eng:** DAS/ARR input model; `--seed` / `--reset-scores` / `--config` flags; panic-safety smoke test.
- **QA eng:** integration test with scripted input sequence + fake clock/rng; cross-platform CI matrix green; manual test doc drafted.

Exit criteria: feature-complete for v0.1 scope; CI matrix green on both OSes; manual test doc complete.

### Sprint 4 — Hardening & release (week 4, converging)

- All: bug bash against the manual test plan, fix regressions, finalize README, tag v0.1.0.
- **Rust systems eng:** release profile tuning (`lto = "thin"`, `codegen-units = 1`), binary size check, release artifacts for macOS (arm64 + x86_64) and Linux (x86_64).
- **QA eng:** re-run snapshot suite on release build; sign off on user-story walkthroughs.

Exit criteria: tagged release, binaries attached, README with install + keybinds.

**Parallelization map.** After sprint 1, Game-logic ↔ TUI are mostly independent because `GameState` is pure and `Renderer` only reads it — the interface is the only coupling point and is nailed down in sprint 1. Systems eng work (persistence, config, CLI) is independent of both. QA work follows game-logic by ~1 day throughout.

## 8. Embedded Changelog

- **v0.1 (current)** — Initial spec. Scope: Rust + ratatui/crossterm; Guideline SRS + 7-bag + ghost + next-preview; Marathon mode; JSON high-score persistence; macOS + Linux CI. Deferred to v0.2: hold piece, T-spin detection, color themes.
