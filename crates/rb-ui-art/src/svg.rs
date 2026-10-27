//! A small SVG builder and the palette of the panel (the flat look of the
//! ui-poc experiment). All coordinates are panel pixels of docs/ui_spec.md.

use std::f32::consts::PI;
use std::fmt::Write;

pub mod pal {
    pub const BG: &str = "#1d2127";
    pub const SLATE: &str = "#3b4656";
    pub const SLATE_DARK: &str = "#2c3542";
    pub const SLATE_LINE: &str = "#56647a";
    pub const INK: &str = "#e9edf2";
    pub const INK_DIM: &str = "#9aa7b8";
    pub const RED: &str = "#e2413b";
    pub const ORANGE: &str = "#f08a24";
    pub const YELLOW: &str = "#f2c531";
    pub const OLIVE: &str = "#8da33a";
    pub const GOLD: &str = "#cdb54a";
    pub const BLUE: &str = "#2f66b3";
    pub const GREEN: &str = "#53d86a";
    pub const GREEN_DIM: &str = "#244a2c";
    pub const AMBER: &str = "#ffb000";
    pub const PAPER: &str = "#ecebe5";
    pub const PAPER_DARK: &str = "#d5d3cb";
    pub const CREAM: &str = "#dcd9d0";
    pub const CHARCOAL: &str = "#2a2b2d";
    pub const BLACK: &str = "#1b1c1e";
    pub const WHITE: &str = "#fbfbf8";
    pub const STEEL: &str = "#c9ced6";
    pub const LCD_BG: &str = "#1a0e0e";
    pub const LCD_ON: &str = "#ff3b2f";
    pub const LCD_OFF: &str = "#231110";
    pub const WOOD: &str = "#7b4f30";
    pub const WOOD_DARK: &str = "#6a4228";
    pub const WOOD_LIGHT: &str = "#8a5c39";
    pub const SHADE: &str = "#00000033";
}

/// A number with at most two decimals and no trailing zeros.
pub fn n(v: f32) -> String {
    let mut s = format!("{v:.2}");
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    if s == "-0" {
        s = "0".into();
    }
    s
}

/// The body of an SVG document (no `<svg>` wrapper).
#[derive(Default, Clone)]
pub struct Svg {
    pub s: String,
}

impl Svg {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn raw(&mut self, raw: &str) -> &mut Self {
        self.s.push_str(raw);
        self
    }

    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, fill: &str) -> &mut Self {
        let (x, y, w, h, r) = (n(x), n(y), n(w), n(h), n(r));
        let _ = write!(
            self.s,
            r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}" fill="{fill}"/>"#
        );
        self
    }

    #[allow(clippy::too_many_arguments)]
    pub fn rect_stroke(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
        stroke: &str,
        sw: f32,
    ) -> &mut Self {
        let (x, y, w, h, r, sw) = (n(x), n(y), n(w), n(h), n(r), n(sw));
        let _ = write!(
            self.s,
            r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}" fill="none" stroke="{stroke}" stroke-width="{sw}"/>"#
        );
        self
    }

    pub fn circle(&mut self, cx: f32, cy: f32, r: f32, fill: &str) -> &mut Self {
        let (cx, cy, r) = (n(cx), n(cy), n(r));
        let _ = write!(
            self.s,
            r#"<circle cx="{cx}" cy="{cy}" r="{r}" fill="{fill}"/>"#
        );
        self
    }

    pub fn circle_stroke(&mut self, cx: f32, cy: f32, r: f32, stroke: &str, sw: f32) -> &mut Self {
        let (cx, cy, r, sw) = (n(cx), n(cy), n(r), n(sw));
        let _ = write!(
            self.s,
            r#"<circle cx="{cx}" cy="{cy}" r="{r}" fill="none" stroke="{stroke}" stroke-width="{sw}"/>"#
        );
        self
    }

    pub fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, stroke: &str, sw: f32) -> &mut Self {
        let (x1, y1, x2, y2, sw) = (n(x1), n(y1), n(x2), n(y2), n(sw));
        let _ = write!(
            self.s,
            r#"<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{stroke}" stroke-width="{sw}" stroke-linecap="round"/>"#
        );
        self
    }

    pub fn path(&mut self, d: &str, fill: &str) -> &mut Self {
        let _ = write!(self.s, r#"<path d="{d}" fill="{fill}"/>"#);
        self
    }

    pub fn path_stroke(&mut self, d: &str, stroke: &str, sw: f32) -> &mut Self {
        let sw = n(sw);
        let _ = write!(
            self.s,
            r#"<path d="{d}" fill="none" stroke="{stroke}" stroke-width="{sw}" stroke-linecap="round" stroke-linejoin="round"/>"#
        );
        self
    }

    /// Opens a group translated by (x, y); close it with `end_g`.
    pub fn g_at(&mut self, x: f32, y: f32) -> &mut Self {
        let _ = write!(self.s, r#"<g transform="translate({} {})">"#, n(x), n(y));
        self
    }

    /// Opens a group rotated by `deg` degrees around (cx, cy).
    pub fn g_rotate(&mut self, deg: f32, cx: f32, cy: f32) -> &mut Self {
        let _ = write!(
            self.s,
            r#"<g transform="rotate({} {} {})">"#,
            n(deg),
            n(cx),
            n(cy)
        );
        self
    }

    pub fn end_g(&mut self) -> &mut Self {
        self.s.push_str("</g>");
        self
    }
}

/// A point at radius `r` and angle `deg` (0 = up, clockwise) around (cx, cy).
pub fn polar(cx: f32, cy: f32, r: f32, deg: f32) -> (f32, f32) {
    let a = deg * PI / 180.0;
    (cx + r * a.sin(), cy - r * a.cos())
}

/// A clockwise arc from angle `a0` to `a1` (degrees, 0 = up).
pub fn arc_path(cx: f32, cy: f32, r: f32, a0: f32, a1: f32) -> String {
    let (a0, a1) = if a1 < a0 { (a1, a0) } else { (a0, a1) };
    let (x0, y0) = polar(cx, cy, r, a0);
    let (x1, y1) = polar(cx, cy, r, a1);
    let large = if a1 - a0 > 180.0 { 1 } else { 0 };
    format!(
        "M{} {} A{} {} 0 {large} 1 {} {}",
        n(x0),
        n(y0),
        n(r),
        n(r),
        n(x1),
        n(y1)
    )
}

/// One named, self-contained piece of art.
#[derive(Clone)]
pub struct Part {
    pub name: String,
    pub w: f32,
    pub h: f32,
    pub body: String,
}

impl Part {
    pub fn new(name: impl Into<String>, w: f32, h: f32, f: impl FnOnce(&mut Svg)) -> Self {
        let mut s = Svg::new();
        f(&mut s);
        Self {
            name: name.into(),
            w,
            h,
            body: s.s,
        }
    }

    /// The part as a standalone SVG file. Width and height are the part's
    /// native size in panel pixels; the viewBox equals them, so any size hint
    /// scales it uniformly.
    pub fn to_svg(&self) -> String {
        let (w, h) = (n(self.w), n(self.h));
        format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">{}</svg>\n",
            self.body
        )
    }
}
