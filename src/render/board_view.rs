//! Board view: draws the 12×24 visible playfield, active piece, and ghost.
//!
//! Dimensions and cell glyphs are intentionally distinct from the canonical
//! 10×20 falling-block presentation (see SPEC §1a — originality pass).

use std::time::Duration;

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, BorderType, Borders};
use ratatui::Frame;

use crate::game::board::{BUFFER_ROWS, COLS as BOARD_COLS, VISIBLE_ROWS as BOARD_VISIBLE_ROWS};
use crate::game::piece::Piece;
use crate::game::state::{
    GameState, LineClearAnim, ANIM_FLASH_MS, ANIM_WIPE_MS, SPAWN_FADE1_MS, SPAWN_FADE_TOTAL_MS,
};
use crate::render::helpers::ghost_y;
use crate::render::theme::{Theme, BASE, MANTLE, OVERLAY};

/// Compute the spawn-fade intensity multiplier (0.0–1.0) for the active piece.
///
/// - 0–40 ms: 0.60 (60 % intensity)
/// - 40–80 ms: 0.80 (80 % intensity)
/// - ≥ 80 ms / no anim: 1.00 (full intensity)
fn spawn_fade_factor(state: &GameState) -> f32 {
    let Some(ref sa) = state.spawn_anim else {
        return 1.0;
    };
    let now = state.now();
    let elapsed = now.saturating_duration_since(sa.started_at);
    if elapsed >= Duration::from_millis(SPAWN_FADE_TOTAL_MS) {
        1.0
    } else if elapsed >= Duration::from_millis(SPAWN_FADE1_MS) {
        0.8
    } else {
        0.6
    }
}

/// Dim an RGB color by `factor` (0.0–1.0) by scaling each channel.
pub fn dim_color(c: Color, factor: f32) -> Color {
    match c {
        Color::Rgb(r, g, b) => Color::Rgb(
            (r as f32 * factor) as u8,
            (g as f32 * factor) as u8,
            (b as f32 * factor) as u8,
        ),
        other => other, // non-RGB colors pass through unchanged
    }
}

/// Intensity multiplier applied to locked cells (originality pass).
///
/// Canonical falling-block presentations render locked cells in the same
/// color as the active piece. We dim them to 60 % so the falling piece
/// visibly pops above the settled stack. See SPEC §1a.
pub const LOCKED_CELL_DIM: f32 = 0.6;

/// Width of one cell in terminal columns (each cell is 2 chars wide).
const CELL_W: u16 = 2;
/// Height of one cell in terminal rows.
const CELL_H: u16 = 1;

/// Number of visible rows (24 on the 48-row playfield).
pub const VISIBLE_ROWS: i32 = BOARD_VISIBLE_ROWS as i32;
/// Number of columns (12 after originality pass).
pub const COLS: i32 = BOARD_COLS as i32;

/// Filled-cell glyph pair.
///
/// `▰` (U+25B0, "Black Parallelogram") is chosen instead of the
/// canonical `█` full-block so the playfield has a deliberate dingbat
/// look that does not resemble any specific commercial falling-block
/// implementation. See SPEC §1a originality note.
pub const FILLED: &str = "▰▰";
/// Legacy ghost-cell glyph (piece-shaped overlay); retained for tests that
/// want to confirm it is no longer rendered on the board.
#[allow(dead_code)]
pub const GHOST: &str = "░░";
/// Floor-line ghost glyph pair.
///
/// Instead of previewing the landing position as a piece-shaped outline
/// (which matches the canonical falling-block ghost), we draw a single
/// horizontal line spanning the columns the piece will occupy at the row
/// where its bottom would rest. `▔` (U+2594 "Upper One Eighth Block")
/// reads as a crisp floor marker without intruding into the cell above.
pub const GHOST_LINE: &str = "▔▔";
/// Empty-cell glyph pair (two spaces).
pub const EMPTY: &str = "  ";
/// Glyph used during the flash phase of the line-clear animation
/// (`▣` = U+25A3 "White Square Containing Black Small Square").
pub const FLASH: &str = "▣▣";

/// Compute how many cells (measured from the board centerline) have been
/// wiped at `elapsed` into a wipe-outward animation.
///
/// The wipe covers `COLS / 2 + 1` cells over `ANIM_WIPE_MS` milliseconds, so
/// the full width of the board is cleared when the wipe phase ends.
pub fn wipe_radius_cells(elapsed_ms: u64) -> u16 {
    let wipe_ms = ANIM_WIPE_MS.max(1);
    let full = (BOARD_COLS as u64 / 2) + 1;
    let clamped = elapsed_ms.min(wipe_ms);
    ((clamped * full) / wipe_ms) as u16
}

/// True if cell `col` should be rendered as wiped (collapsed to background).
///
/// The center-left cell is at col `COLS/2 - 1`; the center-right at `COLS/2`.
/// At `radius=0`, nothing is wiped. At `radius=1`, the two center cells are
/// wiped. At `radius=COLS/2+1`, the entire row is wiped.
pub fn cell_is_wiped(col: usize, radius: u16) -> bool {
    if radius == 0 {
        return false;
    }
    let center_left = (BOARD_COLS / 2).saturating_sub(1) as i32;
    let center_right = (BOARD_COLS / 2) as i32;
    let col = col as i32;
    let dist_left = (col - center_left).abs();
    let dist_right = (col - center_right).abs();
    let min_dist = dist_left.min(dist_right) as u16;
    min_dist < radius
}

/// Draw the visible 12×24 playfield into `area`.
///
/// Renders the board inside a rounded border. Inside:
///   1. Locked board cells (rows BUFFER_ROWS..TOTAL_ROWS).
///      During a line-clear animation, full rows are highlighted:
///      - Flash phase: bright white + BOLD using the flash glyph (`▣▣`).
///      - WipeOutward phase: cells inside the wipe radius (from center) are
///        cleared to background; cells outside are dimmed.
///   2. Ghost piece (`░░`, dimmed) at the drop position.
///   3. Active piece (`▰▰`) at its current position.
pub fn draw(frame: &mut Frame, area: Rect, state: &GameState, theme: &Theme) {
    // Draw the bordered playfield container.
    let board_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(OVERLAY))
        .style(Style::default().bg(BASE));
    let inner = board_block.inner(area);
    frame.render_widget(board_block, area);

    // Animation context: rows being animated + wipe radius given the clock.
    let anim: Option<AnimCtx> = state
        .line_clear_anim
        .as_ref()
        .map(|a| make_anim_ctx(a, state));

    // 1. Draw background / empty cells.
    for vis_row in 0..VISIBLE_ROWS {
        for col in 0..COLS {
            let x = inner.x + (col as u16) * CELL_W;
            let y = inner.y + vis_row as u16 * CELL_H;
            if x + CELL_W <= inner.x + inner.width && y < inner.y + inner.height {
                frame.render_widget(
                    ratatui::widgets::Paragraph::new(Span::styled(
                        EMPTY,
                        Style::default().bg(MANTLE),
                    )),
                    Rect::new(x, y, CELL_W, CELL_H),
                );
            }
        }
    }

    // 2. Locked board cells (rows BUFFER_ROWS..BUFFER_ROWS+VISIBLE_ROWS).
    //
    // Locked cells render DIMMER than the active piece (LOCKED_CELL_DIM).
    // This is an explicit deviation from the canonical convention where a
    // locked cell keeps the same color as its active-piece state — see
    // SPEC §1a.
    for vis_row in 0..VISIBLE_ROWS {
        let board_row = (vis_row as usize) + BUFFER_ROWS;

        for col in 0..COLS {
            if let Some(kind) = state.board.cell_kind(col as usize, board_row) {
                let base_color = if theme.monochrome {
                    Color::Reset
                } else {
                    dim_color(theme.color(kind), LOCKED_CELL_DIM)
                };
                let x = inner.x + (col as u16) * CELL_W;
                let y = inner.y + vis_row as u16 * CELL_H;
                if x + CELL_W > inner.x + inner.width || y >= inner.y + inner.height {
                    continue;
                }

                // Determine rendering mode for this cell given anim state.
                let (text, style) = if let Some(ref ctx) = anim {
                    if ctx.rows.contains(&board_row) {
                        render_anim_cell(ctx, col as usize, base_color, theme.monochrome)
                    } else {
                        default_cell_style(base_color, theme.monochrome)
                    }
                } else {
                    default_cell_style(base_color, theme.monochrome)
                };

                frame.render_widget(
                    ratatui::widgets::Paragraph::new(Span::styled(text, style)),
                    Rect::new(x, y, CELL_W, CELL_H),
                );
            }
        }
    }

    // 3. Ghost (floor line) then active piece (skip during animation).
    if let Some(active) = &state.active {
        let ghost_row = ghost_y(&state.board, active);
        // Only draw the floor-line when the ghost is below the piece body;
        // when ghost_row == active origin the piece is already on the floor
        // (or flush against a stack) and a duplicate line underneath would
        // either render under the piece or outside the playfield.
        if ghost_row > active.origin.1 {
            render_ghost_floor_line(frame, inner, active, ghost_row, theme);
        }
        let fade = spawn_fade_factor(state);
        render_piece(frame, inner, active, theme, fade);
    }
}

/// Draw the originality-pass ghost: a single horizontal floor line spanning
/// the columns the piece would occupy at the row where its bottom rests.
///
/// Renders at the row of the piece's lowest occupied cell (not one below),
/// using `▔▔` which sits at the TOP of the cell — so it reads as a line
/// directly under the active piece without overlapping it.
fn render_ghost_floor_line(
    frame: &mut Frame,
    area: Rect,
    active: &Piece,
    ghost_origin_row: i32,
    theme: &Theme,
) {
    let ghost_piece = Piece {
        origin: (active.origin.0, ghost_origin_row),
        ..*active
    };

    // Collect the (col, row) pairs the piece would occupy at rest.
    let cells = ghost_piece.cells();

    // For each column the ghost body touches, find the LOWEST row. That's
    // the row whose top edge we'll paint with the floor line — giving one
    // line segment per distinct column, aligned to the actual landing row.
    let mut lowest_by_col: std::collections::BTreeMap<i32, i32> = Default::default();
    for &(c, r) in &cells {
        lowest_by_col
            .entry(c)
            .and_modify(|cur| {
                if r > *cur {
                    *cur = r;
                }
            })
            .or_insert(r);
    }

    let color = if theme.monochrome {
        Color::Reset
    } else {
        dim_color(theme.color(active.kind), 0.4)
    };
    let style = if theme.monochrome {
        Style::default()
            .fg(Color::Reset)
            .bg(MANTLE)
            .add_modifier(Modifier::DIM)
    } else {
        Style::default().fg(color).bg(MANTLE)
    };

    for (col, row) in lowest_by_col {
        let vis_row = row - BUFFER_ROWS as i32;
        if !(0..VISIBLE_ROWS).contains(&vis_row) || !(0..COLS).contains(&col) {
            continue;
        }
        let x = area.x + (col as u16) * CELL_W;
        let y = area.y + vis_row as u16 * CELL_H;
        if x + CELL_W > area.x + area.width || y >= area.y + area.height {
            continue;
        }
        frame.render_widget(
            ratatui::widgets::Paragraph::new(Span::styled(GHOST_LINE, style)),
            Rect::new(x, y, CELL_W, CELL_H),
        );
    }
}

/// Per-frame animation context for the line-clear overlay.
struct AnimCtx<'a> {
    rows: &'a [usize],
    in_flash: bool,
    wipe_radius: u16,
}

fn make_anim_ctx<'a>(a: &'a LineClearAnim, state: &GameState) -> AnimCtx<'a> {
    let elapsed = state
        .now()
        .saturating_duration_since(a.started_at)
        .as_millis() as u64;
    let in_flash = elapsed < ANIM_FLASH_MS;
    let wipe_ms = elapsed.saturating_sub(ANIM_FLASH_MS);
    AnimCtx {
        rows: a.rows.as_slice(),
        in_flash,
        wipe_radius: if in_flash {
            0
        } else {
            wipe_radius_cells(wipe_ms)
        },
    }
}

fn render_anim_cell(
    ctx: &AnimCtx,
    col: usize,
    base_color: Color,
    monochrome: bool,
) -> (&'static str, Style) {
    if ctx.in_flash {
        return (
            FLASH,
            Style::default()
                .fg(Color::Rgb(255, 255, 255))
                .bg(Color::Rgb(255, 255, 255))
                .add_modifier(Modifier::BOLD),
        );
    }

    if cell_is_wiped(col, ctx.wipe_radius) {
        // Fully wiped — collapse to the empty background tile so the cell
        // appears to have "dissolved" from the center outward.
        return (EMPTY, Style::default().bg(MANTLE));
    }

    // Cell still visible but dimmed while the wipe travels outward.
    let style = if monochrome {
        Style::default()
            .fg(Color::Reset)
            .add_modifier(Modifier::DIM)
    } else {
        Style::default()
            .fg(base_color)
            .bg(BASE)
            .add_modifier(Modifier::DIM)
    };
    (FILLED, style)
}

fn default_cell_style(base_color: Color, monochrome: bool) -> (&'static str, Style) {
    let style = if monochrome {
        Style::default().fg(base_color)
    } else {
        Style::default().fg(base_color).bg(BASE)
    };
    (FILLED, style)
}

/// Draw the active piece onto the frame area.
///
/// `fade` is a [0.0, 1.0] intensity multiplier for the spawn-fade animation;
/// use 1.0 for full intensity outside the fade-in window.
fn render_piece(frame: &mut Frame, area: Rect, piece: &Piece, theme: &Theme, fade: f32) {
    let color = if theme.monochrome {
        Color::Reset
    } else {
        dim_color(theme.color(piece.kind), fade)
    };

    for (col, row) in piece.cells() {
        let vis_row = row - BUFFER_ROWS as i32;
        if !(0..VISIBLE_ROWS).contains(&vis_row) || !(0..COLS).contains(&col) {
            continue;
        }
        let x = area.x + (col as u16) * CELL_W;
        let y = area.y + vis_row as u16 * CELL_H;
        if x + CELL_W > area.x + area.width || y >= area.y + area.height {
            continue;
        }

        // Use the thematic filled glyph for color modes; monochrome reuses
        // the per-kind letter so pieces stay distinguishable without color.
        let s_owned: String = if theme.monochrome {
            let glyph = theme.glyph(piece.kind);
            glyph.to_string().repeat(CELL_W as usize)
        } else {
            FILLED.to_string()
        };
        let style = if theme.monochrome {
            Style::default().fg(Color::Reset)
        } else {
            Style::default().fg(color).bg(BASE)
        };
        frame.render_widget(
            ratatui::widgets::Paragraph::new(Span::styled(s_owned, style)),
            Rect::new(x, y, CELL_W, CELL_H),
        );
    }
}
