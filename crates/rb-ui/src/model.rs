//! The panel's state and what its controls do to it (docs/ui_spec.md sections 3-9, 11).
//! The model is the UI side only: every change that the engine must hear is also pushed to
//! `Panel::events`, which task-045 forwards to the player. Nothing here makes sound.

use std::collections::HashMap;

use crate::behavior::{self, KNOB_MAX};

/// Ticks per bar.
pub const BAR: i32 = 768;
/// The last bar start (bar 999) in ticks.
pub const MAX_POS: i32 = 766_464;
const MAX_END: i32 = 767_232;

/// A change the engine must hear.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// A control changed: the layout id and the value (knob and fader 0..=127, toggle and
    /// switch 0 or 1, display controls their number, the tempo in BPM * 1000, the
    /// positions in ticks).
    Set { id: String, value: i32 },
    /// A trigger button went down.
    Press { id: String },
    /// A pattern button selected `pattern` (0..=31) in row `row` (0..=3).
    Pattern { row: usize, pattern: u8 },
    Step808 {
        instrument: usize,
        step: usize,
        on: bool,
    },
    Step909 {
        instrument: usize,
        step: usize,
        value: u8,
    },
    /// A 303 step field changed (field 0 note, 1 slide, 2 accent, 3 up, 4 down, 5 rest).
    Step303 {
        synth: usize,
        pattern: u8,
        step: usize,
        field: u8,
        value: u8,
    },
}

/// The transport (ticks, 768 per bar).
#[derive(Clone, Debug)]
pub struct Transport {
    /// BPM * 1000.
    pub tempo: i32,
    pub position: i32,
    pub loop_start: i32,
    pub loop_end: i32,
    pub song_mode: bool,
    pub playing: bool,
    pub recording: bool,
    pub looping: bool,
}

impl Default for Transport {
    fn default() -> Self {
        Self {
            tempo: 140_000,
            position: 0,
            loop_start: 0,
            loop_end: 4 * BAR,
            song_mode: false,
            playing: false,
            recording: false,
            looping: false,
        }
    }
}

impl Transport {
    pub fn tempo_step(&mut self, d: i32) {
        self.tempo = (self.tempo + d * 1000).clamp(20_000, 500_000);
    }

    /// Moves the position by `d` ticks from the start of the current bar (`sub_429DF2`).
    pub fn locate_by(&mut self, d: i32) {
        self.position = (d + BAR * (self.position / BAR)).clamp(0, MAX_POS);
    }

    /// Rewind and fast forward: 10 bars; they also turn record off.
    pub fn wind(&mut self, forward: bool) {
        self.recording = false;
        self.locate_by(if forward { 10 * BAR } else { -10 * BAR });
    }

    /// Bar up / down: one bar; down turns record off, up does not.
    pub fn bar_step(&mut self, up: bool) {
        if !up {
            self.recording = false;
        }
        self.locate_by(if up { BAR } else { -BAR });
    }

    /// Loop start up / down by `d` ticks (`sub_429E5B`); the length is kept.
    pub fn loop_start_step(&mut self, d: i32) {
        let len = self.loop_end - self.loop_start;
        let start = BAR * ((self.loop_start + d).clamp(0, MAX_POS) / BAR);
        self.loop_start = start;
        self.loop_end = (len + start).clamp(start + BAR, MAX_POS);
    }

    /// Loop length up / down by `d` ticks (`sub_429F18`), at least one bar.
    pub fn loop_length_step(&mut self, d: i32) {
        let hi = (MAX_END - self.loop_start).max(BAR);
        let len = BAR * ((self.loop_end - self.loop_start + d).clamp(BAR, hi) / BAR);
        self.loop_end = self.loop_start + len;
    }

    pub fn play(&mut self) {
        self.playing = true;
    }

    /// Stop (`sub_429FE6`): record off; stops when playing; when stopped locates to the loop
    /// start, or to 0 when the position is at or before it.
    pub fn stop(&mut self) {
        self.recording = false;
        if self.playing {
            self.playing = false;
        } else if self.loop_start >= self.position {
            self.position = 0;
        } else {
            self.position = self.loop_start;
        }
    }

    pub fn tempo_display(&self) -> i32 {
        self.tempo / 1000
    }

    pub fn bar_display(&self) -> i32 {
        self.position / BAR + 1
    }

    pub fn loop_start_display(&self) -> i32 {
        self.loop_start / BAR + 1
    }

    pub fn loop_length_display(&self) -> i32 {
        (self.loop_end - self.loop_start) / BAR
    }
}

/// One pattern section (a row of the rack).
#[derive(Clone, Debug)]
pub struct Row {
    pub enabled: bool,
    /// The selected pattern 0..=31.
    pub pattern: u8,
    /// The shown bank 0..=3.
    pub bank: u8,
    /// Per pattern: the length 1..=16 and the shuffle flag.
    pub len: [u8; 32],
    pub shuffle: [bool; 32],
}

impl Default for Row {
    fn default() -> Self {
        Self {
            enabled: true,
            pattern: 0,
            bank: 0,
            len: [16; 32],
            shuffle: [false; 32],
        }
    }
}

impl Row {
    /// The pattern button (0..=7) that is lit: only when the selected pattern is in the shown
    /// bank.
    pub fn lit_button(&self) -> Option<u8> {
        (self.pattern / 8 == self.bank).then_some(self.pattern & 7)
    }
}

/// A step of a TB-303 pattern.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Step303 {
    pub note: u8,
    pub slide: bool,
    pub accent: bool,
    pub up: bool,
    pub down: bool,
    pub rest: bool,
}

impl Default for Step303 {
    /// A cleared step: note 0 (low C), rest on, no flags (`Clear`).
    fn default() -> Self {
        Self {
            note: 0,
            slide: false,
            accent: false,
            up: false,
            down: false,
            rest: true,
        }
    }
}

/// How long the lit note shows after a key in pitch mode, seconds [G].
const NOTE_FLASH: f64 = 0.4;

/// A TB-303's step editor.
#[derive(Clone, Debug)]
pub struct Synth303 {
    /// The edit step 0..=15.
    pub edit: u8,
    pub pitch_mode: bool,
    pub flash_until: f64,
    /// The steps of the 32 patterns.
    pub patterns: Vec<[Step303; 16]>,
}

impl Default for Synth303 {
    fn default() -> Self {
        Self {
            edit: 0,
            pitch_mode: false,
            flash_until: 0.0,
            patterns: vec![[Step303::default(); 16]; 32],
        }
    }
}

impl Synth303 {
    /// Step and Back move the edit step by +1 / -1 modulo 16, whatever the pattern length.
    pub fn move_edit(&mut self, d: i32) {
        self.edit = (self.edit as i32 + d).rem_euclid(16) as u8;
    }

    /// The pitch mode button; switching it on first sets the edit step to step 1.
    pub fn toggle_pitch(&mut self) {
        self.pitch_mode = !self.pitch_mode;
        if self.pitch_mode {
            self.edit = 0;
        }
    }

    /// Whether the key LED of `note` is lit: the edit step's note, shown only briefly after a
    /// key press in pitch mode.
    pub fn note_lit(&self, pattern: u8, note: u8, now: f64) -> bool {
        self.patterns[pattern as usize][self.edit as usize].note == note
            && (!self.pitch_mode || now < self.flash_until)
    }

    pub fn step(&self, pattern: u8) -> &Step303 {
        &self.patterns[pattern as usize][self.edit as usize]
    }
}

/// The whole panel's state.
#[derive(Clone, Debug)]
pub struct Panel {
    pub transport: Transport,
    pub rows: [Row; 4],
    /// The row with the focus 0..=3.
    pub focus: usize,
    pub synth: [Synth303; 2],
    /// The selected instruments (0 = AC) of the two drum machines.
    pub inst808: usize,
    pub inst909: usize,
    pub steps808: [[bool; 16]; 12],
    pub steps909: [[u8; 16]; 12],
    /// The playing step (the running light), from the engine; `None` when stopped.
    pub playing_step808: Option<usize>,
    pub playing_step909: Option<usize>,
    /// The global PCF and Comp routing ids: 0 none, PCF `c + 1`, Comp `c + 2` (master 1).
    pub pcf_id: u8,
    pub comp_id: u8,
    /// Meter states 0..=10 by control id (mixer, master, effect input); engine-fed.
    pub meters: HashMap<String, u8>,
    /// The compressor's nine gain-reduction LEDs: 0 off, 1 green, 2 amber, 3 red.
    pub gr_leds: [u8; 9],
    /// What the engine must hear, drained by the caller.
    pub events: Vec<Event>,
    knobs: HashMap<String, i32>,
    faders: HashMap<String, i32>,
    toggles: HashMap<String, bool>,
    switches: HashMap<String, u8>,
    numbers: HashMap<String, i32>,
    flam_button: bool,
}

impl Default for Panel {
    fn default() -> Self {
        Self::new()
    }
}

/// `c` for a section name like `mixer3` -> 2.
fn index_of(section: &str, prefix: &str) -> Option<usize> {
    section
        .strip_prefix(prefix)?
        .parse::<usize>()
        .ok()
        .and_then(|n| n.checked_sub(1))
}

impl Panel {
    pub fn new() -> Self {
        let mut p = Self {
            transport: Transport::default(),
            rows: Default::default(),
            focus: 0,
            synth: Default::default(),
            inst808: 1,
            inst909: 1,
            steps808: [[false; 16]; 12],
            steps909: [[0; 16]; 12],
            playing_step808: None,
            playing_step909: None,
            pcf_id: 0,
            comp_id: 0,
            meters: HashMap::new(),
            gr_leds: [0; 9],
            events: Vec::new(),
            knobs: HashMap::new(),
            faders: HashMap::new(),
            toggles: HashMap::new(),
            switches: HashMap::new(),
            numbers: HashMap::new(),
            flam_button: false,
        };
        for c in crate::layout::controls() {
            match c.kind.as_str() {
                "knob" => {
                    let v = match c.name() {
                        "shuffle" | "delay" => 0,
                        "selector" => behavior::knob_from_engine(behavior::selector_value(1)),
                        _ => 500,
                    };
                    p.knobs.insert(c.id, v);
                }
                "fader" => {
                    let v = if c.section.starts_with("mixer") || c.section == "master" {
                        750
                    } else {
                        500
                    };
                    p.faders.insert(c.id, v);
                }
                "toggle" if c.name() == "on" || c.name() == "mute" => {
                    // Pattern sections and mixer channels start on, the effects off.
                    let on = c.section.starts_with("pattern") || c.section.starts_with("mixer");
                    p.toggles.insert(c.id, on);
                }
                "hswitch" | "vswitch" => {
                    p.switches.insert(c.id, u8::from(c.section == "tr808"));
                }
                _ => {}
            }
        }
        p.numbers.insert("delay.steps".into(), 3);
        p.numbers.insert("pcf.pattern".into(), 0);
        p
    }

    /// A lively state for screenshots: song mode, playing, some steps, levels and meters.
    pub fn demo() -> Self {
        let mut p = Self::new();
        p.transport.song_mode = true;
        p.transport.playing = true;
        p.transport.position = 4 * BAR;
        p.transport.loop_start = 2 * BAR;
        p.transport.loop_end = 6 * BAR;
        p.transport.looping = true;
        p.rows[1].pattern = 2;
        p.rows[2].pattern = 1;
        p.rows[3].pattern = 5;
        p.rows[3].bank = 0;
        p.rows[0].len[0] = 16;
        p.rows[1].len[2] = 12;
        p.rows[1].shuffle[2] = true;
        for (i, s) in p.synth[0].patterns[0].iter_mut().enumerate() {
            s.note = [0, 3, 7, 10, 12, 7, 3, 0][i % 8];
            s.rest = i % 5 == 4;
            s.accent = i % 4 == 0;
        }
        p.synth[0].edit = 3;
        for i in [0, 4, 8, 12] {
            p.steps808[1][i] = true;
        }
        for i in [4, 12] {
            p.steps808[2][i] = true;
        }
        for i in (0..16).step_by(2) {
            p.steps808[11][i] = true;
        }
        p.playing_step808 = Some(6);
        p.playing_step909 = Some(6);
        for (i, v) in [1, 0, 0, 0, 2, 0, 0, 0, 1, 0, 3, 0, 2, 0, 0, 0]
            .into_iter()
            .enumerate()
        {
            p.steps909[1][i] = v;
        }
        p.pcf_id = 2;
        p.toggles.insert("pcf.on".into(), true);
        p.toggles.insert("comp.on".into(), true);
        p.toggles.insert("delay.on".into(), true);
        p.toggles.insert("transport.record".into(), false);
        p.toggles.insert("mixer1.dist".into(), true);
        p.set_knob("mixer2.delay", 300);
        p.set_knob("transport.shuffle", 300);
        p.set_knob("tb303_1.cutoff", 350);
        p.set_knob("tb303_1.reso", 620);
        p.set_knob("mixer1.pan", 250);
        p.set_knob("mixer4.pan", 800);
        p.set_fader("mixer1.level", 640);
        p.set_fader("mixer3.level", 880);
        for (id, v) in [
            ("mixer1.meter", 6),
            ("mixer2.meter", 5),
            ("mixer3.meter", 8),
            ("mixer4.meter", 3),
            ("master.meter_l", 8),
            ("master.meter_r", 7),
            ("pcf.meter", 4),
            ("comp.meter", 3),
            ("delay.meter", 2),
        ] {
            p.meters.insert(id.into(), v);
        }
        p.gr_leds = [0, 0, 0, 2, 1, 0, 0, 0, 0];
        p.events.clear();
        p
    }

    fn emit(&mut self, e: Event) {
        self.events.push(e);
    }

    fn set_event(&mut self, id: &str, value: i32) {
        self.emit(Event::Set {
            id: id.into(),
            value,
        });
    }

    // ---- knobs, faders, switches --------------------------------------------------------

    pub fn knob(&self, id: &str) -> i32 {
        self.knobs.get(id).copied().unwrap_or(0)
    }

    /// Sets a knob position and sends the value when it changed; the 808 selector also
    /// selects its instrument.
    pub fn set_knob(&mut self, id: &str, p: i32) {
        let p = p.clamp(0, KNOB_MAX);
        let old = self.knobs.insert(id.into(), p).unwrap_or(-1);
        if id == "tr808.selector" {
            self.inst808 = behavior::selector_instrument(behavior::knob_send(p));
        }
        let changed = old < 0 || behavior::knob_send(old) != behavior::knob_send(p);
        // The selector is UI state only.
        if changed && id != "tr808.selector" {
            self.set_event(id, behavior::knob_send(p));
        }
    }

    pub fn fader(&self, id: &str) -> i32 {
        self.faders.get(id).copied().unwrap_or(0)
    }

    pub fn set_fader(&mut self, id: &str, p: i32) {
        let p = p.clamp(0, behavior::FADER_MAX);
        let old = self.faders.insert(id.into(), p).unwrap_or(-1);
        if old < 0 || behavior::fader_send(old) != behavior::fader_send(p) {
            self.set_event(id, behavior::fader_send(p));
        }
    }

    pub fn switch(&self, id: &str) -> u8 {
        if id == "transport.mode" {
            return u8::from(self.transport.song_mode);
        }
        self.switches.get(id).copied().unwrap_or(0)
    }

    pub fn set_switch(&mut self, id: &str, v: u8) {
        if id == "transport.mode" {
            self.transport.song_mode = v != 0;
            if !self.transport.song_mode {
                self.transport.recording = false;
            }
        } else if self.switches.insert(id.into(), v) == Some(v) {
            return;
        }
        self.set_event(id, v as i32);
    }

    /// Selects a TR-808 / TR-909 instrument from a legend click; the 808's selector knob
    /// follows.
    pub fn select_instrument(&mut self, machine808: bool, i: usize) {
        if machine808 {
            self.inst808 = i;
            let p = behavior::knob_from_engine(behavior::selector_value(i));
            self.knobs.insert("tr808.selector".into(), p);
        } else {
            self.inst909 = i;
        }
    }

    // ---- toggles and presses ------------------------------------------------------------

    /// The on/off state of a toggle-like control.
    pub fn toggle_on(&self, id: &str) -> bool {
        let (sec, name) = id.split_once('.').unwrap_or((id, ""));
        match (sec, name) {
            ("transport", "loop") => self.transport.looping,
            ("transport", "record") => self.transport.recording,
            ("tr909", "flam_button") => self.flam_button,
            (_, "shuffle") if sec.starts_with("pattern") => {
                let r = index_of(sec, "pattern").unwrap_or(0).min(3);
                self.rows[r].shuffle[self.rows[r].pattern as usize]
            }
            (_, "on") if sec.starts_with("pattern") => {
                self.rows[index_of(sec, "pattern").unwrap_or(0).min(3)].enabled
            }
            (_, "pcf") if sec.starts_with("mixer") => {
                self.pcf_id as usize == index_of(sec, "mixer").unwrap_or(0) + 1
            }
            (_, "comp") if sec.starts_with("mixer") => {
                self.comp_id as usize == index_of(sec, "mixer").unwrap_or(0) + 2
            }
            ("master", "comp") => self.comp_id == 1,
            _ => self.toggles.get(id).copied().unwrap_or(false),
        }
    }

    /// The mouse-down action of a toggle, trigger, grid, radio or mode control.
    /// `now` is the time in seconds (for the 303 note flash).
    pub fn press(&mut self, id: &str, now: f64) {
        let Some((sec, name)) = id.split_once('.') else {
            return;
        };
        match sec {
            "transport" => self.press_transport(name),
            s if s.starts_with("pattern") => {
                let r = index_of(s, "pattern").unwrap_or(0).min(3);
                self.press_pattern(r, name);
            }
            s if s.starts_with("tb303_") => {
                let k = index_of(s, "tb303_").unwrap_or(0).min(1);
                self.press_303(k, name, now);
            }
            "tr909" if name == "flam_button" => {
                self.flam_button = !self.flam_button;
            }
            s if s.starts_with("mixer") => {
                let c = index_of(s, "mixer").unwrap_or(0).min(3);
                match name {
                    "pcf" => {
                        self.pcf_id = if self.pcf_id as usize == c + 1 {
                            0
                        } else {
                            c as u8 + 1
                        };
                        self.set_event("mixer.pcf_id", self.pcf_id as i32);
                    }
                    "comp" => {
                        self.comp_id = if self.comp_id as usize == c + 2 {
                            0
                        } else {
                            c as u8 + 2
                        };
                        self.set_event("mixer.comp_id", self.comp_id as i32);
                    }
                    _ => self.flip(id),
                }
            }
            "master" if name == "comp" => {
                self.comp_id = if self.comp_id == 1 { 0 } else { 1 };
                self.set_event("mixer.comp_id", self.comp_id as i32);
            }
            _ => {
                if name == "on" {
                    self.flip(id);
                }
            }
        }
    }

    fn flip(&mut self, id: &str) {
        let v = !self.toggles.get(id).copied().unwrap_or(false);
        self.toggles.insert(id.into(), v);
        self.set_event(id, v as i32);
    }

    fn press_transport(&mut self, name: &str) {
        let t = &mut self.transport;
        match name {
            "play" => {
                t.play();
                self.emit(Event::Press {
                    id: "transport.play".into(),
                });
            }
            "stop" => {
                t.stop();
                self.emit(Event::Press {
                    id: "transport.stop".into(),
                });
            }
            "record" => {
                t.recording = !t.recording;
                let v = t.recording as i32;
                self.set_event("transport.record", v);
            }
            "loop" => {
                t.looping = !t.looping;
                let v = t.looping as i32;
                self.set_event("transport.loop", v);
            }
            "mode" => {
                let v = !self.transport.song_mode as u8;
                self.set_switch("transport.mode", v);
            }
            _ => {}
        }
    }

    fn press_pattern(&mut self, r: usize, name: &str) {
        if let Some(n) = name.strip_prefix("pattern") {
            let b: u8 = n.parse().unwrap_or(1);
            let row = &mut self.rows[r];
            row.pattern = (b - 1) + 8 * row.bank;
            let pattern = row.pattern;
            self.focus = r;
            self.emit(Event::Pattern { row: r, pattern });
        } else if let Some(letter) = name.strip_prefix("bank") {
            self.rows[r].bank = letter.as_bytes()[0] - b'A';
        } else if name == "shuffle" {
            let row = &mut self.rows[r];
            let i = row.pattern as usize;
            row.shuffle[i] = !row.shuffle[i];
            let v = row.shuffle[i] as i32;
            self.set_event(&format!("pattern{}.shuffle", r + 1), v);
        } else if name == "on" {
            let row = &mut self.rows[r];
            row.enabled = !row.enabled;
            let v = row.enabled as i32;
            self.set_event(&format!("pattern{}.on", r + 1), v);
        }
    }

    fn press_303(&mut self, k: usize, name: &str, now: f64) {
        let pattern = self.rows[k].pattern;
        let s = &mut self.synth[k];
        let edit = s.edit as usize;
        let mut sent: Option<(u8, u8)> = None;
        if let Some(n) = name.strip_prefix("key") {
            let note: u8 = n.parse().unwrap_or(0);
            s.patterns[pattern as usize][edit].note = note;
            s.flash_until = now + NOTE_FLASH;
            sent = Some((0, note));
            if s.pitch_mode {
                s.move_edit(1);
            }
        } else {
            let st = &mut s.patterns[pattern as usize][edit];
            match name {
                "slide" => {
                    st.slide = !st.slide;
                    sent = Some((1, st.slide as u8));
                }
                "accent" => {
                    st.accent = !st.accent;
                    sent = Some((2, st.accent as u8));
                }
                "up" => {
                    st.up = !st.up;
                    sent = Some((3, st.up as u8));
                }
                "down" => {
                    st.down = !st.down;
                    sent = Some((4, st.down as u8));
                }
                "note_pause" => {
                    st.rest = !st.rest;
                    sent = Some((5, st.rest as u8));
                }
                "step" => s.move_edit(1),
                "back" => s.move_edit(-1),
                "pitch_button" => s.toggle_pitch(),
                "clear" => {
                    s.patterns[pattern as usize] = [Step303::default(); 16];
                    for step in 0..16 {
                        for (field, value) in [(0, 0), (1, 0), (2, 0), (3, 0), (4, 0), (5, 1)] {
                            self.events.push(Event::Step303 {
                                synth: k,
                                pattern,
                                step,
                                field,
                                value,
                            });
                        }
                    }
                }
                _ => {}
            }
        }
        if let Some((field, value)) = sent {
            self.emit(Event::Step303 {
                synth: k,
                pattern,
                step: edit,
                field,
                value,
            });
        }
    }

    // ---- the arrows next to the displays ------------------------------------------------

    /// One step of a repeat or wind button: `id` is the arrow's layout id (`..._up` or
    /// `..._down`).
    pub fn arrow(&mut self, id: &str) {
        if id == "transport.rewind" || id == "transport.ff" {
            self.transport.wind(id == "transport.ff");
            let position = self.transport.position;
            self.set_event("transport.position", position);
            return;
        }
        let Some((sec, name)) = id.split_once('.') else {
            return;
        };
        let (stem, up) = if let Some(s) = name.strip_suffix("_up") {
            (s, true)
        } else if let Some(s) = name.strip_suffix("_down") {
            (s, false)
        } else {
            return;
        };
        let d = if up { 1 } else { -1 };
        let (disp, value) = match (sec, stem) {
            ("transport", "tempo") => {
                self.transport.tempo_step(d);
                (format!("{sec}.{stem}"), self.transport.tempo)
            }
            ("transport", "bar") => {
                self.transport.bar_step(up);
                (format!("{sec}.{stem}"), self.transport.position)
            }
            ("transport", "loop_start") => {
                self.transport.loop_start_step(d * BAR);
                (format!("{sec}.{stem}"), self.transport.loop_start)
            }
            ("transport", "loop_length") => {
                self.transport.loop_length_step(d * BAR);
                (format!("{sec}.{stem}"), self.transport.loop_end)
            }
            (s, "steps") if s.starts_with("pattern") => {
                let r = index_of(s, "pattern").unwrap_or(0).min(3);
                let row = &mut self.rows[r];
                let i = row.pattern as usize;
                row.len[i] = (row.len[i] as i32 + d).clamp(1, 16) as u8;
                (format!("{sec}.{stem}"), row.len[i] as i32)
            }
            ("delay", "steps") => {
                let v = (self.numbers["delay.steps"] + d).clamp(1, 32);
                self.numbers.insert("delay.steps".into(), v);
                ("delay.steps".into(), v)
            }
            ("pcf", "pattern") => {
                let v = (self.numbers["pcf.pattern"] + d).clamp(0, 54);
                self.numbers.insert("pcf.pattern".into(), v);
                ("pcf.pattern".into(), v)
            }
            _ => return,
        };
        self.set_event(&disp, value);
    }

    // ---- the drum machines --------------------------------------------------------------

    /// Sets a TR-808 step of the selected instrument.
    pub fn set_step808(&mut self, step: usize, on: bool) {
        let i = self.inst808;
        if self.steps808[i][step] != on {
            self.steps808[i][step] = on;
            self.emit(Event::Step808 {
                instrument: i,
                step,
                on,
            });
        }
    }

    /// Sets a TR-909 step of the selected instrument; a hi-hat step clears the other hi-hat.
    pub fn set_step909(&mut self, step: usize, value: u8) {
        let i = self.inst909;
        if self.steps909[i][step] == value {
            return;
        }
        self.steps909[i][step] = value;
        self.emit(Event::Step909 {
            instrument: i,
            step,
            value,
        });
        if value != 0 && (i == 8 || i == 9) {
            let other = 17 - i;
            if self.steps909[other][step] != 0 {
                self.steps909[other][step] = 0;
                self.emit(Event::Step909 {
                    instrument: other,
                    step,
                    value: 0,
                });
            }
        }
    }

    /// The new value a mouse down on a 909 step gives, from the current step and modifiers.
    pub fn step909_new_value(&self, step: usize, m: behavior::Mods) -> u8 {
        behavior::tr909_step_value(
            self.steps909[self.inst909][step],
            self.inst909 == 0,
            self.flam_button,
            m,
        )
    }

    // ---- what the displays show ---------------------------------------------------------

    /// The number a display shows; `None` is a disabled display. `id` is the display's
    /// layout id.
    pub fn display(&self, id: &str) -> Option<i32> {
        let (sec, name) = id.split_once('.')?;
        let t = &self.transport;
        match (sec, name) {
            ("transport", "tempo") => Some(t.tempo_display()),
            ("transport", "bar") => t.song_mode.then(|| t.bar_display()),
            ("transport", "loop_start") => t.song_mode.then(|| t.loop_start_display()),
            ("transport", "loop_length") => t.song_mode.then(|| t.loop_length_display()),
            (s, "steps") if s.starts_with("pattern") => {
                let r = &self.rows[index_of(s, "pattern")?.min(3)];
                Some(r.len[r.pattern as usize] as i32)
            }
            (s, "edit_step") if s.starts_with("tb303_") => {
                Some(self.synth[index_of(s, "tb303_")?.min(1)].edit as i32 + 1)
            }
            _ => self.numbers.get(id).copied(),
        }
    }

    /// The part to draw for a control in its current state. `pressed` is a mouse button held
    /// on it, `now` the time in seconds.
    pub fn part_name(&self, c: &crate::layout::Ctl, pressed: bool, now: f64) -> String {
        let fam = c.family();
        let (sec, name) = (c.section.as_str(), c.name());
        let on_off = |on: bool| format!("{fam}{}", if on { "on" } else { "off" });
        let push = || format!("{fam}{}", if pressed { "pressed" } else { "rest" });
        match c.kind.as_str() {
            "toggle" if c.id == "transport.record" => {
                if self.transport.recording {
                    format!("{fam}lit")
                } else {
                    push()
                }
            }
            "toggle" if c.id == "tr909.flam_button" => on_off(self.flam_button),
            "toggle" => on_off(self.toggle_on(&c.id)),
            "radio" => on_off(self.toggle_on(&c.id)),
            "switch" | "hswitch" | "vswitch" => format!("{fam}{}", self.switch(&c.id)),
            "trigger" if c.id == "transport.play" => {
                if self.transport.playing {
                    format!("{fam}lit")
                } else {
                    push()
                }
            }
            "trigger" | "wind" | "repeat" => push(),
            "led" => {
                let on = self.led_on(c, now);
                if fam == "led_11_" {
                    format!("{fam}off")
                } else {
                    on_off(on)
                }
            }
            "grid" => {
                if let Some(n) = name.strip_prefix("pattern") {
                    let r = index_of(sec, "pattern").unwrap_or(0).min(3);
                    let n: u8 = n.parse().unwrap_or(1);
                    on_off(self.rows[r].lit_button() == Some(n - 1))
                } else {
                    let r = index_of(sec, "pattern").unwrap_or(0).min(3);
                    let letter = name.strip_prefix("bank").unwrap_or("A").as_bytes()[0] - b'A';
                    on_off(self.rows[r].bank == letter)
                }
            }
            "steps" => self.step_part(c),
            "meter" if name.starts_with("gr") => {
                let i: usize = name[2..].parse().unwrap_or(1);
                let s = ["off", "green", "amber", "red"][self.gr_leds[i - 1].min(3) as usize];
                format!("gr_led_{s}")
            }
            "meter" => format!(
                "{fam}{}",
                self.meters.get(&c.id).copied().unwrap_or(0).min(10)
            ),
            _ => c.part.clone(),
        }
    }

    fn led_on(&self, c: &crate::layout::Ctl, now: f64) -> bool {
        let Some(k) = index_of(&c.section, "tb303_") else {
            return false;
        };
        let k = k.min(1);
        let s = &self.synth[k.min(1)];
        let pattern = self.rows[k].pattern;
        let step = s.step(pattern);
        match c.name() {
            "pitch_led" => s.pitch_mode,
            "down_led" => step.down,
            "up_led" => step.up,
            "accent_led" => step.accent,
            "slide_led" => step.slide,
            "note_led" => !step.rest,
            "rest_led" => step.rest,
            n => n
                .strip_prefix("key")
                .and_then(|n| n.strip_suffix("_led"))
                .and_then(|n| n.parse::<u8>().ok())
                .is_some_and(|note| s.note_lit(pattern, note, now)),
        }
    }

    fn step_part(&self, c: &crate::layout::Ctl) -> String {
        let step: usize = c
            .name()
            .strip_prefix("step")
            .unwrap()
            .parse::<usize>()
            .unwrap()
            - 1;
        if c.section == "tr808" {
            let lit = self.steps808[self.inst808][step];
            let on = lit != (self.playing_step808 == Some(step));
            let color = ["red", "orange", "yellow", "white"][step / 4];
            format!("step808_{color}_{}", if on { "on" } else { "off" })
        } else {
            let mut v = self.steps909[self.inst909][step];
            if self.playing_step909 == Some(step) {
                v = if v == 0 { 2 } else { 0 };
            }
            let grp = if (step / 4).is_multiple_of(2) {
                "a"
            } else {
                "b"
            };
            format!("step909_{grp}_{v}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behavior::Mods;

    #[test]
    fn tempo_clamps_and_steps_by_one_bpm() {
        let mut t = Transport::default();
        t.tempo_step(1);
        assert_eq!(t.tempo_display(), 141);
        for _ in 0..1000 {
            t.tempo_step(-1);
        }
        assert_eq!(t.tempo, 20_000);
        for _ in 0..1000 {
            t.tempo_step(1);
        }
        assert_eq!(t.tempo, 500_000);
    }

    #[test]
    fn stop_stop_stop_reaches_bar_one() {
        let mut t = Transport {
            playing: true,
            recording: true,
            position: 5 * BAR + 100,
            loop_start: 2 * BAR,
            loop_end: 6 * BAR,
            ..Transport::default()
        };
        t.stop();
        assert!(!t.playing && !t.recording);
        assert_eq!(t.position, 5 * BAR + 100);
        t.stop();
        assert_eq!(t.position, 2 * BAR);
        t.stop();
        assert_eq!(t.position, 0);
        assert_eq!(t.bar_display(), 1);
    }

    #[test]
    fn wind_and_bar_steps() {
        let mut t = Transport {
            position: 3 * BAR + 5,
            recording: true,
            ..Transport::default()
        };
        t.bar_step(true);
        assert_eq!((t.position, t.recording), (4 * BAR, true));
        t.bar_step(false);
        assert_eq!((t.position, t.recording), (3 * BAR, false));
        t.recording = true;
        t.wind(true);
        assert_eq!((t.bar_display(), t.recording), (14, false));
        t.wind(false);
        t.wind(false);
        assert_eq!(t.position, 0);
        for _ in 0..200 {
            t.wind(true);
        }
        assert_eq!(t.position, MAX_POS);
        assert_eq!(t.bar_display(), 999);
    }

    #[test]
    fn loop_start_keeps_length_and_length_has_a_bar_minimum() {
        let mut t = Transport {
            loop_start: 2 * BAR,
            loop_end: 6 * BAR,
            ..Transport::default()
        };
        t.loop_start_step(BAR);
        assert_eq!((t.loop_start_display(), t.loop_length_display()), (4, 4));
        t.loop_length_step(-10 * BAR);
        assert_eq!(t.loop_length_display(), 1);
        t.loop_length_step(2 * BAR);
        assert_eq!(t.loop_length_display(), 3);
        t.loop_start_step(-100 * BAR);
        assert_eq!(t.loop_start, 0);
    }

    #[test]
    fn pattern_bank_and_lit_button() {
        let mut p = Panel::new();
        p.press("pattern1.bankC", 0.0);
        assert_eq!(p.rows[0].pattern, 0);
        assert_eq!(p.rows[0].lit_button(), None);
        p.press("pattern1.pattern3", 0.0);
        assert_eq!(p.rows[0].pattern, 18);
        assert_eq!(p.rows[0].lit_button(), Some(2));
        assert_eq!(
            p.events.last(),
            Some(&Event::Pattern {
                row: 0,
                pattern: 18
            })
        );
        let ctl = |id: &str| {
            crate::layout::controls()
                .into_iter()
                .find(|c| c.id == id)
                .unwrap()
        };
        assert_eq!(
            p.part_name(&ctl("pattern1.pattern3"), false, 0.0),
            "pat_btn_3_on"
        );
        assert_eq!(
            p.part_name(&ctl("pattern1.pattern1"), false, 0.0),
            "pat_btn_1_off"
        );
        assert_eq!(
            p.part_name(&ctl("pattern1.bankC"), false, 0.0),
            "bank_btn_C_on"
        );
        p.press("pattern4.pattern2", 0.0);
        assert_eq!(p.focus, 3);
    }

    #[test]
    fn steps_arrows_clamp_per_pattern() {
        let mut p = Panel::new();
        for _ in 0..40 {
            p.arrow("pattern2.steps_down");
        }
        assert_eq!(p.display("pattern2.steps"), Some(1));
        assert_eq!(p.display("pattern1.steps"), Some(16));
        for _ in 0..40 {
            p.arrow("pattern2.steps_up");
        }
        assert_eq!(p.display("pattern2.steps"), Some(16));
        for _ in 0..40 {
            p.arrow("delay.steps_up");
        }
        assert_eq!(p.display("delay.steps"), Some(32));
        for _ in 0..80 {
            p.arrow("pcf.pattern_down");
        }
        assert_eq!(p.display("pcf.pattern"), Some(0));
    }

    #[test]
    fn displays_are_disabled_in_pattern_mode() {
        let mut p = Panel::new();
        assert_eq!(p.display("transport.bar"), None);
        assert_eq!(p.display("transport.tempo"), Some(140));
        p.set_switch("transport.mode", 1);
        assert_eq!(p.display("transport.bar"), Some(1));
        assert_eq!(p.display("transport.loop_length"), Some(4));
        p.press("transport.record", 0.0);
        assert!(p.transport.recording);
        p.press("transport.mode", 0.0);
        assert!(!p.transport.song_mode && !p.transport.recording);
    }

    #[test]
    fn mixer_pcf_and_comp_are_exclusive() {
        let mut p = Panel::new();
        p.press("mixer2.pcf", 0.0);
        assert!(p.toggle_on("mixer2.pcf") && !p.toggle_on("mixer3.pcf"));
        p.press("mixer3.pcf", 0.0);
        assert!(!p.toggle_on("mixer2.pcf") && p.toggle_on("mixer3.pcf"));
        p.press("mixer3.pcf", 0.0);
        assert_eq!(p.pcf_id, 0);
        p.press("mixer1.comp", 0.0);
        assert_eq!(p.comp_id, 2);
        p.press("master.comp", 0.0);
        assert_eq!(p.comp_id, 1);
        assert!(!p.toggle_on("mixer1.comp") && p.toggle_on("master.comp"));
        // Dist is independent per channel.
        p.press("mixer1.dist", 0.0);
        p.press("mixer2.dist", 0.0);
        assert!(p.toggle_on("mixer1.dist") && p.toggle_on("mixer2.dist"));
    }

    #[test]
    fn knob_events_only_when_the_sent_value_changes() {
        let mut p = Panel::new();
        p.set_knob("tb303_1.cutoff", 500);
        p.events.clear();
        p.set_knob("tb303_1.cutoff", 501);
        assert!(p.events.is_empty());
        p.set_knob("tb303_1.cutoff", 520);
        assert_eq!(
            p.events,
            [Event::Set {
                id: "tb303_1.cutoff".into(),
                value: 66
            }]
        );
    }

    #[test]
    fn selector_knob_and_legend_agree() {
        let mut p = Panel::new();
        assert_eq!(p.inst808, 1);
        p.select_instrument(true, 7);
        assert_eq!(p.inst808, 7);
        assert_eq!(
            behavior::selector_instrument(behavior::knob_send(p.knob("tr808.selector"))),
            7
        );
        p.set_knob("tr808.selector", 1000);
        assert_eq!(p.inst808, 11);
    }

    #[test]
    fn tr909_hihats_exclude_each_other() {
        let mut p = Panel::new();
        p.inst909 = 8;
        p.set_step909(3, 1);
        p.inst909 = 9;
        p.set_step909(3, 2);
        assert_eq!(p.steps909[8][3], 0);
        assert_eq!(p.steps909[9][3], 2);
        p.inst909 = 1;
        assert_eq!(p.step909_new_value(3, Mods::default()), 1);
    }

    #[test]
    fn running_light_inverts_the_playing_step() {
        let mut p = Panel::new();
        p.steps808[1][0] = true;
        p.playing_step808 = Some(0);
        let c = crate::layout::controls()
            .into_iter()
            .find(|c| c.id == "tr808.step1")
            .unwrap();
        assert_eq!(p.part_name(&c, false, 0.0), "step808_red_off");
        p.playing_step808 = Some(1);
        assert_eq!(p.part_name(&c, false, 0.0), "step808_red_on");
        let c9 = crate::layout::controls()
            .into_iter()
            .find(|c| c.id == "tr909.step5")
            .unwrap();
        p.playing_step909 = Some(4);
        assert_eq!(p.part_name(&c9, false, 0.0), "step909_b_2");
    }

    #[test]
    fn synth303_step_editor() {
        let mut p = Panel::new();
        p.press("tb303_1.back", 0.0);
        assert_eq!(p.display("tb303_1.edit_step"), Some(16));
        p.press("tb303_1.step", 0.0);
        assert_eq!(p.display("tb303_1.edit_step"), Some(1));
        // Outside pitch mode a key never advances.
        p.press("tb303_1.key7", 0.0);
        assert_eq!(p.synth[0].step(0).note, 7);
        assert_eq!(p.synth[0].edit, 0);
        // Pitch mode: on resets to step 1, a key advances.
        p.synth[0].edit = 5;
        p.press("tb303_1.pitch_button", 0.0);
        assert_eq!(p.synth[0].edit, 0);
        p.press("tb303_1.key4", 1.0);
        assert_eq!((p.synth[0].patterns[0][0].note, p.synth[0].edit), (4, 1));
        // The lit note flashes in pitch mode.
        assert!(p.synth[0].flash_until > 1.0);
        // Flags flip, rest flips, Clear empties.
        p.press("tb303_1.accent", 0.0);
        p.press("tb303_1.note_pause", 0.0);
        let s = *p.synth[0].step(0);
        assert!(s.accent && !s.rest);
        p.press("tb303_1.clear", 0.0);
        assert_eq!(*p.synth[0].step(0), Step303::default());
        assert_eq!(p.synth[1].edit, 0);
    }

    #[test]
    fn every_control_has_a_part_for_its_state() {
        // No panic and a non-empty part name for all 352 controls in two states.
        let mut p = Panel::demo();
        for pressed in [false, true] {
            for c in crate::layout::controls() {
                if matches!(c.kind.as_str(), "knob" | "fader" | "display" | "area") {
                    continue;
                }
                let n = p.part_name(&c, pressed, 0.0);
                assert!(crate::art::part(&n).is_some(), "{}: no part {n}", c.id);
            }
            p.rows[0].bank = 2;
        }
    }
}
