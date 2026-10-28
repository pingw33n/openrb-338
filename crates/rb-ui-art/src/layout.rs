//! Where every control sits, and the composition of the full panel from the
//! parts. The list is written to `layout.tsv` and drawn into `panel.svg`.
//! The sections and the controls' functions are the original's
//! (docs/ui_spec.md); the placement inside a section is rb's own design and
//! may differ from the original's rectangles.

use std::collections::HashMap;

use crate::parts::{
    EDIT_303, FLAGS_303, KNOB_SWEEP, KNOBS, KNOBS_909, MOD_VIEW_808, STEPS_Y_909, WHITE_303,
    column_909, knob_y_909,
};
use crate::svg::{Part, Svg, arc_path, n, pal};

pub const PANEL_W: f32 = 624.0;
pub const PANEL_H: f32 = 510.0;

/// How a control is composed from parts.
pub enum Extra {
    /// One part at (x, y).
    Plain,
    /// `<knob>_body`, the value arc, and `<knob>_pointer` rotated by the value.
    Knob {
        design: usize,
        v: f32,
        bipolar: bool,
    },
    /// `fader_track` and `fader_handle` at `(1 - v) * 43` px from the top.
    Fader { v: f32 },
    /// `n` digit frames, right-aligned in the display rectangle.
    Digits { n: usize, text: String },
}

pub struct Ctl {
    pub id: String,
    pub section: String,
    pub kind: &'static str,
    pub part: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub extra: Extra,
}

pub struct Layout {
    /// Section backgrounds: part name and panel position.
    pub backgrounds: Vec<(String, f32, f32)>,
    pub ctls: Vec<Ctl>,
}

struct Builder<'a> {
    sizes: &'a HashMap<String, (f32, f32)>,
    layout: Layout,
    section: String,
    ox: f32,
    oy: f32,
}

impl Builder<'_> {
    fn section(&mut self, name: &str, bg: &str, ox: f32, oy: f32) {
        self.section = name.to_string();
        self.ox = ox;
        self.oy = oy;
        self.layout.backgrounds.push((bg.to_string(), ox, oy));
    }

    fn size(&self, part: &str) -> (f32, f32) {
        *self
            .sizes
            .get(part)
            .unwrap_or_else(|| panic!("layout names a missing part {part}"))
    }

    fn push(&mut self, id: &str, kind: &'static str, part: &str, x: f32, y: f32, extra: Extra) {
        let (w, h) = match &extra {
            Extra::Knob { design, .. } => (KNOBS[*design].w, KNOBS[*design].h),
            _ => self.size(part),
        };
        self.layout.ctls.push(Ctl {
            id: format!("{}.{id}", self.section),
            section: self.section.clone(),
            kind,
            part: part.to_string(),
            x: self.ox + x,
            y: self.oy + y,
            w,
            h,
            extra,
        });
    }

    fn part(&mut self, id: &str, kind: &'static str, part: &str, x: f32, y: f32) {
        self.push(id, kind, part, x, y, Extra::Plain);
    }

    fn knob(&mut self, id: &str, design: usize, x: f32, y: f32, v: f32) {
        let name = KNOBS[design].name;
        self.push(
            id,
            "knob",
            name,
            x,
            y,
            Extra::Knob {
                design,
                v,
                bipolar: false,
            },
        );
    }

    fn pan(&mut self, id: &str, design: usize, x: f32, y: f32, v: f32) {
        let name = KNOBS[design].name;
        self.push(
            id,
            "knob",
            name,
            x,
            y,
            Extra::Knob {
                design,
                v,
                bipolar: true,
            },
        );
    }

    fn fader(&mut self, id: &str, x: f32, y: f32, v: f32) {
        self.push(id, "fader", "fader_track", x, y, Extra::Fader { v });
    }

    /// A seven-segment display: the rectangle holds `digits` frames of 9 x 16.
    fn display(&mut self, id: &str, x: f32, y: f32, w: f32, digits: usize, text: &str) {
        self.layout.ctls.push(Ctl {
            id: format!("{}.{id}", self.section),
            section: self.section.clone(),
            kind: "display",
            part: "seg_0..9,seg_blank".into(),
            x: self.ox + x,
            y: self.oy + y,
            w,
            h: 16.0,
            extra: Extra::Digits {
                n: digits,
                text: text.to_string(),
            },
        });
    }
}

const KNOB_303: usize = 0;
const KNOB_808_RED: usize = 1;
const KNOB_808_WHITE: usize = 2;
const KNOB_909: usize = 3;
const KNOB_22: usize = 4;

/// The control rectangles of the whole panel, with the demo state used for
/// the composed preview (pattern mode off, song mode, playing).
pub fn layout(parts: &[Part]) -> Layout {
    let sizes: HashMap<String, (f32, f32)> =
        parts.iter().map(|p| (p.name.clone(), (p.w, p.h))).collect();
    let mut b = Builder {
        sizes: &sizes,
        layout: Layout {
            backgrounds: Vec::new(),
            ctls: Vec::new(),
        },
        section: String::new(),
        ox: 0.0,
        oy: 0.0,
    };
    b.section("wood_left", "wood", 0.0, 0.0);
    b.section("wood_right", "wood", 608.0, 0.0);
    transport(&mut b);
    for r in 0..4 {
        pattern(&mut b, r);
    }
    tb303(&mut b, 0);
    tb303(&mut b, 1);
    tr808(&mut b);
    tr909(&mut b);
    for c in 0..4 {
        mixer(&mut b, c);
    }
    master(&mut b);
    effects(&mut b);
    b.layout
}

fn transport(b: &mut Builder) {
    b.section("transport", "bg_transport", 16.0, 0.0);
    // centered with its label in the space left of the tempo display (0..44)
    b.knob("shuffle", KNOB_22, 11.0, 10.3, 0.3);
    b.display("tempo", 44.0, 27.0, 27.0, 3, "140");
    b.part("tempo_up", "repeat", "arrow_inc_rest", 73.0, 26.0);
    b.part("tempo_down", "repeat", "arrow_dec_rest", 73.0, 35.0);
    b.part("mode", "switch", "switch_mode_1", 161.0, 2.0);
    b.part("play", "trigger", "btn_play_lit", 96.0, 21.0);
    b.part("stop", "trigger", "btn_stop_rest", 128.0, 21.0);
    b.part("rewind", "wind", "btn_rewind_rest", 160.0, 21.0);
    b.part("ff", "wind", "btn_ff_rest", 192.0, 21.0);
    b.part("record", "toggle", "btn_record_rest", 224.0, 21.0);
    b.display("bar", 271.0, 27.0, 27.0, 3, "  5");
    b.part("bar_up", "wind", "arrow_inc_rest", 300.0, 26.0);
    b.part("bar_down", "wind", "arrow_dec_rest", 300.0, 35.0);
    b.display("loop_start", 329.0, 27.0, 27.0, 3, "  5");
    b.part("loop_start_up", "repeat", "arrow_inc_rest", 358.0, 26.0);
    b.part("loop_start_down", "repeat", "arrow_dec_rest", 358.0, 35.0);
    b.display("loop_length", 372.0, 27.0, 27.0, 3, "  4");
    b.part("loop_length_up", "repeat", "arrow_inc_rest", 401.0, 26.0);
    b.part("loop_length_down", "repeat", "arrow_dec_rest", 401.0, 35.0);
    // level with the length arrows (26..44)
    b.part("loop", "toggle", "toggle_loop_on", 417.0, 26.0);
}

fn pattern(b: &mut Builder, r: usize) {
    let oy = 50.0 + 115.0 * r as f32;
    b.section(&format!("pattern{}", r + 1), "bg_pattern", 16.0, oy);
    b.part("on", "toggle", "power_13x13_on", 4.0, 4.0);
    let lit = [0usize, 2, 1, 5][r];
    for i in 0..8 {
        let state = if i == lit { "on" } else { "off" };
        b.part(
            &format!("pattern{}", i + 1),
            "grid",
            &format!("pat_btn_{}_{state}", i + 1),
            1.0 + 17.0 * (i % 4) as f32,
            19.0 + 20.0 * (i / 4) as f32,
        );
    }
    for (i, name) in ["A", "B", "C", "D"].into_iter().enumerate() {
        let state = if i == 0 { "on" } else { "off" };
        b.part(
            &format!("bank{name}"),
            "grid",
            &format!("bank_btn_{name}_{state}"),
            1.0 + 17.0 * i as f32,
            66.0,
        );
    }
    // level with the steps display (95..111)
    b.part("shuffle", "toggle", "toggle_shuffle_off", 3.0, 97.0);
    b.display("steps", 36.0, 95.0, 19.0, 2, "16");
    b.part("steps_up", "repeat", "arrow_inc_rest", 56.0, 94.0);
    b.part("steps_down", "repeat", "arrow_dec_rest", 56.0, 103.0);
    b.section(
        &format!("focus{}", r + 1),
        if r == 0 { "focus_on" } else { "focus_off" },
        86.0,
        oy,
    );
}

fn tb303(b: &mut Builder, r: usize) {
    let oy = 50.0 + 115.0 * r as f32;
    b.section(&format!("tb303_{}", r + 1), "bg_tb303", 90.0, oy);
    // saw (15.5..23.5), the switch, square (45.5..52.5): centered over pitch mode and
    // clear (x 34), level with the knobs' centers
    b.part("wave", "hswitch", "switch_wave_0", 26.5, 21.0);
    let vals = [0.5, 0.35, 0.6, 0.45, 0.4, 0.55];
    for (k, name) in ["tune", "cutoff", "reso", "envmod", "decay", "accent"]
        .into_iter()
        .enumerate()
    {
        b.knob(name, KNOB_303, 66.0 + 40.0 * k as f32, 12.0, vals[k]);
    }
    let (ex, ey) = EDIT_303;
    b.display("edit_step", ex, ey, 19.0, 2, " 1");
    b.part("step", "trigger", "arrow303_inc_rest", ex + 20.0, ey - 1.0);
    b.part("back", "trigger", "arrow303_dec_rest", ex + 20.0, ey + 8.0);
    // Pitch mode and clear, centered in the band's left section (x 12..56).
    b.part("pitch_led", "led", "led_8_off", 30.0, 55.0);
    b.part("pitch_button", "trigger", "btn_pill_17x10_rest", 25.5, 65.0);
    b.part("clear", "trigger", "btn_pill_17x10_rest", 25.5, 92.0);
    // The keys; each LED is centered 3 px above its key.
    let white_notes = [0, 2, 4, 5, 7, 9, 11, 12];
    let black_notes = [(0, 1), (1, 3), (3, 6), (4, 8), (5, 10)];
    let note_lit = 0; // C
    for (x, note) in WHITE_303.into_iter().zip(white_notes) {
        b.part(&format!("key{note}"), "trigger", "key_white_rest", x, 89.0);
        let led = if note == note_lit {
            "led_8_on"
        } else {
            "led_8_off"
        };
        b.part(&format!("key{note}_led"), "led", led, x + 0.5, 78.0);
    }
    for (i, note) in black_notes {
        let x = WHITE_303[i] + 11.0;
        b.part(&format!("key{note}"), "trigger", "key_black_rest", x, 60.0);
        b.part(&format!("key{note}_led"), "led", "led_8_off", x, 49.0);
    }
    // The flags and note/rest (its LED lit = note) in one row, LEDs like the keys'.
    for (x, (name, led, lit)) in FLAGS_303.into_iter().zip([
        ("down", "down_led", false),
        ("up", "up_led", false),
        ("accent", "accent_led", true),
        ("slide", "slide_led", false),
        ("note_pause", "note_led", true),
    ]) {
        b.part(name, "trigger", "btn_flag_rest", x, 89.0);
        b.part(
            led,
            "led",
            if lit { "led_8_on" } else { "led_8_off" },
            x + 0.5,
            78.0,
        );
    }
}

/// (x, y, design, id) of the 808's knobs.
const KNOBS_808: [(f32, f32, usize, &str); 22] = [
    (14.0, 9.0, KNOB_808_RED, "ac_level"),
    (36.0, 9.0, KNOB_808_RED, "bd_level"),
    (36.0, 31.0, KNOB_808_WHITE, "bd_tone"),
    (36.0, 54.0, KNOB_808_WHITE, "bd_decay"),
    (58.0, 9.0, KNOB_808_RED, "sd_level"),
    (58.0, 31.0, KNOB_808_WHITE, "sd_tone"),
    (58.0, 54.0, KNOB_808_WHITE, "sd_snappy"),
    (80.0, 9.0, KNOB_808_RED, "lt_level"),
    (80.0, 31.0, KNOB_808_WHITE, "lt_tuning"),
    (102.0, 9.0, KNOB_808_RED, "mt_level"),
    (102.0, 31.0, KNOB_808_WHITE, "mt_tuning"),
    (124.0, 9.0, KNOB_808_RED, "ht_level"),
    (124.0, 31.0, KNOB_808_WHITE, "ht_tuning"),
    (146.0, 9.0, KNOB_808_RED, "rs_level"),
    (168.0, 9.0, KNOB_808_RED, "cp_level"),
    (190.0, 9.0, KNOB_808_RED, "cb_level"),
    (212.0, 9.0, KNOB_808_RED, "cy_level"),
    (212.0, 31.0, KNOB_808_WHITE, "cy_tone"),
    (212.0, 54.0, KNOB_808_WHITE, "cy_decay"),
    (234.0, 9.0, KNOB_808_RED, "oh_level"),
    (234.0, 54.0, KNOB_808_WHITE, "oh_decay"),
    (256.0, 9.0, KNOB_808_RED, "ch_level"),
];

fn tr808(b: &mut Builder) {
    b.section("tr808", "bg_tr808", 90.0, 280.0);
    for (i, &(x, y, design, id)) in KNOBS_808.iter().enumerate() {
        b.knob(id, design, x, y, 0.35 + 0.03 * (i % 12) as f32);
    }
    for (i, x) in [81.0, 103.0, 125.0, 146.0, 169.0].into_iter().enumerate() {
        b.part(
            &format!("slot{}_switch", i + 3),
            "vswitch",
            "switch_808_1",
            x,
            55.0,
        );
    }
    let (x, y, _, _) = MOD_VIEW_808;
    b.part("mod_view", "modview", "mod_view", x, y);
    b.part("select_bd", "area", "sel_22x14", 32.0, 71.0);
    let colors = ["red", "orange", "yellow", "white"];
    for i in 0..16 {
        let on = i % 4 == 0;
        b.part(
            &format!("step{}", i + 1),
            "steps",
            &format!(
                "step808_{}_{}",
                colors[i / 4],
                if on { "on" } else { "off" }
            ),
            // centered under the legend row (10..274)
            14.0 + 16.0 * i as f32,
            91.0,
        );
    }
}

fn tr909(b: &mut Builder) {
    b.section("tr909", "bg_tr909", 90.0, 395.0);
    let half = KNOBS[KNOB_909].w / 2.0;
    for (i, (group, col, lower, id, _)) in KNOBS_909.into_iter().enumerate() {
        let (x, y) = (column_909(group, col) - half, knob_y_909(group, lower));
        b.knob(id, KNOB_909, x, y, 0.3 + 0.025 * (i % 16) as f32);
    }
    // level with the steps' lights, centered in the AC column
    b.part(
        "flam_button",
        "toggle",
        "toggle_flam_off",
        8.0,
        STEPS_Y_909 + 1.5,
    );
    b.part("select_bd", "area", "sel_42x14", 26.0, 4.0);
    for i in 0..16 {
        let value = [1, 0, 0, 0, 2, 0, 0, 0, 1, 0, 3, 0, 2, 0, 0, 0][i];
        let grp = if (i / 4) % 2 == 0 { "a" } else { "b" };
        b.part(
            &format!("step{}", i + 1),
            "steps",
            &format!("step909_{grp}_{value}"),
            // under the name row's cells (26..362)
            26.0 + 21.0 * i as f32,
            STEPS_Y_909,
        );
    }
}

fn mixer(b: &mut Builder, c: usize) {
    b.section(
        &format!("mixer{}", c + 1),
        "bg_mixer",
        456.0,
        50.0 + 115.0 * c as f32,
    );
    // The header (2..20): the on/off button and the meter 2 px from its edges.
    b.part("mute", "toggle", "power_14x14_on", 4.0, 4.0);
    b.part(
        "meter",
        "meter",
        &format!("meter_8x14_{}", [6, 5, 8, 3][c]),
        58.0,
        4.0,
    );
    // Two columns: pan and level centered at x 19, the routing and delay at x 51.
    b.part("dist", "toggle", "mixsw_dist_off", 41.0, 23.0);
    b.part(
        "pcf",
        "radio",
        if c == 1 {
            "mixsw_pcf_on"
        } else {
            "mixsw_pcf_off"
        },
        41.0,
        37.0,
    );
    b.part("comp", "radio", "mixsw_comp_off", 41.0, 51.0);
    // pan centered in the space between the header (20) and the fader (58)
    b.pan("pan", KNOB_22, 8.0, 25.5, 0.5);
    b.fader("level", 11.5, 58.0, [0.7, 0.7, 0.8, 0.75][c]);
    b.knob("delay", KNOB_22, 40.0, 76.0, [0.0, 0.25, 0.1, 0.0][c]);
}

fn master(b: &mut Builder) {
    b.section("master", "bg_master", 526.0, 50.0);
    b.part("meter_l", "meter", "meter_15x56_8", 16.0, 24.0);
    b.part("meter_r", "meter", "meter_15x56_7", 51.0, 24.0);
    b.fader("fader", 33.0, 25.0, 0.8);
    b.part("comp", "radio", "mixsw_comp_off", 31.0, 81.0);
}

/// An effect's header (2..20): the on/off button and the meter 2 px from its edges.
fn fx_header(b: &mut Builder, on: bool, level: u8) {
    let state = if on { "on" } else { "off" };
    b.part("on", "toggle", &format!("power_14x14_{state}"), 4.0, 4.0);
    b.part("meter", "meter", &format!("meter_8x14_{level}"), 70.0, 4.0);
}

fn effects(b: &mut Builder) {
    b.section("pcf", "bg_pcf", 526.0, 148.0);
    fx_header(b, true, 4);
    b.display("pattern", 10.0, 23.0, 18.0, 2, "25");
    b.part("pattern_up", "repeat", "arrow_inc_rest", 29.0, 22.0);
    b.part("pattern_down", "repeat", "arrow_dec_rest", 29.0, 31.0);
    // level with the pattern arrows (22..40)
    b.part("mode", "vswitch", "switch_fx_1", 49.0, 22.0);
    for (x, id, v) in [
        (4.0, "freq", 0.6),
        (23.0, "q", 0.3),
        (42.0, "amt", 0.5),
        (61.0, "dec", 0.4),
    ] {
        b.fader(id, x, 46.0, v);
    }

    // The knobs are centered in the two halves (x 21 and 62).
    b.section("delay", "bg_delay", 526.0, 259.0);
    fx_header(b, false, 0);
    b.display("steps", 9.0, 23.0, 19.0, 2, " 3");
    b.part("steps_up", "repeat", "arrow_inc_rest", 29.0, 22.0);
    b.part("steps_down", "repeat", "arrow_dec_rest", 29.0, 31.0);
    b.part("triplet", "vswitch", "switch_fx_0", 49.0, 22.0);
    b.pan("pan", KNOB_22, 10.0, 55.0, 0.5);
    b.knob("feedback", KNOB_22, 51.0, 55.0, 0.4);

    b.section("dist", "bg_dist", 526.0, 352.0);
    fx_header(b, false, 0);
    b.knob("amount", KNOB_22, 10.0, 27.0, 0.4);
    b.knob("shape", KNOB_22, 51.0, 27.0, 0.5);

    b.section("comp", "bg_comp", 526.0, 417.0);
    fx_header(b, true, 3);
    for i in 0..9 {
        // Demo: a little reduction (LEDs left of the middle), the rest dark.
        let state = match i {
            0..=2 => "off",
            3 => "amber",
            4 => "green",
            _ => "off",
        };
        b.part(
            &format!("gr{}", i + 1),
            "meter",
            &format!("gr_led_{state}"),
            // the 63 px meter centered at x 41
            9.5 + 7.0 * i as f32,
            25.0,
        );
    }
    b.knob("ratio", KNOB_22, 10.0, 54.0, 0.3);
    b.knob("threshold", KNOB_22, 51.0, 54.0, 0.6);
}

/// The `layout.tsv` text: one control per line, panel pixel rectangles.
pub fn tsv(l: &Layout) -> String {
    let mut s = String::from("id\tsection\tkind\tpart\tx\ty\tw\th\n");
    for c in &l.ctls {
        s.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            c.id,
            c.section,
            c.kind,
            c.part,
            n(c.x),
            n(c.y),
            n(c.w),
            n(c.h)
        ));
    }
    s
}

/// The whole panel as one SVG (the demo state), 624 x 510 panel pixels.
pub fn compose(parts: &[Part], l: &Layout) -> String {
    let by: HashMap<&str, &Part> = parts.iter().map(|p| (p.name.as_str(), p)).collect();
    let get = |name: &str| {
        *by.get(name)
            .unwrap_or_else(|| panic!("missing part {name}"))
    };
    let mut s = Svg::new();
    s.rect(0.0, 0.0, PANEL_W, PANEL_H, 0.0, pal::BG);
    for (name, x, y) in &l.backgrounds {
        s.g_at(*x, *y).raw(&get(name).body).end_g();
    }
    for c in &l.ctls {
        match &c.extra {
            Extra::Plain => {
                s.g_at(c.x, c.y).raw(&get(&c.part).body).end_g();
            }
            Extra::Knob { design, v, bipolar } => {
                let k = &KNOBS[*design];
                s.g_at(c.x, c.y);
                s.raw(&get(&format!("{}_body", k.name)).body);
                let a = -KNOB_SWEEP + v * 2.0 * KNOB_SWEEP;
                if k.ring_r > 0.0 {
                    let a0 = if *bipolar { 0.0 } else { -KNOB_SWEEP };
                    if (a - a0).abs() > 0.5 {
                        s.path_stroke(&arc_path(k.cx, k.cy, k.ring_r, a0, a), k.arc, k.ring_w);
                    }
                }
                s.g_rotate(a, k.cx, k.cy);
                s.raw(&get(&format!("{}_pointer", k.name)).body);
                s.end_g().end_g();
            }
            Extra::Fader { v } => {
                s.g_at(c.x, c.y).raw(&get("fader_track").body).end_g();
                s.g_at(c.x, c.y + (1.0 - v) * 43.0)
                    .raw(&get("fader_handle").body)
                    .end_g();
            }
            Extra::Digits { n: count, text } => {
                let chars: Vec<char> = text.chars().collect();
                for i in 0..*count {
                    let ch = chars.get(i).copied().unwrap_or(' ');
                    let name = if ch == ' ' {
                        "seg_blank".to_string()
                    } else {
                        format!("seg_{ch}")
                    };
                    let x = c.x + c.w - 9.0 * (*count - i) as f32;
                    s.g_at(x, c.y).raw(&get(&name).body).end_g();
                }
            }
        }
    }
    let (w, h) = (n(PANEL_W), n(PANEL_H));
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">{}</svg>\n",
        s.s
    )
}
