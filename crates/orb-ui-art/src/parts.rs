//! Every part of the panel's art, drawn in code (flat style of the ui-poc
//! experiment). A part's size is the size of the control in docs/ui_spec.md,
//! in panel pixels; its viewBox equals that size.
//!
//! Naming: `<control>_<state>`. Push buttons have `rest` and `pressed`,
//! toggles `off` and `on`, buttons that light up also `lit`.

use crate::font::{self, Anchor, label, text};
use crate::svg::{Part, Svg, arc_path, n, pal, polar};

/// The sweep of a knob: the pointer turns from -KNOB_SWEEP to +KNOB_SWEEP
/// degrees (0 = straight up, clockwise positive) as the value goes 0 to 1.
pub const KNOB_SWEEP: f32 = 135.0;

/// A knob style: size, the center, the value ring, the body and the pointer.
pub struct KnobDesign {
    pub name: &'static str,
    pub w: f32,
    pub h: f32,
    pub cx: f32,
    pub cy: f32,
    /// Radius of the value ring, 0 for none (the app paints the arc itself).
    pub ring_r: f32,
    pub ring_w: f32,
    pub body_r: f32,
    pub ptr0: f32,
    pub ptr1: f32,
    pub ptr_w: f32,
    pub body: &'static str,
    pub pointer: &'static str,
    /// The color of the value arc the app paints over the ring track.
    pub arc: &'static str,
    pub track: &'static str,
}

pub const KNOBS: [KnobDesign; 5] = [
    KnobDesign {
        name: "knob_303",
        w: 30.0,
        h: 30.0,
        cx: 15.0,
        cy: 15.0,
        ring_r: 13.6,
        ring_w: 1.6,
        body_r: 11.0,
        ptr0: 3.0,
        ptr1: 9.6,
        ptr_w: 1.8,
        body: pal::STEEL,
        pointer: pal::BLACK,
        arc: pal::ORANGE,
        track: "#00000026",
    },
    KnobDesign {
        name: "knob_808_red",
        w: 13.0,
        h: 13.0,
        cx: 6.5,
        cy: 6.5,
        ring_r: 5.9,
        ring_w: 0.9,
        body_r: 4.7,
        ptr0: 1.4,
        ptr1: 4.0,
        ptr_w: 1.0,
        body: pal::RED,
        pointer: pal::WHITE,
        arc: pal::YELLOW,
        track: "#ffffff22",
    },
    KnobDesign {
        name: "knob_808_white",
        w: 13.0,
        h: 13.0,
        cx: 6.5,
        cy: 6.5,
        ring_r: 5.9,
        ring_w: 0.9,
        body_r: 4.7,
        ptr0: 1.4,
        ptr1: 4.0,
        ptr_w: 1.0,
        body: "#e6e8ea",
        pointer: pal::BLACK,
        arc: pal::ORANGE,
        track: "#ffffff22",
    },
    KnobDesign {
        name: "knob_909",
        w: 12.0,
        h: 12.0,
        cx: 6.0,
        cy: 6.0,
        ring_r: 5.4,
        ring_w: 0.9,
        body_r: 4.2,
        ptr0: 1.2,
        ptr1: 3.7,
        ptr_w: 1.0,
        body: pal::BLACK,
        pointer: pal::WHITE,
        arc: pal::RED,
        track: "#00000022",
    },
    // The mixer's and the effects' knobs.
    KnobDesign {
        name: "knob_22",
        w: 22.0,
        h: 22.0,
        cx: 11.0,
        cy: 11.0,
        ring_r: 10.1,
        ring_w: 1.4,
        body_r: 7.9,
        ptr0: 2.4,
        ptr1: 6.7,
        ptr_w: 1.5,
        body: pal::SLATE_DARK,
        pointer: pal::INK,
        arc: pal::AMBER,
        track: "#00000040",
    },
];

fn knobs(out: &mut Vec<Part>) {
    for k in &KNOBS {
        out.push(Part::new(format!("{}_body", k.name), k.w, k.h, |s| {
            s.path_stroke(
                &arc_path(k.cx, k.cy, k.ring_r, -KNOB_SWEEP, KNOB_SWEEP),
                k.track,
                k.ring_w,
            );
            s.circle(k.cx, k.cy, k.body_r, k.body);
            s.circle_stroke(k.cx, k.cy, k.body_r - 0.6, "#ffffff2a", 0.5);
        }));
        out.push(Part::new(format!("{}_pointer", k.name), k.w, k.h, |s| {
            // Drawn pointing up; the app rotates it around (cx, cy).
            let (x0, y0) = polar(k.cx, k.cy, k.ptr0, 0.0);
            let (x1, y1) = polar(k.cx, k.cy, k.ptr1, 0.0);
            s.line(x0, y0, x1, y1, k.pointer, k.ptr_w);
        }));
    }
}

/// The four colors of the 808's steps, which the app icon and the wordmark's underline reuse.
const STEP_COLORS: [&str; 4] = [pal::RED, pal::ORANGE, pal::YELLOW, pal::WHITE];

/// The app icon (task-115): the 303's saw running into a square wave in orange over four
/// step keys in the 808's colors, on a slate tile; 64 x 64 with the tile inset like a
/// macOS icon's.
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

fn fader(out: &mut Vec<Part>) {
    out.push(Part::new("fader_track", 15.0, 54.0, |s| {
        for i in 0..=10 {
            let y = 5.5 + i as f32 * 4.3;
            let (x0, x1) = if i % 5 == 0 { (0.8, 3.8) } else { (1.8, 3.8) };
            s.line(x0, y, x1, y, pal::INK_DIM, 0.6);
            s.line(15.0 - x1, y, 15.0 - x0, y, pal::INK_DIM, 0.6);
        }
        s.rect(6.0, 3.0, 3.0, 48.0, 1.5, "#000000a0");
    }));
    out.push(Part::new("fader_handle", 15.0, 11.0, |s| {
        s.rect(1.0, 0.0, 13.0, 11.0, 2.5, pal::STEEL);
        s.rect(1.0, 0.0, 13.0, 3.0, 2.5, "#ffffff40");
        s.line(3.0, 5.5, 12.0, 5.5, pal::BLACK, 1.2);
    }));
}

/// A slide switch with a thumb at position `pos` (0 = left or top).
fn switch(out: &mut Vec<Part>, name: &str, w: f32, h: f32, horizontal: bool) {
    for pos in 0..2 {
        out.push(Part::new(format!("{name}_{pos}"), w, h, |s| {
            s.rect(0.0, 0.0, w, h, 2.0, pal::BLACK);
            if horizontal {
                let tw = w / 2.0;
                s.rect(
                    pos as f32 * tw + 1.0,
                    1.0,
                    tw - 2.0,
                    h - 2.0,
                    1.5,
                    pal::STEEL,
                );
            } else {
                let th = h / 2.0;
                s.rect(
                    1.0,
                    pos as f32 * th + 1.0,
                    w - 2.0,
                    th - 2.0,
                    1.5,
                    pal::STEEL,
                );
            }
        }));
    }
}

/// The saw and square glyphs with their left edges at `sx` and `qx`, 6 px tall around `yc`.
fn wave_glyphs(s: &mut Svg, sx: f32, qx: f32, yc: f32, color: &str) {
    let (t, b) = (yc - 3.0, yc + 3.0);
    s.path_stroke(
        &format!(
            "M{sx} {b} L{} {t} V{b} L{} {t} V{b}",
            n(sx + 3.75),
            n(sx + 7.5)
        ),
        color,
        1.0,
    );
    s.path_stroke(
        &format!(
            "M{qx} {b} V{t} H{} V{b} H{} V{t} H{}",
            n(qx + 2.5),
            n(qx + 5.0),
            n(qx + 7.5)
        ),
        color,
        1.0,
    );
}

/// A round LED lens of radius `r` in a recessed socket, centered at (`cx`, `cy`): a radial
/// gradient gives it the dome, a white ellipse the gloss, a halo the glow when lit. The
/// socket is translucent, so it takes the color of what it sits on. `hue` is "green" or
/// "amber".
fn led_3d(s: &mut Svg, cx: f32, cy: f32, r: f32, on: bool, hue: &str) {
    let (id, stops) = match (hue, on) {
        ("green", true) => (
            "ledg1",
            concat!(
                "<stop offset=\"0\" stop-color=\"#c8f8cc\"/>",
                "<stop offset=\"0.3\" stop-color=\"#6fe07e\"/>",
                "<stop offset=\"0.75\" stop-color=\"#38b84d\"/>",
                "<stop offset=\"1\" stop-color=\"#1f7a30\"/>",
            ),
        ),
        ("green", false) => (
            "ledg0",
            concat!(
                "<stop offset=\"0\" stop-color=\"#4f6a55\"/>",
                "<stop offset=\"0.5\" stop-color=\"#2f4536\"/>",
                "<stop offset=\"1\" stop-color=\"#1c2a20\"/>",
            ),
        ),
        (_, true) => (
            "leda1",
            concat!(
                "<stop offset=\"0\" stop-color=\"#fff0b8\"/>",
                "<stop offset=\"0.3\" stop-color=\"#ffd04a\"/>",
                "<stop offset=\"0.75\" stop-color=\"#f0a300\"/>",
                "<stop offset=\"1\" stop-color=\"#9a6400\"/>",
            ),
        ),
        (_, false) => (
            "leda0",
            concat!(
                "<stop offset=\"0\" stop-color=\"#5d5340\"/>",
                "<stop offset=\"0.5\" stop-color=\"#3b3526\"/>",
                "<stop offset=\"1\" stop-color=\"#231f14\"/>",
            ),
        ),
    };
    s.raw(&format!(
        "<defs><radialGradient id=\"{id}\" cx=\"0.38\" cy=\"0.32\" r=\"0.75\">{stops}\
         </radialGradient></defs>"
    ));
    // the socket: the lighter ring peeking out below reads as the recess's lower edge
    s.circle_stroke(cx, cy + 0.45, r + 1.3, "#ffffff2c", 0.7);
    s.circle(cx, cy, r + 1.4, "#00000062");
    s.circle_stroke(cx, cy - 0.15, r + 0.9, "#00000050", 0.8);
    if on {
        let halo = if hue == "green" { "#5fe070" } else { "#ffb000" };
        s.circle(cx, cy, r + 2.6, &format!("{halo}24"));
    }
    s.circle(cx, cy, r, &format!("url(#{id})"));
    s.raw(&format!(
        "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"#ffffff\" opacity=\"{}\"/>",
        n(cx - r * 0.25),
        n(cy - r * 0.45),
        n(r * 0.45),
        n(r * 0.28),
        if on { "0.55" } else { "0.18" }
    ));
}

/// A keycap: a face over a 1.8 px darker lip, with a thin outline.
fn keycap(s: &mut Svg, x: f32, y: f32, w: f32, h: f32, face: &str, lip: &str) {
    s.rect(x, y, w, h, 2.2, lip);
    s.rect(x, y, w, h - 1.8, 2.2, face);
    s.rect_stroke(x + 0.25, y + 0.25, w - 0.5, h - 0.5, 2.2, "#00000055", 0.5);
}

/// The discreet 3D of the 303's keys and pills: a darker base, the face inset by 0.5 px on
/// the sides and 1.5 px short at the bottom. `r` is the corner radius.
#[allow(clippy::too_many_arguments)]
fn key_3d(s: &mut Svg, x: f32, y: f32, w: f32, h: f32, r: f32, base: &str, face: &str) {
    s.rect(x, y, w, h, r, base);
    s.rect(x + 0.5, y, w - 1.0, h - 1.5, r - 0.3, face);
}

/// The mixer's routing key, 28 x 13: a face over a darker base that shows as a soft rim and a
/// lip, an amber LED at the left and the label left-aligned after it.
fn mixsw_body(s: &mut Svg, label_: &str, on: bool) {
    s.rect(0.0, 0.0, 28.0, 13.0, 2.5, "#2a323d");
    s.rect(0.6, 0.6, 26.8, 10.6, 2.0, "#4b5767");
    s.rect(0.6, 0.6, 26.8, 4.6, 2.0, "#ffffff0c");
    led_3d(s, 5.4, 6.2, 1.8, on, "amber");
    let color = if on { pal::INK } else { pal::INK_DIM };
    text(s, 10.4, 6.2, 3.8, Anchor::Start, color, label_);
}

fn switches(out: &mut Vec<Part>) {
    // The 303's waveform switch: one 40 x 14 black frame with the saw and the square printed
    // on it, an 18 x 12 track between them and the thumb at position 0 (saw) or 1 (square).
    for pos in 0..2 {
        out.push(Part::new(format!("switch_wave_{pos}"), 40.0, 14.0, |s| {
            s.rect(0.0, 0.0, 40.0, 14.0, 3.0, pal::BLACK);
            s.rect(0.6, 0.6, 38.8, 12.8, 2.5, "#26282b");
            s.rect(11.0, 1.0, 18.0, 12.0, 2.0, "#0e0f10");
            wave_glyphs(s, 2.0, 30.5, 7.0, pal::INK);
            let x = 12.0 + pos as f32 * 8.0;
            s.rect(x, 1.5, 8.0, 11.0, 2.0, "#8c929b");
            s.rect(x, 1.5, 8.0, 9.5, 2.0, pal::STEEL);
            s.line(x + 2.0, 6.0, x + 6.0, 6.0, "#00000040", 0.8);
        }));
    }
    switch(out, "switch_808", 11.0, 17.0, false);
    switch(out, "switch_fx", 9.0, 18.0, false);
    switch(out, "switch_mode", 26.0, 19.0, true);
    // Dist, PCF and Comp routing keys of the mixer (and the master's).
    for (kind, text_) in [("dist", "DIST"), ("pcf", "PCF"), ("comp", "COMP")] {
        for (state, on) in [("off", false), ("on", true)] {
            out.push(Part::new(
                format!("mixsw_{kind}_{state}"),
                28.0,
                13.0,
                |s| mixsw_body(s, text_, on),
            ));
        }
    }
}

fn power_buttons(out: &mut Vec<Part>) {
    // The pattern sections' (13) and the mixer's and effects' (14) on/off buttons: a round 3D
    // LED, no bezel.
    for (w, h) in [(13.0, 13.0), (14.0, 14.0)] {
        for (state, on) in [("off", false), ("on", true)] {
            out.push(Part::new(
                format!("power_{}x{}_{state}", n(w), n(h)),
                w,
                h,
                |s| led_3d(s, w / 2.0, h / 2.0, 2.6, on, "green"),
            ));
        }
    }
}

fn transport_buttons(out: &mut Vec<Part>) {
    let icons: [(&str, bool); 5] = [
        ("play", true),
        ("stop", false),
        ("rewind", false),
        ("ff", false),
        ("record", true),
    ];
    for (name, has_lit) in icons {
        let states: &[&str] = if has_lit {
            &["rest", "pressed", "lit"]
        } else {
            &["rest", "pressed"]
        };
        for &state in states {
            out.push(Part::new(format!("btn_{name}_{state}"), 32.0, 25.0, |s| {
                let fill = match (state, name) {
                    ("lit", "record") => pal::RED,
                    ("lit", _) => pal::SLATE_LINE,
                    _ => pal::SLATE_DARK,
                };
                s.rect(1.0, 1.0, 30.0, 23.0, 4.0, fill);
                if state == "pressed" {
                    s.rect(1.0, 1.0, 30.0, 23.0, 4.0, pal::SHADE);
                    s.rect_stroke(1.5, 1.5, 29.0, 22.0, 3.5, "#00000066", 1.0);
                }
                let (cx, cy) = (16.0, 12.5);
                let ink = pal::INK;
                match name {
                    "play" => {
                        s.path(
                            &format!(
                                "M{} {} L{} {} L{} {} Z",
                                n(cx - 4.5),
                                n(cy - 6.0),
                                n(cx + 6.0),
                                n(cy),
                                n(cx - 4.5),
                                n(cy + 6.0)
                            ),
                            pal::GREEN,
                        );
                    }
                    "stop" => {
                        s.rect(cx - 5.0, cy - 5.0, 10.0, 10.0, 1.2, ink);
                    }
                    "rewind" => {
                        for dx in [-5.5, 1.5] {
                            s.path(
                                &format!(
                                    "M{} {} L{} {} L{} {} Z",
                                    n(cx + dx + 5.5),
                                    n(cy - 5.5),
                                    n(cx + dx),
                                    n(cy),
                                    n(cx + dx + 5.5),
                                    n(cy + 5.5)
                                ),
                                ink,
                            );
                        }
                    }
                    "ff" => {
                        for dx in [-6.5, 0.0] {
                            s.path(
                                &format!(
                                    "M{} {} L{} {} L{} {} Z",
                                    n(cx + dx),
                                    n(cy - 5.5),
                                    n(cx + dx + 5.5),
                                    n(cy),
                                    n(cx + dx),
                                    n(cy + 5.5)
                                ),
                                ink,
                            );
                        }
                    }
                    _ => {
                        s.circle(
                            cx,
                            cy,
                            5.5,
                            if state == "lit" { pal::WHITE } else { pal::RED },
                        );
                    }
                }
            }));
        }
    }
    // The tempo, bar and loop arrows.
    for (name, up) in [("inc", true), ("dec", false)] {
        for state in ["rest", "pressed"] {
            out.push(Part::new(format!("arrow_{name}_{state}"), 11.0, 9.0, |s| {
                s.rect(
                    0.5,
                    0.5,
                    10.0,
                    8.0,
                    1.8,
                    if state == "pressed" {
                        pal::SLATE_LINE
                    } else {
                        pal::SLATE_DARK
                    },
                );
                let d = if up {
                    "M2.8 6 L5.5 2.6 L8.2 6 Z"
                } else {
                    "M2.8 3 L5.5 6.4 L8.2 3 Z"
                };
                s.path(d, pal::INK);
            }));
        }
    }
    // The pattern/song mode has switch_mode; loop on/off and shuffle are toggles.
    for (state, on) in [("off", false), ("on", true)] {
        out.push(Part::new(format!("toggle_loop_{state}"), 18.0, 18.0, |s| {
            s.rect(
                0.5,
                0.5,
                17.0,
                17.0,
                3.0,
                if on { pal::AMBER } else { pal::SLATE_DARK },
            );
            let ink = if on { pal::BLACK } else { pal::INK };
            // The repeat sign: two rounded half-loops, each ending in an arrowhead (the
            // top one points right, the bottom one left), symmetric about the center.
            s.path_stroke("M4.5 9.5 V7.5 Q4.5 5.5 6.5 5.5 H11", ink, 1.3);
            s.path_stroke("M13.5 8.5 V10.5 Q13.5 12.5 11.5 12.5 H7", ink, 1.3);
            s.path("M11 3 L14 5.5 L11 8 Z", ink);
            s.path("M7 15 L4 12.5 L7 10 Z", ink);
        }));
        out.push(Part::new(
            format!("toggle_shuffle_{state}"),
            30.0,
            12.0,
            |s| {
                if on {
                    keycap(s, 0.0, 0.0, 30.0, 12.0, "#e8503f", "#9c2b25");
                } else {
                    keycap(s, 0.0, 0.0, 30.0, 12.0, "#dde1e6", "#a7afba");
                }
                let ink = if on { pal::WHITE } else { pal::BLACK };
                label(s, 15.0, 5.3, 3.2, ink, "SHUFFLE");
            },
        ));
        // The 909's flam button: the shape of a step button's light.
        out.push(Part::new(format!("toggle_flam_{state}"), 14.0, 8.0, |s| {
            s.rect(
                0.0,
                0.0,
                14.0,
                8.0,
                1.5,
                if on { pal::RED } else { "#b9b5aa" },
            );
        }));
    }
}

fn pattern_buttons(out: &mut Vec<Part>) {
    for k in 1..=8 {
        for (state, on) in [("off", false), ("on", true)] {
            out.push(Part::new(format!("pat_btn_{k}_{state}"), 14.0, 15.0, |s| {
                if on {
                    keycap(s, 0.0, 0.0, 14.0, 15.0, "#e8503f", "#9c2b25");
                } else {
                    keycap(s, 0.0, 0.0, 14.0, 15.0, "#dde1e6", "#a7afba");
                }
                let ink = if on { pal::WHITE } else { pal::BLACK };
                label(s, 7.0, 6.8, 5.5, ink, &k.to_string());
            }));
        }
    }
    for bank in ["A", "B", "C", "D"] {
        for (state, on) in [("off", false), ("on", true)] {
            out.push(Part::new(
                format!("bank_btn_{bank}_{state}"),
                14.0,
                15.0,
                |s| {
                    if on {
                        keycap(s, 0.0, 0.0, 14.0, 15.0, pal::ORANGE, "#a85d14");
                    } else {
                        keycap(s, 0.0, 0.0, 14.0, 15.0, "#4e5a6b", "#2c3542");
                    }
                    let ink = if on { pal::BLACK } else { pal::INK_DIM };
                    label(s, 7.0, 6.8, 5.5, ink, bank);
                },
            ));
        }
    }
}

fn small_buttons(out: &mut Vec<Part>) {
    for (w, h) in [(22.0, 10.0)] {
        for state in ["rest", "pressed"] {
            out.push(Part::new(
                format!("btn_pill_{}x{}_{state}", n(w), n(h)),
                w,
                h,
                |s| {
                    key_3d(s, 0.5, 0.0, w - 1.0, h, 2.0, "#a4aab2", pal::STEEL);
                    if state == "pressed" {
                        s.rect(0.5, 0.5, w - 1.0, h - 1.0, 2.0, "#00000040");
                    }
                },
            ));
        }
    }
    for state in ["rest", "pressed"] {
        // The 303's edit step arrows (Step up, Back down), in the paper's colors.
        for (name, up) in [("inc", true), ("dec", false)] {
            out.push(Part::new(
                format!("arrow303_{name}_{state}"),
                11.0,
                9.0,
                |s| {
                    s.rect(
                        0.5,
                        0.5,
                        10.0,
                        8.0,
                        1.8,
                        if state == "pressed" {
                            "#b4b1a7"
                        } else {
                            pal::PAPER_DARK
                        },
                    );
                    let d = if up {
                        "M2.8 6 L5.5 2.6 L8.2 6 Z"
                    } else {
                        "M2.8 3 L5.5 6.4 L8.2 3 Z"
                    };
                    s.path(d, pal::BLACK);
                },
            ));
        }
        out.push(Part::new(format!("key_white_{state}"), 9.0, 16.0, |s| {
            key_3d(s, 0.5, 0.0, 8.0, 16.0, 1.5, "#c4c7cb", pal::WHITE);
            if state == "pressed" {
                s.rect(0.5, 0.0, 8.0, 16.0, 1.5, "#80808066");
            }
        }));
        out.push(Part::new(format!("key_black_{state}"), 8.0, 15.0, |s| {
            s.rect(0.0, 0.0, 8.0, 15.0, 1.5, "#0e0f10");
            s.rect(1.0, 0.0, 6.0, 12.5, 1.0, "#26282b");
            if state == "pressed" {
                s.rect(0.0, 0.0, 8.0, 15.0, 1.5, "#ffffff30");
            }
        }));
        out.push(Part::new(format!("btn_flag_{state}"), 9.0, 16.0, |s| {
            key_3d(s, 0.5, 0.0, 8.0, 16.0, 1.8, "#a4aab2", pal::STEEL);
            if state == "pressed" {
                s.rect(0.5, 0.0, 8.0, 16.0, 1.8, "#00000050");
            }
        }));
    }
}

fn leds(out: &mut Vec<Part>) {
    for (state, color) in [("off", "#4a2a28"), ("on", pal::RED)] {
        out.push(Part::new(format!("led_8_{state}"), 8.0, 8.0, |s| {
            if state == "on" {
                s.circle(4.0, 4.0, 4.0, "#e2413b40");
            }
            s.circle(4.0, 4.0, 3.0, color);
            s.circle_stroke(4.0, 4.0, 3.0, "#00000040", 0.5);
        }));
    }
    // The 9 LEDs of the compressor's gain-reduction meter.
    for (state, color) in [
        ("off", "#00000066"),
        ("green", pal::GREEN),
        ("amber", pal::AMBER),
        ("red", pal::RED),
    ] {
        out.push(Part::new(format!("gr_led_{state}"), 7.0, 10.0, |s| {
            s.rect(0.5, 0.5, 6.0, 9.0, 1.5, "#15181d");
            s.rect(1.5, 1.5, 4.0, 7.0, 1.0, color);
        }));
    }
}

fn steps(out: &mut Vec<Part>) {
    let colors = [
        ("red", pal::RED),
        ("orange", pal::ORANGE),
        ("yellow", pal::YELLOW),
        ("white", pal::WHITE),
    ];
    for (name, color) in colors {
        for (state, on) in [("off", false), ("on", true)] {
            out.push(Part::new(
                format!("step808_{name}_{state}"),
                16.0,
                17.0,
                |s| {
                    s.rect(0.5, 0.0, 15.0, 17.0, 2.0, color);
                    let led = if !on {
                        "#00000040"
                    } else if name == "red" {
                        pal::WHITE
                    } else {
                        pal::RED
                    };
                    s.rect(2.5, 1.5, 11.0, 5.0, 1.5, led);
                },
            ));
        }
    }
    for (grp, fill) in [("a", "#f6f3ea"), ("b", "#ebe7dc")] {
        for (value, led) in [
            (0, "#00000030"),
            (1, pal::ORANGE),
            (2, pal::RED),
            (3, pal::GREEN),
        ] {
            out.push(Part::new(
                format!("step909_{grp}_{value}"),
                21.0,
                16.0,
                |s| {
                    s.rect(0.5, 0.0, 20.0, 16.0, 2.0, fill);
                    s.rect(3.5, 1.5, 14.0, 8.0, 1.5, led);
                },
            ));
        }
    }
    // Overlays that mark the selected instrument in the legend rows.
    for (w, h) in [(22.0, 14.0), (42.0, 14.0), (21.0, 14.0)] {
        out.push(Part::new(format!("sel_{}x{}", n(w), n(h)), w, h, |s| {
            s.rect(0.0, 0.0, w, h, 2.0, "#f08a2440");
            s.rect_stroke(0.75, 0.75, w - 1.5, h - 1.5, 1.5, pal::ORANGE, 1.5);
        }));
    }
}

const SEG: [(f32, f32, f32, f32); 7] = [
    (2.0, 1.0, 5.0, 1.6),  // a
    (6.2, 2.4, 1.6, 5.2),  // b
    (6.2, 8.4, 1.6, 5.2),  // c
    (2.0, 13.4, 5.0, 1.6), // d
    (1.2, 8.4, 1.6, 5.2),  // e
    (1.2, 2.4, 1.6, 5.2),  // f
    (2.0, 7.2, 5.0, 1.6),  // g
];

fn mask(c: char) -> u8 {
    match c {
        '0' => 0b0111111,
        '1' => 0b0000110,
        '2' => 0b1011011,
        '3' => 0b1001111,
        '4' => 0b1100110,
        '5' => 0b1101101,
        '6' => 0b1111101,
        '7' => 0b0000111,
        '8' => 0b1111111,
        '9' => 0b1101111,
        _ => 0,
    }
}

/// The 11 frames of the seven-segment display: digits 0..9 and blank. The
/// unlit segments stay faintly visible (a blank is the disabled display).
fn digits(out: &mut Vec<Part>) {
    for c in "0123456789 ".chars() {
        let name = if c == ' ' {
            "seg_blank".to_string()
        } else {
            format!("seg_{c}")
        };
        out.push(Part::new(name, 9.0, 16.0, |s| {
            for (k, &(x, y, w, h)) in SEG.iter().enumerate() {
                let on = mask(c) & (1 << k) != 0;
                s.rect(x, y, w, h, 0.6, if on { pal::LCD_ON } else { pal::LCD_OFF });
            }
        }));
    }
}

/// A level meter with 10 segments and 11 states (0 = silent, 10 = clip).
fn meters(out: &mut Vec<Part>) {
    // The channels' and effects' header meter (bare segments on the header), and the
    // master's two (segments in a dark well).
    for (w, h) in [(8.0, 14.0), (15.0, 56.0)] {
        let header = w == 8.0;
        let pad = if header { 0.0 } else { 1.5 };
        for level in 0..=10 {
            out.push(Part::new(
                format!("meter_{}x{}_{level}", n(w), n(h)),
                w,
                h,
                |s| {
                    if !header {
                        s.rect(0.0, 0.0, w, h, 1.5, "#101216");
                    }
                    let seg_h = (h - 2.0 * pad) / 10.0;
                    for i in 0..10 {
                        let y = h - pad - (i as f32 + 1.0) * seg_h;
                        let color = match i {
                            0..=5 => pal::GREEN,
                            6..=7 => pal::AMBER,
                            _ => pal::RED,
                        };
                        let on = i < level;
                        let off = if header { "#00000040" } else { "#ffffff14" };
                        s.rect(
                            pad,
                            y + 0.25,
                            w - 2.0 * pad,
                            seg_h - 0.5,
                            0.3,
                            if on { color } else { off },
                        );
                    }
                },
            ));
        }
    }
}

// ---------------------------------------------------------------- backgrounds

/// The dark seam color first, then the section's face inset by one pixel.
fn face(s: &mut Svg, w: f32, h: f32, fill: &str) {
    s.rect(0.0, 0.0, w, h, 0.0, pal::BG);
    s.rect(1.0, 1.0, w - 2.0, h - 2.0, 3.0, fill);
}

/// `hex` (`#rrggbb`) with each channel times `f`.
fn shade(hex: &str, f: f32) -> String {
    let v = u32::from_str_radix(&hex[1..7], 16).expect("a #rrggbb color");
    let c = |sh: u32| (((v >> sh) & 0xff) as f32 * f).round().clamp(0.0, 255.0) as u32;
    format!("#{:02x}{:02x}{:02x}", c(16), c(8), c(0))
}

/// A section header with a 3D look: a lip in the header color darkened to 0.62 under the
/// face, which is 1.3 px shorter, a vertical gradient over the face and a thin white line
/// just below its top.
fn header(s: &mut Svg, x: f32, y: f32, w: f32, h: f32, fill: &str) {
    s.raw(concat!(
        "<defs><linearGradient id=\"hdr\" x1=\"0\" y1=\"0\" x2=\"0\" y2=\"1\">",
        "<stop offset=\"0\" stop-color=\"#ffffff\" stop-opacity=\"0.17\"/>",
        "<stop offset=\"0.55\" stop-color=\"#ffffff\" stop-opacity=\"0.02\"/>",
        "<stop offset=\"1\" stop-color=\"#000000\" stop-opacity=\"0.16\"/>",
        "</linearGradient></defs>",
    ));
    s.rect(x, y, w, h, 2.0, &shade(fill, 0.62));
    s.rect(x, y, w, h - 1.3, 2.0, fill);
    s.rect(x, y, w, h - 1.3, 2.0, "url(#hdr)");
    s.line(x + 2.0, y + 0.5, x + w - 2.0, y + 0.5, "#ffffff2e", 0.6);
}

/// The pattern sections' header color (rust).
const PATTERN_HEADER: &str = "#bf4a2b";
/// The mixer channels' header color (sea).
const MIXER_HEADER: &str = "#3b8a8f";

fn well(s: &mut Svg, x: f32, y: f32, w: f32, h: f32) {
    s.rect(x - 0.5, y - 0.5, w + 1.0, h + 1.0, 2.0, "#0c0707");
    s.rect(x, y, w, h, 1.5, pal::LCD_BG);
}

fn wood(out: &mut Vec<Part>) {
    out.push(Part::new("wood", 16.0, 510.0, |s| {
        // Three planks with seams and a few grain strokes.
        let tones = [pal::WOOD, pal::WOOD_LIGHT, pal::WOOD_DARK];
        for (i, tone) in tones.iter().enumerate() {
            s.rect(i as f32 * 5.34, 0.0, 5.34, 510.0, 0.0, tone);
        }
        s.line(5.34, 0.0, 5.34, 510.0, "#00000040", 0.8);
        s.line(10.67, 0.0, 10.67, 510.0, "#00000040", 0.8);
        let mut seed = 0x9e37_79b9_u32;
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed
        };
        for _ in 0..60 {
            let x = 0.8 + (next() % 1440) as f32 / 100.0;
            let y = (next() % 49000) as f32 / 100.0;
            let len = 8.0 + (next() % 2800) as f32 / 100.0;
            s.line(x, y, x + 0.4, y + len, "#00000026", 0.5);
        }
        s.line(0.5, 0.0, 0.5, 510.0, "#ffffff18", 1.0);
        s.line(15.5, 0.0, 15.5, 510.0, "#00000040", 1.0);
    }));
}

fn transport_bg(out: &mut Vec<Part>) {
    out.push(Part::new("bg_transport", 592.0, 50.0, |s| {
        face(s, 592.0, 50.0, pal::SLATE);
        let dim = pal::INK_DIM;
        // centered with its knob in the space left of the tempo display (0..44)
        label(s, 22.0, 38.0, 3.4, dim, "SHUFFLE");
        label(s, 57.5, 21.5, 3.4, dim, "TEMPO");
        for x in [44.0, 271.0, 329.0, 372.0] {
            well(s, x, 27.0, 27.0, 16.0);
        }
        text(s, 158.0, 11.5, 4.0, Anchor::End, pal::INK, "PATTERN");
        text(s, 190.0, 11.5, 4.0, Anchor::Start, pal::INK, "SONG");
        label(s, 284.5, 21.5, 3.4, dim, "BAR");
        label(s, 342.5, 21.5, 3.4, dim, "START");
        label(s, 385.5, 21.5, 3.4, dim, "LENGTH");
        s.line(263.0, 8.0, 263.0, 42.0, pal::SLATE_LINE, 1.0);
    }));
}

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

/// Our own wordmark, `OPENRB-338`: the whole, and its pieces (`LOGO_PIECES`) for task-114.
/// The underline is four segments in the 808's step colors.
fn logo(out: &mut Vec<Part>) {
    let cap = 14.0;
    let x0 = 8.0;
    // 'OPEN' and 'RB-' are one INK run in the whole word: each piece draws its glyphs of that
    // run (`text_range`), so the pieces sit exactly where the whole put them.
    let x_338 = x0 + font::text_width(cap, "OPENRB-") + font::gap(cap);
    let run = |s: &mut Svg, from, to| {
        font::text_range(
            s,
            x0,
            17.0,
            cap,
            Anchor::Start,
            pal::INK,
            "OPENRB-",
            from..to,
        );
    };
    let open = |s: &mut Svg| run(s, 0, 4);
    let rb338 = |s: &mut Svg| {
        run(s, 4, 7);
        text(s, x_338, 17.0, cap, Anchor::Start, pal::ORANGE, "338");
    };
    let line = |s: &mut Svg, i: usize| {
        s.rect(8.0 + 29.9 * i as f32, 28.0, 28.4, 1.5, 0.75, STEP_COLORS[i]);
    };
    let sub = |s: &mut Svg| {
        let y = 37.0;
        text(
            s,
            8.0,
            y,
            3.4,
            Anchor::Start,
            pal::INK_DIM,
            "BASS LINE AND DRUM MACHINE",
        );
    };
    out.push(Part::new("logo", LOGO_W, LOGO_H, |s| {
        open(s);
        rb338(s);
        for i in 0..4 {
            line(s, i);
        }
        sub(s);
    }));
    out.push(Part::new("logo_open", LOGO_W, LOGO_H, open));
    out.push(Part::new("logo_rb338", LOGO_W, LOGO_H, rb338));
    for i in 0..4 {
        out.push(Part::new(format!("logo_line_{i}"), LOGO_W, LOGO_H, |s| {
            line(s, i)
        }));
    }
    out.push(Part::new("logo_sub", LOGO_W, LOGO_H, sub));
}

fn pattern_bg(out: &mut Vec<Part>) {
    out.push(Part::new("bg_pattern", 70.0, 115.0, |s| {
        face(s, 70.0, 115.0, pal::SLATE);
        header(s, 2.0, 2.0, 66.0, 17.0, PATTERN_HEADER);
        // centered between the on/off button (4..17) and the header's right edge
        label(s, 42.5, 10.5, 4.0, pal::WHITE, "PATTERN");
        label(s, 45.5, 91.3, 3.2, pal::INK_DIM, "STEPS");
        well(s, 36.0, 95.0, 19.0, 16.0);
    }));
    for (name, fill) in [("focus_off", pal::SLATE_DARK), ("focus_on", pal::ORANGE)] {
        out.push(Part::new(name, 4.0, 115.0, |s| {
            s.rect(0.0, 0.0, 4.0, 115.0, 0.0, pal::BG);
            s.rect(1.0, 2.0, 2.0, 111.0, 1.0, fill);
        }));
    }
}

/// The 303's key band: three blocks of one height 6 px apart, (x, y, w, h): pitch mode and
/// clear, the keys, and the step flags with the edit step display. All span y 46..112.
pub const BAND_303: [(f32, f32, f32, f32); 3] = [
    (12.0, 46.0, 34.0, 66.0),
    (52.0, 46.0, 184.0, 66.0),
    (242.0, 46.0, 112.0, 66.0),
];
/// The left edges of the 303's white keys and its flags (DOWN, UP, ACCENT, SLIDE, NOTE),
/// the flags 22 px apart.
pub const WHITE_303: [f32; 8] = [66.0, 87.0, 108.0, 129.0, 150.0, 171.0, 192.0, 213.0];
pub const FLAGS_303: [f32; 5] = [250.5, 272.5, 294.5, 316.5, 338.5];
/// The edit step display's left and top, in the flags block's top row (the display and its
/// arrows are 31 px wide).
pub const EDIT_303: (f32, f32) = (296.0, 50.0);
/// The first knob's left edge: the six knobs, 40 px apart, are centered in the paper.
pub const KNOB_X0_303: f32 = 95.0;
/// The center x of the pitch mode and clear block.
pub const LEFT_X_303: f32 = 29.0;

fn tb303_bg(out: &mut Vec<Part>) {
    out.push(Part::new("bg_tb303", 366.0, 115.0, |s| {
        face(s, 366.0, 115.0, pal::PAPER);
        let ink = pal::BLACK;
        // The waveform switch (40 x 14 at (15, 20)) is labeled like a knob.
        label(s, 35.0, 5.5, 3.6, ink, "WAVEFORM");
        for (k, name) in ["TUNE", "CUTOFF", "RESO", "ENV MOD", "DECAY", "ACCENT"]
            .into_iter()
            .enumerate()
        {
            label(s, KNOB_X0_303 + 15.0 + 40.0 * k as f32, 5.5, 3.6, ink, name);
        }
        // The key band: pitch mode and clear, the keys on gray, the flags.
        for (i, (x, y, w, h)) in BAND_303.into_iter().enumerate() {
            s.rect(x, y, w, h, 4.0, if i == 1 { "#7d8087" } else { pal::BLACK });
        }
        let (ex, ey) = EDIT_303;
        // the display is the flags block's top row, "EDIT STEP" left of it
        label(s, 279.5, ey + 8.0, 3.2, pal::INK, "EDIT STEP");
        well(s, ex, ey, 19.0, 16.0);
        let ink = pal::INK;
        label(s, LEFT_X_303, 51.5, 3.2, ink, "PITCH");
        label(s, LEFT_X_303, 88.5, 3.2, ink, "CLEAR");
        for (x, name) in FLAGS_303
            .into_iter()
            .zip(["DOWN", "UP", "ACCENT", "SLIDE", "NOTE"])
        {
            label(s, x + 4.5, 109.0, 3.2, ink, name);
        }
    }));
}

const LEVEL_808: [f32; 12] = [
    14.0, 36.0, 58.0, 80.0, 102.0, 124.0, 146.0, 168.0, 190.0, 212.0, 234.0, 256.0,
];
const NAMES_808: [&str; 12] = [
    "AC", "BD", "SD", "LT", "MT", "HT", "RS", "CP", "CB", "CY", "OH", "CH",
];
/// The mod's window (x, y, w, h) in the 808's space right of the legend.
pub const MOD_VIEW_808: (f32, f32, f32, f32) = (278.0, 5.0, 84.0, 105.0);

fn tr808_bg(out: &mut Vec<Part>) {
    out.push(Part::new("bg_tr808", 366.0, 115.0, |s| {
        face(s, 366.0, 115.0, pal::CHARCOAL);
        let ink = pal::INK;
        for x in LEVEL_808 {
            label(s, x + 6.5, 5.0, 2.8, ink, "LEVEL");
        }
        for (x, t) in [
            (36.0, "TONE"),
            (58.0, "TONE"),
            (80.0, "TUNE"),
            (102.0, "TUNE"),
            (124.0, "TUNE"),
            (212.0, "TONE"),
        ] {
            label(s, x + 6.5, 27.7, 2.8, ink, t);
        }
        for (x, t) in [
            (36.0, "DECAY"),
            (58.0, "SNAPPY"),
            (212.0, "DECAY"),
            (234.0, "DECAY"),
        ] {
            label(s, x + 6.5, 50.7, 2.8, ink, t);
        }
        for (k, t) in ["LC", "MC", "HC", "CL", "MA"].into_iter().enumerate() {
            label(s, 86.5 + 22.0 * k as f32, 49.0, 3.0, pal::INK_DIM, t);
        }
        // The legend row: 12 cells of 22 px.
        for (i, name) in NAMES_808.into_iter().enumerate() {
            let x = 10.0 + 22.0 * i as f32;
            s.rect(x + 0.5, 71.0, 21.0, 14.0, 2.0, "#1c1d1f");
            label(s, x + 11.0, 78.0, 3.8, ink, name);
        }
    }));
    // The mod's window right of the legend: a dark well for the mod's splash.jpg and its
    // name, which the app paints into it (MOD_VIEW_808).
    let (_, _, w, h) = MOD_VIEW_808;
    out.push(Part::new("mod_view", w, h, move |s| {
        s.rect(0.0, 0.0, w, h, 2.0, "#101216");
        s.rect_stroke(0.25, 0.25, w - 0.5, h - 0.5, 2.0, "#000000", 0.5);
    }));
}

/// The 909's instrument groups: their left edges in section pixels.
pub const EDGES_909: [f32; 13] = [
    4.0, 26.0, 68.0, 110.0, 152.0, 194.0, 236.0, 257.0, 278.0, 299.0, 320.0, 341.0, 362.0,
];
const NAMES_909: [&str; 12] = [
    "AC", "BD", "SD", "LT", "MT", "HT", "RS", "CP", "CH", "OH", "CC", "RC",
];

/// Where a 909 knob sits: (instrument group, column, lower row, id, label). The column of a
/// two-knob-wide group is 0 (left half) or 1 (right half); 2 is the whole group.
pub const KNOBS_909: [(usize, usize, bool, &str, &str); 28] = [
    (0, 2, false, "ac_level", "LEVEL"),
    (1, 0, false, "bd_tune", "TUNE"),
    (1, 0, true, "bd_attack", "ATTACK"),
    (1, 1, false, "bd_level", "LEVEL"),
    (1, 1, true, "bd_decay", "DECAY"),
    (2, 0, false, "sd_tune", "TUNE"),
    (2, 0, true, "sd_tone", "TONE"),
    (2, 1, false, "sd_level", "LEVEL"),
    (2, 1, true, "sd_snappy", "SNAPPY"),
    (3, 0, false, "lt_tune", "TUNE"),
    (3, 0, true, "lt_decay", "DECAY"),
    (3, 1, false, "lt_level", "LEVEL"),
    (4, 0, false, "mt_tune", "TUNE"),
    (4, 0, true, "mt_decay", "DECAY"),
    (4, 1, false, "mt_level", "LEVEL"),
    (5, 0, false, "ht_tune", "TUNE"),
    (5, 0, true, "ht_decay", "DECAY"),
    (5, 1, false, "ht_level", "LEVEL"),
    (6, 2, false, "rs_level", "LEVEL"),
    (7, 2, false, "cp_level", "LEVEL"),
    (8, 2, true, "ch_decay", "DECAY"),
    (9, 2, false, "hh_level", "LEVEL"),
    (9, 2, true, "oh_decay", "DECAY"),
    (10, 2, false, "cc_level", "LEVEL"),
    (10, 2, true, "cc_tune", "TUNE"),
    (11, 2, false, "rc_level", "LEVEL"),
    (11, 2, true, "rc_tune", "TUNE"),
    (0, 2, true, "flam", "FLAM"),
];
/// The 909's rows: the knobs' tops (upper, lower), the name row's top and the steps' top.
pub const KNOB_Y_909: [f32; 2] = [29.0, 56.0];
pub const NAMES_Y_909: f32 = 74.0;
pub const STEPS_Y_909: f32 = 94.0;
/// The FLAM knob's top: at the bottom of the AC column, 3 px above the flam button.
const FLAM_Y_909: f32 = 80.5;

/// The top of a 909 knob (see `KNOBS_909`).
pub fn knob_y_909(group: usize, lower: bool) -> f32 {
    if group == 0 && lower {
        FLAM_Y_909
    } else {
        KNOB_Y_909[usize::from(lower)]
    }
}

/// The center x of a 909 knob column (see `KNOBS_909`).
pub fn column_909(group: usize, col: usize) -> f32 {
    let (x0, x1) = (EDGES_909[group], EDGES_909[group + 1]);
    match col {
        0 => x0 + (x1 - x0) / 4.0,
        1 => x0 + 3.0 * (x1 - x0) / 4.0,
        _ => (x0 + x1) / 2.0,
    }
}

fn tr909_bg(out: &mut Vec<Part>) {
    out.push(Part::new("bg_tr909", 366.0, 115.0, |s| {
        face(s, 366.0, 115.0, pal::CREAM);
        let ink = pal::BLACK;
        for (i, name) in NAMES_909.into_iter().enumerate() {
            let (x0, x1) = (EDGES_909[i], EDGES_909[i + 1]);
            s.rect(x0 + 0.5, 4.0, x1 - x0 - 1.0, 14.0, 2.0, "#e0a06a");
            label(s, (x0 + x1) / 2.0, 11.0, 3.8, ink, name);
            if i > 0 {
                let y = NAMES_Y_909;
                s.rect(x0 + 0.5, y, x1 - x0 - 1.0, 15.0, 2.0, "#cfcabd");
                label(s, (x0 + x1) / 2.0, y + 7.5, 3.6, ink, name);
                s.line(x0, 21.0, x0, 70.0, "#c4c0b5", 0.8);
            }
        }
        for (group, col, lower, _, t) in KNOBS_909 {
            let y = knob_y_909(group, lower) - 3.7;
            label(s, column_909(group, col), y, 2.8, ink, t);
        }
    }));
}

fn mixer_bg(out: &mut Vec<Part>) {
    out.push(Part::new("bg_mixer", 70.0, 115.0, |s| {
        face(s, 70.0, 115.0, pal::SLATE);
        header(s, 2.0, 2.0, 66.0, 18.0, MIXER_HEADER);
        // centered between the on/off button (4..18) and the meter (58..66)
        label(s, 38.0, 11.0, 4.4, pal::WHITE, "MIX");
        // pan and delay send side by side; the fader and the routing keys below
        label(s, 19.0, 52.5, 3.0, pal::INK_DIM, "PAN");
        label(s, 51.0, 52.5, 3.0, pal::INK_DIM, "DELAY");
    }));
}

fn master_bg(out: &mut Vec<Part>) {
    out.push(Part::new("bg_master", 82.0, 98.0, |s| {
        face(s, 82.0, 98.0, pal::SLATE);
        header(s, 2.0, 2.0, 78.0, 16.0, pal::GOLD);
        label(s, 41.0, 10.0, 4.4, pal::BLACK, "MASTER");
        for (f, t) in [
            (0.06, "CLIP"),
            (0.24, "-6"),
            (0.44, "-12"),
            (0.68, "-24"),
            (0.92, "-36"),
        ] {
            let y = 24.0 + 56.0 * f;
            text(s, 14.5, y, 2.8, Anchor::End, pal::INK_DIM, t);
            text(s, 67.5, y, 2.8, Anchor::Start, pal::INK_DIM, t);
        }
    }));
}

/// An effect's header: the title centered between the power button (4..18) and the meter
/// (70..78).
fn fx_head(s: &mut Svg, w: f32, title: &str) {
    header(s, 2.0, 2.0, w - 4.0, 18.0, pal::BLUE);
    label(s, 44.0, 11.0, 4.4, pal::WHITE, title);
}

fn fx_bg(out: &mut Vec<Part>) {
    out.push(Part::new("bg_pcf", 82.0, 111.0, |s| {
        face(s, 82.0, 111.0, pal::SLATE);
        fx_head(s, 82.0, "PCF");
        well(s, 10.0, 23.0, 18.0, 16.0);
        label(s, 19.0, 43.5, 2.8, pal::INK_DIM, "PATTERN");
        text(s, 60.0, 26.5, 3.2, Anchor::Start, pal::INK_DIM, "BP");
        text(s, 60.0, 35.5, 3.2, Anchor::Start, pal::INK_DIM, "LP");
        label(s, 53.5, 43.5, 2.8, pal::INK_DIM, "MODE");
        for (x, t) in [(11.5, "FREQ"), (30.5, "Q"), (49.5, "AMT"), (68.5, "DEC")] {
            label(s, x, 104.0, 3.0, pal::INK_DIM, t);
        }
    }));
    out.push(Part::new("bg_delay", 82.0, 93.0, |s| {
        face(s, 82.0, 93.0, pal::SLATE);
        fx_head(s, 82.0, "DELAY");
        well(s, 9.0, 23.0, 19.0, 16.0);
        label(s, 18.5, 43.5, 2.8, pal::INK_DIM, "STEPS");
        text(s, 60.0, 26.5, 3.2, Anchor::Start, pal::INK_DIM, "3");
        text(s, 60.0, 35.5, 3.2, Anchor::Start, pal::INK_DIM, "16");
        label(s, 53.5, 43.5, 2.8, pal::INK_DIM, "MODE");
        label(s, 21.0, 81.5, 3.0, pal::INK_DIM, "PAN");
        label(s, 62.0, 81.5, 3.0, pal::INK_DIM, "FEEDBACK");
    }));
    out.push(Part::new("bg_dist", 82.0, 65.0, |s| {
        face(s, 82.0, 65.0, pal::SLATE);
        fx_head(s, 82.0, "DIST");
        label(s, 21.0, 53.5, 3.0, pal::INK_DIM, "AMOUNT");
        label(s, 62.0, 53.5, 3.0, pal::INK_DIM, "SHAPE");
    }));
    out.push(Part::new("bg_comp", 82.0, 93.0, |s| {
        face(s, 82.0, 93.0, pal::SLATE);
        fx_head(s, 82.0, "COMP");
        // under the reduction meter's LEDs 1, 3, 5, 7 and 9 (centered at 41)
        for (i, t) in ["-40", "-20", "0", "+20", "+40"].into_iter().enumerate() {
            label(s, 13.0 + 14.0 * i as f32, 39.5, 2.8, pal::INK_DIM, t);
        }
        label(s, 21.0, 80.0, 3.0, pal::INK_DIM, "RATIO");
        label(s, 62.0, 80.0, 3.0, pal::INK_DIM, "THRES");
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
    switches(&mut v);
    power_buttons(&mut v);
    transport_buttons(&mut v);
    pattern_buttons(&mut v);
    small_buttons(&mut v);
    leds(&mut v);
    steps(&mut v);
    digits(&mut v);
    meters(&mut v);
    app_icon(&mut v);
    v
}
