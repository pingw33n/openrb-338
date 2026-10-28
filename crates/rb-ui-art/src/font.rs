//! A stroke font for the panel's labels. Text is drawn as vector paths, so
//! the art needs no font file and renders the same everywhere (usvg has no
//! guaranteed system fonts). Capital letters, digits and a few signs; the
//! glyph grid is 6 units high (cap height) and the stroke is round.

use crate::svg::{Svg, n};

/// The clear space between two glyphs' strokes in glyph units, and at least `MIN_CLEAR`
/// panel pixels, so small labels (whose stroke is thickened) do not run together.
const CLEAR: f32 = 0.56;
const MIN_CLEAR: f32 = 0.6;

/// The stroke width in glyph units at cap height `cap`: at least half a panel pixel so
/// small labels stay legible.
fn stroke_units(cap: f32) -> f32 {
    (cap * 0.14).max(0.55) / (cap / 6.0)
}

/// The gap between two glyphs' boxes in glyph units: the stroke plus the clear space.
fn gap_units(cap: f32) -> f32 {
    stroke_units(cap) + CLEAR.max(MIN_CLEAR / (cap / 6.0))
}

/// (width, path) of a glyph on a 6-unit-high grid, or None.
fn glyph(c: char) -> Option<(f32, &'static str)> {
    Some(match c.to_ascii_uppercase() {
        'A' => (4.0, "M0 6 L2 0 L4 6 M0.8 4 L3.2 4"),
        'B' => (
            4.0,
            "M0 0 V6 M0 0 H2.4 A1.1 1.5 0 0 1 2.4 3 H0 M2.4 3 H2.9 A1.1 1.5 0 0 1 2.9 6 H0",
        ),
        'C' => (4.0, "M3.7 1.4 A2 3 0 1 0 3.7 4.6"),
        'D' => (4.0, "M0 0 V6 H1.6 A2.4 3 0 0 0 1.6 0 Z"),
        'E' => (3.6, "M3.6 0 H0 V6 H3.6 M0 3 H2.8"),
        'F' => (3.6, "M3.6 0 H0 V6 M0 3 H2.8"),
        'G' => (4.0, "M3.7 1.4 A2 3 0 1 0 3.8 4.9 V3.2 H2.2"),
        'H' => (4.0, "M0 0 V6 M4 0 V6 M0 3 H4"),
        'I' => (2.0, "M0.2 0 H1.8 M1 0 V6 M0.2 6 H1.8"),
        'J' => (3.4, "M3.4 0 V4.2 A1.7 1.8 0 0 1 0 4.2"),
        'K' => (4.0, "M0 0 V6 M4 0 L0 3.6 M1.4 2.6 L4 6"),
        'L' => (3.4, "M0 0 V6 H3.4"),
        'M' => (5.0, "M0 6 V0 L2.5 4 L5 0 V6"),
        'N' => (4.0, "M0 6 V0 L4 6 V0"),
        'O' => (4.0, "M2 0 A2 3 0 1 0 2 6 A2 3 0 1 0 2 0 Z"),
        'P' => (4.0, "M0 6 V0 H2.5 A1.5 1.7 0 0 1 2.5 3.4 H0"),
        'Q' => (4.0, "M2 0 A2 3 0 1 0 2 6 A2 3 0 1 0 2 0 Z M2.4 4.4 L4 6.4"),
        'R' => (4.0, "M0 6 V0 H2.5 A1.5 1.7 0 0 1 2.5 3.4 H0 M2 3.4 L4 6"),
        'S' => (
            4.0,
            "M3.7 1.2 C3.4 0.4 2.8 0 2 0 C1 0 0.3 0.6 0.3 1.5 C0.3 2.6 1.4 2.7 2 3 C2.8 3.3 3.7 3.6 3.7 4.6 C3.7 5.4 3 6 2 6 C1.1 6 0.5 5.6 0.3 4.8",
        ),
        'T' => (4.0, "M0 0 H4 M2 0 V6"),
        'U' => (4.0, "M0 0 V3.8 A2 2.2 0 0 0 4 3.8 V0"),
        'V' => (4.0, "M0 0 L2 6 L4 0"),
        'W' => (5.6, "M0 0 L1.4 6 L2.8 1.6 L4.2 6 L5.6 0"),
        'X' => (4.0, "M0 0 L4 6 M4 0 L0 6"),
        'Y' => (4.0, "M0 0 L2 3 L4 0 M2 3 V6"),
        'Z' => (4.0, "M0 0 H4 L0 6 H4"),
        '0' => (3.8, "M1.9 0 A1.9 3 0 1 0 1.9 6 A1.9 3 0 1 0 1.9 0 Z"),
        '1' => (2.0, "M0.1 1.4 L1.8 0 V6"),
        '2' => (
            3.8,
            "M0.2 1.6 A1.7 1.6 0 1 1 3.4 2.4 C3.2 3.8 1.5 4.6 0 6 H3.8",
        ),
        '3' => (
            3.8,
            "M0.3 1 C0.8 0.3 1.3 0 2 0 C3 0 3.6 0.6 3.6 1.5 C3.6 2.4 2.9 3 1.8 3 C3 3 3.8 3.6 3.8 4.5 C3.8 5.4 3 6 1.9 6 C1.1 6 0.5 5.7 0.1 5",
        ),
        '4' => (4.0, "M3 6 V0 L0 4.2 H4"),
        '5' => (
            3.8,
            "M3.6 0 H0.6 L0.3 2.8 C0.8 2.5 1.3 2.4 1.9 2.4 C3 2.4 3.8 3.1 3.8 4.2 C3.8 5.2 3 6 1.9 6 C1.1 6 0.5 5.7 0.1 5",
        ),
        '6' => (
            3.8,
            "M3.4 0.8 C3 0.3 2.5 0 1.9 0 C0.8 0 0.1 1.2 0.1 3.2 C0.1 5 0.8 6 1.9 6 C3 6 3.8 5.2 3.8 4.2 C3.8 3.2 3 2.6 1.9 2.6 C1 2.6 0.3 3.1 0.1 3.8",
        ),
        '7' => (3.8, "M0 0 H3.8 L1.4 6"),
        '8' => (
            3.8,
            "M1.9 3 C0.9 3 0.3 2.4 0.3 1.5 C0.3 0.6 0.9 0 1.9 0 C2.9 0 3.5 0.6 3.5 1.5 C3.5 2.4 2.9 3 1.9 3 C0.8 3 0.1 3.7 0.1 4.5 C0.1 5.4 0.8 6 1.9 6 C3 6 3.7 5.4 3.7 4.5 C3.7 3.7 3 3 1.9 3 Z",
        ),
        '9' => (
            3.8,
            "M0.4 5.2 C0.8 5.7 1.3 6 1.9 6 C3 6 3.7 4.8 3.7 2.8 C3.7 1 3 0 1.9 0 C0.8 0 0.1 0.8 0.1 1.8 C0.1 2.8 0.8 3.4 1.9 3.4 C2.8 3.4 3.5 2.9 3.7 2.2",
        ),
        '-' => (2.6, "M0 3 H2.6"),
        '+' => (3.4, "M0 3 H3.4 M1.7 1.3 V4.7"),
        '.' => (0.2, "M0.1 6 H0.12"),
        '/' => (2.6, "M0 6 L2.6 0"),
        '_' => (3.2, "M0 7.2 H3.2"),
        '#' => (
            4.4,
            "M1.3 0.8 L0.9 5.2 M3.3 0.8 L2.9 5.2 M0 2.2 H4.4 M0 3.8 H4.4",
        ),
        ' ' => (2.0, ""),
        _ => return None,
    })
}

#[derive(Clone, Copy, PartialEq)]
pub enum Anchor {
    Start,
    Middle,
    End,
}

/// The width of `text` in glyph units at cap height `cap`.
fn units(cap: f32, text: &str) -> f32 {
    let mut w = 0.0;
    let mut first = true;
    for c in text.chars() {
        let (gw, _) = glyph(c).unwrap_or_else(|| panic!("no glyph for {c:?}"));
        if !first {
            w += gap_units(cap);
        }
        first = false;
        w += gw;
    }
    w
}

/// The gap between two glyphs in pixels at cap height `cap`.
pub fn gap(cap: f32) -> f32 {
    gap_units(cap) * cap / 6.0
}

/// The width of `text` in pixels at cap height `cap`.
pub fn text_width(cap: f32, text: &str) -> f32 {
    units(cap, text) * cap / 6.0
}

/// Draws `text` with cap height `cap`, its vertical center at `yc`.
pub fn text(s: &mut Svg, x: f32, yc: f32, cap: f32, anchor: Anchor, color: &str, text: &str) {
    text_range(s, x, yc, cap, anchor, color, text, 0..text.chars().count());
}

/// Draws only the glyphs `range` (character indices) of `text`, laid out exactly as `text`
/// lays them out: the pieces of one run drawn apart add up to the run, pixel for pixel.
#[allow(clippy::too_many_arguments)]
pub fn text_range(
    s: &mut Svg,
    x: f32,
    yc: f32,
    cap: f32,
    anchor: Anchor,
    color: &str,
    text: &str,
    range: std::ops::Range<usize>,
) {
    let scale = cap / 6.0;
    let w = units(cap, text) * scale;
    let x0 = match anchor {
        Anchor::Start => x,
        Anchor::Middle => x - w / 2.0,
        Anchor::End => x - w,
    };
    let stroke = stroke_units(cap);
    s.raw(&format!(
        r#"<g transform="translate({} {}) scale({})" fill="none" stroke="{color}" stroke-width="{}" stroke-linecap="round" stroke-linejoin="round">"#,
        n(x0),
        n(yc - cap / 2.0),
        n(scale),
        n(stroke)
    ));
    let mut dx = 0.0;
    for (i, c) in text.chars().enumerate() {
        let (gw, d) = glyph(c).unwrap();
        if !d.is_empty() && range.contains(&i) {
            s.raw(&format!(
                r#"<path transform="translate({})" d="{d}"/>"#,
                n(dx)
            ));
        }
        dx += gw + gap_units(cap);
    }
    s.raw("</g>");
}

/// Centered text.
pub fn label(s: &mut Svg, x: f32, yc: f32, cap: f32, color: &str, t: &str) {
    text(s, x, yc, cap, Anchor::Middle, color, t);
}
