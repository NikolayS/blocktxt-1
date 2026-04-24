//! Tests for `render::theme` — color/glyph detection.

use blocktxt::render::theme::{
    Palette, Theme, CM_I, CM_J, CM_L, CM_O, CM_S, CM_T, CM_Z, DR_I, DR_J, DR_L, DR_O, DR_S, DR_T,
    DR_Z, GV_I, GV_J, GV_L, GV_O, GV_S, GV_T, GV_Z, NO_I, NO_J, NO_L, NO_O, NO_S, NO_T, NO_Z, TN_I,
    TN_J, TN_L, TN_O, TN_S, TN_T, TN_Z,
};
use ratatui::style::Color;
use serial_test::serial;

#[test]
fn theme_monochrome_when_no_color_flag() {
    let theme = Theme::detect(true /* no_color_flag */, Palette::default());
    assert!(
        theme.monochrome,
        "no_color flag should yield monochrome theme"
    );
}

#[test]
#[serial]
fn theme_monochrome_when_no_color_env() {
    // Temporarily set NO_COLOR in the environment.
    // `#[serial]` prevents data races with other tests that touch env vars.
    std::env::set_var("NO_COLOR", "1");
    let theme = Theme::detect(false, Palette::default());
    std::env::remove_var("NO_COLOR");
    assert!(
        theme.monochrome,
        "NO_COLOR env should yield monochrome theme"
    );
}

#[test]
#[serial]
fn theme_uses_256_color_when_colorterm_truecolor() {
    std::env::remove_var("NO_COLOR");
    std::env::set_var("COLORTERM", "truecolor");
    let theme = Theme::detect(false, Palette::default());
    std::env::remove_var("COLORTERM");
    assert!(
        !theme.monochrome,
        "truecolor COLORTERM should yield color theme"
    );
}

#[test]
fn theme_glyphs_are_all_distinct_monochrome() {
    let theme = Theme::monochrome();
    let glyphs = &theme.glyphs;
    // All 7 glyphs must be different characters.
    let unique: std::collections::HashSet<char> = glyphs.iter().copied().collect();
    assert_eq!(
        unique.len(),
        7,
        "monochrome glyphs must all be distinct: {:?}",
        glyphs
    );
}

#[test]
#[serial]
fn theme_detect_no_color_env_empty_string_is_not_set() {
    // Per NO_COLOR spec: the variable must be non-empty to activate.
    // Empty string should NOT activate monochrome.
    std::env::set_var("NO_COLOR", "");
    std::env::set_var("COLORTERM", "truecolor");
    let theme = Theme::detect(false, Palette::default());
    std::env::remove_var("NO_COLOR");
    std::env::remove_var("COLORTERM");
    assert!(
        !theme.monochrome,
        "empty NO_COLOR should not activate monochrome"
    );
}

// ── new palette tests (#50) ───────────────────────────────────────────────────

#[test]
fn palette_default_is_tokyo_night() {
    assert_eq!(
        Palette::default(),
        Palette::TokyoNight,
        "default palette must be TokyoNight"
    );
}

#[test]
#[serial]
fn theme_tokyo_night_colors_match_spec() {
    std::env::remove_var("NO_COLOR");
    std::env::set_var("COLORTERM", "truecolor");
    let theme = Theme::detect(false, Palette::TokyoNight);
    std::env::remove_var("COLORTERM");

    use blocktxt::game::piece::PieceKind;
    // Originality-pass piece→slot mapping (SPEC §1a):
    //   I → Orange(TN_L), O → Pink(TN_Z), T → Green(TN_S), S → Blue(TN_J),
    //   Z → Yellow(TN_O), J → Purple(TN_T), L → Cyan(TN_I).
    assert_eq!(
        theme.color(PieceKind::I),
        TN_L,
        "I piece should adopt the orange slot (was cyan)"
    );
    assert_eq!(
        theme.color(PieceKind::O),
        TN_Z,
        "O piece should adopt the pink slot (was gold)"
    );
    assert_eq!(
        theme.color(PieceKind::Z),
        TN_O,
        "Z piece should adopt the yellow slot (was red)"
    );
    // Spot-check background colors.
    assert_eq!(theme.base, Color::Rgb(26, 27, 38), "base should be #1a1b26");
}

#[test]
#[serial]
fn theme_catppuccin_still_works() {
    std::env::remove_var("NO_COLOR");
    std::env::set_var("COLORTERM", "truecolor");
    let theme = Theme::detect(false, Palette::CatppuccinMocha);
    std::env::remove_var("COLORTERM");

    use blocktxt::game::piece::PieceKind;
    // I → Orange(CM_L peach), O → Pink(CM_Z), T → Green(CM_S).
    assert_eq!(
        theme.color(PieceKind::I),
        CM_L,
        "I piece should adopt the orange/peach slot"
    );
    assert_eq!(
        theme.color(PieceKind::O),
        CM_Z,
        "O piece should adopt the pink slot"
    );
    assert_eq!(
        theme.color(PieceKind::T),
        CM_S,
        "T piece should adopt the green slot"
    );
}

#[test]
fn cli_theme_flag_parses_valid() {
    for input in &["tokyo-night", "tn", "catppuccin-mocha"] {
        let result: Result<Palette, _> = input.parse();
        assert!(result.is_ok(), "expected Ok for theme '{input}'");
    }
}

#[test]
fn cli_theme_flag_rejects_invalid() {
    let result: Result<Palette, String> = "purple".parse();
    assert!(result.is_err(), "expected Err for unknown theme 'purple'");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("tokyo-night") && msg.contains("catppuccin-mocha"),
        "error message should list valid themes, got: {msg}"
    );
}

#[test]
#[serial]
fn no_color_overrides_palette() {
    // Even with TokyoNight or CatppuccinMocha, NO_COLOR must give monochrome.
    std::env::set_var("NO_COLOR", "1");
    let tn = Theme::detect(false, Palette::TokyoNight);
    let cm = Theme::detect(false, Palette::CatppuccinMocha);
    std::env::remove_var("NO_COLOR");

    assert!(tn.monochrome, "TokyoNight: NO_COLOR must yield monochrome");
    assert!(
        cm.monochrome,
        "CatppuccinMocha: NO_COLOR must yield monochrome"
    );
}

// ── new palette tests (#012) — Gruvbox Dark, Nord, Dracula ───────────────────

#[test]
fn palette_parses_gruvbox_dark() {
    for alias in &["gruvbox-dark", "gruvbox_dark", "gruvbox", "gv"] {
        let result: Result<Palette, _> = alias.parse();
        assert_eq!(
            result.unwrap(),
            Palette::GruvboxDark,
            "expected GruvboxDark for alias '{alias}'"
        );
    }
}

#[test]
fn palette_parses_nord() {
    for alias in &["nord", "nord-dark"] {
        let result: Result<Palette, _> = alias.parse();
        assert_eq!(
            result.unwrap(),
            Palette::Nord,
            "expected Nord for alias '{alias}'"
        );
    }
}

#[test]
fn palette_parses_dracula() {
    for alias in &["dracula", "dr"] {
        let result: Result<Palette, _> = alias.parse();
        assert_eq!(
            result.unwrap(),
            Palette::Dracula,
            "expected Dracula for alias '{alias}'"
        );
    }
}

#[test]
fn palette_spec_gruvbox_colors() {
    // GV_I = #83a598 aqua, GV_Z = #fb4934 red
    assert_eq!(GV_I, Color::Rgb(131, 165, 152), "GV_I should be #83a598");
    assert_eq!(GV_Z, Color::Rgb(251, 73, 52), "GV_Z should be #fb4934");
}

#[test]
fn palette_spec_nord_colors() {
    // NO_I = #88c0d0 nord8, NO_Z = #bf616a nord11
    assert_eq!(NO_I, Color::Rgb(136, 192, 208), "NO_I should be #88c0d0");
    assert_eq!(NO_Z, Color::Rgb(191, 97, 106), "NO_Z should be #bf616a");
}

#[test]
fn palette_spec_dracula_colors() {
    // DR_I = #8be9fd cyan, DR_Z = #ff5555 red
    assert_eq!(DR_I, Color::Rgb(139, 233, 253), "DR_I should be #8be9fd");
    assert_eq!(DR_Z, Color::Rgb(255, 85, 85), "DR_Z should be #ff5555");
}

#[test]
fn palette_invalid_lists_all_five() {
    let result: Result<Palette, String> = "solarized".parse();
    assert!(result.is_err());
    let msg = result.unwrap_err();
    for name in &[
        "tokyo-night",
        "catppuccin-mocha",
        "gruvbox-dark",
        "nord",
        "dracula",
    ] {
        assert!(
            msg.contains(name),
            "error message should list '{name}', got: {msg}"
        );
    }
}

// ── Originality-pass piece→color mapping consistency ─────────────────────────

/// Slot tuple: `(cyan, orange, yellow, pink, green, blue, purple)`.
type PaletteSlots = (Color, Color, Color, Color, Color, Color, Color);

/// The piece→slot permutation (SPEC §1a) must be applied identically in
/// every palette. For each kind, verify that the color the theme returns
/// is the palette's slot color for that kind.
#[test]
fn piece_color_mapping_consistent_across_palettes() {
    use blocktxt::game::piece::PieceKind;

    // Expected mapping: (piece, expected_slot)
    //   I → Orange, O → Pink, T → Green, S → Blue, Z → Yellow, J → Purple, L → Cyan
    let palettes: &[(Palette, PaletteSlots)] = &[
        (
            Palette::TokyoNight,
            (TN_I, TN_L, TN_O, TN_Z, TN_S, TN_J, TN_T),
        ),
        (
            Palette::CatppuccinMocha,
            (CM_I, CM_L, CM_O, CM_Z, CM_S, CM_J, CM_T),
        ),
        (
            Palette::GruvboxDark,
            (GV_I, GV_L, GV_O, GV_Z, GV_S, GV_J, GV_T),
        ),
        (Palette::Nord, (NO_I, NO_L, NO_O, NO_Z, NO_S, NO_J, NO_T)),
        (Palette::Dracula, (DR_I, DR_L, DR_O, DR_Z, DR_S, DR_J, DR_T)),
    ];

    for (palette, (cy, or, ye, pi, gr, bl, pu)) in palettes {
        let theme = Theme::truecolor(*palette);
        assert_eq!(theme.color(PieceKind::I), *or, "{palette:?}: I → orange");
        assert_eq!(theme.color(PieceKind::O), *pi, "{palette:?}: O → pink");
        assert_eq!(theme.color(PieceKind::T), *gr, "{palette:?}: T → green");
        assert_eq!(theme.color(PieceKind::S), *bl, "{palette:?}: S → blue");
        assert_eq!(theme.color(PieceKind::Z), *ye, "{palette:?}: Z → yellow");
        assert_eq!(theme.color(PieceKind::J), *pu, "{palette:?}: J → purple");
        assert_eq!(theme.color(PieceKind::L), *cy, "{palette:?}: L → cyan");
    }
}

// Keep these imports used (suppress unused-import lint).
const _: Color = TN_J;
const _: Color = TN_L;
const _: Color = TN_S;
const _: Color = TN_T;
const _: Color = CM_J;
const _: Color = CM_L;
const _: Color = CM_S;
// Extra keep-alive references used by palette consistency test imports.
const _: Color = GV_Z;
const _: Color = NO_Z;
const _: Color = DR_Z;
