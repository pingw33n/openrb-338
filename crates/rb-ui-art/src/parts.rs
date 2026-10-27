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

pub const KNOBS: [KnobDesign; 6] = [
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
    KnobDesign {
        name: "knob_sel",
        w: 38.0,
        h: 38.0,
        cx: 19.0,
        cy: 19.0,
        ring_r: 0.0,
        ring_w: 0.0,
        body_r: 16.5,
        ptr0: 4.0,
        ptr1: 14.5,
        ptr_w: 2.4,
        body: pal::BLACK,
        pointer: pal::RED,
        arc: pal::ORANGE,
        track: "#00000000",
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
            if k.ring_r > 0.0 {
                s.path_stroke(
                    &arc_path(k.cx, k.cy, k.ring_r, -KNOB_SWEEP, KNOB_SWEEP),
                    k.track,
                    k.ring_w,
                );
            }
            s.circle(k.cx, k.cy, k.body_r, k.body);
            s.circle_stroke(k.cx, k.cy, k.body_r - 0.6, "#ffffff2a", 0.5);
            if k.ring_r == 0.0 {
                // The selector knob: a darker cap inside the rim.
                s.circle(k.cx, k.cy, k.body_r - 3.5, "#2c2c2e");
            }
        }));
        out.push(Part::new(format!("{}_pointer", k.name), k.w, k.h, |s| {
            // Drawn pointing up; the app rotates it around (cx, cy).
            let (x0, y0) = polar(k.cx, k.cy, k.ptr0, 0.0);
            let (x1, y1) = polar(k.cx, k.cy, k.ptr1, 0.0);
            s.line(x0, y0, x1, y1, k.pointer, k.ptr_w);
        }));
    }
}

/// The app icon: a 303 knob turned to three quarters on a slate tile, 64 x 64 with the
/// tile inset like a macOS icon's.
fn app_icon(out: &mut Vec<Part>) {
    out.push(Part::new("app_icon", 64.0, 64.0, |s| {
        s.rect(6.0, 6.0, 52.0, 52.0, 12.0, pal::SLATE_DARK);
        s.rect(6.0, 6.0, 52.0, 26.0, 12.0, "#ffffff0c");
        s.rect_stroke(6.5, 6.5, 51.0, 51.0, 11.5, "#ffffff1a", 1.0);
        let (cx, cy) = (32.0, 33.0);
        let a = KNOB_SWEEP * 0.5;
        s.path_stroke(
            &arc_path(cx, cy, 19.5, -KNOB_SWEEP, KNOB_SWEEP),
            "#00000050",
            3.4,
        );
        s.path_stroke(&arc_path(cx, cy, 19.5, -KNOB_SWEEP, a), pal::ORANGE, 3.4);
        s.circle(cx, cy + 0.8, 15.0, "#00000040");
        s.circle(cx, cy, 15.0, pal::STEEL);
        s.circle_stroke(cx, cy, 14.2, "#ffffff40", 0.8);
        let (x0, y0) = polar(cx, cy, 4.0, a);
        let (x1, y1) = polar(cx, cy, 12.5, a);
        s.line(x0, y0, x1, y1, pal::BLACK, 3.2);
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

fn switches(out: &mut Vec<Part>) {
    switch(out, "switch_wave", 16.0, 12.0, true);
    switch(out, "switch_808", 11.0, 17.0, false);
    switch(out, "switch_fx", 9.0, 18.0, false);
    switch(out, "switch_mode", 26.0, 19.0, true);
    // Dist, PCF and Comp routing switches of the mixer (and the master's).
    for (kind, text_) in [("dist", "DIST"), ("pcf", "PCF"), ("comp", "COMP")] {
        for (state, on) in [("off", false), ("on", true)] {
            out.push(Part::new(
                format!("mixsw_{kind}_{state}"),
                20.0,
                11.0,
                |s| {
                    s.rect(
                        0.5,
                        0.0,
                        19.0,
                        11.0,
                        2.0,
                        if on { pal::AMBER } else { pal::SLATE_DARK },
                    );
                    label(
                        s,
                        10.0,
                        5.5,
                        4.0,
                        if on { pal::BLACK } else { pal::INK },
                        text_,
                    );
                },
            ));
        }
    }
}

fn power_buttons(out: &mut Vec<Part>) {
    // The pattern sections' (13) and the mixer's and effects' (14) on/off buttons.
    for (w, h) in [(13.0, 13.0), (14.0, 14.0)] {
        for (state, on) in [("off", false), ("on", true)] {
            out.push(Part::new(
                format!("power_{}x{}_{state}", n(w), n(h)),
                w,
                h,
                // flat: the lamp alone
                |s| {
                    s.rect(
                        0.0,
                        0.0,
                        w,
                        h,
                        2.0,
                        if on { pal::GREEN } else { pal::GREEN_DIM },
                    );
                },
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
                s.rect(
                    0.5,
                    0.5,
                    29.0,
                    11.0,
                    2.5,
                    if on { pal::RED } else { "#dfe3e8" },
                );
                label(
                    s,
                    15.0,
                    6.0,
                    3.2,
                    if on { pal::WHITE } else { pal::BLACK },
                    "SHUFFLE",
                );
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
            out.push(Part::new(format!("pat_btn_{k}_{state}"), 17.0, 20.0, |s| {
                s.rect(
                    1.0,
                    1.0,
                    15.0,
                    18.0,
                    2.5,
                    if on { pal::RED } else { "#dfe3e8" },
                );
                label(
                    s,
                    8.5,
                    10.0,
                    7.0,
                    if on { pal::WHITE } else { pal::BLACK },
                    &k.to_string(),
                );
            }));
        }
    }
    for bank in ["A", "B", "C", "D"] {
        for (state, on) in [("off", false), ("on", true)] {
            out.push(Part::new(
                format!("bank_btn_{bank}_{state}"),
                17.0,
                20.0,
                |s| {
                    s.rect(
                        1.0,
                        1.0,
                        15.0,
                        18.0,
                        2.5,
                        if on { pal::ORANGE } else { "#dfe3e8" },
                    );
                    label(s, 8.5, 10.0, 7.0, pal::BLACK, bank);
                },
            ));
        }
    }
}

fn small_buttons(out: &mut Vec<Part>) {
    for (w, h) in [(17.0, 10.0)] {
        for state in ["rest", "pressed"] {
            out.push(Part::new(
                format!("btn_pill_{}x{}_{state}", n(w), n(h)),
                w,
                h,
                |s| {
                    s.rect(0.5, 0.5, w - 1.0, h - 1.0, 2.0, pal::STEEL);
                    s.rect_stroke(0.5, 0.5, w - 1.0, h - 1.0, 2.0, "#00000050", 0.6);
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
            s.rect(0.5, 0.0, 8.0, 16.0, 1.5, pal::WHITE);
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
            s.rect(0.5, 0.0, 8.0, 16.0, 1.8, pal::STEEL);
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

fn header(s: &mut Svg, x: f32, y: f32, w: f32, h: f32, fill: &str) {
    s.rect(x, y, w, h, 2.0, fill);
}

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
        // The logo: our own wordmark.
        s.g_at(458.0, 1.0);
        let cap = 14.0;
        let x0 = 8.0;
        text(s, x0, 17.0, cap, Anchor::Start, pal::INK, "OPENRB-");
        let x1 = x0 + font::text_width(cap, "OPENRB-") + font::gap(cap);
        text(s, x1, 17.0, cap, Anchor::Start, pal::ORANGE, "338");
        s.rect(8.0, 28.0, 118.0, 1.5, 0.75, pal::ORANGE);
        text(
            s,
            8.0,
            37.0,
            3.4,
            Anchor::Start,
            dim,
            "BASS LINE AND DRUM MACHINE",
        );
        s.end_g();
    }));
}

fn pattern_bg(out: &mut Vec<Part>) {
    out.push(Part::new("bg_pattern", 70.0, 115.0, |s| {
        face(s, 70.0, 115.0, pal::SLATE);
        header(s, 2.0, 2.0, 66.0, 17.0, pal::RED);
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

/// The 303's key band: three sections 6 px apart, (x, y, w, h): pitch mode and clear, the
/// keys, and one row of the step flags and note/rest under the edit step display. All end
/// at y 112.
pub const BAND_303: [(f32, f32, f32, f32); 3] = [
    (12.0, 46.0, 44.0, 66.0),
    (62.0, 46.0, 172.0, 66.0),
    (240.0, 74.0, 114.0, 38.0),
];
/// The left edges of the 303's white keys and its flags (DOWN, UP, ACCENT, SLIDE, NOTE),
/// the flags 22 px apart and centered in the band's right section.
pub const WHITE_303: [f32; 8] = [70.0, 91.0, 112.0, 133.0, 154.0, 175.0, 196.0, 217.0];
pub const FLAGS_303: [f32; 5] = [248.5, 270.5, 292.5, 314.5, 336.5];
/// The edit step display's left and top, centered over the band's right section (the
/// display and its arrows are 31 px wide).
pub const EDIT_303: (f32, f32) = (281.5, 55.0);

fn tb303_bg(out: &mut Vec<Part>) {
    out.push(Part::new("bg_tb303", 366.0, 115.0, |s| {
        face(s, 366.0, 115.0, pal::PAPER);
        let ink = pal::BLACK;
        // The waveform switch between saw (left) and square (right), level with the knobs;
        // the switch is centered over pitch mode and clear (x 34).
        s.path_stroke("M15.5 30 L19.5 24 V30 L23.5 24 V30", ink, 1.0);
        s.path_stroke("M45.5 30 V24 H48 V30 H50.5 V24 H52.5", ink, 1.0);
        for (k, name) in ["TUNE", "CUTOFF", "RESO", "ENV MOD", "DECAY", "ACCENT"]
            .into_iter()
            .enumerate()
        {
            label(s, 81.0 + 40.0 * k as f32, 5.5, 3.6, ink, name);
        }
        let (ex, ey) = EDIT_303;
        label(s, ex + 15.5, ey - 5.5, 3.4, ink, "EDIT STEP");
        well(s, ex, ey, 19.0, 16.0);
        // The key band: pitch mode and clear, the keys on gray, the flags.
        for (i, (x, y, w, h)) in BAND_303.into_iter().enumerate() {
            s.rect(x, y, w, h, 4.0, if i == 1 { "#7d8087" } else { pal::BLACK });
        }
        let ink = pal::INK;
        label(s, 34.0, 51.5, 3.2, ink, "PITCH");
        label(s, 34.0, 87.5, 3.2, ink, "CLEAR");
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
/// The center of the 808's instrument selector, in the space right of the legend.
pub const SELECTOR_808: (f32, f32) = (320.0, 57.0);

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
        // The instrument selector's ring of 12 positions, at the angles its pointer snaps
        // to (the knob's -135..135 sweep), centered right of the legend.
        let (cx, cy) = SELECTOR_808;
        for (i, name) in NAMES_808.into_iter().enumerate() {
            let a = -KNOB_SWEEP + i as f32 * 2.0 * KNOB_SWEEP / 11.0;
            let (x0, y0) = polar(cx, cy, 20.0, a);
            let (x1, y1) = polar(cx, cy, 22.5, a);
            s.line(x0, y0, x1, y1, pal::INK_DIM, 0.9);
            let (tx, ty) = polar(cx, cy, 28.0, a);
            label(s, tx, ty, 3.2, ink, name);
        }
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
        header(s, 2.0, 2.0, 66.0, 18.0, pal::OLIVE);
        // centered between the on/off button (4..18) and the meter (58..66)
        label(s, 38.0, 11.0, 4.4, pal::WHITE, "MIX");
        label(s, 19.0, 51.0, 3.0, pal::INK_DIM, "PAN");
        label(s, 51.0, 102.5, 3.0, pal::INK_DIM, "DELAY");
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
