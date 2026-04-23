# v0.1.0 → v0.1.1 polish — visual diff

| | v0.1.0 (before) | v0.1.1 (after) |
|---|---|---|
| Playfield border | dashed `\|` / `+` straight lines | rounded Unicode frame (`╭─╮`/`╰─╯`) |
| Cell glyph | `[]` brackets (1-char wide) | `██` solid double-wide blocks |
| Cell width | 1 char | 2 chars (double-wide) |
| Palette | harsh primaries (red, yellow, green, blue) | Catppuccin Mocha pastels (RGB confirmed) |
| Next queue | 2-char `[]` rectangles, all same shape | actual piece shapes via `piece.cells()` |
| HUD labels | `Stats >`, `SCORE`, `LEVEL`, `NEXT` uppercase | lowercase `stats`/`next`, `score`/`level`/`lines` inline |
| HUD layout | HUD occupies ~full width above playfield | narrow (~24 char) right-side panel |
| Score format | plain digits | space-as-thousands-separator (e.g. `1 234 567`) |
| Game Over overlay | basic box | centered modal w/ "NEW BEST!" highlight (code-confirmed) |
| Pause overlay | basic | centered modal, 18-wide, rounded border (code-confirmed) |
| Ghost piece | none visible | `░░` light-shade chars at drop position (dimmed) |
| `just build` | didn't exist | 14-recipe Justfile ships |

## Evidence

- **Before**: [`docs/qa/v010-before.gif`](v010-before.gif)
- **After**: [`docs/qa/v011-after.gif`](v011-after.gif)

## Notes

Overlays (pause, game-over) were not captured in the PTY session because
piped stdin in headless mode does not deliver key events to the game loop.
Both overlays were verified in source (`src/render/hud.rs`:
`draw_pause_overlay`, `draw_game_over_overlay`).

Score was 0 during the short capture (no lines cleared), so thousand-separator
formatting is code-confirmed via `format_score` in `src/render/helpers.rs`
rather than visually confirmed.
