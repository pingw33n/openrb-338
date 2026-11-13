//! A small SVG builder, the shared definitions (gradients and filters) of the panel's art, and
//! the part type. All coordinates are panel pixels of docs/ui_spec.md.
//!
//! The look is the redesign of task-179 (the mockup approved on 2026-11-13): every gradient
//! and filter of the mockup is in [`defs`] under the mockup's id, and a part's SVG carries only
//! the definitions its body refers to (`url(#id)`), so each part file is self-contained.

use std::f32::consts::PI;
use std::fmt::Write;

/// The colors of the app icon and the contact sheet (the flat look of task-042; the icon is
/// out of task-179's scope and keeps them).
pub mod pal {
    pub const SLATE_DARK: &str = "#2c3542";
    pub const RED: &str = "#e2413b";
    pub const ORANGE: &str = "#f08a24";
    pub const YELLOW: &str = "#f2c531";
    pub const WHITE: &str = "#fbfbf8";
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
    /// The prefix of the ids this body defines itself (clip paths): the part's name, so that
    /// the ids stay unique when `layout::compose` puts every part into one document.
    pub prefix: String,
    ids: u32,
}

impl Svg {
    pub fn new() -> Self {
        Self::default()
    }

    /// A new id for a definition of this body.
    pub fn uid(&mut self, what: &str) -> String {
        self.ids += 1;
        format!("{}-{what}{}", self.prefix, self.ids)
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
            "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"{r}\" fill=\"none\" \
             stroke=\"{stroke}\" stroke-width=\"{sw}\"/>"
        );
        self
    }

    pub fn path_stroke(&mut self, d: &str, stroke: &str, sw: f32) -> &mut Self {
        let sw = n(sw);
        let _ = write!(
            self.s,
            "<path d=\"{d}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"{sw}\" \
             stroke-linecap=\"round\" stroke-linejoin=\"round\"/>"
        );
        self
    }

    /// A rectangle with extra attributes (the mockup's `rect`).
    #[allow(clippy::too_many_arguments)]
    pub fn rect_e(&mut self, x: f32, y: f32, w: f32, h: f32, rx: f32, fill: &str, extra: &str) {
        let _ = write!(
            self.s,
            r#"<rect x="{}" y="{}" width="{}" height="{}" rx="{}" fill="{fill}" {extra}/>"#,
            n(x),
            n(y),
            n(w),
            n(h),
            n(rx)
        );
    }

    /// A line with extra attributes (the mockup's `line`).
    #[allow(clippy::too_many_arguments)]
    pub fn line_e(
        &mut self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        stroke: &str,
        w: f32,
        extra: &str,
    ) {
        let _ = write!(
            self.s,
            concat!(
                r#"<line x1="{}" y1="{}" x2="{}" y2="{}" "#,
                r#"stroke="{}" stroke-width="{}" {}/>"#
            ),
            n(x1),
            n(y1),
            n(x2),
            n(y2),
            stroke,
            n(w),
            extra
        );
    }

    /// A circle with extra attributes (the mockup's `circle`).
    pub fn circle_e(&mut self, cx: f32, cy: f32, r: f32, fill: &str, extra: &str) {
        let _ = write!(
            self.s,
            r#"<circle cx="{}" cy="{}" r="{}" fill="{fill}" {extra}/>"#,
            n(cx),
            n(cy),
            n(r)
        );
    }

    /// Opens a group with an optional transform and extra attributes; close it with `end_g`.
    pub fn g(&mut self, transform: &str, extra: &str) {
        if transform.is_empty() {
            let _ = write!(self.s, "<g {extra}>");
        } else {
            let _ = write!(self.s, r#"<g transform="{transform}" {extra}>"#);
        }
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

/// A linear gradient: stops of (offset, color) or (offset, color, opacity).
fn lg(id: &str, stops: &[(f32, &str, f32)], x: [f32; 4]) -> (String, String) {
    let mut s = format!(
        r#"<linearGradient id="{id}" x1="{}" y1="{}" x2="{}" y2="{}">"#,
        n(x[0]),
        n(x[1]),
        n(x[2]),
        n(x[3])
    );
    for &(o, c, a) in stops {
        if a < 1.0 {
            let _ = write!(
                s,
                r#"<stop offset="{}" stop-color="{c}" stop-opacity="{}"/>"#,
                n(o),
                n(a)
            );
        } else {
            let _ = write!(s, r#"<stop offset="{}" stop-color="{c}"/>"#, n(o));
        }
    }
    s.push_str("</linearGradient>");
    (id.to_string(), s)
}

/// A radial gradient centered at (cx, cy) of radius r (bounding box fractions).
fn rg(id: &str, stops: &[(f32, &str, f32)], cx: f32, cy: f32, r: f32) -> (String, String) {
    let mut s = format!(
        r#"<radialGradient id="{id}" cx="{}" cy="{}" r="{}">"#,
        n(cx),
        n(cy),
        n(r)
    );
    for &(o, c, a) in stops {
        if a < 1.0 {
            let _ = write!(
                s,
                r#"<stop offset="{}" stop-color="{c}" stop-opacity="{}"/>"#,
                n(o),
                n(a)
            );
        } else {
            let _ = write!(s, r#"<stop offset="{}" stop-color="{c}"/>"#, n(o));
        }
    }
    s.push_str("</radialGradient>");
    (id.to_string(), s)
}

/// A speckle filter: fractal noise thresholded to black or white specks, kept inside the
/// shape it is applied to. `ch` is the noise channel, `k` and `off` the threshold.
#[allow(clippy::too_many_arguments)]
fn grain(
    id: &str,
    freq: &str,
    octaves: u32,
    seed: u32,
    ch: usize,
    k: f32,
    off: f32,
    white: bool,
    stitch: bool,
) -> (String, String) {
    let c = if white { 1 } else { 0 };
    let mut row = [
        "0".to_string(),
        "0".into(),
        "0".into(),
        "0".into(),
        "0".into(),
    ];
    row[ch] = n(k);
    row[4] = n(off);
    let stitch = if stitch {
        r#" stitchTiles="stitch""#
    } else {
        ""
    };
    (
        id.to_string(),
        format!(
            concat!(
                r#"<filter id="{id}" x="0" y="0" width="1" height="1" "#,
                r#"color-interpolation-filters="sRGB">"#,
                r#"<feTurbulence type="fractalNoise" baseFrequency="{freq}" "#,
                r#"numOctaves="{octaves}" seed="{seed}"{stitch}/>"#,
                r#"<feColorMatrix type="matrix" values="0 0 0 0 {c}  0 0 0 0 {c}  "#,
                r#"0 0 0 0 {c}  {row}"/>"#,
                r#"<feComposite in2="SourceGraphic" operator="in"/></filter>"#
            ),
            id = id,
            freq = freq,
            octaves = octaves,
            seed = seed,
            stitch = stitch,
            c = c,
            row = row.join(" ")
        ),
    )
}

/// One, opaque.
const O: f32 = 1.0;

/// Every shared definition of the art, in the mockup's order: (id, element).
pub fn defs() -> Vec<(String, String)> {
    let mut v = Vec::new();
    for (id, sd) in [("b05", 0.5), ("b1", 0.9), ("b2", 1.8), ("b3", 3.0)] {
        v.push((
            id.to_string(),
            format!(
                concat!(
                    r#"<filter id="{}" x="-60%" y="-60%" width="220%" height="220%">"#,
                    r#"<feGaussianBlur stdDeviation="{}"/></filter>"#
                ),
                id,
                n(sd)
            ),
        ));
    }
    let s = false;
    v.push(grain("grD", "1.3", 2, 3, 0, 2.6, -1.25, false, s));
    v.push(grain("grL", "1.3", 2, 11, 1, 2.6, -1.3, true, s));
    v.push(grain("blotD", "0.09", 3, 5, 0, 2.2, -1.0, false, s));
    v.push(grain("blotL", "0.07", 3, 21, 2, 2.2, -1.05, true, s));
    v.push(grain("woodD", "0.9 0.012", 3, 4, 0, 2.4, -0.95, false, s));
    v.push(grain("woodL", "1.4 0.02", 2, 9, 1, 2.4, -1.15, true, s));
    v.push(grain("marble", "0.035 0.05", 4, 8, 0, 2.6, -1.2, false, s));
    // the desk's tile (the window's backdrop): the same speckle, stitched so the tile repeats
    // without a seam, and the faint horizontal brushing
    v.push(grain("grDs", "1.3", 2, 3, 0, 2.6, -1.25, false, true));
    v.push(grain("grLs", "1.3", 2, 11, 1, 2.6, -1.3, true, true));
    // The fine speckle of the plates as a repeating tile of 64 x 64 panel pixels (task-179):
    // the noise is rasterized once per part for the tile, not over the whole plate (a large
    // part of the start's cost); the coarse blotches stay noise over the whole plate.
    for (id, f) in [("pgrD", "grDs"), ("pgrL", "grLs")] {
        v.push((
            id.to_string(),
            format!(
                concat!(
                    r#"<pattern id="{}" patternUnits="userSpaceOnUse" width="{t}" height="{t}">"#,
                    r##"<rect width="{t}" height="{t}" fill="#000" filter="url(#{})"/></pattern>"##
                ),
                id,
                f,
                t = GRAIN_TILE
            ),
        ));
    }
    v.push((
        "brush".into(),
        concat!(
            r#"<filter id="brush" x="0" y="0" width="1" height="1" "#,
            r#"color-interpolation-filters="sRGB">"#,
            r#"<feTurbulence type="fractalNoise" baseFrequency="0.006 0.55" numOctaves="3" "#,
            r#"seed="12" stitchTiles="stitch"/>"#,
            r#"<feColorMatrix type="matrix" values="0 0 0 0 1  0 0 0 0 1  0 0 0 0 1  "#,
            r#"2.2 0 0 0 -0.95"/>"#,
            r#"<feComposite in2="SourceGraphic" operator="in"/></filter>"#
        )
        .into(),
    ));
    let down = [0.0, 0.0, 0.0, 1.0];
    let right = [0.0, 0.0, 1.0, 0.0];
    let diag = [0.0, 0.0, 1.0, 1.0];
    let tl = [0.15, 0.1, 0.85, 0.9];
    let st = |o: f32, c| (o, c, O);
    v.push(lg(
        "gHdrPat",
        &[st(0.0, "#c23148"), st(0.5, "#a82439"), st(1.0, "#8a1a2c")],
        down,
    ));
    v.push(lg(
        "gHdrMix",
        &[st(0.0, "#b8b055"), st(0.5, "#a39b41"), st(1.0, "#878033")],
        down,
    ));
    v.push(lg(
        "gHdrFx",
        &[st(0.0, "#3d62b8"), st(0.5, "#2c4b9c"), st(1.0, "#213a80")],
        down,
    ));
    v.push(lg(
        "gSheen",
        &[(0.0, "#fff", 0.10), (0.5, "#fff", 0.0), (1.0, "#000", 0.14)],
        down,
    ));
    v.push(lg(
        "g303top",
        &[st(0.0, "#f6f6f6"), st(0.6, "#e3e4e5"), st(1.0, "#cfd0d2")],
        down,
    ));
    v.push(lg(
        "g303bezel",
        &[st(0.0, "#dcdddf"), st(1.0, "#b9babd")],
        down,
    ));
    v.push(lg(
        "gKeyW",
        &[st(0.0, "#f2f2f2"), st(0.85, "#ffffff"), st(1.0, "#d9d9d9")],
        down,
    ));
    v.push(lg("gKeyB", &[st(0.0, "#2a2a2a"), st(1.0, "#0c0c0c")], down));
    v.push(lg("g808", &[st(0.0, "#4a473d"), st(1.0, "#3a372f")], down));
    v.push(lg("g909", &[st(0.0, "#efeee9"), st(1.0, "#dddbd3")], down));
    v.push(lg(
        "g909hdr",
        &[st(0.0, "#5b5c5c"), st(1.0, "#3c3d3d")],
        down,
    ));
    v.push(lg(
        "gSilver",
        &[st(0.0, "#fbfbfc"), st(0.45, "#d5d8db"), st(1.0, "#a4a8ad")],
        down,
    ));
    v.push(lg(
        "gPillFlatLit",
        &[st(0.0, "#eaecee"), st(1.0, "#cacdd1")],
        diag,
    ));
    v.push(lg(
        "gPill",
        &[st(0.0, "#f4f5f6"), st(0.5, "#c3c6ca"), st(1.0, "#8e9297")],
        diag,
    ));
    v.push(lg(
        "gCap",
        &[st(0.0, "#ffffff"), st(0.55, "#e4e6e8"), st(1.0, "#b8bcc1")],
        down,
    ));
    v.push(lg(
        "gCapLit",
        &[st(0.0, "#fff6f3"), st(0.5, "#ffc9bf"), st(1.0, "#ff8c7c")],
        down,
    ));
    v.push(lg(
        "gTrans",
        &[
            st(0.0, "#f1f3f4"),
            st(0.1, "#d9dee0"),
            st(0.2, "#aeb6ba"),
            st(0.75, "#8c969b"),
            st(1.0, "#69737a"),
        ],
        down,
    ));
    v.push(lg(
        "gFader",
        &[
            st(0.0, "#6b7077"),
            st(0.2, "#3b3f45"),
            st(0.46, "#17191c"),
            st(0.54, "#17191c"),
            st(0.8, "#34383d"),
            st(1.0, "#16181b"),
        ],
        down,
    ));
    v.push(lg(
        "gSlot",
        &[st(0.0, "#050506"), st(0.6, "#131416"), st(1.0, "#3d4349")],
        right,
    ));
    v.push(lg(
        "g303Wall",
        &[st(0.0, "#8f9398"), st(0.45, "#4a4d51"), st(1.0, "#17181a")],
        tl,
    ));
    v.push(lg(
        "g303Edge",
        &[st(0.0, "#ffffff"), st(0.5, "#d0d2d5"), st(1.0, "#5c5f63")],
        tl,
    ));
    v.push(lg(
        "gGrayCollar",
        &[st(0.0, "#3a3d42"), st(0.5, "#141517"), st(1.0, "#020203")],
        tl,
    ));
    v.push(lg(
        "gGrayTop",
        &[st(0.0, "#949aab"), st(1.0, "#6a7083")],
        down,
    ));
    v.push(lg(
        "g808RedF",
        &[st(0.0, "#f6473a"), st(1.0, "#d41f15")],
        down,
    ));
    v.push(lg(
        "g808WhiteF",
        &[st(0.0, "#f8fdff"), st(1.0, "#d3e4ea")],
        down,
    ));
    v.push(lg(
        "g808Collar",
        &[st(0.0, "#2c2c2a"), st(0.5, "#0d0d0c"), st(1.0, "#000000")],
        tl,
    ));
    v.push(lg(
        "g909Wall",
        &[st(0.0, "#4a4a47"), st(1.0, "#141413")],
        tl,
    ));
    v.push(lg("g909Top", &[st(0.0, "#5a5a56"), st(1.0, "#343431")], tl));
    v.push(lg(
        "gRimLight",
        &[
            (0.0, "#fff", 0.55),
            (0.5, "#fff", 0.05),
            (1.0, "#000", 0.35),
        ],
        tl,
    ));
    for (name, c) in [
        ("R", ["#ff6a55", "#e8372b", "#a8140e"]),
        ("O", ["#ffb65a", "#f58a21", "#c9600c"]),
        ("Y", ["#fff27a", "#f3dc3c", "#c4a916"]),
        ("W", ["#ffffff", "#f2f2ee", "#c4c5c0"]),
    ] {
        v.push(lg(
            &format!("g808k{name}"),
            &[
                st(0.0, c[0]),
                st(0.28, c[1]),
                st(0.34, c[0]),
                st(0.6, c[1]),
                st(1.0, c[2]),
            ],
            down,
        ));
    }
    let led = |id: &str, hi, mid, lo| {
        rg(
            id,
            &[st(0.0, hi), st(0.45, mid), st(1.0, lo)],
            0.4,
            0.35,
            0.65,
        )
    };
    v.push(led("ledR", "#ffd9d2", "#ff3b2c", "#b3140c"));
    v.push(led("ledRoff", "#8a3a34", "#5a1512", "#2a0605"));
    v.push(led("ledG", "#eaffe0", "#43e03a", "#12861a"));
    v.push(led("ledGoff", "#3f6a43", "#1f4426", "#0d2311"));
    v.push(led("ledY", "#fffbd0", "#ffd42e", "#b98a08"));
    v.push(led("ledYoff", "#7a6a35", "#4f431c", "#261f0a"));
    for (id, c, a) in [
        ("haloR", "#ff4a36", 0.75),
        ("haloG", "#5cff4a", 0.7),
        ("haloY", "#ffd640", 0.7),
        ("haloO", "#ff9a30", 0.7),
    ] {
        v.push(rg(id, &[(0.0, c, a), (1.0, c, 0.0)], 0.5, 0.5, 0.5));
    }
    v.push(lg(
        "gWood",
        &[
            st(0.0, "#5a3b2e"),
            st(0.25, "#8a5d47"),
            st(0.6, "#7a503c"),
            st(1.0, "#4e3227"),
        ],
        right,
    ));
    v.push(lg(
        "gLever",
        &[
            st(0.0, "#8a9197"),
            st(0.35, "#ffffff"),
            st(0.7, "#b9c0c6"),
            st(1.0, "#5e656b"),
        ],
        right,
    ));
    v.push(lg(
        "gScreen",
        &[(0.0, "#000", 0.55), (0.12, "#000", 0.0), (1.0, "#000", 0.0)],
        down,
    ));
    v.push(lg(
        "gSlideD",
        &[st(0.0, "#5a5d61"), st(1.0, "#2b2d30")],
        down,
    ));
    v
}

/// The side of the fine speckle's tile (`pgrD`, `pgrL`), in panel pixels.
pub const GRAIN_TILE: u32 = 64;

/// The `<defs>` element with the shared definitions `body` refers to (and those they refer
/// to), or nothing.
pub fn defs_for(body: &str) -> String {
    let all = defs();
    let mut text = body.to_string();
    let mut used = vec![false; all.len()];
    loop {
        let mut more = false;
        for (i, (id, d)) in all.iter().enumerate() {
            if !used[i] && text.contains(&format!("url(#{id})")) {
                used[i] = true;
                text.push_str(d);
                more = true;
            }
        }
        if !more {
            break;
        }
    }
    let used: Vec<String> = all
        .into_iter()
        .zip(used)
        .filter(|(_, u)| *u)
        .map(|((_, d), _)| d)
        .collect();
    if used.is_empty() {
        String::new()
    } else {
        format!("<defs>{}</defs>", used.concat())
    }
}

/// One named, self-contained piece of art.
///
/// `w` x `h` is the control's rectangle; the art may reach `bleed` panel pixels beyond it on
/// every side (shadows, halos, glows), so the file's canvas is `w + 2 bleed` x `h + 2 bleed`
/// with the control's rectangle at (0, 0) of its viewBox (`-bleed -bleed ...`).
#[derive(Clone)]
pub struct Part {
    pub name: String,
    pub w: f32,
    pub h: f32,
    pub bleed: f32,
    pub body: String,
}

impl Part {
    pub fn new(name: impl Into<String>, w: f32, h: f32, f: impl FnOnce(&mut Svg)) -> Self {
        Self::with_bleed(name, w, h, 0.0, f)
    }

    pub fn with_bleed(
        name: impl Into<String>,
        w: f32,
        h: f32,
        bleed: f32,
        f: impl FnOnce(&mut Svg),
    ) -> Self {
        let name = name.into();
        let mut s = Svg {
            prefix: name.clone(),
            ..Svg::new()
        };
        f(&mut s);
        Self {
            name,
            w,
            h,
            bleed,
            body: s.s,
        }
    }

    /// The canvas: the control's rectangle grown by the bleed.
    pub fn canvas(&self) -> (f32, f32) {
        (self.w + 2.0 * self.bleed, self.h + 2.0 * self.bleed)
    }

    /// The part as a standalone SVG file: the canvas in panel pixels, its viewBox starting at
    /// `-bleed`, so any size hint scales it uniformly.
    pub fn to_svg(&self) -> String {
        let (cw, ch) = self.canvas();
        let b = n(-self.bleed);
        format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" \
             viewBox=\"{b} {b} {w} {h}\">{}{}</svg>\n",
            defs_for(&self.body),
            self.body,
            w = n(cw),
            h = n(ch),
        )
    }
}
