//! Every part of the panel's art, drawn in code: the redesign of task-179, ported from the
//! mockup the user approved on 2026-11-13 (its numbers, colors, gradients and filters; the
//! mockup's `mock.py` drew the panel as one SVG, here it is cut into the app's parts).
//!
//! A part's `w` x `h` is its control's rectangle in panel pixels (the hit area), with the art
//! drawn relative to the rectangle's top-left; `bleed` is how far the art reaches beyond it
//! (shadows, halos, glows). A background is drawn relative to its section's origin.
//!
//! Naming: `<control>_<state>`. Push buttons have `rest` and `pressed`, toggles `off` and `on`,
//! buttons that light up also `lit`.

#![allow(clippy::too_many_arguments)]

use crate::svg::{Part, Svg, n, pal};
use crate::text::{self, Anchor, Label};

/// The sweep of a knob: the pointer turns from -KNOB_SWEEP to +KNOB_SWEEP degrees (0 =
/// straight up, clockwise positive) as the value goes 0 to 1.
pub const KNOB_SWEEP: f32 = 135.0;

/// A knob style: its rectangle and the center the pointer turns about. The layers are
/// `<name>_body` (static) and `<name>_pointer` (drawn straight up); no value arc.
pub struct KnobDesign {
    pub name: &'static str,
    pub w: f32,
    pub h: f32,
    pub cx: f32,
    pub cy: f32,
    pub bleed: f32,
}

pub const KNOBS: [KnobDesign; 5] = [
    KnobDesign {
        name: "knob_303",
        w: 30.0,
        h: 30.0,
        cx: 15.0,
        cy: 15.0,
        bleed: 1.0,
    },
    KnobDesign {
        name: "knob_808_red",
        w: 13.0,
        h: 13.0,
        cx: 6.5,
        cy: 6.5,
        bleed: 2.5,
    },
    KnobDesign {
        name: "knob_808_white",
        w: 13.0,
        h: 13.0,
        cx: 6.5,
        cy: 6.5,
        bleed: 2.5,
    },
    KnobDesign {
        name: "knob_909",
        w: 12.0,
        h: 12.0,
        cx: 6.0,
        cy: 6.0,
        bleed: 2.0,
    },
    // The gray sections' knobs: shuffle, the mixer's pan and delay send, the effects'.
    KnobDesign {
        name: "knob_gray",
        w: 18.0,
        h: 19.0,
        cx: 9.0,
        cy: 9.5,
        bleed: 3.0,
    },
];

// ---------------------------------------------------------------- primitives

/// The gray of the plates.
pub const GRAY: &str = "#5f7079";
/// The panel's dark seams between the plates.
pub const SEAM: &str = "#14181b";

fn rect(s: &mut Svg, x: f32, y: f32, w: f32, h: f32, rx: f32, fill: &str, extra: &str) {
    s.rect_e(x, y, w, h, rx, fill, extra);
}

fn line(s: &mut Svg, x1: f32, y1: f32, x2: f32, y2: f32, stroke: &str, w: f32, extra: &str) {
    s.line_e(x1, y1, x2, y2, stroke, w, extra);
}

fn circle(s: &mut Svg, cx: f32, cy: f32, r: f32, fill: &str, extra: &str) {
    s.circle_e(cx, cy, r, fill, extra);
}

/// The attributes of a stroke of `color` and width `w`.
fn sk(color: &str, w: f32) -> String {
    format!(r#"stroke="{color}" stroke-width="{}""#, n(w))
}

/// An unfilled path `d` stroked in `color`, width `w`, with `extra` attributes.
fn stroke_path(s: &mut Svg, d: &str, color: &str, w: f32, extra: &str) {
    s.raw(&format!(
        r#"<path d="{d}" fill="none" {} {extra}/>"#,
        sk(color, w)
    ));
}

/// A text's style: the mockup's `text` and `stext` arguments.
#[derive(Clone, Copy)]
struct St {
    fill: &'static str,
    anchor: Anchor,
    weight: u16,
    ls: f32,
}

impl St {
    /// The printed label on gray (the mockup's `stext` defaults).
    const fn label() -> Self {
        Self {
            fill: "#edf1f3",
            anchor: Anchor::Middle,
            weight: 500,
            ls: 0.15,
        }
    }
    /// Plain text (the mockup's `text` defaults).
    const fn plain(fill: &'static str) -> Self {
        Self {
            fill,
            anchor: Anchor::Middle,
            weight: 700,
            ls: 0.0,
        }
    }
    const fn start(mut self) -> Self {
        self.anchor = Anchor::Start;
        self
    }
    const fn end(mut self) -> Self {
        self.anchor = Anchor::End;
        self
    }
    const fn w(mut self, weight: u16) -> Self {
        self.weight = weight;
        self
    }
    const fn ls(mut self, ls: f32) -> Self {
        self.ls = ls;
        self
    }
    const fn fill(mut self, fill: &'static str) -> Self {
        self.fill = fill;
        self
    }
}

fn label<'a>(x: f32, y: f32, t: &'a str, size: f32, st: St) -> Label<'a> {
    Label {
        x,
        y,
        text: t,
        size,
        anchor: st.anchor,
        weight: st.weight,
        spacing: st.ls,
    }
}

/// Text in Inter, as outlines; (x, y) is the baseline point of the anchor.
fn text(s: &mut Svg, x: f32, y: f32, t: &str, size: f32, st: St, extra: &str) {
    text::draw(s, &label(x, y, t, size, st), st.fill, extra);
}

/// A label printed on the panel: the text with a soft dark copy 0.45 px down and right.
fn stext(s: &mut Svg, x: f32, y: f32, t: &str, size: f32, st: St) {
    let d = text::outline(&label(x, y, t, size, st));
    if d.is_empty() {
        return;
    }
    s.raw(&format!(
        r##"<path transform="translate(0.45 0.45)" d="{d}" fill="#1b262c" opacity="0.75"/>"##
    ));
    s.raw(&format!(r#"<path d="{d}" fill="{}"/>"#, st.fill));
}

/// A section plate: the dark seam around and the face.
fn plate(s: &mut Svg, x: f32, y: f32, w: f32, h: f32, fill: &str) {
    rect(s, x, y, w, h, 0.0, SEAM, "");
    rect(s, x + 0.5, y + 0.5, w - 1.0, h - 1.0, 0.8, fill, "");
}

/// A plate's light top and left edge and dark bottom and right edge.
fn plate_edges(s: &mut Svg, x: f32, y: f32, w: f32, h: f32) {
    let d = format!(
        "M{} {} V{} H{}",
        n(x + 0.9),
        n(y + h - 1.0),
        n(y + 0.9),
        n(x + w - 1.0)
    );
    stroke_path(s, &d, "#fff", 0.6, r#"stroke-opacity="0.28""#);
    let d = format!(
        "M{} {} V{} H{}",
        n(x + w - 0.9),
        n(y + 1.2),
        n(y + h - 0.9),
        n(x + 1.2)
    );
    stroke_path(s, &d, "#000", 0.6, r#"stroke-opacity="0.45""#);
}

/// The speckle of the plates (filter, opacity), and the 808's and the 909's.
const GRAIN_GRAY: [(&str, f32); 4] = [
    ("grD", 0.46),
    ("grL", 0.32),
    ("blotD", 0.11),
    ("blotL", 0.06),
];
const GRAIN_808: [(&str, f32); 3] = [("grD", 0.5), ("grL", 0.10), ("blotD", 0.14)];
const GRAIN_909: [(&str, f32); 2] = [("marble", 0.07), ("grD", 0.08)];

/// The speckle over the plate (x, y, w, h) of a part placed at (ox, oy) of the panel: the
/// noise is laid in panel coordinates, so every plate has its own speckle, the speckle the
/// mockup laid over the whole panel at once.
fn grain(s: &mut Svg, r: (f32, f32, f32, f32), ox: f32, oy: f32, layers: &[(&str, f32)]) {
    let (x, y, w, h) = r;
    let id = s.uid("grain");
    s.raw(&format!(
        r#"<clipPath id="{id}"><rect x="{}" y="{}" width="{}" height="{}" rx="0.8"/></clipPath>"#,
        n(x + 0.5),
        n(y + 0.5),
        n(w - 1.0),
        n(h - 1.0)
    ));
    s.g("", &format!(r#"clip-path="url(#{id})""#));
    s.g(&format!("translate({} {})", n(-ox), n(-oy)), "");
    for (f, op) in layers {
        // the fine speckle is a repeating tile (`pgrD`, `pgrL`), the rest noise over the plate
        let (fill, extra) = match *f {
            "grD" | "grL" => (format!("url(#p{f})"), format!(r#"opacity="{}""#, n(*op))),
            _ => (
                "#000".to_string(),
                format!(r#"filter="url(#{f})" opacity="{}""#, n(*op)),
            ),
        };
        rect(s, x + ox, y + oy, w, h, 0.0, &fill, &extra);
    }
    s.end_g().end_g();
}

/// A round LED of radius `r` at (cx, cy); `color` is R, G or Y. Lit, it has a highlight and a
/// halo of 2.6 r.
fn led_round(s: &mut Svg, cx: f32, cy: f32, r: f32, color: &str, on: bool) {
    if on {
        circle(s, cx, cy, r * 2.6, &format!("url(#halo{color})"), "");
    }
    circle(s, cx, cy, r + 0.45, "#0b0b0b", r#"opacity="0.85""#);
    let off = if on { "" } else { "off" };
    circle(s, cx, cy, r, &format!("url(#led{color}{off})"), "");
    if on {
        circle(
            s,
            cx - r * 0.3,
            cy - r * 0.35,
            r * 0.3,
            "#fff",
            r#"opacity="0.8""#,
        );
    }
}

/// The dark frame and the face of a seven-segment display (the digits are parts).
fn display_frame(s: &mut Svg, x: f32, y: f32, w: f32, h: f32) {
    rect(s, x - 0.8, y - 0.8, w + 1.6, h + 1.6, 0.8, "#0a0a0a", "");
    rect(s, x, y, w, h, 0.4, "#1c0505", "");
}

/// The dark frame of a pair of arrow keys (up at (x, y), down at (x, y + 9)).
fn arrows_frame(s: &mut Svg, x: f32, y: f32) {
    rect(s, x - 0.4, y - 0.4, 11.8, 18.8, 0.9, "#0d0d0d", "");
}

/// The tick marks of a knob centered at (cx, cy) between radii r0 and r1.
fn knob_ticks(
    s: &mut Svg,
    cx: f32,
    cy: f32,
    r0: f32,
    r1: f32,
    count: usize,
    stroke: &str,
    w: f32,
    opacity: f32,
) {
    let mut d = String::new();
    for i in 0..count {
        let ang = (-135.0 + 270.0 * i as f32 / (count - 1) as f32).to_radians();
        let (sn, c) = (ang.sin(), -ang.cos());
        d += &format!(
            "M{} {} L{} {}",
            n(cx + r0 * sn),
            n(cy + r0 * c),
            n(cx + r1 * sn),
            n(cy + r1 * c)
        );
    }
    let extra = format!(r#"stroke-linecap="round" opacity="{}""#, n(opacity));
    stroke_path(s, &d, stroke, w, &extra);
}

/// A knob's cast shadow: the body swept from the center toward the lower right, twice.
fn knob_shadow(s: &mut Svg, dx: f32, dy: f32, w0: f32, a0: f32, w1: f32, a1: f32) {
    for (w, a) in [(w0, a0), (w1, a1)] {
        let d = format!("M0 0 L{} {}", n(dx), n(dy));
        let extra = format!(r#"stroke-linecap="round" opacity="{}""#, n(a));
        stroke_path(s, &d, "#000", w, &extra);
    }
}

/// The pointer line of a knob, straight up from the center (cx, cy).
fn pointer(s: &mut Svg, cx: f32, cy: f32, y0: f32, y1: f32, stroke: &str, w: f32, cap: bool) {
    let cap = if cap {
        r#" stroke-linecap="round""#
    } else {
        ""
    };
    s.raw(&format!(
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{stroke}" stroke-width="{}"{cap}/>"#,
        n(cx),
        n(cy + y0),
        n(cx),
        n(cy + y1),
        n(w)
    ));
}

/// A pie slice of radius r around the origin; angles in degrees, 0 = up, clockwise.
fn wedge(r: f32, mid: f32, half: f32) -> String {
    let (a0, a1) = ((mid - half).to_radians(), (mid + half).to_radians());
    format!(
        "M0 0 L{} {} A{} {} 0 0 1 {} {} Z",
        n(r * a0.sin()),
        n(-r * a0.cos()),
        n(r),
        n(r),
        n(r * a1.sin()),
        n(-r * a1.cos())
    )
}

/// The knobs' two layers: the static body (ticks, cast shadow, wall, top) and the pointer.
fn knobs(out: &mut Vec<Part>) {
    for k in &KNOBS {
        let (cx, cy) = (k.cx, k.cy);
        out.push(Part::with_bleed(
            format!("{}_body", k.name),
            k.w,
            k.h,
            k.bleed,
            |s| knob_body(s, k),
        ));
        out.push(Part::with_bleed(
            format!("{}_pointer", k.name),
            k.w,
            k.h,
            k.bleed,
            |s| match k.name {
                "knob_303" => pointer(s, cx, cy, -1.6, -8.0, "#151617", 1.4, false),
                "knob_808_red" => pointer(s, cx, cy, -2.1, -4.8, "#210403", 1.2, false),
                "knob_808_white" => pointer(s, cx, cy, -2.1, -4.8, "#101112", 1.2, false),
                "knob_909" => pointer(s, cx, cy, -1.9, -4.4, "#ff7a1c", 1.25, false),
                _ => pointer(s, cx, cy, -0.7, -5.2, "#fff", 1.1, true),
            },
        ));
    }
}

/// The knurling of a wall between radii `r` and `t`: a dashed ring.
fn knurl(s: &mut Svg, radius: f32, color: &str, op: f32, w: f32, dash: &str) {
    let extra = format!(r#"stroke-opacity="{}" stroke-dasharray="{dash}""#, n(op));
    s.raw(&format!(
        r#"<circle cx="0" cy="0" r="{}" fill="none" {} {extra}/>"#,
        n(radius),
        sk(color, w)
    ));
}

/// The static body of a knob: its ticks, cast shadow, wall and top.
fn knob_body(s: &mut Svg, k: &KnobDesign) {
    let (cx, cy) = (k.cx, k.cy);
    let rim = |w: f32| format!(r#"stroke="url(#gRimLight)" stroke-width="{}""#, n(w));
    match k.name {
        "knob_303" => {
            // a machined aluminum cylinder seen from above: a dark knurled wall and a flat top
            // whose brushed finish reflects in sectors
            knob_ticks(s, cx, cy, 11.9, 14.5, 11, "#26282a", 1.25, 1.0);
            let (r, t) = (10.0, 8.5);
            s.g_at(cx, cy);
            knob_shadow(s, 1.7, 2.1, 2.0 * r + 1.4, 0.09, 2.0 * r, 0.24);
            circle(s, 0.0, 0.0, r, "url(#g303Wall)", &sk("#141516", 0.45));
            knurl(s, (r + t) / 2.0, "#e8eaec", 0.55, r - t - 0.3, "0.42 0.6");
            circle(s, 0.0, 0.0, t, "#d9dbde", "");
            let clip = s.uid("top");
            s.raw(&format!(
                r#"<clipPath id="{clip}"><circle cx="0" cy="0" r="8.5"/></clipPath>"#
            ));
            s.raw(&format!(
                concat!(
                    r#"<g clip-path="url(#{})"><g filter="url(#b1)">"#,
                    r##"<path d="{} {}" fill="#ffffff"/>"##,
                    r##"<path d="{} {}" fill="#878b90" opacity="0.62"/></g></g>"##
                ),
                clip,
                wedge(13.0, 315.0, 34.0),
                wedge(13.0, 135.0, 30.0),
                wedge(13.0, 45.0, 26.0),
                wedge(13.0, 225.0, 28.0)
            ));
            let edge = r#"stroke="url(#g303Edge)" stroke-width="0.55""#;
            circle(s, 0.0, 0.0, t, "none", edge);
            let inner = format!(r#"{} stroke-opacity="0.10""#, sk("#000", 0.35));
            circle(s, 0.0, 0.0, t - 0.75, "none", &inner);
            circle(s, 0.0, 0.0, 0.9, "#8b8e93", r#"opacity="0.5""#);
            s.end_g();
        }
        "knob_808_red" | "knob_808_white" => {
            let (r, t) = (5.75, 4.7);
            s.g_at(cx, cy);
            knob_shadow(s, 1.3, 1.6, 2.0 * r + 1.6, 0.16, 2.0 * r, 0.42);
            circle(s, 0.0, 0.0, r, "url(#g808Collar)", "");
            let face = if k.name == "knob_808_red" {
                "url(#g808RedF)"
            } else {
                "url(#g808WhiteF)"
            };
            circle(s, 0.0, 0.0, t, face, "");
            circle(s, 0.0, 0.0, t - 0.2, "none", &rim(0.4));
            s.end_g();
        }
        "knob_909" => {
            knob_ticks(s, cx, cy, 5.7, 7.0, 11, "#4b4b48", 0.5, 0.9);
            let (r, t) = (5.25, 4.55);
            s.g_at(cx, cy);
            knob_shadow(s, 1.1, 1.4, 2.0 * r + 1.2, 0.08, 2.0 * r, 0.25);
            circle(s, 0.0, 0.0, r, "url(#g909Wall)", &sk("#0c0c0b", 0.3));
            circle(s, 0.0, 0.0, t, "url(#g909Top)", "");
            let rim7 = format!(r#"{} opacity="0.7""#, rim(0.35));
            circle(s, 0.0, 0.0, t - 0.15, "none", &rim7);
            s.end_g();
        }
        _ => {
            // the gray sections': a flat blue-gray cap in a black knurled collar
            knob_ticks(s, cx, cy, 9.4, 11.6, 7, "#e6ecef", 0.7, 0.9);
            let (r, t) = (7.7, 5.75);
            s.g_at(cx, cy);
            knob_shadow(s, 1.4, 1.7, 2.0 * r + 1.4, 0.10, 2.0 * r, 0.30);
            circle(s, 0.0, 0.0, r, "url(#gGrayCollar)", &sk("#050506", 0.35));
            knurl(
                s,
                (r + t) / 2.0 + 0.1,
                "#8a9098",
                0.35,
                r - t - 0.5,
                "0.4 0.62",
            );
            circle(s, 0.0, 0.0, t, "url(#gGrayTop)", "");
            circle(s, 0.0, 0.0, t - 0.2, "none", &rim(0.4));
            s.end_g();
        }
    }
}

/// The level fader: the track (11 ticks and the slot, 15 x 54) and the handle (15 x 11).
fn fader(out: &mut Vec<Part>) {
    out.push(Part::with_bleed("fader_track", 15.0, 54.0, 0.5, |s| {
        for i in 0..11 {
            let y = 5.5 + 4.3 * i as f32;
            line(s, 0.8, y, 14.2, y, "#d5dde2", 0.5, r#"opacity="0.8""#);
            line(
                s,
                0.8,
                y + 0.5,
                14.2,
                y + 0.5,
                "#000",
                0.4,
                r#"opacity="0.25""#,
            );
        }
        rect(s, 6.4, 1.5, 2.2, 51.0, 1.1, "url(#gSlot)", "");
    }));
    out.push(Part::with_bleed("fader_handle", 15.0, 11.0, 4.5, |s| {
        rect(
            s,
            1.6,
            2.2,
            14.0,
            11.0,
            1.4,
            "#000",
            r#"opacity="0.45" filter="url(#b1)""#,
        );
        rect(
            s,
            0.4,
            0.0,
            14.2,
            11.0,
            1.3,
            "url(#gFader)",
            r##"stroke="#050506" stroke-width="0.4""##,
        );
        rect(s, 1.1, 0.5, 12.8, 0.6, 0.3, "#fff", r#"opacity="0.35""#);
        rect(s, 0.9, 5.05, 13.2, 0.9, 0.2, "#e9edf0", "");
        for dy in [2.3, 3.6, 7.6, 8.9] {
            rect(s, 1.3, dy, 12.4, 0.35, 0.0, "#000", r#"opacity="0.55""#);
        }
    }));
}

/// A small silver push button of w x h at (x, y) in a dark rim: at rest a light bevel and a
/// soft shadow; pressed, "variant F, dimmed" (the user's choice, 2026-11-13): the bevel and
/// the shadow go, the face is flat and a little lighter with a faint gradient.
fn pill(s: &mut Svg, x: f32, y: f32, w: f32, h: f32, pressed: bool, r: Option<f32>) {
    let r = r.unwrap_or(w.min(h) * 0.22);
    let fr0 = (r - 0.6).max(0.3);
    if !pressed {
        rect(
            s,
            x + 0.6,
            y + 0.9,
            w,
            h,
            r,
            "#000",
            r#"opacity="0.35" filter="url(#b05)""#,
        );
        rect(s, x, y, w, h, r, "#1f2022", "");
        rect(
            s,
            x + 0.8,
            y + 0.8,
            w - 1.6,
            h - 1.6,
            fr0,
            "url(#gPill)",
            "",
        );
        let d = format!(
            "M{} {} V{} H{}",
            n(x + 1.2),
            n(y + h - 1.3),
            n(y + 1.2),
            n(x + w - 1.3)
        );
        stroke_path(s, &d, "#fff", 0.5, r#"stroke-opacity="0.9""#);
    } else {
        rect(s, x, y, w, h, r, "#1f2022", "");
        rect(
            s,
            x + 0.8,
            y + 0.8,
            w - 1.6,
            h - 1.6,
            fr0,
            "url(#gPillFlatLit)",
            "",
        );
        flat_ring(
            s,
            x + 1.15,
            y + 1.15,
            w - 2.3,
            h - 2.3,
            (fr0 - 0.3).max(0.2),
        );
    }
}

/// The faint inner line of a pressed (variant F) face.
fn flat_ring(s: &mut Svg, x: f32, y: f32, w: f32, h: f32, r: f32) {
    rect(
        s,
        x,
        y,
        w,
        h,
        r,
        "none",
        r##"stroke="#000" stroke-opacity="0.10" stroke-width="0.35""##,
    );
}

/// A white keycap of `size` centered at (cx, cy) with a label; lit, it is tinted red and glows.
fn keycap(s: &mut Svg, cx: f32, cy: f32, size: f32, label_: &str, lit: bool) {
    let (x, y) = (cx - size / 2.0, cy - size / 2.0);
    if lit {
        rect(
            s,
            x - 1.0,
            y - 1.0,
            size + 2.0,
            size + 2.0,
            3.2,
            "#ff3a2a",
            r#"opacity="0.8" filter="url(#b1)""#,
        );
    }
    let (fill, stroke) = if lit {
        ("url(#gCapLit)", r##"stroke="#e53b2d" stroke-width="0.6""##)
    } else {
        ("url(#gCap)", r##"stroke="#5d6166" stroke-width="0.3""##)
    };
    rect(s, x, y, size, size, 2.4, fill, stroke);
    let op = if lit { "0.55" } else { "0.7" };
    rect(
        s,
        x + 1.3,
        y + 1.0,
        size - 2.6,
        size * 0.36,
        1.8,
        "#fff",
        &format!(r#"opacity="{op}""#),
    );
    if !label_.is_empty() {
        let fs = 7.4;
        let ink = if lit { "#a8251b" } else { "#3c4046" };
        text(s, cx, cy + fs * 0.36, label_, fs, St::plain(ink), "");
    }
}

/// A vertical two-position slide switch of w x h at (x, y); pos 0 = top.
fn slide_v(s: &mut Svg, x: f32, y: f32, w: f32, h: f32, pos: u8) {
    rect(
        s,
        x,
        y,
        w,
        h,
        1.2,
        "#060606",
        r##"stroke="#000" stroke-width="0.3""##,
    );
    rect(s, x + 0.5, y + 0.5, w - 1.0, h - 1.0, 0.9, "#1a1a1a", "");
    let th = h * 0.5;
    let ty = if pos == 0 { y + 0.8 } else { y + h - th - 0.8 };
    rect(
        s,
        x + 0.9,
        ty,
        w - 1.8,
        th,
        0.8,
        "url(#gSlideD)",
        r##"stroke="#000" stroke-width="0.3""##,
    );
    for i in 0..3 {
        let yy = ty + th * (0.28 + 0.22 * i as f32);
        line(
            s,
            x + 1.7,
            yy,
            x + w - 1.7,
            yy,
            "#000",
            0.45,
            r#"opacity="0.6""#,
        );
        line(
            s,
            x + 1.7,
            yy + 0.45,
            x + w - 1.7,
            yy + 0.45,
            "#fff",
            0.3,
            r#"opacity="0.3""#,
        );
    }
}

/// A chrome bat toggle seen from above, at (cx, cy), thrown up or down.
fn lever(s: &mut Svg, cx: f32, cy: f32, down: bool) {
    s.raw(&format!(
        r##"<ellipse cx="{}" cy="{}" rx="3.3" ry="4.1" fill="#0b0c0d" {}/>"##,
        n(cx),
        n(cy),
        sk("#8b949a", 0.5)
    ));
    let dy: f32 = if down { 5.6 } else { -5.6 };
    let (y0, y1) = if dy > 0.0 {
        (cy, cy + dy)
    } else {
        (cy + dy, cy)
    };
    s.raw(&format!(
        r##"<ellipse cx="{}" cy="{}" rx="2.2" ry="4.4" fill="#000" {}/>"##,
        n(cx + 0.9),
        n(cy + dy * 0.55 + 1.2),
        r#"opacity="0.4" filter="url(#b05)""#
    ));
    let stroke = r##"stroke="#33383c" stroke-width="0.35""##;
    rect(
        s,
        cx - 1.75,
        y0 - 1.6,
        3.5,
        y1 - y0 + 3.2,
        1.75,
        "url(#gLever)",
        stroke,
    );
    circle(s, cx, cy + dy, 1.9, "url(#gLever)", stroke);
}

/// The green on/off lamp of a header, `sz` square in a frame of `frame`, centered in w x h at
/// (x, y).
#[allow(clippy::too_many_arguments)]
fn onoff(s: &mut Svg, x: f32, y: f32, w: f32, h: f32, sz: f32, on: bool, frame: &str) {
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    if on {
        circle(s, cx, cy, sz * 1.25, "url(#haloG)", "");
    }
    let (x0, y0) = (cx - sz / 2.0, cy - sz / 2.0);
    rect(s, x0 - 1.3, y0 - 1.3, sz + 2.6, sz + 2.6, 1.4, frame, "");
    rect(
        s,
        x0 - 0.5,
        y0 - 0.5,
        sz + 1.0,
        sz + 1.0,
        0.9,
        "#0c1a0e",
        "",
    );
    rect(
        s,
        x0,
        y0,
        sz,
        sz,
        0.7,
        if on { "url(#ledG)" } else { "url(#ledGoff)" },
        "",
    );
    if on {
        rect(
            s,
            x0 + 0.7,
            y0 + 0.6,
            sz - 1.4,
            sz * 0.32,
            0.5,
            "#fff",
            r#"opacity="0.55""#,
        );
    }
}

/// The 4-segment meter of a header, w x h at (x, y), `level` segments lit from the bottom.
fn meter4(s: &mut Svg, x: f32, y: f32, w: f32, h: f32, level: usize) {
    rect(s, x + 1.5, y + 1.0, w - 3.0, h - 2.0, 0.8, "#101010", "");
    let cols = [
        ("#ff3a2c", "#4a120e"),
        ("#ffb02e", "#4a3410"),
        ("#d8f23a", "#3c4312"),
        ("#48e83c", "#12401a"),
    ];
    let sh = (h - 4.4) / 4.0;
    for (i, (lit, dark)) in cols.into_iter().enumerate() {
        let on = 3 - i < level;
        let yy = y + 2.2 + i as f32 * sh;
        if on {
            rect(
                s,
                x + 1.6,
                yy - 0.6,
                w - 3.2,
                sh + 0.4,
                1.0,
                lit,
                r#"opacity="0.5" filter="url(#b05)""#,
            );
        }
        rect(
            s,
            x + 3.0,
            yy,
            w - 6.0,
            sh - 0.9,
            0.3,
            if on { lit } else { dark },
            "",
        );
    }
}

/// A section header: a glossy bar with a soft shadow and its title in Inter Bold.
fn header(
    s: &mut Svg,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    grad: &str,
    title: &str,
    tx: f32,
    size: f32,
    ls: f32,
) {
    rect(
        s,
        x + 0.5,
        y + 0.8,
        w,
        h,
        1.2,
        "#000",
        r#"opacity="0.45" filter="url(#b05)""#,
    );
    rect(s, x, y, w, h, 1.1, &format!("url(#{grad})"), "");
    rect(s, x, y, w, h, 1.1, "url(#gSheen)", "");
    rect(
        s,
        x + 0.6,
        y + 0.4,
        w - 1.2,
        0.6,
        0.3,
        "#fff",
        r#"opacity="0.35""#,
    );
    let by = y + h / 2.0 + size * 0.36;
    text(
        s,
        tx + 0.5,
        by + 0.5,
        title,
        size,
        St::plain("#000").ls(ls),
        r#"opacity="0.45""#,
    );
    text(s, tx, by, title, size, St::plain("#f7f7f2").ls(ls), "");
}

/// The printed scale and title of a gray knob at (kx, ky).
fn knob_scale(s: &mut Svg, kx: f32, ky: f32, lo: &str, hi: &str, title: &str) {
    stext(s, kx - 2.6, ky + 24.2, lo, 5.4, St::label());
    stext(s, kx + 20.8, ky + 24.2, hi, 5.4, St::label());
    stext(s, kx + 9.0, ky + 31.2, title, 6.2, St::label());
}

/// A gray plate of w x h at the part's origin, placed at (ox, oy) of the panel.
fn gray_plate(s: &mut Svg, w: f32, h: f32, ox: f32, oy: f32) {
    plate(s, 0.0, 0.0, w, h, GRAY);
    grain(s, (0.0, 0.0, w, h), ox, oy, &GRAIN_GRAY);
    plate_edges(s, 0.0, 0.0, w, h);
}

// ---------------------------------------------------------------- the wood and the transport

fn wood(out: &mut Vec<Part>) {
    for (name, ox) in [("wood_left", 0.0), ("wood_right", 608.0)] {
        out.push(Part::new(name, 16.0, 510.0, |s| {
            rect(s, 0.0, 0.0, 16.0, 510.0, 0.0, "url(#gWood)", "");
            let clip = s.uid("wood");
            s.raw(&format!(
                r#"<clipPath id="{clip}"><rect x="0" y="0" width="16" height="510"/></clipPath>"#
            ));
            s.g("", &format!(r#"clip-path="url(#{clip})""#));
            s.g(&format!("translate({} 0)", n(-ox)), "");
            rect(
                s,
                ox,
                0.0,
                16.0,
                510.0,
                0.0,
                "#000",
                r#"filter="url(#woodD)" opacity="0.55""#,
            );
            rect(
                s,
                ox,
                0.0,
                16.0,
                510.0,
                0.0,
                "#000",
                r#"filter="url(#woodL)" opacity="0.28""#,
            );
            s.end_g().end_g();
            rect(s, 0.0, 0.0, 1.0, 510.0, 0.0, "#000", r#"opacity="0.45""#);
            rect(s, 15.0, 0.0, 1.0, 510.0, 0.0, "#000", r#"opacity="0.55""#);
            rect(s, 1.0, 0.0, 0.6, 510.0, 0.0, "#fff", r#"opacity="0.12""#);
        }));
    }
}

/// The transport's labels and well color.
const LIGHT: &str = "#c9d3d8";

fn transport_bg(out: &mut Vec<Part>) {
    out.push(Part::new("bg_transport", 592.0, 50.0, |s| {
        gray_plate(s, 592.0, 50.0, 16.0, 0.0);
        let l = St::label();
        stext(s, 8.0, 37.6, "0", 5.4, l);
        stext(s, 32.4, 37.6, "10", 5.4, l);
        stext(s, 20.0, 45.8, "SHUFFLE", 5.6, l.w(700).ls(0.1));
        // Sync and MIDI: drawn, dark (MIDI is out of scope)
        led_round(s, 44.5, 7.5, 1.9, "G", false);
        stext(s, 50.0, 10.2, "Sync", 7.2, l.start().fill(LIGHT));
        led_round(s, 79.5, 7.5, 1.9, "G", false);
        stext(s, 85.0, 10.2, "MIDI", 7.2, l.start().fill(LIGHT));
        let title = l.start().w(700).ls(0.0);
        stext(s, 44.0, 24.2, "Tempo", 8.6, title);
        display_frame(s, 44.0, 27.0, 27.0, 16.0);
        arrows_frame(s, 73.0, 26.0);
        stext(s, 158.0, 13.4, "Pattern", 9.2, l.end().ls(0.0));
        stext(s, 192.0, 13.4, "Song mode", 9.2, l.start().ls(0.0));
        // the well of the five transport keys
        rect(
            s,
            95.4,
            20.2,
            162.2,
            27.2,
            2.2,
            "#000",
            r#"opacity="0.5" filter="url(#b1)""#,
        );
        rect(s, 94.8, 19.8, 162.4, 27.4, 2.0, "#0c0d0e", "");
        stext(s, 276.0, 24.2, "Bar", 8.6, title);
        display_frame(s, 271.0, 27.0, 27.0, 16.0);
        arrows_frame(s, 300.0, 26.0);
        stext(s, 362.5, 12.4, "Loop", 9.2, l.ls(0.0));
        for (x1, x2) in [(334.5, 348.0), (377.5, 400.0)] {
            line(
                s,
                x1 + 0.4,
                9.9,
                x2 + 0.4,
                9.9,
                "#1b262c",
                0.7,
                r#"opacity="0.7""#,
            );
            line(s, x1, 9.5, x2, 9.5, "#edf1f3", 0.7, "");
        }
        stext(s, 331.0, 24.2, "Start", 8.6, title);
        display_frame(s, 329.0, 27.0, 27.0, 16.0);
        arrows_frame(s, 358.0, 26.0);
        stext(s, 372.5, 24.2, "Length", 8.6, title);
        display_frame(s, 372.0, 27.0, 27.0, 16.0);
        arrows_frame(s, 401.0, 26.0);
    }));
}

/// The color of the wind keys' arrows.
const WIND: &str = "#4b545a";

/// The five transport keys, 32 x 25 each.
fn transport_buttons(out: &mut Vec<Part>) {
    for (name, states) in [
        ("play", &["rest", "pressed", "lit"][..]),
        ("stop", &["rest", "pressed"][..]),
        ("rewind", &["rest", "pressed"][..]),
        ("ff", &["rest", "pressed"][..]),
        ("record", &["rest", "pressed", "lit"][..]),
    ] {
        for &state in states {
            out.push(Part::with_bleed(
                format!("btn_{name}_{state}"),
                32.0,
                25.0,
                1.0,
                |s| {
                    if state == "pressed" {
                        // variant F: flat, lighter, no highlight and no lower shade
                        rect(s, 0.6, 0.6, 30.8, 23.8, 1.0, "url(#gPillFlatLit)", "");
                        flat_ring(s, 0.95, 0.95, 30.1, 23.1, 0.7);
                    } else {
                        rect(s, 0.6, 0.6, 30.8, 23.8, 1.0, "url(#gTrans)", "");
                        rect(s, 1.0, 1.0, 30.0, 1.0, 0.5, "#fff", r#"opacity="0.8""#);
                        rect(s, 0.6, 22.4, 30.8, 2.0, 0.8, "#000", r#"opacity="0.25""#);
                    }
                    let (cx, cy) = (16.0, 13.0);
                    let p = |v: f32| n(v);
                    match name {
                        "play" => {
                            let (fill, stroke) = if state == "lit" {
                                circle(s, cx, cy, 8.0, "url(#haloG)", "");
                                ("#35e22a", "#0e6a12")
                            } else {
                                // at rest: no halo, a darker green
                                ("#23891c", "#0b3f0d")
                            };
                            let d = format!(
                                "M{} {} L{} {} L{} {} Z",
                                p(cx - 3.6),
                                p(cy - 4.6),
                                p(cx + 4.8),
                                p(cy),
                                p(cx - 3.6),
                                p(cy + 4.6)
                            );
                            s.raw(&format!(
                                r#"<path d="{d}" fill="{fill}" {} stroke-linejoin="round"/>"#,
                                sk(stroke, 0.4)
                            ));
                        }
                        "stop" => {
                            let edge = sk("#3c1806", 0.4);
                            rect(s, cx - 3.2, cy - 3.2, 6.4, 6.4, 0.5, "#8a3b12", &edge);
                        }
                        "rewind" => {
                            s.raw(&format!(
                            r##"<path d="M{} {} V{} L{} {} Z M{} {} V{} L{} {} Z" fill="{}"/>"##,
                            p(cx - 0.4),
                            p(cy - 3.6),
                            p(cy + 3.6),
                            p(cx - 6.0),
                            p(cy),
                            p(cx + 5.4),
                            p(cy - 3.6),
                            p(cy + 3.6),
                            p(cx - 0.2),
                            p(cy),
                            WIND
                        ));
                        }
                        "ff" => {
                            s.raw(&format!(
                            r##"<path d="M{} {} V{} L{} {} Z M{} {} V{} L{} {} Z" fill="{}"/>"##,
                            p(cx + 0.4),
                            p(cy - 3.6),
                            p(cy + 3.6),
                            p(cx + 6.0),
                            p(cy),
                            p(cx - 5.4),
                            p(cy - 3.6),
                            p(cy + 3.6),
                            p(cx + 0.2),
                            p(cy),
                            WIND
                        ));
                        }
                        _ => {
                            if state == "lit" {
                                // recording: lit red with a halo, as Play's green
                                circle(s, cx, cy, 8.0, "url(#haloR)", "");
                                circle(s, cx, cy, 3.5, "#ff3b2c", &sk("#8a0e08", 0.4));
                                let hi = r#"opacity="0.8""#;
                                circle(s, cx - 1.0, cy - 1.1, 1.0, "#ffd9d2", hi);
                            } else {
                                circle(s, cx, cy, 3.5, "#9a1b14", &sk("#4a0a07", 0.4));
                                let hi = r#"opacity="0.5""#;
                                circle(s, cx - 1.0, cy - 1.1, 1.0, "#ff8a7a", hi);
                            }
                        }
                    }
                },
            ));
        }
    }
    // The arrows of every value display: 11 x 9 silver keys (their frame is in the
    // background).
    for (name, up) in [("inc", true), ("dec", false)] {
        for (state, pressed) in [("rest", false), ("pressed", true)] {
            out.push(Part::with_bleed(
                format!("arrow_{name}_{state}"),
                11.0,
                9.0,
                0.5,
                |s| {
                    if pressed {
                        rect(s, 0.7, 0.7, 9.6, 7.6, 0.7, "url(#gPillFlatLit)", "");
                        flat_ring(s, 1.05, 1.05, 8.9, 6.9, 0.4);
                    } else {
                        rect(s, 0.7, 0.7, 9.6, 7.6, 0.7, "url(#gSilver)", "");
                        rect(s, 1.1, 1.0, 8.8, 0.6, 0.3, "#fff", r#"opacity="0.85""#);
                    }
                    let d = if up {
                        "M3.1 6 L5.5 2.6 L7.9 6 Z"
                    } else {
                        "M3.1 3 L5.5 6.4 L7.9 3 Z"
                    };
                    s.raw(&format!(r##"<path d="{d}" fill="#24272b"/>"##));
                },
            ));
        }
    }
    // The pattern / song mode switch: the green "Pattern" LED, the bat toggle and the yellow
    // "Song" LED (26 x 19 at (161, 2) of the transport); 0 pattern, 1 song.
    for pos in 0..2 {
        out.push(Part::with_bleed(
            format!("switch_mode_{pos}"),
            26.0,
            19.0,
            3.0,
            |s| {
                led_round(s, 4.5, 9.5, 1.9, "G", pos == 0);
                lever(s, 13.0, 9.5, pos == 1);
                led_round(s, 21.5, 9.5, 1.9, "Y", pos == 1);
            },
        ));
    }
    // Loop on / off: the bat toggle and its LED (15 x 18 at (317, 3) of the transport).
    for (state, on) in [("off", false), ("on", true)] {
        out.push(Part::with_bleed(
            format!("toggle_loop_{state}"),
            15.0,
            18.0,
            4.0,
            |s| {
                lever(s, 4.5, 9.0, on);
                led_round(s, 12.2, 9.3, 1.7, "Y", on);
            },
        ));
    }
}

// ---------------------------------------------------------------- the wordmark

/// The size of the logo parts, and of the logo's click area (`layout::on_logo` of orb-ui).
pub const LOGO_W: f32 = 134.0;
pub const LOGO_H: f32 = 48.0;
/// Where the logo sits on the panel (the transport section is at x = 16, the logo at 458 in it).
pub const LOGO_X: f32 = 474.0;
pub const LOGO_Y: f32 = 1.0;
/// The pieces of the logo, each on the whole `LOGO_W` x `LOGO_H` canvas: every piece is placed
/// at (0, 0) of the logo, so an animation only offsets or fades a piece.
pub const LOGO_PIECES: [&str; 7] = [
    "logo_open",
    "logo_rb338",
    "logo_line_0",
    "logo_line_1",
    "logo_line_2",
    "logo_line_3",
    "logo_sub",
];
/// The wordmark's glyphs: the stroke font of `font.rs` at cap 14 (scale 2.33), the run
/// starting at x 8 and 338 at x 92.
const LOGO_X0: f32 = 8.0;
const LOGO_338_X: f32 = 92.0;

/// The glyphs `from..to` of `word`, laid out as `font.rs` lays the whole word out, each a
/// `<path>` translated to its place in glyph units.
fn glyph_run(word: &str, range: std::ops::Range<usize>) -> String {
    let cap = 14.0;
    let mut out = String::new();
    let mut dx = 0.0;
    for (i, c) in word.chars().enumerate() {
        let (gw, d) = crate::font::glyph_path(c);
        if range.contains(&i) && !d.is_empty() {
            out += &format!(r#"<path transform="translate({})" d="{d}"/>"#, n(dx));
        }
        dx += gw + crate::font::gap(cap) / (cap / 6.0);
    }
    out
}

/// One run of the wordmark: the glyphs pressed into the plate (a dark copy below-right and a
/// light copy above-left of the stroke).
fn logo_run(s: &mut Svg, tx: f32, paths: &str, stroke: &str) {
    let sw = 1.02;
    let run = |s: &mut Svg, x: f32, y: f32, col: &str, extra: &str, w: f32| {
        s.raw(&format!(
            concat!(
                r#"<g transform="translate({} {}) scale(2.33)" fill="none" {} {}"#,
                r#" stroke-linecap="round" stroke-linejoin="round">{}</g>"#
            ),
            n(x),
            n(y),
            sk(col, w),
            extra,
            paths
        ));
    };
    for (dx, dy, col, op, w) in [
        (0.55, 0.75, "#0b1216", "0.75", sw + 0.25),
        (-0.3, -0.35, "#ffffff", "0.35", sw),
    ] {
        let extra = format!(r#"stroke-opacity="{op}""#);
        run(s, tx + dx, 9.5 + dy, col, &extra, w);
    }
    run(s, tx, 9.5, stroke, "", sw);
}

/// The four underline bars: the 808's step key colors, as glossy as its keys.
pub const LOGO_LINE_COLORS: [&str; 4] = ["R", "O", "Y", "W"];

fn logo_piece(s: &mut Svg, name: &str) {
    match name {
        "logo_open" => logo_run(s, LOGO_X0, &glyph_run("OPENRB-", 0..4), "#f2f5f7"),
        "logo_rb338" => {
            logo_run(s, LOGO_X0, &glyph_run("OPENRB-", 4..7), "#f2f5f7");
            logo_run(s, LOGO_338_X, &glyph_run("338", 0..3), "#ff9a2e");
        }
        "logo_sub" => {
            let st = St::label().start().ls(1.02).fill("#d7dfe4");
            stext(s, 8.0, 40.4, "BASS LINE AND DRUM MACHINE", 5.5, st);
        }
        _ => {
            let i: usize = name["logo_line_".len()..].parse().unwrap();
            let bx = 8.0 + 29.9 * i as f32;
            rect(
                s,
                bx + 0.4,
                28.2,
                28.4,
                2.6,
                1.0,
                "#000",
                r#"opacity="0.55" filter="url(#b05)""#,
            );
            rect(
                s,
                bx,
                27.6,
                28.4,
                2.6,
                0.9,
                &format!("url(#g808k{})", LOGO_LINE_COLORS[i]),
                r##"stroke="#101010" stroke-width="0.3""##,
            );
        }
    }
}

/// Our own wordmark, `OPENRB-338`: the whole (`logo`, the pieces in the order of
/// `LOGO_PIECES`) and its pieces for the startup animation (task-114).
fn logo(out: &mut Vec<Part>) {
    out.push(Part::new("logo", LOGO_W, LOGO_H, |s| {
        for p in LOGO_PIECES {
            logo_piece(s, p);
        }
    }));
    for p in LOGO_PIECES {
        out.push(Part::new(p, LOGO_W, LOGO_H, |s| logo_piece(s, p)));
    }
}

/// The plate under the wordmark in Help > About (268 x 92 at (92, 14) of the window's
/// content): the transport's gray, bevel and speckle, with a soft shadow.
pub const ABOUT_PLATE: (f32, f32, f32, f32) = (92.0, 14.0, 268.0, 92.0);

fn about_plate(out: &mut Vec<Part>) {
    let (x, y, w, h) = ABOUT_PLATE;
    out.push(Part::with_bleed("about_plate", w, h, 8.0, |s| {
        rect(
            s,
            1.5,
            2.5,
            w,
            h,
            2.0,
            "#000",
            r#"opacity="0.6" filter="url(#b2)""#,
        );
        gray_plate(s, w, h, x, y);
    }));
}

/// The window's desk around the unit (task-179): a tile of 256 x 256 points with the
/// speckle and a faint horizontal brushing, stitched so that it repeats without a seam. The
/// app paints the desk's gradient, lays this tile over it and paints the unit's shadow.
pub const DESK_TILE: f32 = 256.0;

fn desk_tile(out: &mut Vec<Part>) {
    out.push(Part::new("desk_tile", DESK_TILE, DESK_TILE, |s| {
        for (f, op) in [("brush", "0.10"), ("grDs", "0.5"), ("grLs", "0.07")] {
            rect(
                s,
                0.0,
                0.0,
                DESK_TILE,
                DESK_TILE,
                0.0,
                "#000",
                &format!(r#"filter="url(#{f})" opacity="{op}""#),
            );
        }
    }));
}

// ---------------------------------------------------------------- the pattern sections

/// The centers of the pattern keys (x of the columns, y of the two rows) and of the bank
/// keys' row, and the keycaps' size.
pub const KEY_X: [f32; 4] = [11.6, 27.2, 42.8, 58.4];
pub const KEY_Y: [f32; 2] = [30.9, 46.5];
pub const BANK_Y: f32 = 72.9;
pub const KEYCAP: f32 = 14.6;

fn pattern_bg(out: &mut Vec<Part>) {
    for r in 0..4 {
        let oy = 50.0 + 115.0 * r as f32;
        out.push(Part::new(
            format!("bg_pattern_{}", r + 1),
            70.0,
            115.0,
            |s| {
                gray_plate(s, 70.0, 115.0, 16.0, oy);
                header(
                    s, 4.0, 4.0, 62.0, 13.6, "gHdrPat", "PATTERN", 39.5, 8.4, 0.35,
                );
                let well = |s: &mut Svg, x: f32, y: f32, w: f32, h: f32| {
                    rect(s, x, y, w, h, 3.0, "#0d0e0f", "");
                    rect(
                        s,
                        x,
                        y,
                        w,
                        h,
                        3.0,
                        "none",
                        r##"stroke="#fff" stroke-opacity="0.18" stroke-width="0.35""##,
                    );
                };
                well(s, 3.5, 22.8, 63.0, 31.8);
                let bold = St::label().start().w(700);
                stext(s, 5.0, 63.0, "BANK:", 6.0, bold);
                well(s, 3.5, 64.8, 63.0, 16.2);
                stext(s, 5.0, 91.6, "SHUFFLE", 5.4, bold.ls(0.0));
                stext(s, 36.5, 91.6, "STEPS", 5.4, bold.ls(0.2));
                well(s, 3.7, 93.9, 16.6, 16.6);
                display_frame(s, 36.0, 95.0, 19.0, 16.0);
                arrows_frame(s, 56.0, 94.0);
            },
        ));
    }
    // the row's focus bar, right of the section (4 x 115 at (86, 50 + 115 r) of the panel)
    for (name, on) in [("focus_off", false), ("focus_on", true)] {
        out.push(Part::with_bleed(name, 4.0, 115.0, 2.0, |s| {
            rect(s, 0.0, 0.0, 4.0, 115.0, 0.0, "#101214", "");
            rect(
                s,
                1.2,
                1.0,
                1.8,
                113.0,
                0.6,
                if on { "#ff3b30" } else { "#4a1512" },
                "",
            );
            if on {
                rect(
                    s,
                    0.6,
                    1.0,
                    3.0,
                    113.0,
                    1.0,
                    "#ff3b30",
                    r#"opacity="0.55" filter="url(#b05)""#,
                );
            }
        }));
    }
    for k in 1..=8 {
        for (state, on) in [("off", false), ("on", true)] {
            out.push(Part::with_bleed(
                format!("pat_btn_{k}_{state}"),
                KEYCAP,
                KEYCAP,
                3.0,
                |s| keycap(s, KEYCAP / 2.0, KEYCAP / 2.0, KEYCAP, &k.to_string(), on),
            ));
        }
    }
    for bank in ["A", "B", "C", "D"] {
        for (state, on) in [("off", false), ("on", true)] {
            out.push(Part::with_bleed(
                format!("bank_btn_{bank}_{state}"),
                KEYCAP,
                KEYCAP,
                3.0,
                |s| keycap(s, KEYCAP / 2.0, KEYCAP / 2.0, KEYCAP, bank, on),
            ));
        }
    }
    for (state, on) in [("off", false), ("on", true)] {
        out.push(Part::with_bleed(
            format!("toggle_shuffle_{state}"),
            15.0,
            15.0,
            3.0,
            |s| keycap(s, 7.5, 7.5, 15.0, "", on),
        ));
    }
    // The header on/off lamps: the pattern sections' (13 x 13.6), the mixer's (17 x 14) and
    // the effects' (16 x 16), each in its header's frame color. The effects' lamp is as big as
    // the mixer's (9.6, the user's choice), centered in its larger control.
    for (name, w, h, sz, frame, bleed) in [
        ("power_pat", 13.0, 13.6, 8.6, "#3a0d14", 5.0),
        ("power_mix", 17.0, 14.0, 9.6, "#4f4a1a", 5.5),
        ("power_fx", 16.0, 16.0, 9.6, "#14244f", 7.0),
    ] {
        for (state, on) in [("off", false), ("on", true)] {
            out.push(Part::with_bleed(
                format!("{name}_{state}"),
                w,
                h,
                bleed,
                |s| onoff(s, 0.0, 0.0, w, h, sz, on, frame),
            ));
        }
    }
}

// ---------------------------------------------------------------- the 303

fn note_icon(s: &mut Svg, x: f32, y: f32, scale: f32, ink: &str) {
    s.g(
        &format!("translate({} {}) scale({})", n(x), n(y), n(scale)),
        "",
    );
    s.raw(&format!(
        concat!(
            r#"<ellipse cx="0" cy="0" rx="2.5" ry="1.8" transform="rotate(-22)" fill="{ink}"/>"#,
            r#"<path d="M2.1 -0.6 V-11.6" stroke="{ink}" stroke-width="0.9" fill="none"/>"#,
            r#"<path d="M2.1 -11.6 C3 -8.6 6.2 -8.2 5.2 -4.6 C5.4 -6.8 3.4 -7.6 2.1 -8.2 Z "#,
            r#"M2.1 -8.4 C3 -5.6 6 -5.2 5 -1.8 C5.2 -3.8 3.4 -4.6 2.1 -5.2 Z" fill="{ink}"/>"#
        ),
        ink = ink
    ));
    s.end_g();
}

fn rest_icon(s: &mut Svg, x: f32, y: f32) {
    s.g_at(x, y);
    let round = r#"stroke-linecap="round""#;
    stroke_path(s, "M3.6 -6 L0.2 6", "#151515", 1.0, round);
    let flags = "M3.5 -5.4 C2.4 -3.6 0.6 -3.4 -0.6 -4.2 M2.3 -1.6 C1.2 0.2 -0.6 0.4 -1.8 -0.4";
    stroke_path(s, flags, "#151515", 0.8, round);
    circle(s, -0.6, -4.9, 1.25, "#151515", "");
    circle(s, -1.8, -1.1, 1.25, "#151515", "");
    s.end_g();
}

/// The 303's knob row: the knobs' left edges (y 12).
pub const KNOB_X_303: [f32; 6] = [72.0, 105.0, 138.0, 171.0, 204.0, 237.0];
/// The 303's black keys: (center x, semitone); their pills are 8.4 x 15.4 at (x - 4.2, 62.6),
/// their LEDs at (x, 59).
pub const BLACK_303: [(f32, u8); 5] = [(67.0, 1), (88.0, 3), (130.0, 6), (151.0, 8), (172.0, 10)];
/// The 303's white keys' semitones; their pills are 9 x 16 at (52 + 21 i, 89), their LEDs at
/// (56.5 + 21 i, 85).
pub const WHITE_303: [u8; 8] = [0, 2, 4, 5, 7, 9, 11, 12];

const INK_303: &str = "#1b1c1e";

fn tb303_bg(out: &mut Vec<Part>) {
    out.push(Part::new("bg_tb303", 366.0, 115.0, |s| {
        rect(s, 0.0, 0.0, 366.0, 115.0, 0.0, SEAM, "");
        rect(s, 0.5, 0.5, 365.0, 45.0, 1.0, "url(#g303top)", "");
        stroke_path(s, "M1 45 V1 H365", "#fff", 0.7, r#"opacity="0.9""#);
        stroke_path(s, "M365.2 1.4 V45.2 H1.4", "#000", 0.6, r#"opacity="0.3""#);
        let ink = St::plain(INK_303);
        text(s, 32.65, 8.4, "WAVEFORM", 6.7, ink.ls(0.4), "");
        for d in [
            "M6 29 V20 L13 29 V20 L20 29",
            "M45.3 29 V20 H49.3 V29 H53.3 V20 H57.3 V29 H59.3",
        ] {
            stroke_path(s, d, INK_303, 1.05, r#"stroke-linejoin="miter""#);
        }
        for x in [65.3, 274.7] {
            line(s, x, 6.0, x, 40.0, "#2a2b2d", 0.7, "");
            line(
                s,
                x + 0.6,
                6.0,
                x + 0.6,
                40.0,
                "#fff",
                0.5,
                r#"opacity="0.8""#,
            );
        }
        for (i, name) in ["TUNE", "CUTOFF", "RESO", "ENV.MOD", "DECAY", "ACCENT"]
            .into_iter()
            .enumerate()
        {
            text(s, 87.0 + 33.0 * i as f32, 8.4, name, 6.7, ink.ls(0.3), "");
        }
        text(s, 339.5, 13.2, "EDIT", 6.7, ink.ls(0.4), "");
        text(s, 339.5, 21.2, "STEP", 6.7, ink.ls(0.4), "");
        display_frame(s, 330.0, 25.0, 19.0, 16.0);
        // the key plate: a silver bezel around an inset frame
        rect(s, 0.5, 46.2, 365.0, 68.3, 1.0, "url(#g303bezel)", "");
        stroke_path(s, "M1 114 V46.8 H365", "#fff", 0.7, r#"opacity="0.85""#);
        stroke_path(s, "M365.2 47 V114.3 H1.4", "#000", 0.6, r#"opacity="0.35""#);
        rect(
            s,
            6.6,
            48.4,
            352.2,
            61.6,
            1.2,
            "#f5f5f5",
            r##"stroke="#8e9195" stroke-width="0.5""##,
        );
        rect(s, 8.0, 49.4, 349.4, 59.4, 0.5, "#101010", "");
        let (x0, y0, y1) = (8.6, 50.0, 108.2);
        let white = St::plain("#f2f2f2").ls(0.3);
        rect(s, x0, y0, 36.4, 37.0, 0.0, "#0b0b0b", "");
        text(s, 27.0, 58.4, "PITCH", 6.6, white, "");
        text(s, 27.0, 65.0, "MODE", 6.6, white, "");
        rect(s, x0, 87.4, 36.4, y1 - 87.4, 0.0, "url(#gKeyW)", "");
        text(s, 27.0, 94.2, "CLEAR", 6.6, ink.ls(0.3), "");
        for i in 0..8 {
            let kx = 46.0 + 21.0 * i as f32;
            rect(s, kx + 0.35, y0, 20.3, y1 - y0, 0.0, "url(#gKeyW)", "");
            s.raw(&format!(
                r##"<path d="M{} {} V{} H{} V{}" fill="#000" opacity="0.10"/>"##,
                n(kx + 0.35),
                n(y1 - 2.0),
                n(y1),
                n(kx + 20.65),
                n(y1 - 2.0)
            ));
        }
        for (bx, _) in BLACK_303 {
            s.raw(&format!(
                r#"<path d="M{} {} H{} V{} Q{} {} {} {} H{} Q{} {} {} {} Z" fill="url(#gKeyB)"/>"#,
                n(bx - 7.0),
                n(y0),
                n(bx + 7.0),
                n(79.4),
                n(bx + 7.0),
                n(81.6),
                n(bx + 4.8),
                n(81.6),
                n(bx - 4.8),
                n(bx - 7.0),
                n(81.6),
                n(bx - 7.0),
                n(79.4)
            ));
            s.raw(&format!(
                r##"<path d="M{} {} V{}" stroke="#000" stroke-opacity="0.18" stroke-width="1"/>"##,
                n(bx + 7.5),
                n(y0),
                n(80.0)
            ));
        }
        // the note and rest cells
        rect(s, 215.2, y0, 51.2, 20.6, 0.0, "url(#gKeyW)", "");
        rect(s, 267.2, y0, 49.6, 20.6, 0.0, "url(#gKeyW)", "");
        note_icon(s, 231.0, 66.6, 0.95, "#151515");
        rest_icon(s, 277.0, 61.6);
        for (i, name) in ["DOWN", "UP", "ACCENT", "SLIDE"].into_iter().enumerate() {
            let cx0 = 215.2 + 25.4 * i as f32;
            text(
                s,
                cx0 + 12.7,
                77.8,
                name,
                5.9,
                St::plain("#f2f2f2").ls(0.15),
                "",
            );
            rect(s, cx0, 80.2, 24.8, y1 - 80.2, 0.0, "url(#gKeyW)", "");
        }
        // back and step
        rect(s, 317.6, y0, 39.2, 28.8, 0.0, "url(#gKeyW)", "");
        rect(s, 317.6, 79.6, 39.2, y1 - 79.6, 0.0, "url(#gKeyW)", "");
        text(s, 337.2, 58.8, "BACK", 6.6, ink.ls(0.3), "");
        text(s, 337.2, 87.6, "STEP", 6.6, ink.ls(0.3), "");
    }));
}

fn tb303_controls(out: &mut Vec<Part>) {
    // the waveform switch: a slide between the saw and the square, 16 x 12; 0 saw, 1 square
    for pos in 0..2 {
        out.push(Part::with_bleed(
            format!("switch_wave_{pos}"),
            16.0,
            12.0,
            0.5,
            |s| {
                rect(s, 0.0, 0.0, 16.0, 12.0, 1.0, "#080808", "");
                let tx = if pos == 0 { 0.9 } else { 8.1 };
                rect(
                    s,
                    tx,
                    0.9,
                    7.0,
                    10.2,
                    0.7,
                    "#3a3c3f",
                    r##"stroke="#000" stroke-width="0.3""##,
                );
                for i in 0..3 {
                    let xx = tx + 1.8 + 1.7 * i as f32;
                    line(s, xx, 2.0, xx, 10.0, "#000", 0.5, r#"opacity="0.7""#);
                    line(
                        s,
                        xx + 0.45,
                        2.0,
                        xx + 0.45,
                        10.0,
                        "#fff",
                        0.3,
                        r#"opacity="0.25""#,
                    );
                }
            },
        ));
    }
    // the LEDs: 6 x 6 around the lamp; r 1.8 (pitch mode, black keys) and 1.9 (the others)
    for (name, r) in [("led_303s", 1.8), ("led_303", 1.9)] {
        for (state, on) in [("off", false), ("on", true)] {
            out.push(Part::with_bleed(
                format!("{name}_{state}"),
                6.0,
                6.0,
                2.5,
                |s| led_round(s, 3.0, 3.0, r, "R", on),
            ));
        }
    }
    // the push buttons
    for (name, w, h, r) in [
        ("btn_pill_15x8", 15.0, 8.4, None),
        ("btn_pill_17x10", 17.0, 10.0, None),
        ("btn_step", 25.4, 17.0, Some(1.6)),
        ("key_black", 8.4, 15.4, Some(1.2)),
        ("key_white", 9.0, 16.0, Some(1.2)),
        ("btn_flag", 9.0, 16.0, Some(1.2)),
    ] {
        for (state, pressed) in [("rest", false), ("pressed", true)] {
            out.push(Part::with_bleed(
                format!("{name}_{state}"),
                w,
                h,
                3.0,
                |s| {
                    pill(s, 0.0, 0.0, w, h, pressed, r);
                    if name == "btn_step" {
                        // the grip: a dark and a light line
                        stroke_path(s, "M6.8 1.4 V15.6", "#000", 0.5, r#"stroke-opacity="0.25""#);
                        stroke_path(s, "M18.8 1.4 V15.6", "#fff", 0.5, r#"stroke-opacity="0.7""#);
                    }
                },
            ));
        }
    }
}

// ---------------------------------------------------------------- the 808

const CREAM: &str = "#efe9c6";
/// The 808's instruments, left to right, and the labels of the shared slots 3..7.
pub const NAMES_808: [&str; 12] = [
    "AC", "BD", "SD", "LT", "MT", "HT", "RS", "CP", "CB", "CY", "OH", "CH",
];
pub const UPPER_808: [&str; 5] = ["LC", "MC", "HC", "CL", "MA"];
/// The 808's chips: the legend row's at (11.6 + 22 i, 72.6), 17.8 x 9.2; the upper labels' at
/// (78.6 + 22 k, 44.4), 17.8 x 8.8.
pub const LEGEND_808: (f32, f32, f32, f32) = (11.6, 72.6, 17.8, 9.2);
pub const UPPER_CHIP_808: (f32, f32, f32, f32) = (78.6, 44.4, 17.8, 8.8);
/// The mod's window: the well (x, y, w, h) in the 808 and the picture's square in it.
pub const MOD_VIEW_808: (f32, f32, f32, f32) = (277.2, 4.2, 84.4, 106.0);
pub const MOD_PICTURE: (f32, f32, f32) = (4.2, 3.7, 76.0);

/// An 808 chip of w x h at (x, y) with its label; selected, lighter with an orange ring.
fn chip808(s: &mut Svg, x: f32, y: f32, w: f32, h: f32, label_: &str, sel: bool, shadow: bool) {
    if shadow {
        rect(
            s,
            x + 0.5,
            y + 0.7,
            w,
            h,
            1.6,
            "#000",
            r#"opacity="0.5" filter="url(#b05)""#,
        );
    }
    let fill = if sel { "#fbf6d6" } else { CREAM };
    rect(
        s,
        x,
        y,
        w,
        h,
        1.6,
        fill,
        r##"stroke="#8a8462" stroke-width="0.3""##,
    );
    rect(
        s,
        x + 0.8,
        y + 0.5,
        w - 1.6,
        h * 0.35,
        1.0,
        "#fff",
        r#"opacity="0.45""#,
    );
    if sel {
        let (rx, ry, rw, rh) = (x - 0.9, y - 0.9, w + 1.8, h + 1.8);
        rect(
            s,
            rx,
            ry,
            rw,
            rh,
            2.3,
            "none",
            r##"stroke="#ff8a1e" stroke-width="1.1""##,
        );
        rect(
            s,
            rx,
            ry,
            rw,
            rh,
            2.3,
            "none",
            r##"stroke="#ff8a1e" stroke-width="1.6" opacity="0.5" filter="url(#b05)""##,
        );
    }
    let st = St::plain("#25231a").ls(0.25);
    text(s, x + w / 2.0, y + h / 2.0 + 2.45, label_, 6.8, st, "");
}

/// The 808's knobs' column lines and labels color.
const LAB_808: &str = "#d8d3b6";

fn tr808_bg(out: &mut Vec<Part>) {
    out.push(Part::new("bg_tr808", 366.0, 115.0, |s| {
        rect(s, 0.0, 0.0, 366.0, 115.0, 0.0, SEAM, "");
        rect(s, 0.5, 0.5, 365.0, 114.0, 1.0, "url(#g808)", "");
        grain(s, (0.0, 0.0, 366.0, 115.0), 90.0, 280.0, &GRAIN_808);
        stroke_path(s, "M1 114 V1 H365", "#fff", 0.6, r#"opacity="0.16""#);
        stroke_path(s, "M365.2 1.4 V114.2 H1.4", "#000", 0.6, r#"opacity="0.5""#);
        for i in 0..13 {
            let x = 9.5 + 22.0 * i as f32;
            line(s, x, 4.5, x, 70.0, "#b9b49a", 0.45, r#"opacity="0.55""#);
            line(
                s,
                x + 0.45,
                4.5,
                x + 0.45,
                70.0,
                "#000",
                0.4,
                r#"opacity="0.5""#,
            );
        }
        let lab = St::plain(LAB_808).w(500).ls(0.1);
        for i in 0..12 {
            text(s, 20.5 + 22.0 * i as f32, 7.9, "LEVEL", 5.3, lab, "");
        }
        let row2 = [
            (1, "TONE"),
            (2, "TUNE"),
            (3, "TUNE"),
            (4, "TUNE"),
            (5, "TUNE"),
            (9, "TONE"),
        ];
        for (i, name) in row2 {
            text(s, 20.5 + 22.0 * i as f32, 29.6, name, 5.3, lab, "");
        }
        for (i, name) in [(1, "DECAY"), (2, "SNAPPY"), (9, "DECAY"), (10, "DECAY")] {
            let (size, st) = if name == "SNAPPY" {
                (5.0, lab.ls(0.0))
            } else {
                (5.3, lab)
            };
            text(s, 20.5 + 22.0 * i as f32, 52.3, name, size, st, "");
        }
        // (the upper labels' chips lie over the tone knobs' shadows: `fg_tr808`)
        let (lx, ly, lw, lh) = LEGEND_808;
        for (i, name) in NAMES_808.into_iter().enumerate() {
            chip808(s, lx + 22.0 * i as f32, ly, lw, lh, name, false, true);
        }
    }));
}

/// The foregrounds: static art drawn over the controls, where a control's shadow or handle
/// would otherwise lie over it (part, its rectangle in the section).
pub const FG_TR808: (f32, f32, f32, f32) = (78.0, 44.0, 107.0, 10.0);
pub const FG_PCF: (f32, f32, f32, f32) = (4.0, 103.0, 76.0, 7.0);

fn foregrounds(out: &mut Vec<Part>) {
    // the 808's upper labels, over the tone knobs' cast shadows
    let (fx, fy, fw, fh) = FG_TR808;
    out.push(Part::with_bleed("fg_tr808", fw, fh, 2.0, |s| {
        let (ux, uy, uw, uh) = UPPER_CHIP_808;
        for (k, name) in UPPER_808.into_iter().enumerate() {
            chip808(
                s,
                ux + 22.0 * k as f32 - fx,
                uy - fy,
                uw,
                uh,
                name,
                false,
                true,
            );
        }
    }));
    // the PCF's fader captions, over the handles at the bottom of their travel
    let (fx, fy, fw, fh) = FG_PCF;
    out.push(Part::with_bleed("fg_pcf", fw, fh, 1.0, |s| {
        for (i, name) in ["FREQ", "Q", "AMT", "DEC"].into_iter().enumerate() {
            let x = 11.5 + 19.0 * i as f32 - fx;
            stext(s, x, 108.4 - fy, name, 5.8, St::label().ls(0.1));
        }
    }));
}

fn tr808_controls(out: &mut Vec<Part>) {
    // the shared slots' switches, 11 x 17: 0 the upper label, 1 the legend's
    for pos in 0..2 {
        out.push(Part::with_bleed(
            format!("switch_808_{pos}"),
            11.0,
            17.0,
            0.5,
            |s| slide_v(s, 0.0, 0.0, 11.0, 17.0, pos),
        ));
    }
    // the step keys, 12.6 x 19, with their LED at the top
    for (color, c) in [
        ("red", "R"),
        ("orange", "O"),
        ("yellow", "Y"),
        ("white", "W"),
    ] {
        for (state, on) in [("off", false), ("on", true)] {
            out.push(Part::with_bleed(
                format!("step808_{color}_{state}"),
                12.6,
                19.0,
                3.0,
                |s| {
                    rect(
                        s,
                        0.8,
                        1.0,
                        12.6,
                        18.6,
                        1.2,
                        "#000",
                        r#"opacity="0.6" filter="url(#b05)""#,
                    );
                    rect(
                        s,
                        0.0,
                        0.0,
                        12.6,
                        19.0,
                        1.0,
                        &format!("url(#g808k{c})"),
                        r##"stroke="#0b0b0a" stroke-width="0.45""##,
                    );
                    rect(s, 0.7, 0.6, 11.2, 0.6, 0.3, "#fff", r#"opacity="0.6""#);
                    led_round(s, 6.3, 3.2, 1.15, "R", on);
                },
            ));
        }
    }
    // the selected instrument: its chip, lighter, with the orange ring (drawn over the chip)
    let (_, _, lw, lh) = LEGEND_808;
    for name in NAMES_808 {
        out.push(Part::with_bleed(
            format!("sel808_{name}"),
            lw,
            lh,
            2.5,
            |s| chip808(s, 0.0, 0.0, lw, lh, name, true, false),
        ));
    }
    let (_, _, uw, uh) = UPPER_CHIP_808;
    for name in UPPER_808 {
        out.push(Part::with_bleed(
            format!("sel808_{name}"),
            uw,
            uh,
            2.5,
            |s| chip808(s, 0.0, 0.0, uw, uh, name, true, false),
        ));
    }
    // the mod's window: a dark well for the kit's splash and its name (the app paints both),
    // and the glass laid over the picture
    let (_, _, w, h) = MOD_VIEW_808;
    out.push(Part::with_bleed("mod_view", w, h, 1.0, |s| {
        rect(s, 0.4, 0.4, w, h, 2.2, "#000", r#"opacity="0.55""#);
        rect(
            s,
            0.0,
            0.0,
            w,
            h,
            2.2,
            "#16150f",
            r##"stroke="#000" stroke-width="0.5""##,
        );
        let hi = r#"stroke-opacity="0.14""#;
        stroke_path(s, "M0.8 105.4 H83.8 V0.8", "#fff", 0.5, hi);
        rect(s, 3.7, 3.2, 77.0, 77.0, 0.8, "#000", "");
        line(s, 4.8, 85.2, 79.8, 85.2, "#fff", 0.4, r#"opacity="0.12""#);
    }));
    let (_, _, side) = MOD_PICTURE;
    out.push(Part::new("mod_view_glass", side, side, |s| {
        rect(s, 0.0, 0.0, side, side, 0.0, "url(#gScreen)", "");
    }));
}

// ---------------------------------------------------------------- the 909

pub const NAMES_909: [&str; 12] = [
    "AC", "BD", "SD", "LT", "MT", "HT", "RS", "CP", "CH", "OH", "CC", "RC",
];
const INK_909: &str = "#3a3a37";

/// The 909's instrument chips at the top: (x, w, label x) of each instrument's label, y 4.6,
/// h 9.2; RS..RC share a chip two by two.
pub fn chip_909(i: usize) -> (f32, f32, f32) {
    match i {
        0 => (6.4, 18.4, 15.6),
        1..=5 => {
            let x = 26.8 + 42.0 * (i - 1) as f32;
            (x, 39.6, x + 19.8)
        }
        _ => {
            let x = 236.8 + 42.0 * ((i - 6) / 2) as f32;
            if i.is_multiple_of(2) {
                (x, 19.8, x + 10.6)
            } else {
                (x + 19.8, 19.8, x + 29.4)
            }
        }
    }
}

/// The selection LED of each 909 instrument: the AC box's (r 1.1) and the name labels'
/// (r 1.05), their centers.
pub fn led_909(i: usize) -> (f32, f32) {
    match i {
        0 => (10.0, 53.8),
        1..=5 => (28.4 + 42.0 * (i - 1) as f32 + 3.6, 78.3),
        _ => {
            let x = 238.4 + 42.0 * ((i - 6) / 2) as f32;
            (
                if i.is_multiple_of(2) {
                    x + 3.6
                } else {
                    x + 32.8
                },
                78.3,
            )
        }
    }
}

/// The 909's knobs: (x, y of the left top, id, label). Labels are centered over them.
pub const KNOBS_909: [(f32, f32, &str, &str); 28] = [
    (9.0, 28.0, "ac_level", ""),
    (30.0, 28.0, "bd_tune", "TUNE"),
    (50.0, 28.0, "bd_level", "LEVEL"),
    (72.0, 28.0, "sd_tune", "TUNE"),
    (92.0, 28.0, "sd_level", "LEVEL"),
    (114.0, 28.0, "lt_tune", "TUNE"),
    (134.0, 28.0, "lt_level", "LEVEL"),
    (156.0, 28.0, "mt_tune", "TUNE"),
    (176.0, 28.0, "mt_level", "LEVEL"),
    (198.0, 28.0, "ht_tune", "TUNE"),
    (218.0, 28.0, "ht_level", "LEVEL"),
    (240.0, 28.0, "rs_level", "LEVEL"),
    (260.0, 28.0, "cp_level", "LEVEL"),
    (302.0, 28.0, "hh_level", "LEVEL"),
    (324.0, 28.0, "cc_level", "LEVEL"),
    (344.0, 28.0, "rc_level", "LEVEL"),
    (30.0, 54.0, "bd_attack", "ATT"),
    (50.0, 54.0, "bd_decay", "DEC"),
    (72.0, 54.0, "sd_tone", "TONE"),
    (92.0, 54.0, "sd_snappy", "SNAP"),
    (114.0, 54.0, "lt_decay", "DEC"),
    (156.0, 54.0, "mt_decay", "DEC"),
    (198.0, 54.0, "ht_decay", "DEC"),
    (282.0, 54.0, "ch_decay", "DEC"),
    (302.0, 54.0, "oh_decay", "DEC"),
    (324.0, 54.0, "cc_tune", "TUNE"),
    (344.0, 54.0, "rc_tune", "TUNE"),
    (9.0, 76.0, "flam", ""),
];

fn tr909_bg(out: &mut Vec<Part>) {
    out.push(Part::new("bg_tr909", 366.0, 115.0, |s| {
        rect(s, 0.0, 0.0, 366.0, 115.0, 0.0, SEAM, "");
        rect(s, 0.5, 0.5, 365.0, 114.0, 1.4, "url(#g909)", "");
        grain(s, (0.0, 0.0, 366.0, 115.0), 90.0, 395.0, &GRAIN_909);
        stroke_path(s, "M1 114 V1 H365", "#fff", 0.8, r#"opacity="0.95""#);
        stroke_path(s, "M365.2 1.4 V114.2 H1.4", "#000", 0.7, r#"opacity="0.4""#);
        let frame = format!(r#"{} opacity="0.7""#, sk("#8f8f8a", 0.4));
        rect(s, 2.4, 2.4, 361.2, 110.2, 1.0, "none", &frame);
        // the instrument chips, unlit (the app lays the selected one's lit label over it)
        let chip = |s: &mut Svg, x: f32, w: f32, labels: &[(f32, &str)]| {
            rect(s, x, 4.6, w, 9.2, 0.6, "url(#g909hdr)", &sk("#1e1e1e", 0.3));
            rect(
                s,
                x + 0.4,
                4.9,
                w - 0.8,
                0.5,
                0.0,
                "#fff",
                r#"opacity="0.25""#,
            );
            for &(lx, lab) in labels {
                text(s, lx, 12.3, lab, 8.2, St::plain("#f0732a").ls(0.5), "");
            }
        };
        chip(s, 6.4, 18.4, &[(15.6, "AC")]);
        for k in 0..5 {
            chip(
                s,
                26.8 + 42.0 * k as f32,
                39.6,
                &[(46.6 + 42.0 * k as f32, NAMES_909[k + 1])],
            );
        }
        for k in 0..3 {
            let x = 236.8 + 42.0 * k as f32;
            chip(
                s,
                x,
                39.6,
                &[
                    (x + 10.6, NAMES_909[6 + 2 * k]),
                    (x + 29.4, NAMES_909[7 + 2 * k]),
                ],
            );
        }
        let mut xs = vec![4.6, 25.8];
        xs.extend((0..8).map(|k| 67.8 + 42.0 * k as f32));
        for x in xs {
            let y1 = if x > 26.0 { 67.0 } else { 61.0 };
            line(s, x, 15.5, x, y1, "#777772", 0.5, "");
            line(
                s,
                x + 0.5,
                15.5,
                x + 0.5,
                y1,
                "#fff",
                0.4,
                r#"opacity="0.9""#,
            );
        }
        let lab = St::plain(INK_909).ls(0.1);
        for (x, y, _, name) in KNOBS_909 {
            if !name.is_empty() {
                let ly = if y < 50.0 { 24.2 } else { 50.4 };
                text(s, x + 6.0, ly, name, 5.2, lab, "");
            }
        }
        // the AC box, its selection LED unlit
        rect(
            s,
            6.2,
            49.6,
            17.6,
            8.4,
            1.0,
            "none",
            r##"stroke="#55554f" stroke-width="0.5""##,
        );
        led_round(s, 10.0, 53.8, 1.1, "R", false);
        text(s, 17.0, 55.7, "AC", 5.0, lab, "");
        // the flam box
        rect(
            s,
            4.6,
            62.6,
            20.6,
            50.2,
            1.0,
            "none",
            r##"stroke="#77776f" stroke-width="0.5""##,
        );
        text(s, 15.0, 70.6, "FLAM", 5.2, lab, "");
        // the name labels, their selection LEDs unlit
        let names = ["BASS DRUM", "SNARE DRUM", "LOW TOM", "MID TOM", "HI TOM"];
        let cell = r##"stroke="#4c4c47" stroke-width="0.5""##;
        for (k, full) in names.into_iter().enumerate() {
            let x = 28.4 + 42.0 * k as f32;
            rect(s, x, 74.4, 36.4, 7.8, 1.2, "#f6f5f1", cell);
            led_round(s, x + 3.6, 78.3, 1.05, "R", false);
            text(s, x + 20.6, 79.8, full, 3.9, St::plain(INK_909), "");
        }
        for k in 0..3 {
            let x = 238.4 + 42.0 * k as f32;
            rect(s, x, 74.4, 36.4, 7.8, 1.2, "#f6f5f1", cell);
            led_round(s, x + 3.6, 78.3, 1.05, "R", false);
            led_round(s, x + 32.8, 78.3, 1.05, "R", false);
            text(s, x + 11.6, 80.0, NAMES_909[6 + 2 * k], 4.6, lab, "");
            text(s, x + 24.8, 80.0, NAMES_909[7 + 2 * k], 4.6, lab, "");
        }
        // the dark strip under the step keys with their numbers
        rect(s, 26.6, 103.6, 337.0, 7.6, 0.6, "url(#g909hdr)", "");
        for i in 0..16 {
            let x = 29.0 + 21.0 * i as f32;
            let st = St::plain("#f1f1ee");
            text(s, x + 8.2, 109.25, &(i + 1).to_string(), 5.2, st, "");
        }
    }));
}

fn tr909_controls(out: &mut Vec<Part>) {
    // the step keys, 16.4 x 16.8: a silver key with a light bar, unlit or orange, red or green
    // for the values 1, 2 and 3
    for (v, col) in [
        (0, None),
        (1, Some(("#ff9a3a", "O"))),
        (2, Some(("#ff3b2c", "R"))),
        (3, Some(("#49e03c", "G"))),
    ] {
        out.push(Part::with_bleed(
            format!("step909_{v}"),
            16.4,
            16.8,
            3.0,
            |s| {
                rect(
                    s,
                    0.9,
                    1.2,
                    16.2,
                    16.6,
                    1.8,
                    "#000",
                    r#"opacity="0.5" filter="url(#b05)""#,
                );
                rect(s, 0.0, 0.0, 16.4, 16.8, 1.7, "#2c2c2a", "");
                rect(s, 1.0, 1.0, 14.4, 14.8, 1.2, "url(#gSilver)", "");
                rect(s, 2.2, 5.2, 12.0, 9.0, 1.4, "#fff", r#"opacity="0.55""#);
                if let Some((_, h)) = col {
                    s.raw(&format!(
                        r#"<ellipse cx="8.2" cy="3.8" rx="7" ry="4" fill="url(#halo{h})"/>"#
                    ));
                }
                let fill = col.map_or("#6a1d16", |c| c.0);
                rect(
                    s,
                    4.6,
                    2.4,
                    7.2,
                    2.8,
                    0.7,
                    fill,
                    r##"stroke="#2a0a07" stroke-width="0.3""##,
                );
            },
        ));
    }
    // the flam button, 14.8 x 14.8; lit, red with a glow
    for (state, on) in [("off", false), ("on", true)] {
        out.push(Part::with_bleed(
            format!("toggle_flam_{state}"),
            14.8,
            14.8,
            4.0,
            |s| {
                if on {
                    circle(s, 7.4, 7.4, 11.0, "url(#haloR)", "");
                }
                rect(s, 0.0, 0.0, 14.8, 14.8, 1.2, "#3b3b39", "");
                rect(
                    s,
                    1.0,
                    1.0,
                    12.8,
                    12.8,
                    0.8,
                    if on { "url(#gCapLit)" } else { "url(#gSilver)" },
                    "",
                );
                if on {
                    rect(
                        s,
                        3.8,
                        3.8,
                        7.2,
                        7.2,
                        1.0,
                        "#ff3a2a",
                        r#"opacity="0.8" filter="url(#b05)""#,
                    );
                    rect(
                        s,
                        4.6,
                        4.6,
                        5.6,
                        5.6,
                        0.6,
                        "url(#ledR)",
                        r##"stroke="#b3140c" stroke-width="0.3""##,
                    );
                    rect(s, 5.3, 5.2, 4.2, 1.6, 0.4, "#fff", r#"opacity="0.55""#);
                } else {
                    rect(
                        s,
                        4.6,
                        4.6,
                        5.6,
                        5.6,
                        0.6,
                        "#b9bab6",
                        r##"stroke="#8a8b87" stroke-width="0.3""##,
                    );
                }
            },
        ));
    }
    // the selected instrument: its chip's label lit (a patch of the chip under it) and its LED
    for (i, name) in NAMES_909.into_iter().enumerate() {
        let (x, w, lx) = chip_909(i);
        out.push(Part::with_bleed(
            format!("sel909_{name}"),
            w,
            9.2,
            3.0,
            |s| {
                rect(s, 0.15, 0.15, w - 0.3, 8.9, 0.45, "url(#g909hdr)", "");
                rect(s, 0.4, 0.3, w - 0.8, 0.5, 0.0, "#fff", r#"opacity="0.25""#);
                let st = St::plain("#ff9a3c").ls(0.5);
                text(
                    s,
                    lx - x,
                    7.7,
                    name,
                    8.2,
                    st,
                    r#"filter="url(#b1)" opacity="0.9""#,
                );
                text(s, lx - x, 7.7, name, 8.2, st.fill("#ffb061"), "");
            },
        ));
    }
    for (name, r) in [("led909_on", 1.05), ("led909ac_on", 1.1)] {
        out.push(Part::with_bleed(name, 5.0, 5.0, 2.0, |s| {
            led_round(s, 2.5, 2.5, r, "R", true)
        }));
    }
}

// ---------------------------------------------------------------- the mixer and the master

fn mixer_bg(out: &mut Vec<Part>) {
    for c in 0..4 {
        let oy = 50.0 + 115.0 * c as f32;
        out.push(Part::new(format!("bg_mixer_{}", c + 1), 70.0, 115.0, |s| {
            gray_plate(s, 70.0, 115.0, 456.0, oy);
            header(s, 3.0, 4.0, 64.0, 14.0, "gHdrMix", "MIX", 35.0, 9.2, 1.2);
            knob_scale(s, 10.0, 25.0, "L", "R", "PAN");
            for (i, name) in ["DIST", "PCF", "COMP"].into_iter().enumerate() {
                let ls = if name == "PCF" { 0.5 } else { 0.2 };
                stext(
                    s,
                    44.3,
                    36.0 + 18.0 * i as f32,
                    name,
                    6.2,
                    St::label().start().ls(ls),
                );
            }
            knob_scale(s, 42.0, 79.0, "0", "10", "DELAY");
        }));
    }
    // the routing keys (DIST, PCF, COMP, the master's COMP), 24 x 11: a key that shows pressed
    // only while the mouse is down on it (the user's choice; it never stays down) and a yellow
    // lamp at its right that shows the state
    for (state, on) in [("off", false), ("on", true)] {
        for (suffix, pressed) in [("", false), ("_pressed", true)] {
            out.push(Part::with_bleed(
                format!("mixsw_{state}{suffix}"),
                24.0,
                11.0,
                3.0,
                |s| {
                    pill(s, 0.5, 1.2, 15.5, 8.6, pressed, Some(1.6));
                    led_round(s, 20.3, 5.6, 1.9, "Y", on);
                },
            ));
        }
    }
    // the header meters (mixer and effects), 14 x 14, 0..4 segments lit
    for level in 0..=4 {
        out.push(Part::with_bleed(
            format!("meter_14x14_{level}"),
            14.0,
            14.0,
            1.5,
            |s| meter4(s, 0.0, 0.0, 14.0, 14.0, level),
        ));
    }
}

fn master_bg(out: &mut Vec<Part>) {
    out.push(Part::new("bg_master", 82.0, 98.0, |s| {
        gray_plate(s, 82.0, 98.0, 526.0, 50.0);
        header(s, 4.0, 4.0, 74.0, 14.0, "gHdrMix", "MASTER", 41.0, 9.0, 0.6);
        for x in [16.0, 51.0] {
            rect(s, x + 4.2, 26.4, 6.6, 51.2, 0.8, "#0d0f10", "");
        }
        for (i, t) in ["Clip", "-6", "-12", "-24", "-36"].into_iter().enumerate() {
            let y = 31.4 + 10.0 * i as f32;
            stext(s, 14.4, y, t, 5.6, St::label().end().ls(0.0));
            stext(s, 67.6, y, t, 5.6, St::label().start().ls(0.0));
        }
        stext(s, 39.6, 88.7, "COMP", 6.2, St::label().end());
    }));
    // the master's two meters: 10 segments in a 15 x 56 column, 11 states
    for level in 0..=10 {
        out.push(Part::with_bleed(
            format!("meter_15x56_{level}"),
            15.0,
            56.0,
            1.5,
            |s| {
                for i in 0..10 {
                    let on = 9 - i < level;
                    let (lit, dark) = match i {
                        0..=1 => ("#ff3b2c", "#4a1310"),
                        2..=3 => ("#ffc42e", "#4a3a10"),
                        _ => ("#43e03a", "#143f1a"),
                    };
                    let y = 3.6 + 5.0 * i as f32;
                    if on {
                        rect(
                            s,
                            3.6,
                            y - 0.7,
                            7.8,
                            4.4,
                            1.0,
                            lit,
                            r#"opacity="0.45" filter="url(#b05)""#,
                        );
                    }
                    rect(s, 5.2, y, 4.6, 2.8, 0.4, if on { lit } else { dark }, "");
                }
            },
        ));
    }
}

// ---------------------------------------------------------------- the effects

fn fx_header(s: &mut Svg, title: &str, y: f32, ls: f32) {
    header(s, 3.0, y, 76.0, 15.0, "gHdrFx", title, 41.0, 9.2, ls);
}

fn fx_bg(out: &mut Vec<Part>) {
    out.push(Part::new("bg_pcf", 82.0, 111.0, |s| {
        gray_plate(s, 82.0, 111.0, 526.0, 148.0);
        fx_header(s, "PCF", 2.5, 3.2);
        display_frame(s, 10.0, 23.0, 18.0, 16.0);
        arrows_frame(s, 29.0, 22.0);
        let l = St::label();
        stext(s, 61.0, 28.2, "BP", 5.8, l.start());
        stext(s, 61.0, 37.6, "LP", 5.8, l.start());
        stext(s, 24.5, 46.8, "PATTERN", 6.0, l);
        stext(s, 63.0, 46.8, "MODE", 6.0, l);
        // (the fader captions are `fg_pcf`)
    }));
    out.push(Part::new("bg_delay", 82.0, 93.0, |s| {
        gray_plate(s, 82.0, 93.0, 526.0, 259.0);
        fx_header(s, "DELAY", 3.0, 0.4);
        display_frame(s, 9.0, 23.0, 19.0, 16.0);
        arrows_frame(s, 29.0, 22.0);
        // the triplet sign and a note
        s.raw(concat!(
            r##"<path d="M60.6 26.6 Q61.4 23.6 64.2 23.4 M68.6 23.4 Q71.4 23.6 72.2 26.6" "##,
            r##"fill="none" stroke="#edf1f3" stroke-width="0.6" stroke-linecap="round"/>"##
        ));
        stext(s, 66.4, 26.4, "3", 5.6, St::label().ls(0.0));
        note_icon(s, 66.0, 39.2, 0.62, "#edf1f3");
        stext(s, 18.5, 46.6, "STEPS", 6.0, St::label());
        knob_scale(s, 12.0, 58.0, "L", "R", "PAN");
        knob_scale(s, 53.0, 58.0, "0", "10", "F.BACK");
    }));
    out.push(Part::new("bg_dist", 82.0, 65.0, |s| {
        gray_plate(s, 82.0, 65.0, 526.0, 352.0);
        fx_header(s, "DIST", 3.0, 0.6);
        knob_scale(s, 12.0, 28.0, "0", "10", "AMOUNT");
        knob_scale(s, 53.0, 28.0, "0", "10", "SHAPE");
    }));
    out.push(Part::new("bg_comp", 82.0, 93.0, |s| {
        gray_plate(s, 82.0, 93.0, 526.0, 417.0);
        fx_header(s, "COMP", 3.0, 0.8);
        for (i, t) in ["-40", "-20", "0", "+20", "+40"].into_iter().enumerate() {
            stext(s, 11.5 + 14.0 * i as f32, 41.4, t, 5.2, St::label().ls(0.0));
        }
        knob_scale(s, 12.0, 57.0, "0", "10", "RATIO");
        knob_scale(s, 53.0, 57.0, "0", "10", "THRES");
    }));
    // the PCF's BP / LP and the delay's triplet / sixteenth switch, 9 x 18
    for pos in 0..2 {
        out.push(Part::with_bleed(
            format!("switch_fx_{pos}"),
            9.0,
            18.0,
            0.5,
            |s| slide_v(s, 0.0, 0.0, 9.0, 18.0, pos),
        ));
    }
    // the compressor's nine gain-reduction LEDs, 7 x 7 around the lamp: lit in the color the
    // app gives, unlit in their place's color (`GR_OFF`)
    for (name, c, on) in [
        ("gr_led_green", "G", true),
        ("gr_led_amber", "Y", true),
        ("gr_led_red", "R", true),
        ("gr_led_off_g", "G", false),
        ("gr_led_off_y", "Y", false),
        ("gr_led_off_r", "R", false),
    ] {
        out.push(Part::with_bleed(name, 7.0, 7.0, 2.0, |s| {
            led_round(s, 3.5, 3.5, 1.9, c, on)
        }));
    }
}

/// The unlit color of each gain-reduction LED, left to right: the center (0) green, the two
/// beside it amber and the outer ones red, as the app lights them (docs/ui_spec.md section 16).
pub const GR_OFF: [&str; 9] = ["r", "r", "y", "y", "g", "y", "y", "r", "r"];

// ---------------------------------------------------------------- seven-segment digits

const SEG: [(char, &str); 11] = [
    ('0', "abcdef"),
    ('1', "bc"),
    ('2', "abged"),
    ('3', "abgcd"),
    ('4', "fgbc"),
    ('5', "afgcd"),
    ('6', "afgedc"),
    ('7', "abc"),
    ('8', "abcdefg"),
    ('9', "abcdfg"),
    (' ', ""),
];

/// The seven segments of a 9 x 16 digit cell at (x, y): (name, path).
fn seg_paths(x: f32, y: f32) -> [(char, String); 7] {
    let (l, r, t, m, b, k) = (x + 1.7, x + 7.3, y + 2.0, y + 8.0, y + 14.0, 0.85);
    let hz = |yy: f32| {
        format!(
            "M{} {} L{} {} H{} L{} {} L{} {} H{} Z",
            n(l + 0.5),
            n(yy),
            n(l + 1.3),
            n(yy - k),
            n(r - 1.3),
            n(r - 0.5),
            n(yy),
            n(r - 1.3),
            n(yy + k),
            n(l + 1.3)
        )
    };
    let vt = |xx: f32, y0: f32, y1: f32| {
        format!(
            "M{} {} L{} {} V{} L{} {} L{} {} V{} Z",
            n(xx),
            n(y0 + 0.5),
            n(xx + k),
            n(y0 + 1.3),
            n(y1 - 1.3),
            n(xx),
            n(y1 - 0.5),
            n(xx - k),
            n(y1 - 1.3),
            n(y0 + 1.3)
        )
    };
    [
        ('a', hz(t)),
        ('g', hz(m)),
        ('d', hz(b)),
        ('f', vt(l, t, m)),
        ('b', vt(r, t, m)),
        ('e', vt(l, m, b)),
        ('c', vt(r, m, b)),
    ]
}

/// The 11 frames of a seven-segment digit, 9 x 16: 0..9 and blank. The unlit segments stay
/// faintly visible (a blank is the disabled display); the lit ones glow. The cell is skewed 5
/// degrees about its middle row and carries the glass's sheen.
fn digits(out: &mut Vec<Part>) {
    for (c, lit) in SEG {
        let name = if c == ' ' {
            "seg_blank".to_string()
        } else {
            format!("seg_{c}")
        };
        out.push(Part::with_bleed(name, 9.0, 16.0, 2.0, |s| {
            let (mut off, mut on) = (Vec::new(), Vec::new());
            for (k, d) in seg_paths(0.0, 0.0) {
                if lit.contains(k) {
                    on.push(d);
                } else {
                    off.push(d);
                }
            }
            s.g("translate(4.5 8) skewX(-5) translate(-4.5 -8)", "");
            s.raw(&format!(
                r##"<path d="{}" fill="#4a0f0c"/>"##,
                off.join(" ")
            ));
            if !on.is_empty() {
                let d = on.join(" ");
                s.raw(&format!(
                    r##"<path d="{d}" fill="#ff3020" filter="url(#b05)" opacity="0.7"/>"##
                ));
                s.raw(&format!(r##"<path d="{d}" fill="#ff5545"/>"##));
            }
            s.end_g();
            rect(s, 0.0, 0.0, 9.0, 7.2, 0.0, "#fff", r#"opacity="0.05""#);
        }));
    }
}

/// The four colors of the 808's steps, which the app icon reuses (task-115).
const STEP_COLORS: [&str; 4] = [pal::RED, pal::ORANGE, pal::YELLOW, pal::WHITE];

/// The app icon (task-115, unchanged by task-179): the 303's saw running into a square wave in
/// orange over four step keys in the 808's colors, on a slate tile; 64 x 64 with the tile
/// inset like a macOS icon's.
fn app_icon(out: &mut Vec<Part>) {
    out.push(Part::new("app_icon", 64.0, 64.0, |s| {
        s.rect(6.0, 6.0, 52.0, 52.0, 12.0, pal::SLATE_DARK);
        s.rect(6.0, 6.0, 52.0, 26.0, 12.0, "#ffffff0c");
        s.rect_stroke(6.5, 6.5, 51.0, 51.0, 11.5, "#ffffff1a", 1.0);
        s.path_stroke("M13 33 L24 17 V33 H30 V17 H41 V33 H51", "#00000050", 5.0);
        s.path_stroke("M13 32 L24 16 V32 H30 V16 H41 V32 H51", pal::ORANGE, 4.0);
        for (i, color) in STEP_COLORS.into_iter().enumerate() {
            let x = 12.0 + 10.75 * i as f32;
            s.rect(x, 39.0, 8.5, 9.0, 2.0, "#00000050");
            s.rect(x, 38.0, 8.5, 9.0, 2.0, color);
        }
    }));
}

pub fn all_parts() -> Vec<Part> {
    let mut v = Vec::new();
    wood(&mut v);
    transport_bg(&mut v);
    logo(&mut v);
    pattern_bg(&mut v);
    tb303_bg(&mut v);
    tr808_bg(&mut v);
    tr909_bg(&mut v);
    mixer_bg(&mut v);
    master_bg(&mut v);
    fx_bg(&mut v);
    knobs(&mut v);
    fader(&mut v);
    transport_buttons(&mut v);
    tb303_controls(&mut v);
    tr808_controls(&mut v);
    tr909_controls(&mut v);
    foregrounds(&mut v);
    digits(&mut v);
    about_plate(&mut v);
    desk_tile(&mut v);
    app_icon(&mut v);
    v
}
