//! Where every control sits, and the composition of the full panel from the parts. The list is
//! written to `layout.tsv` and drawn into `panel.svg`. The sections and the controls' places
//! are the original's (docs/ui_spec.md sections 3 to 9) as the redesign of task-179 draws them
//! (the approved mockup; docs/ui_spec.md section 19 lists where it leaves the spec's tables).
//! A control's rectangle is its hit area; its part may draw beyond it (`Part::bleed`).

use std::collections::HashMap;

use crate::parts::{
    BANK_Y, BLACK_303, FG_PCF, FG_TR808, GR_OFF, KEY_X, KEY_Y, KEYCAP, KNOB_SWEEP, KNOB_X_303,
    KNOBS, KNOBS_909, LEGEND_808, LOGO_X, LOGO_Y, MOD_PICTURE, MOD_VIEW_808, NAMES_808, NAMES_909,
    SEAM, WHITE_303, chip_909, led_909,
};
use crate::svg::{Part, Svg, defs_for, n};

pub const PANEL_W: f32 = 624.0;
pub const PANEL_H: f32 = 510.0;

/// How a control is composed from parts.
pub enum Extra {
    /// One part at (x, y).
    Plain,
    /// `<knob>_body` and `<knob>_pointer` rotated by the value.
    Knob { design: usize, v: f32 },
    /// `fader_track` and `fader_handle` at `(1 - v) * 43` px from the top.
    Fader { v: f32 },
    /// `n` digit frames, centered in the display rectangle.
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
    /// What the app lays over the controls in the demo state: the selected instruments' marks
    /// and the glass over the mod's picture (part, panel position).
    pub overlays: Vec<(String, f32, f32)>,
    /// Static art drawn over the controls (part, panel position): `fg_tr808`, `fg_pcf`.
    pub foregrounds: Vec<(String, f32, f32)>,
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
        self.push(
            id,
            "knob",
            KNOBS[design].name,
            x,
            y,
            Extra::Knob { design, v },
        );
    }

    fn fader(&mut self, id: &str, x: f32, y: f32, v: f32) {
        self.push(id, "fader", "fader_track", x, y, Extra::Fader { v });
    }

    /// A seven-segment display: the rectangle holds `digits` cells of 9 x 16, centered.
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

    /// The up and down arrows of a display, at (x, y) and (x, y + 9).
    fn arrows(&mut self, id: &str, kind: &'static str, x: f32, y: f32) {
        self.part(&format!("{id}_up"), kind, "arrow_inc_rest", x, y);
        self.part(&format!("{id}_down"), kind, "arrow_dec_rest", x, y + 9.0);
    }

    /// A part laid over the controls in the demo (section coordinates).
    fn overlay(&mut self, part: &str, x: f32, y: f32) {
        self.layout
            .overlays
            .push((part.to_string(), self.ox + x, self.oy + y));
    }
}

pub const KNOB_303: usize = 0;
pub const KNOB_808_RED: usize = 1;
pub const KNOB_808_WHITE: usize = 2;
pub const KNOB_909: usize = 3;
pub const KNOB_GRAY: usize = 4;

/// The control rectangles of the whole panel, with the demo state of the mockup (song mode,
/// playing, loop on, tempo 140, bar 9; row 1 has the focus; README.md of assets/ui).
pub fn layout(parts: &[Part]) -> Layout {
    let sizes: HashMap<String, (f32, f32)> =
        parts.iter().map(|p| (p.name.clone(), (p.w, p.h))).collect();
    let mut b = Builder {
        sizes: &sizes,
        layout: Layout {
            backgrounds: Vec::new(),
            ctls: Vec::new(),
            overlays: Vec::new(),
            foregrounds: Vec::new(),
        },
        section: String::new(),
        ox: 0.0,
        oy: 0.0,
    };
    b.section("wood_left", "wood_left", 0.0, 0.0);
    b.section("wood_right", "wood_right", 608.0, 0.0);
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
    // the wordmark over the transport background (its pieces are not drawn here)
    b.layout
        .backgrounds
        .push(("logo".to_string(), LOGO_X, LOGO_Y));
    b.knob("shuffle", KNOB_GRAY, 11.0, 13.0, 0.35);
    b.display("tempo", 44.0, 27.0, 27.0, 3, "140");
    b.arrows("tempo", "repeat", 73.0, 26.0);
    b.part("mode", "switch", "switch_mode_1", 161.0, 2.0);
    b.part("play", "trigger", "btn_play_lit", 96.0, 21.0);
    b.part("stop", "trigger", "btn_stop_rest", 128.0, 21.0);
    b.part("rewind", "wind", "btn_rewind_rest", 160.0, 21.0);
    b.part("ff", "wind", "btn_ff_rest", 192.0, 21.0);
    b.part("record", "toggle", "btn_record_rest", 224.0, 21.0);
    b.display("bar", 271.0, 27.0, 27.0, 3, "  9");
    b.arrows("bar", "wind", 300.0, 26.0);
    b.display("loop_start", 329.0, 27.0, 27.0, 3, "  1");
    b.arrows("loop_start", "repeat", 358.0, 26.0);
    b.display("loop_length", 372.0, 27.0, 27.0, 3, " 16");
    b.arrows("loop_length", "repeat", 401.0, 26.0);
    b.part("loop", "toggle", "toggle_loop_on", 317.0, 3.0);
}

fn pattern(b: &mut Builder, r: usize) {
    let oy = 50.0 + 115.0 * r as f32;
    b.section(
        &format!("pattern{}", r + 1),
        &format!("bg_pattern_{}", r + 1),
        16.0,
        oy,
    );
    let on = if r == 1 { "off" } else { "on" };
    b.part("on", "toggle", &format!("power_pat_{on}"), 5.0, 4.0);
    let lit = if r == 3 { 1 } else { 0 };
    let half = KEYCAP / 2.0;
    for i in 0..8 {
        let state = if i == lit { "on" } else { "off" };
        b.part(
            &format!("pattern{}", i + 1),
            "grid",
            &format!("pat_btn_{}_{state}", i + 1),
            KEY_X[i % 4] - half,
            KEY_Y[i / 4] - half,
        );
    }
    for (i, name) in ["A", "B", "C", "D"].into_iter().enumerate() {
        let state = if i == 0 { "on" } else { "off" };
        b.part(
            &format!("bank{name}"),
            "grid",
            &format!("bank_btn_{name}_{state}"),
            KEY_X[i] - half,
            BANK_Y - half,
        );
    }
    let shuffle = if r == 1 { "on" } else { "off" };
    b.part(
        "shuffle",
        "toggle",
        &format!("toggle_shuffle_{shuffle}"),
        4.5,
        94.7,
    );
    b.display("steps", 36.0, 95.0, 19.0, 2, "16");
    b.arrows("steps", "repeat", 56.0, 94.0);
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
    let first = r == 0;
    b.part(
        "wave",
        "hswitch",
        if first {
            "switch_wave_1"
        } else {
            "switch_wave_0"
        },
        24.65,
        18.5,
    );
    let vals = if first {
        [0.42, 0.34, 0.5, 0.32, 0.5, 0.5]
    } else {
        [0.5, 0.28, 0.62, 0.3, 0.72, 0.4]
    };
    for (k, name) in ["tune", "cutoff", "reso", "envmod", "decay", "accent"]
        .into_iter()
        .enumerate()
    {
        b.knob(name, KNOB_303, KNOB_X_303[k], 12.0, vals[k]);
    }
    b.display("edit_step", 330.0, 25.0, 19.0, 2, " 1");
    let pressed = if first { "rest" } else { "pressed" };
    b.part(
        "back",
        "trigger",
        &format!("btn_pill_17x10_{pressed}"),
        328.0,
        62.0,
    );
    b.part(
        "step",
        "trigger",
        &format!("btn_step_{pressed}"),
        324.6,
        90.0,
    );
    let led = |on: bool| if on { "on" } else { "off" };
    b.part(
        "pitch_led",
        "led",
        &format!("led_303s_{}", led(first)),
        24.0,
        67.0,
    );
    b.part("pitch_button", "trigger", "btn_pill_15x8_rest", 19.5, 73.6);
    b.part("clear", "trigger", "btn_pill_17x10_rest", 18.0, 96.0);
    // the keys (C lit) with their LEDs
    for (i, note) in WHITE_303.into_iter().enumerate() {
        let x = 52.0 + 21.0 * i as f32;
        b.part(&format!("key{note}"), "trigger", "key_white_rest", x, 89.0);
        let state = led(note == 0);
        b.part(
            &format!("key{note}_led"),
            "led",
            &format!("led_303_{state}"),
            x + 1.5,
            82.0,
        );
    }
    for (bx, note) in BLACK_303 {
        b.part(
            &format!("key{note}"),
            "trigger",
            "key_black_rest",
            bx - 4.2,
            62.6,
        );
        b.part(
            &format!("key{note}_led"),
            "led",
            "led_303s_off",
            bx - 3.0,
            56.0,
        );
    }
    // the step's note and rest LEDs and the note / pause button, then the flags
    let flags: &[&str] = if first {
        &["rest"]
    } else {
        &["note", "accent"]
    };
    b.part(
        "note_led",
        "led",
        &format!("led_303_{}", led(flags.contains(&"note"))),
        247.0,
        60.0,
    );
    b.part(
        "rest_led",
        "led",
        &format!("led_303_{}", led(flags.contains(&"rest"))),
        285.0,
        60.0,
    );
    b.part("note_pause", "trigger", "btn_pill_17x10_rest", 294.0, 58.0);
    for (i, name) in ["down", "up", "accent", "slide"].into_iter().enumerate() {
        let x = 224.0 + 25.0 * i as f32;
        b.part(name, "trigger", "btn_flag_rest", x, 89.0);
        let state = led(flags.contains(&name));
        b.part(
            &format!("{name}_led"),
            "led",
            &format!("led_303_{state}"),
            x + 1.5,
            82.0,
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

/// The 808's step keys: their left edges are 16 + 16 i, their tops 90.2.
pub const STEP_X0_808: f32 = 16.0;
pub const STEP_Y_808: f32 = 90.2;

fn tr808(b: &mut Builder) {
    b.section("tr808", "bg_tr808", 90.0, 280.0);
    let level = [
        0.62, 0.78, 0.66, 0.7, 0.7, 0.7, 0.72, 0.68, 0.64, 0.7, 0.66, 0.7,
    ];
    for &(x, y, design, id) in KNOBS_808.iter() {
        let col = ((x - 14.0) / 22.0) as usize;
        let v = match y as i32 {
            9 => level[col],
            31 => 0.5,
            _ => [0.45, 0.55, 0.3, 0.6][(col * 7) % 4],
        };
        b.knob(id, design, x, y, v);
    }
    for (i, x) in [81.0, 103.0, 125.0, 146.0, 169.0].into_iter().enumerate() {
        let pos = if i < 3 { 1 } else { 0 };
        b.part(
            &format!("slot{}_switch", i + 3),
            "vswitch",
            &format!("switch_808_{pos}"),
            x,
            55.0,
        );
    }
    let (fx, fy, _, _) = FG_TR808;
    b.layout
        .foregrounds
        .push(("fg_tr808".into(), b.ox + fx, b.oy + fy));
    let (x, y, _, _) = MOD_VIEW_808;
    b.part("mod_view", "modview", "mod_view", x, y);
    let (px, py, _) = MOD_PICTURE;
    b.overlay("mod_view_glass", x + px, y + py);
    // BD selected: its legend chip's mark
    let (lx, ly, _, _) = LEGEND_808;
    b.part("select_bd", "area", "sel808_BD", lx + 22.0, ly);
    b.overlay(&format!("sel808_{}", NAMES_808[1]), lx + 22.0, ly);
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
            STEP_X0_808 + 16.0 * i as f32,
            STEP_Y_808,
        );
    }
}

/// The 909's step keys: their left edges are 29 + 21 i, their tops 85.4.
pub const STEP_X0_909: f32 = 29.0;
pub const STEP_Y_909: f32 = 85.4;
/// The 909's flam button.
pub const FLAM_909: (f32, f32) = (7.6, 92.6);

fn tr909(b: &mut Builder) {
    b.section("tr909", "bg_tr909", 90.0, 395.0);
    let r1 = [
        0.5, 0.74, 0.5, 0.66, 0.5, 0.66, 0.5, 0.66, 0.5, 0.66, 0.66, 0.66, 0.66, 0.66, 0.66,
    ];
    for (i, (x, y, id, label)) in KNOBS_909.into_iter().enumerate() {
        let v = match id {
            "ac_level" => 0.5,
            "flam" => 0.75,
            _ if y < 50.0 => r1[i - 1],
            _ if label == "DEC" && x < 250.0 => 0.86,
            _ => 0.5,
        };
        b.knob(id, KNOB_909, x, y, v);
    }
    b.part(
        "flam_button",
        "toggle",
        "toggle_flam_on",
        FLAM_909.0,
        FLAM_909.1,
    );
    // BD selected: its chip's label lit and its name label's LED
    let (cx, _, _) = chip_909(1);
    b.part("select_bd", "area", "sel909_BD", cx, 4.6);
    b.overlay(&format!("sel909_{}", NAMES_909[1]), cx, 4.6);
    let (lx, ly) = led_909(1);
    b.overlay("led909_on", lx - 2.5, ly - 2.5);
    for i in 0..16 {
        let value = match i {
            0 | 4 | 8 | 12 => 2,
            14 => 1,
            10 => 3,
            _ => 0,
        };
        b.part(
            &format!("step{}", i + 1),
            "steps",
            &format!("step909_{value}"),
            STEP_X0_909 + 21.0 * i as f32,
            STEP_Y_909,
        );
    }
}

fn mixer(b: &mut Builder, c: usize) {
    b.section(
        &format!("mixer{}", c + 1),
        &format!("bg_mixer_{}", c + 1),
        456.0,
        50.0 + 115.0 * c as f32,
    );
    b.part("mute", "toggle", "power_mix_on", 3.0, 4.0);
    b.part(
        "meter",
        "meter",
        &format!("meter_14x14_{}", [2, 2, 3, 1][c]),
        51.0,
        4.0,
    );
    b.knob("pan", KNOB_GRAY, 10.0, 25.0, 0.5);
    b.knob("delay", KNOB_GRAY, 42.0, 79.0, 0.2);
    b.fader("level", 10.0, 59.0, 0.72);
    let dist = if c == 1 { "mixsw_on" } else { "mixsw_off" };
    b.part("dist", "toggle", dist, 42.0, 19.0);
    b.part("pcf", "radio", "mixsw_off", 42.0, 37.0);
    b.part("comp", "radio", "mixsw_off", 42.0, 55.0);
}

fn master(b: &mut Builder) {
    b.section("master", "bg_master", 526.0, 50.0);
    b.part("meter_l", "meter", "meter_15x56_6", 16.0, 24.0);
    b.part("meter_r", "meter", "meter_15x56_6", 51.0, 24.0);
    b.fader("fader", 33.0, 25.0, 0.8);
    b.part("comp", "radio", "mixsw_on", 42.0, 81.0);
}

/// An effect's header lamp and meter, the header at `y` of the section.
fn fx_header(b: &mut Builder, y: f32, on: bool, level: u8) {
    let state = if on { "on" } else { "off" };
    b.part("on", "toggle", &format!("power_fx_{state}"), 3.0, y - 0.5);
    b.part(
        "meter",
        "meter",
        &format!("meter_14x14_{level}"),
        63.0,
        y + 0.5,
    );
}

fn effects(b: &mut Builder) {
    b.section("pcf", "bg_pcf", 526.0, 148.0);
    fx_header(b, 2.5, true, 0);
    b.display("pattern", 10.0, 23.0, 18.0, 2, "25");
    b.arrows("pattern", "repeat", 29.0, 22.0);
    b.part("mode", "vswitch", "switch_fx_1", 49.0, 21.0);
    for (i, (id, v)) in [("freq", 0.0), ("q", 0.55), ("amt", 1.0), ("dec", 0.3)]
        .into_iter()
        .enumerate()
    {
        b.fader(id, 4.0 + 19.0 * i as f32, 50.0, v);
    }
    let (fx, fy, _, _) = FG_PCF;
    b.layout
        .foregrounds
        .push(("fg_pcf".into(), b.ox + fx, b.oy + fy));

    b.section("delay", "bg_delay", 526.0, 259.0);
    fx_header(b, 3.0, true, 4);
    b.display("steps", 9.0, 23.0, 19.0, 2, " 3");
    b.arrows("steps", "repeat", 29.0, 22.0);
    b.part("triplet", "vswitch", "switch_fx_1", 49.0, 22.0);
    b.knob("pan", KNOB_GRAY, 12.0, 58.0, 0.5);
    b.knob("feedback", KNOB_GRAY, 53.0, 58.0, 0.42);

    b.section("dist", "bg_dist", 526.0, 352.0);
    fx_header(b, 3.0, true, 1);
    b.knob("amount", KNOB_GRAY, 12.0, 28.0, 0.5);
    b.knob("shape", KNOB_GRAY, 53.0, 28.0, 0.36);

    b.section("comp", "bg_comp", 526.0, 417.0);
    fx_header(b, 3.0, true, 3);
    for (i, off) in GR_OFF.into_iter().enumerate() {
        // the demo's reduction: the center and the four LEDs left of it, as the app lights them
        let state = match i {
            0 | 1 => "red".to_string(),
            2 | 3 => "amber".to_string(),
            4 => "green".to_string(),
            _ => format!("off_{off}"),
        };
        b.part(
            &format!("gr{}", i + 1),
            "meter",
            &format!("gr_led_{state}"),
            8.0 + 7.0 * i as f32,
            26.5,
        );
    }
    b.knob("ratio", KNOB_GRAY, 12.0, 57.0, 0.52);
    b.knob("threshold", KNOB_GRAY, 53.0, 57.0, 0.5);
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
    s.rect(0.0, 0.0, PANEL_W, PANEL_H, 0.0, SEAM);
    for (name, x, y) in &l.backgrounds {
        s.g_at(*x, *y).raw(&get(name).body).end_g();
    }
    for c in &l.ctls {
        match &c.extra {
            Extra::Plain => {
                if c.kind != "area" {
                    s.g_at(c.x, c.y).raw(&get(&c.part).body).end_g();
                }
            }
            Extra::Knob { design, v } => {
                let k = &KNOBS[*design];
                s.g_at(c.x, c.y);
                s.raw(&get(&format!("{}_body", k.name)).body);
                let a = -KNOB_SWEEP + v * 2.0 * KNOB_SWEEP;
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
                let x0 = c.x + (c.w - 9.0 * *count as f32) / 2.0;
                for i in 0..*count {
                    let ch = chars.get(i).copied().unwrap_or(' ');
                    let name = if ch == ' ' {
                        "seg_blank".to_string()
                    } else {
                        format!("seg_{ch}")
                    };
                    s.g_at(x0 + 9.0 * i as f32, c.y)
                        .raw(&get(&name).body)
                        .end_g();
                }
            }
        }
    }
    for (name, x, y) in l.foregrounds.iter().chain(&l.overlays) {
        s.g_at(*x, *y).raw(&get(name).body).end_g();
    }
    let (w, h) = (n(PANEL_W), n(PANEL_H));
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" \
         viewBox=\"0 0 {w} {h}\">{}{}</svg>\n",
        defs_for(&s.s),
        s.s
    )
}
