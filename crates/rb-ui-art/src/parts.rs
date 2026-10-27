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
        name: "knob_18",
        w: 18.0,
        h: 19.0,
        cx: 9.0,
        cy: 9.5,
        ring_r: 8.3,
        ring_w: 1.2,
        body_r: 6.5,
        ptr0: 2.0,
        ptr1: 5.5,
        ptr_w: 1.3,
        body: pal::SLATE_DARK,
        pointer: pal::INK,
        arc: pal::AMBER,
        track: "#00000040",
    },
    KnobDesign {
        name: "knob_sel",
        w: 29.0,
        h: 31.0,
        cx: 14.5,
        cy: 15.5,
        ring_r: 0.0,
        ring_w: 0.0,
        body_r: 12.5,
        ptr0: 3.0,
        ptr1: 11.0,
        ptr_w: 1.9,
        body: pal::BLACK,
        pointer: pal::RED,
        arc: pal::ORANGE,
        track: "#00000000",
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
                24.0,
                11.0,
                |s| {
                    s.rect(
                        0.5,
                        0.0,
                        23.0,
                        11.0,
                        2.0,
                        if on { pal::AMBER } else { pal::SLATE_DARK },
                    );
                    label(
                        s,
                        12.0,
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
    for (w, h) in [(13.0, 14.0), (17.0, 15.0), (16.0, 16.0)] {
        for (state, on) in [("off", false), ("on", true)] {
            out.push(Part::new(
                format!("power_{}x{}_{state}", n(w), n(h)),
                w,
                h,
                |s| {
                    s.rect(0.0, 0.0, w, h, 2.5, "#15181d");
                    let (iw, ih) = (w - 6.0, h - 6.0);
                    if on {
                        s.rect(2.0, 2.0, w - 4.0, h - 4.0, 2.0, "#53d86a30");
                    }
                    s.rect(
                        3.0,
                        3.0,
                        iw,
                        ih,
                        1.5,
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
        out.push(Part::new(format!("toggle_loop_{state}"), 17.0, 21.0, |s| {
            s.rect(
                0.5,
                0.5,
                16.0,
                20.0,
                3.0,
                if on { pal::AMBER } else { pal::SLATE_DARK },
            );
            let ink = if on { pal::BLACK } else { pal::INK };
            // A loop arrow: a rounded rectangle path with an arrow head.
            s.path_stroke(
                "M4.5 9 V7.5 Q4.5 5.5 6.5 5.5 H11.5 Q13 5.5 13 7.5",
                ink,
                1.3,
            );
            s.path_stroke(
                "M12.5 12 V13.5 Q12.5 15.5 10.5 15.5 H5.5 Q4 15.5 4 13.5",
                ink,
                1.3,
            );
            s.path("M10.5 3 L14.5 7 L10.5 11 Z", ink);
            s.path("M6.5 17.5 L2.5 13.5 L6.5 9.5 Z", ink);
        }));
        out.push(Part::new(
            format!("toggle_shuffle_{state}"),
            18.0,
            20.0,
            |s| {
                s.rect(
                    0.5,
                    0.5,
                    17.0,
                    19.0,
                    3.0,
                    if on { pal::RED } else { "#dfe3e8" },
                );
                label(
                    s,
                    9.0,
                    10.0,
                    3.6,
                    if on { pal::WHITE } else { pal::BLACK },
                    "SHUF",
                );
            },
        ));
        out.push(Part::new(format!("toggle_flam_{state}"), 14.0, 14.0, |s| {
            s.rect(
                0.5,
                0.5,
                13.0,
                13.0,
                2.5,
                if on { pal::RED } else { "#c8c4b8" },
            );
            s.circle(7.0, 7.0, 2.2, if on { pal::WHITE } else { "#8f8b80" });
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
                30.0,
                |s| {
                    s.rect(
                        1.0,
                        3.0,
                        15.0,
                        24.0,
                        2.5,
                        if on { pal::ORANGE } else { "#dfe3e8" },
                    );
                    label(s, 8.5, 15.0, 7.0, pal::BLACK, bank);
                },
            ));
        }
    }
}

fn small_buttons(out: &mut Vec<Part>) {
    for (w, h) in [(15.0, 8.0), (17.0, 10.0)] {
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
        out.push(Part::new(format!("btn_step_{state}"), 25.0, 17.0, |s| {
            s.rect(0.5, 0.5, 24.0, 16.0, 3.0, pal::PAPER_DARK);
            s.path("M8.5 4.5 L16.5 8.5 L8.5 12.5 Z", pal::BLACK);
            s.rect(17.5, 4.5, 1.8, 8.0, 0.6, pal::BLACK);
            if state == "pressed" {
                s.rect(0.5, 0.5, 24.0, 16.0, 3.0, "#00000040");
            }
        }));
        out.push(Part::new(format!("key_white_{state}"), 9.0, 16.0, |s| {
            s.rect(0.5, 0.0, 8.0, 16.0, 1.5, pal::WHITE);
            if state == "pressed" {
                s.rect(0.5, 0.0, 8.0, 16.0, 1.5, "#80808066");
            }
        }));
        out.push(Part::new(format!("key_black_{state}"), 8.0, 15.0, |s| {
            s.rect(0.0, 0.0, 8.0, 15.0, 1.5, "#4a4d54");
            s.rect(1.0, 0.0, 6.0, 12.5, 1.0, "#2a2c30");
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
    for (state, color) in [("off", "#3a3f48"), ("green", pal::GREEN), ("red", pal::RED)] {
        out.push(Part::new(format!("led_11_{state}"), 11.0, 11.0, |s| {
            s.circle(5.5, 5.5, 4.6, color);
            s.circle_stroke(5.5, 5.5, 4.6, "#00000040", 0.6);
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
    for (w, h) in [(22.0, 14.0), (42.0, 14.0), (21.0, 14.0), (22.0, 18.0)] {
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
    for (w, h) in [(14.0, 15.0), (13.0, 16.0), (15.0, 56.0)] {
        for level in 0..=10 {
            out.push(Part::new(
                format!("meter_{}x{}_{level}", n(w), n(h)),
                w,
                h,
                |s| {
                    s.rect(0.0, 0.0, w, h, 1.5, "#101216");
                    let seg_h = (h - 3.0) / 10.0;
                    for i in 0..10 {
                        let y = h - 1.5 - (i as f32 + 1.0) * seg_h;
                        let color = match i {
                            0..=5 => pal::GREEN,
                            6..=7 => pal::AMBER,
                            _ => pal::RED,
                        };
                        let on = i < level;
                        s.rect(
                            1.5,
                            y + 0.25,
                            w - 3.0,
                            seg_h - 0.5,
                            0.3,
                            if on { color } else { "#ffffff14" },
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
        label(s, 20.0, 38.5, 3.4, dim, "SHUFFLE");
        text(s, 52.0, 7.5, 3.4, Anchor::Start, dim, "SYNC");
        text(s, 87.0, 7.5, 3.4, Anchor::Start, dim, "MIDI");
        label(s, 57.5, 21.5, 3.4, dim, "TEMPO");
        for x in [44.0, 271.0, 329.0, 372.0] {
            well(s, x, 27.0, 27.0, 16.0);
        }
        text(s, 158.0, 11.5, 4.0, Anchor::End, pal::INK, "PATTERN");
        text(s, 190.0, 11.5, 4.0, Anchor::Start, pal::INK, "SONG");
        label(s, 284.5, 21.5, 3.4, dim, "BAR");
        text(s, 336.0, 12.5, 4.0, Anchor::Start, pal::INK, "LOOP");
        label(s, 342.5, 21.5, 3.4, dim, "START");
        label(s, 385.5, 21.5, 3.4, dim, "LENGTH");
        s.line(263.0, 8.0, 263.0, 42.0, pal::SLATE_LINE, 1.0);
        // The logo: our own wordmark.
        s.g_at(458.0, 1.0);
        s.rect(0.0, 0.0, 134.0, 48.0, 3.0, pal::SLATE_DARK);
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
        text(s, 21.0, 10.5, 4.0, Anchor::Start, pal::WHITE, "PATTERN");
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

fn tb303_bg(out: &mut Vec<Part>) {
    out.push(Part::new("bg_tb303", 366.0, 115.0, |s| {
        face(s, 366.0, 115.0, pal::PAPER);
        let ink = pal::BLACK;
        label(s, 32.0, 5.5, 3.6, ink, "WAVEFORM");
        // Saw (left position) and square (right position) under the switch.
        s.path_stroke("M19.5 35 L23.5 29 V35 L27.5 29 V35", ink, 1.0);
        s.path_stroke("M36.5 35 V29 H39 V35 H41.5 V29 H43.5", ink, 1.0);
        for (k, name) in ["TUNE", "CUTOFF", "RESO", "ENV MOD", "DECAY", "ACCENT"]
            .into_iter()
            .enumerate()
        {
            label(s, 87.0 + 33.0 * k as f32, 5.5, 3.6, ink, name);
        }
        text(s, 284.0, 7.0, 4.0, Anchor::Start, ink, "BASS SYNTH");
        label(s, 339.5, 19.5, 3.4, ink, "EDIT STEP");
        well(s, 330.0, 25.0, 19.0, 16.0);
        label(s, 336.5, 57.0, 3.2, ink, "BACK");
        label(s, 337.5, 111.0, 3.2, ink, "STEP");
        // The key band.
        s.rect(12.0, 49.0, 304.0, 64.0, 4.0, pal::BLACK);
        let ink = pal::INK;
        label(s, 27.5, 58.5, 3.2, ink, "PITCH");
        label(s, 27.5, 87.0, 3.2, ink, "MODE");
        label(s, 26.5, 92.8, 3.2, ink, "CLEAR");
        for (x, name) in [
            (228.5, "DOWN"),
            (253.5, "UP"),
            (278.5, "ACCENT"),
            (303.5, "SLIDE"),
        ] {
            label(s, x, 109.0, 3.2, ink, name);
        }
        label(s, 250.0, 54.5, 3.2, ink, "NOTE");
        label(s, 288.0, 54.5, 3.2, ink, "REST");
        label(s, 302.5, 73.5, 3.0, ink, "NOTE/REST");
    }));
}

const LEVEL_808: [f32; 12] = [
    14.0, 36.0, 58.0, 80.0, 102.0, 124.0, 146.0, 168.0, 190.0, 212.0, 234.0, 256.0,
];
const NAMES_808: [&str; 12] = [
    "AC", "BD", "SD", "LT", "MT", "HT", "RS", "CP", "CB", "CY", "OH", "CH",
];

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
            (80.0, "TUNING"),
            (102.0, "TUNING"),
            (124.0, "TUNING"),
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
        // The instrument selector's ring of 12 positions.
        let (cx, cy) = (320.5, 56.5);
        for (i, name) in NAMES_808.into_iter().enumerate() {
            let a = -150.0 + i as f32 * 300.0 / 11.0;
            let (x0, y0) = polar(cx, cy, 15.5, a);
            let (x1, y1) = polar(cx, cy, 17.5, a);
            s.line(x0, y0, x1, y1, pal::INK_DIM, 0.8);
            let (tx, ty) = polar(cx, cy, 22.0, a);
            label(s, tx, ty, 2.8, ink, name);
        }
        text(s, 292.0, 100.0, 3.6, Anchor::Start, ink, "DRUM MACHINE A");
    }));
}

/// The 909's instrument groups: their left edges in section pixels.
pub const EDGES_909: [f32; 13] = [
    4.0, 26.0, 68.0, 110.0, 152.0, 194.0, 236.0, 257.0, 278.0, 299.0, 320.0, 341.0, 362.0,
];
const NAMES_909: [&str; 12] = [
    "AC", "BD", "SD", "LT", "MT", "HT", "RS", "CP", "CH", "OH", "CC", "RC",
];

fn tr909_bg(out: &mut Vec<Part>) {
    out.push(Part::new("bg_tr909", 366.0, 115.0, |s| {
        face(s, 366.0, 115.0, pal::CREAM);
        let ink = pal::BLACK;
        for (i, name) in NAMES_909.into_iter().enumerate() {
            let (x0, x1) = (EDGES_909[i], EDGES_909[i + 1]);
            s.rect(x0 + 0.5, 4.0, x1 - x0 - 1.0, 14.0, 2.0, "#e0a06a");
            label(s, (x0 + x1) / 2.0, 11.0, 3.8, ink, name);
            if i > 0 {
                let y = 70.0;
                s.rect(x0 + 0.5, y, x1 - x0 - 1.0, 15.0, 2.0, "#cfcabd");
                label(s, (x0 + x1) / 2.0, y + 7.5, 3.6, ink, name);
                s.line(x0, 21.0, x0, 66.0, "#c4c0b5", 0.8);
            }
        }
        label(s, 15.0, 53.0, 3.0, ink, "ACCENT");
        let upper = [
            (9.0, "LEVEL"),
            (30.0, "TUNE"),
            (50.0, "LEVEL"),
            (72.0, "TUNE"),
            (92.0, "LEVEL"),
            (114.0, "TUNE"),
            (134.0, "LEVEL"),
            (156.0, "TUNE"),
            (176.0, "LEVEL"),
            (198.0, "TUNE"),
            (218.0, "LEVEL"),
            (240.0, "LEVEL"),
            (260.0, "LEVEL"),
            (302.0, "LEVEL"),
            (324.0, "LEVEL"),
            (344.0, "LEVEL"),
        ];
        for (x, t) in upper {
            label(s, x + 6.0, 24.3, 2.8, ink, t);
        }
        let lower = [
            (30.0, "ATTACK"),
            (50.0, "DECAY"),
            (72.0, "TONE"),
            (92.0, "SNAPPY"),
            (114.0, "DECAY"),
            (156.0, "DECAY"),
            (198.0, "DECAY"),
            (282.0, "DECAY"),
            (302.0, "DECAY"),
            (324.0, "TUNE"),
            (344.0, "TUNE"),
        ];
        for (x, t) in lower {
            label(s, x + 6.0, 50.3, 2.8, ink, t);
        }
        label(s, 15.0, 73.0, 3.0, ink, "FLAM");
        text(
            s,
            30.0,
            108.5,
            3.6,
            Anchor::Start,
            "#6a665e",
            "DRUM MACHINE B",
        );
    }));
}

fn mixer_bg(out: &mut Vec<Part>) {
    out.push(Part::new("bg_mixer", 70.0, 115.0, |s| {
        face(s, 70.0, 115.0, pal::SLATE);
        header(s, 2.0, 2.0, 66.0, 18.0, pal::OLIVE);
        label(s, 35.0, 11.5, 4.0, pal::WHITE, "ON");
        for i in 0..=10 {
            let y = 59.0 + 5.5 + i as f32 * 3.8;
            s.line(
                26.5,
                y,
                if i % 5 == 0 { 30.0 } else { 28.5 },
                y,
                pal::INK_DIM,
                0.6,
            );
        }
        label(s, 19.0, 47.0, 3.0, pal::INK_DIM, "PAN");
        label(s, 51.0, 101.0, 3.0, pal::INK_DIM, "DELAY");
    }));
}

fn master_bg(out: &mut Vec<Part>) {
    out.push(Part::new("bg_master", 82.0, 98.0, |s| {
        face(s, 82.0, 98.0, pal::SLATE);
        header(s, 2.0, 2.0, 78.0, 16.0, pal::GOLD);
        label(s, 41.0, 10.0, 4.4, pal::BLACK, "MASTER");
        for (f, t) in [
            (0.06, "0"),
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

/// An effect's header: the title between the power button and the meter.
fn fx_head(s: &mut Svg, w: f32, title: &str, cap: f32) {
    header(s, 2.0, 1.5, w - 4.0, 18.0, pal::BLUE);
    label(s, 41.5, 10.5, cap, pal::WHITE, title);
}

fn fx_bg(out: &mut Vec<Part>) {
    out.push(Part::new("bg_pcf", 82.0, 111.0, |s| {
        face(s, 82.0, 111.0, pal::SLATE);
        fx_head(s, 82.0, "PCF", 4.4);
        well(s, 10.0, 23.0, 18.0, 16.0);
        label(s, 19.0, 43.5, 2.8, pal::INK_DIM, "PATTERN");
        text(s, 60.0, 25.5, 3.2, Anchor::Start, pal::INK_DIM, "BP");
        text(s, 60.0, 35.5, 3.2, Anchor::Start, pal::INK_DIM, "LP");
        for (x, t) in [(11.5, "FREQ"), (30.5, "Q"), (49.5, "AMT"), (68.5, "DEC")] {
            label(s, x, 50.0, 3.0, pal::INK_DIM, t);
        }
    }));
    out.push(Part::new("bg_delay", 82.0, 93.0, |s| {
        face(s, 82.0, 93.0, pal::SLATE);
        fx_head(s, 82.0, "DELAY", 4.4);
        well(s, 9.0, 23.0, 19.0, 16.0);
        label(s, 18.5, 43.5, 2.8, pal::INK_DIM, "STEPS");
        text(s, 60.0, 26.5, 3.2, Anchor::Start, pal::INK_DIM, "3");
        text(s, 60.0, 37.5, 3.2, Anchor::Start, pal::INK_DIM, "16");
        label(s, 21.0, 81.5, 3.0, pal::INK_DIM, "PAN");
        label(s, 62.0, 81.5, 3.0, pal::INK_DIM, "F.BACK");
    }));
    out.push(Part::new("bg_dist", 82.0, 65.0, |s| {
        face(s, 82.0, 65.0, pal::SLATE);
        fx_head(s, 82.0, "DISTORTION", 3.6);
        label(s, 21.0, 51.0, 3.0, pal::INK_DIM, "AMOUNT");
        label(s, 62.0, 51.0, 3.0, pal::INK_DIM, "SHAPE");
    }));
    out.push(Part::new("bg_comp", 82.0, 93.0, |s| {
        face(s, 82.0, 93.0, pal::SLATE);
        fx_head(s, 82.0, "COMPRESSOR", 3.6);
        for (i, t) in ["-40", "-20", "0", "+20", "+40"].into_iter().enumerate() {
            label(s, 11.5 + 14.0 * i as f32, 39.5, 2.8, pal::INK_DIM, t);
        }
        label(s, 39.5, 46.5, 2.6, pal::INK_DIM, "GAIN REDUCTION");
        label(s, 21.0, 78.5, 3.0, pal::INK_DIM, "RATIO");
        label(s, 62.0, 78.5, 3.0, pal::INK_DIM, "THRES");
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
    v
}
