//! Renderer entry point.
//!
//! `render::render(frame, &state, &theme)` is the single public drawing
//! entry point. It splits the terminal area into:
//!
//!   - Playfield: 26 chars wide (12 board cols × 2 char each + 2 border).
//!   - HUD: 24 chars wide (stats + next-piece preview).
//!
//! The composition (50 chars total) is centered horizontally when the
//! terminal is wider than needed.
//!
//! The renderer only reads `&GameState` and never mutates game state.

pub mod board_view;
pub mod helpers;
pub mod hud;
pub mod theme;
pub mod title;

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::game::state::{GameState, Phase};
use crate::persistence::HighScoreStore;
pub use theme::Theme;

use theme::{BASE, OVERLAY, SUBTEXT, TEXT};

/// Playfield width: 12 cells × 2 chars + 2 border = 26 chars.
const PLAYFIELD_W: u16 = 26;
/// HUD width: 24 chars (enough for stats + piece shapes).
const HUD_W: u16 = 24;
/// Total composition width.
const COMPOSITION_W: u16 = PLAYFIELD_W + HUD_W;

/// Minimum terminal width required to display the game.
///
/// Composition (50) + 2 chars margin = 52.
pub const MIN_WIDTH: u16 = 52;

/// Minimum terminal height required to display the game.
///
/// 24 visible rows + 2 border + 2 margin = 28.
pub const MIN_HEIGHT: u16 = 28;

/// Draw one full frame: board + HUD.
///
/// If the terminal is smaller than `MIN_WIDTH × MIN_HEIGHT`, draws the
/// too-small overlay instead of the game.
pub fn render(frame: &mut Frame, state: &GameState, theme: &Theme) {
    render_with_scores(frame, state, theme, None);
}

/// Like `render`, but routes an optional `HighScoreStore` to the HUD so
/// the new-best banner on the GameOver overlay can light up end-to-end.
///
/// Preferred entry point from `main.rs` once persistence is wired.
pub fn render_with_scores(
    frame: &mut Frame,
    state: &GameState,
    theme: &Theme,
    high_scores: Option<&HighScoreStore>,
) {
    let area = frame.area();

    // Fill the entire terminal with the base background.
    frame.render_widget(Block::default().style(Style::default().bg(BASE)), area);

    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        draw_too_small_overlay(frame, area);
        return;
    }

    // Title and ConfirmResetScores phases use the title renderer.
    if matches!(state.phase, Phase::Title | Phase::ConfirmResetScores) {
        title::draw(frame, area, high_scores, theme);
        return;
    }

    // Center the composition horizontally when terminal is wider.
    let h_margin = area.width.saturating_sub(COMPOSITION_W) / 2;
    let v_margin = area.height.saturating_sub(MIN_HEIGHT) / 2;
    let comp_area = Rect::new(
        area.x + h_margin,
        area.y + v_margin,
        COMPOSITION_W.min(area.width),
        area.height.saturating_sub(v_margin),
    );

    // Split horizontally: playfield left, HUD right.
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(PLAYFIELD_W), Constraint::Length(HUD_W)])
        .split(comp_area);

    board_view::draw(frame, chunks[0], state, theme);
    hud::draw_with_scores(frame, chunks[1], state, theme, high_scores);
}

/// Draw a centered "terminal too small" overlay.
///
/// Replaces the entire frame with a plain message so the user knows
/// to resize.  Does not crash or leave corrupted output.
pub fn draw_too_small_overlay(frame: &mut Frame, area: Rect) {
    let overlay_w = 34u16.min(area.width.max(1));
    let overlay_h = 5u16.min(area.height.max(1));
    let x = area.x + area.width.saturating_sub(overlay_w) / 2;
    let y = area.y + area.height.saturating_sub(overlay_h) / 2;
    let overlay_area = Rect::new(x, y, overlay_w, overlay_h);

    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(Text::from(vec![
            Line::from(""),
            Line::from(Span::styled(
                "Terminal too small",
                Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!("Resize to at least {}×{}", MIN_WIDTH, MIN_HEIGHT),
                Style::default().fg(SUBTEXT),
            )),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(OVERLAY))
                .style(Style::default().bg(BASE)),
        )
        .alignment(Alignment::Center),
        overlay_area,
    );
}
