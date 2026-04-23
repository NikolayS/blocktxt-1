//! Color and glyph theme for the renderer.
//!
//! `Theme::detect()` reads the environment to select the best available
//! output mode:
//!
//!   1. If `NO_COLOR` is set (non-empty) OR `--no-color` was passed, use
//!      monochrome ASCII glyphs — one distinct letter per piece kind.
//!   2. Else if `COLORTERM=truecolor`, use full RGB colors + block glyph.
//!   3. Else use 256-color palette + distinctive glyphs as a safe fallback.
//!
//! Index into `colors` and `glyphs` arrays using `PieceKind as usize`:
//!   I=0, O=1, T=2, S=3, Z=4, J=5, L=6

use ratatui::style::Color;

use crate::game::piece::PieceKind;

// ── Catppuccin Mocha palette ──────────────────────────────────────────────────
// https://github.com/catppuccin/catppuccin

/// Background — 1e1e2e
pub const BASE: Color = Color::Rgb(30, 30, 46);
/// Secondary background — 181825
pub const MANTLE: Color = Color::Rgb(24, 24, 37);
/// Darkest background — 11111b
pub const CRUST: Color = Color::Rgb(17, 17, 27);
/// Primary text — cdd6f4
pub const TEXT: Color = Color::Rgb(205, 214, 244);
/// Secondary text — a6adc8
pub const SUBTEXT: Color = Color::Rgb(166, 173, 200);
/// Dim / border color — 6c7086
pub const OVERLAY: Color = Color::Rgb(108, 112, 134);

/// I piece — sky
pub const I_COLOR: Color = Color::Rgb(137, 220, 235);
/// O piece — yellow
pub const O_COLOR: Color = Color::Rgb(249, 226, 175);
/// T piece — mauve
pub const T_COLOR: Color = Color::Rgb(203, 166, 247);
/// S piece — green
pub const S_COLOR: Color = Color::Rgb(166, 227, 161);
/// Z piece — pink
pub const Z_COLOR: Color = Color::Rgb(243, 139, 168);
/// J piece — blue
pub const J_COLOR: Color = Color::Rgb(137, 180, 250);
/// L piece — peach
pub const L_COLOR: Color = Color::Rgb(250, 179, 135);

/// Ghost piece fill — surface1
pub const GHOST_MOD: Color = Color::Rgb(69, 71, 90);
/// "NEW BEST!" highlight — yellow accent
pub const NEW_BEST: Color = Color::Rgb(249, 226, 175);

// ── Theme struct ──────────────────────────────────────────────────────────────

/// Rendering theme: one color and one glyph per piece kind.
#[derive(Debug, Clone)]
pub struct Theme {
    /// ANSI colors indexed by `PieceKind as usize`.
    pub colors: [Color; 7],
    /// Glyphs indexed by `PieceKind as usize`.
    pub glyphs: [char; 7],
    /// True when no color should be applied (monochrome mode).
    pub monochrome: bool,
}

impl Theme {
    /// Detect the best available color mode and return an appropriate theme.
    ///
    /// `no_color_flag` should be `true` when the user passed `--no-color`.
    pub fn detect(no_color_flag: bool) -> Self {
        let no_color_env = std::env::var("NO_COLOR")
            .map(|v| !v.is_empty())
            .unwrap_or(false);

        if no_color_flag || no_color_env {
            return Self::monochrome();
        }

        // Check for truecolor support.
        let colorterm = std::env::var("COLORTERM").unwrap_or_default();
        if colorterm == "truecolor" || colorterm == "24bit" {
            return Self::truecolor();
        }

        // Default: 256-color palette with distinctive glyphs.
        Self::color256()
    }

    /// Monochrome theme: distinct ASCII letters, no color attributes.
    ///
    /// Letters match piece names so they're meaningful without color:
    ///   I, O, T, S, Z, J, L — same as the piece names in the spec.
    pub fn monochrome() -> Self {
        Self {
            colors: [Color::Reset; 7],
            // One unique letter per piece kind (I, O, T, S, Z, J, L).
            glyphs: ['I', 'O', 'T', 'S', 'Z', 'J', 'L'],
            monochrome: true,
        }
    }

    /// Full RGB truecolor theme — Catppuccin Mocha palette.
    pub fn truecolor() -> Self {
        Self {
            colors: [
                I_COLOR, // I — sky
                O_COLOR, // O — yellow
                T_COLOR, // T — mauve
                S_COLOR, // S — green
                Z_COLOR, // Z — pink
                J_COLOR, // J — blue
                L_COLOR, // L — peach
            ],
            glyphs: ['█', '█', '█', '█', '█', '█', '█'],
            monochrome: false,
        }
    }

    /// 256-color palette theme with distinctive single-char glyphs.
    pub fn color256() -> Self {
        Self {
            colors: [
                Color::Cyan,    // I
                Color::Yellow,  // O
                Color::Magenta, // T
                Color::Green,   // S
                Color::Red,     // Z
                Color::Blue,    // J
                Color::White,   // L (orange unavailable in 16-color)
            ],
            // Visually distinct glyphs for accessibility and monochrome
            // terminals that claim 256-color support.
            glyphs: ['▓', '▒', '░', '■', '▪', '▫', '▬'],
            monochrome: false,
        }
    }

    /// Return the color for a given piece kind.
    #[inline]
    pub fn color(&self, kind: PieceKind) -> Color {
        self.colors[kind as usize]
    }

    /// Return the glyph for a given piece kind.
    #[inline]
    pub fn glyph(&self, kind: PieceKind) -> char {
        self.glyphs[kind as usize]
    }
}
