//! Title screen renderer.
//!
//! Draws the BLOCKTXT logo, tagline, top-5 leaderboard, controls hint,
//! and a "press any key to start" footer.

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::persistence::HighScoreStore;
use crate::render::theme::Theme;

// ── ASCII-art logo ────────────────────────────────────────────────────────────

/// BLOCKTXT logo in box-drawing / block characters (6 lines × ~67 chars).
///
/// Rendered with full-block Unicode glyphs, so it is visually bold even in
/// monochrome. Width fits in a 80-col terminal with side margins.
const LOGO: &[&str] = &[
    "██████╗ ██╗      ██████╗  ██████╗██╗  ██╗████████╗██╗  ██╗████████╗",
    "██╔══██╗██║     ██╔═══██╗██╔════╝██║ ██╔╝╚══██╔══╝╚██╗██╔╝╚══██╔══╝",
    "██████╔╝██║     ██║   ██║██║     █████╔╝    ██║    ╚███╔╝    ██║   ",
    "██╔══██╗██║     ██║   ██║██║     ██╔═██╗    ██║    ██╔██╗    ██║   ",
    "██████╔╝███████╗╚██████╔╝╚██████╗██║  ██╗   ██║   ██╔╝ ██╗   ██║   ",
    "╚═════╝ ╚══════╝ ╚═════╝  ╚═════╝╚═╝  ╚═╝   ╚═╝   ╚═╝  ╚═╝   ╚═╝   ",
];

const TAGLINE: &str = "a terminal falling-block puzzle game";

const CONTROLS: &[&str] = &[
    "move:  ← →   drop: space    pause:  p",
    "rotate: z x  soft: ↓        quit:   q",
];

const FOOTER: &str = "press any key to start     ·     r  reset scores";

// ── Public entry point ────────────────────────────────────────────────────────

/// Draw the title screen into `area`.
///
/// `store` is `None` only when persistence is completely unavailable;
/// pass `Some(&empty_store)` to show the "no scores yet" message.
pub fn draw(frame: &mut Frame, area: Rect, store: Option<&HighScoreStore>, theme: &Theme) {
    let text_style = Style::default().fg(theme.text).add_modifier(Modifier::BOLD);
    let dim_style = Style::default().fg(theme.overlay);
    let sub_style = Style::default().fg(theme.subtext);

    // Compute how many rows each section needs.
    let logo_h = LOGO.len() as u16;
    let tagline_h: u16 = 1;
    let gap1: u16 = 1;
    let leaderboard_h: u16 = 9; // border top + 5 rows + border bottom + 1 gap
    let controls_h = CONTROLS.len() as u16;
    let gap2: u16 = 1;
    let footer_h: u16 = 1;

    let total_h = logo_h + tagline_h + gap1 + leaderboard_h + controls_h + gap2 + footer_h;

    // Vertically center when area is tall enough.
    let v_pad = area.height.saturating_sub(total_h) / 2;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(v_pad),         // top padding
            Constraint::Length(logo_h),        // logo
            Constraint::Length(tagline_h),     // tagline
            Constraint::Length(gap1),          // spacer
            Constraint::Length(leaderboard_h), // leaderboard
            Constraint::Length(controls_h),    // controls
            Constraint::Length(gap2),          // spacer
            Constraint::Length(footer_h),      // footer
            Constraint::Min(0),                // bottom remainder
        ])
        .split(area);

    // ── Logo ─────────────────────────────────────────────────────────────────
    let logo_lines: Vec<Line> = LOGO
        .iter()
        .map(|row| Line::from(Span::styled(*row, text_style)))
        .collect();
    frame.render_widget(
        Paragraph::new(Text::from(logo_lines)).alignment(Alignment::Center),
        chunks[1],
    );

    // ── Tagline ───────────────────────────────────────────────────────────────
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(TAGLINE, sub_style))).alignment(Alignment::Center),
        chunks[2],
    );

    // ── Leaderboard ───────────────────────────────────────────────────────────
    draw_leaderboard(frame, chunks[4], store, theme);

    // ── Controls ──────────────────────────────────────────────────────────────
    let ctrl_lines: Vec<Line> = CONTROLS
        .iter()
        .map(|row| Line::from(Span::styled(*row, dim_style)))
        .collect();
    frame.render_widget(
        Paragraph::new(Text::from(ctrl_lines)).alignment(Alignment::Center),
        chunks[5],
    );

    // ── Footer ────────────────────────────────────────────────────────────────
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(FOOTER, dim_style))).alignment(Alignment::Center),
        chunks[7],
    );
}

// ── Leaderboard helper ────────────────────────────────────────────────────────

fn draw_leaderboard(frame: &mut Frame, area: Rect, store: Option<&HighScoreStore>, theme: &Theme) {
    let score_style = Style::default().fg(theme.text).add_modifier(Modifier::BOLD);
    let dim_style = Style::default().fg(theme.overlay);

    let scores = store.map(|s| s.top(5));

    let content: Vec<Line> = match scores {
        None | Some([]) => {
            // Keep short so it fits the 20-char inner width of the box.
            vec![
                Line::from(""),
                Line::from(Span::styled("no scores yet", dim_style)),
                Line::from(""),
            ]
        }
        Some(entries) => {
            let mut lines = Vec::new();
            for (i, hs) in entries.iter().enumerate() {
                let rank = i + 1;
                // Format score with thin-space separator every 3 digits.
                let score_str = format_score(hs.score);
                let row = format!("{rank}.  {score_str:>10}");
                lines.push(Line::from(Span::styled(row, score_style)));
            }
            // Pad with dashes for missing entries (up to 5 total).
            for i in entries.len()..5 {
                let rank = i + 1;
                lines.push(Line::from(Span::styled(
                    format!("{rank}.         —"),
                    dim_style,
                )));
            }
            lines
        }
    };

    let block = Block::default()
        .title("─ high scores ─")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(dim_style);

    // Center the block horizontally.
    let block_w: u16 = 22;
    let h_pad = area.width.saturating_sub(block_w) / 2;
    let inner_area = Rect::new(area.x + h_pad, area.y, block_w.min(area.width), area.height);

    frame.render_widget(
        Paragraph::new(Text::from(content))
            .block(block)
            .alignment(Alignment::Center),
        inner_area,
    );
}

/// Format a score with spaces every 3 digits (e.g. 12345 → "12 345").
fn format_score(score: u32) -> String {
    let s = score.to_string();
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    for (i, ch) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            out.push(' ');
        }
        out.push(*ch);
    }
    out
}
