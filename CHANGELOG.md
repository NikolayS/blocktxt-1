# Changelog

All notable changes to this project will be documented in this file.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

---

## [0.1.0] — 2026-04-23

First public release. A single-player terminal falling-block puzzle
game. Single static binary for macOS (Apple Silicon + Intel) and Linux
(x86_64). Zero deps beyond what ships in the binary. No network, no
sound, no GUI.

### Added

- **Core gameplay**: SRS rotation with JLSTZ + I kick tables (derived
  from public mathematical spec), 7-piece bag randomizer with correct
  aligned-window invariant and max-gap ≤ 12, ghost piece, next-piece
  preview (up to 5), Guideline-style scoring
  (100/300/500/800 × level), Back-to-Back 4-line clears at ×1.5,
  gravity curve to level 20, lock delay with 500 ms timer + 15-move
  reset cap, soft-drop cap (`max(natural/20, 30ms/cell)`), hard-drop
  bypass, top-out (block-out + lock-out) state machine, pause, restart
  from Game Over.
- **Input**: arrow keys / WASD / vim-style. DAS 160 ms / ARR 30 ms
  with kitty keyboard protocol probe + 160 ms release-inference
  fallback for terminals without press/release events.
- **Rendering**: `ratatui` + `crossterm` TUI at an 8 ms poll /
  16 ms frame cadence; line-clear animation (flash / dim / collapse
  over 200 ms); pause overlay; Game Over overlay with **NEW BEST!**
  highlight when the score beats the stored top score.
- **Persistence**: top-5 high-score store at the platform-standard
  config directory, atomic write via `tempfile::NamedTempFile::persist`
  with fsync + parent-dir fsync + 0o600 file mode in a 0o700 directory
  (symlink-refusing, world-writable-refusing, ownership-verified).
  Corrupted score files are moved aside to
  `scores.json.corrupt.<unix_secs>` (cap 5), and the game continues
  with in-memory scores.
- **CLI flags**: `--seed <u64>` for reproducible piece sequences,
  `--no-color` for monochrome/glyph fallback (supplements `NO_COLOR`
  env), `--reset-scores` to prompt (in cooked mode) and delete the
  high-score file, `--version`, `--help`.
- **Accessibility**: `NO_COLOR`-aware theme with distinct glyphs for
  16-color and monochrome terminals; minimum terminal size 44 × 24
  with a "too small" overlay while smaller.
- **Terminal lifecycle**: `TerminalGuard` with ordered setup +
  idempotent Drop-based restore; panic hook writes ANSI reset bytes
  directly to fd 2 (async-signal-safe). Flag-based signal handlers
  (via `signal-hook`) for SIGINT/SIGTERM/SIGTSTP/SIGCONT/SIGWINCH —
  handlers only `AtomicBool::store`; the main loop performs all
  terminal I/O per SPEC §4 async-signal-safe design.
- **Docs**: README with install/play/flags/high-scores/accessibility,
  trademark-free; `docs/manual-test-plan.md` — 26-item checklist
  across 7 sections for release bug bash.
- **Release workflow**: `.github/workflows/release.yml` matrix builds
  for `x86_64-unknown-linux-gnu` / `aarch64-apple-darwin` /
  `x86_64-apple-darwin` on tag; 8 MiB binary-size guard; tarballs
  uploaded to the GitHub release.

### Changed

- **Runtime RNG is `StdRng` (ChaCha12)**, not `ChaCha8`. `rand_chacha`
  is a dev-dep only; runtime code uses `rand::rngs::StdRng`. Seeds
  produce different bag sequences than prior dev versions — no prior
  public release carried the earlier RNG.

### Tests

- 147 tests across the suite: unit + property-based (`proptest`) for
  SRS rotation conformance, 7-bag invariant, scoring/level progression,
  lock-delay state transitions; snapshot tests via `insta` for HUD /
  board / ghost / pause / Game Over / line-clear / too-small overlay
  variants; PTY-based lifecycle tests via `rexpect` (Linux-only where
  macOS PTY master/slave termios semantics diverge); 5000-tick seeded
  e2e stress test with no-panic + state-consistency assertions each
  tick; DAS/ARR boundary sweep 0–500 ms; persistence failure-mode
  suite (symlink, ownership, world-writable parent, corrupt +
  timestamped backups capped at 5).

### Known limitations

- macOS `SIGTSTP`/`SIGCONT` termios round-trip is not tested via PTY
  (reliable cross-platform PTY termios observation is a research
  project); the flag-based handler implementation is exercised on
  Linux CI.
- Windows support is via WSL only; no native Win32 target in this
  release.
