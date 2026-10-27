//! The panel wired to a real player (task-045), headless: synthetic pointer events run through
//! `App::show` on the panel at 1:1, the link sends the original panel's messages to a real
//! `rb_player::Player`, and the test drives the audio callback (`Audio::render_i16`) itself, one
//! engine call (1024 frames) at a time. What the panel sent is read from `Link::log` and compared
//! with the messages of `docs/song_load.md` section 4.10; what the engine did is read from the
//! snapshot and from the engine's pattern arrays.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2};
use rb_engine::samples::SampleSet;
use rb_engine::song::Device;
use rb_engine::tables::Tables;
use rb_player::{Audio, CALL_FRAMES, Snapshot};
use rb_ui::app::App;
use rb_ui::layout::{self, PANEL_H, PANEL_W};
use rb_ui::link::{Link, Msg};
use rb_ui::model::{BAR, Panel};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn tables() -> Arc<Tables> {
    static T: OnceLock<Arc<Tables>> = OnceLock::new();
    T.get_or_init(|| Arc::new(Tables::new())).clone()
}

fn samples() -> Arc<SampleSet> {
    static S: OnceLock<Arc<SampleSet>> = OnceLock::new();
    S.get_or_init(|| {
        let dll = std::fs::read(root().join("original/ReBirth.dll")).unwrap();
        Arc::new(SampleSet::from_dll(&dll).unwrap())
    })
    .clone()
}

struct Harness {
    ctx: egui::Context,
    app: App,
    audio: Audio,
    time: f64,
    pos: Pos2,
}

impl Harness {
    /// A panel on a new empty song.
    fn new() -> Self {
        let ctx = egui::Context::default();
        egui_extras::install_image_loaders(&ctx);
        let (mut link, audio) = Link::new(tables(), Some(samples()));
        link.log = Some(vec![]);
        let mut h = Self {
            ctx,
            app: App::with_link(Panel::new(), link, None),
            audio,
            time: 0.0,
            pos: Pos2::ZERO,
        };
        h.calls(1);
        h.take_log();
        h
    }

    /// A panel on a corpus song (`tests/songs/...`), loaded and (as the menu will) made current.
    fn with_song(path: &str) -> Self {
        let mut h = Self::new();
        h.load(path);
        h
    }

    fn load(&mut self, path: &str) {
        let song = rb_ui::session::load_song(&root().join(path)).unwrap();
        let link = self.app.link.as_mut().unwrap();
        link.load(&song, Some(samples()), &mut self.app.panel)
            .unwrap();
        self.calls(2);
        self.take_log();
    }

    fn frame(&mut self, events: Vec<Event>) {
        self.frame_mods(events, Modifiers::NONE);
    }

    fn frame_mods(&mut self, mut events: Vec<Event>, modifiers: Modifiers) {
        self.time += 0.016;
        events.insert(0, Event::ModifiersChanged(modifiers));
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(PANEL_W, PANEL_H))),
            time: Some(self.time),
            events,
            ..Default::default()
        };
        let app = &mut self.app;
        let mut out = self.ctx.run_ui(input, |ui| app.show(ui));
        out.textures_delta.clear();
    }

    /// Runs `n` engine calls (the audio callback), then a UI frame that takes the snapshot.
    /// Returns the output.
    fn calls(&mut self, n: usize) -> Vec<i16> {
        let mut out = vec![0i16; 2 * CALL_FRAMES * n];
        for c in out.chunks_mut(2 * CALL_FRAMES) {
            self.audio.render_i16(c);
        }
        self.frame(vec![]);
        out
    }

    fn snap(&mut self) -> Snapshot {
        *self.app.link.as_mut().unwrap().player().snapshot()
    }

    /// The messages sent since the last call.
    fn take_log(&mut self) -> Vec<Msg> {
        std::mem::take(self.app.link.as_mut().unwrap().log.as_mut().unwrap())
    }

    /// The engine's pattern array of row `r`.
    fn engine_patterns(&mut self, r: usize) -> Vec<u8> {
        self.audio.current_mut().frame.devices.channels[r]
            .patterns_mut()
            .unwrap()
            .to_vec()
    }

    fn center(id: &str) -> Pos2 {
        let c = layout::controls().into_iter().find(|c| c.id == id).unwrap();
        Pos2::new(c.x + c.w / 2.0, c.y + c.h / 2.0)
    }

    fn center_of(kind: &str, id: &str) -> Pos2 {
        let c = layout::controls()
            .into_iter()
            .find(|c| c.id == id && c.kind == kind)
            .unwrap();
        Pos2::new(c.x + c.w / 2.0, c.y + c.h / 2.0)
    }

    fn move_to(&mut self, p: Pos2) {
        self.pos = p;
        self.frame(vec![Event::PointerMoved(p)]);
    }

    fn down(&mut self, mods: Modifiers) {
        let pos = self.pos;
        self.frame_mods(
            vec![Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: mods,
            }],
            mods,
        );
    }

    fn up(&mut self) {
        let pos = self.pos;
        self.frame(vec![Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }]);
    }

    fn click_at(&mut self, p: Pos2) {
        self.move_to(p);
        self.down(Modifiers::NONE);
        self.up();
    }

    fn click(&mut self, id: &str) {
        self.click_at(Self::center(id));
    }

    /// Drags a knob or fader `dy` panel pixels up (negative: down) from its center.
    fn drag(&mut self, id: &str, dy: f32) {
        let start = Self::center(id);
        self.move_to(start);
        self.down(Modifiers::NONE);
        self.move_to(start - Vec2::new(0.0, dy));
        self.up();
    }

    /// Presses a key for one frame.
    fn key(&mut self, key: egui::Key) {
        let ev = |pressed| Event::Key {
            key,
            physical_key: Some(key),
            pressed,
            repeat: false,
            modifiers: Modifiers::NONE,
        };
        self.frame(vec![ev(true)]);
        self.frame(vec![ev(false)]);
    }
}

/// A device message.
fn dev(d: Device, sub: u8, data: &[u8]) -> Msg {
    Msg::Device {
        dev: d.id(),
        sub,
        data: data.to_vec(),
    }
}

fn global(id: u8, value: u32) -> Msg {
    Msg::Global { id, value }
}

// ---- the list of controls and what they send -----------------------------------------------

/// The `{ctrl, v}` sub of the devices that have one (`docs/song_load.md` section 4.10).
fn sub_of(d: Device) -> u8 {
    match d {
        Device::Tb303_1 | Device::Tb303_2 => 40,
        Device::Tr808 => 57,
        Device::Tr909 => 74,
        Device::Delay => 84,
        Device::Distortion => 92,
        Device::Compressor => 102,
        Device::Pcf => 114,
        Device::Mixer => unreachable!(),
    }
}

/// Every knob, fader and switch that sends a device `{ctrl, v}` (docs/ui_spec.md sections 5 to
/// 9): control id, device, controller.
fn param_controls() -> Vec<(String, Device, u8)> {
    use Device::*;
    let mut v: Vec<(String, Device, u8)> = vec![];
    for (k, d) in [(1, Tb303_1), (2, Tb303_2)] {
        for (n, c) in [
            ("wave", 8),
            ("tune", 2),
            ("cutoff", 3),
            ("reso", 4),
            ("envmod", 5),
            ("decay", 6),
            ("accent", 7),
        ] {
            v.push((format!("tb303_{k}.{n}"), d, c));
        }
    }
    let tr808 = [
        ("ac_level", 2),
        ("bd_level", 3),
        ("bd_tone", 4),
        ("bd_decay", 5),
        ("sd_level", 6),
        ("sd_tone", 7),
        ("sd_snappy", 8),
        ("lt_level", 9),
        ("lt_tuning", 10),
        ("slot3_switch", 11),
        ("mt_level", 12),
        ("mt_tuning", 13),
        ("slot4_switch", 14),
        ("ht_level", 15),
        ("ht_tuning", 16),
        ("slot5_switch", 17),
        ("rs_level", 18),
        ("slot6_switch", 19),
        ("cp_level", 20),
        ("slot7_switch", 21),
        ("cb_level", 22),
        ("cy_level", 23),
        ("cy_tone", 24),
        ("cy_decay", 25),
        ("oh_level", 26),
        ("oh_decay", 27),
        ("ch_level", 28),
    ];
    for (n, c) in tr808 {
        v.push((format!("tr808.{n}"), Tr808, c));
    }
    let tr909 = [
        ("ac_level", 2),
        ("bd_level", 3),
        ("bd_tune", 4),
        ("bd_attack", 5),
        ("bd_decay", 6),
        ("sd_level", 7),
        ("sd_tune", 8),
        ("sd_tone", 9),
        ("sd_snappy", 10),
        ("lt_level", 11),
        ("lt_tune", 12),
        ("lt_decay", 13),
        ("mt_level", 14),
        ("mt_tune", 15),
        ("mt_decay", 16),
        ("ht_level", 17),
        ("ht_tune", 18),
        ("ht_decay", 19),
        ("rs_level", 20),
        ("cp_level", 21),
        ("hh_level", 22),
        ("ch_decay", 23),
        ("oh_decay", 24),
        ("cc_level", 25),
        ("cc_tune", 26),
        ("rc_level", 27),
        ("rc_tune", 28),
        ("flam", 29),
    ];
    for (n, c) in tr909 {
        v.push((format!("tr909.{n}"), Tr909, c));
    }
    for (id, d, c) in [
        ("delay.on", Delay, 0),
        ("delay.triplet", Delay, 2),
        ("delay.feedback", Delay, 3),
        ("delay.pan", Delay, 4),
        ("dist.on", Distortion, 0),
        ("dist.amount", Distortion, 1),
        ("dist.shape", Distortion, 2),
        ("comp.on", Compressor, 0),
        ("comp.ratio", Compressor, 1),
        ("comp.threshold", Compressor, 2),
        ("pcf.on", Pcf, 0),
        ("pcf.freq", Pcf, 1),
        ("pcf.q", Pcf, 2),
        ("pcf.amt", Pcf, 3),
        ("pcf.dec", Pcf, 5),
        ("pcf.mode", Pcf, 6),
    ] {
        v.push((id.to_string(), d, c));
    }
    v
}

/// The controls with no message: UI state, a display, a meter, or sent by other paths (listed in
/// the test that checks them).
const OTHER: &[&str] = &["tr808.selector", "tr909.flam_button", "tr909.flam"];

/// The live byte index of a device's controller in the snapshot's live block.
fn live_index(d: Device, ctrl: u8) -> usize {
    if d == Device::Tr909 {
        rb_engine::tr909::internal_index(ctrl as usize)
    } else {
        ctrl as usize
    }
}

#[test]
fn every_knob_fader_and_switch_sends_its_device_message_and_the_engine_takes_it() {
    let mut h = Harness::new();
    let mut checked = vec![];
    for (id, d, ctrl) in param_controls() {
        let c = layout::controls()
            .into_iter()
            .find(|c| c.id == id && !(c.kind == "trigger"))
            .unwrap();
        h.take_log();
        match c.kind.as_str() {
            // up, or down from the top
            "knob" => {
                let up = h.app.panel.knob(&id) < 500;
                h.drag(&id, if up { 20.0 } else { -20.0 })
            }
            "hswitch" | "vswitch" => {
                // the half that changes the value
                let one = h.app.panel.switch(&id) == 1;
                let (fx, fy) = if c.kind == "hswitch" {
                    (if one { 0.25 } else { 0.75 }, 0.5)
                } else {
                    (0.5, if one { 0.25 } else { 0.75 })
                };
                h.click_at(Pos2::new(c.x + c.w * fx, c.y + c.h * fy));
            }
            "fader" => {
                let up = h.app.panel.fader(&id) < 500;
                h.drag(&id, if up { 12.0 } else { -12.0 })
            }
            "toggle" => h.click(&id),
            k => panic!("{id}: kind {k}"),
        }
        let value = match c.kind.as_str() {
            "knob" => rb_ui::behavior::knob_send(h.app.panel.knob(&id)),
            "fader" => rb_ui::behavior::fader_send(h.app.panel.fader(&id)),
            "toggle" => h.app.panel.toggle_on(&id) as i32,
            _ => h.app.panel.switch(&id) as i32,
        } as u8;
        let log = h.take_log();
        // a fader pressed off its handle first jumps to the pointer: its last value counts
        let expected = dev(d, sub_of(d), &[ctrl, value]);
        assert_eq!(log.last(), Some(&expected), "{id}: wrong message");
        let all = if c.kind == "fader" { 2 } else { 1 };
        assert!(log.len() <= all, "{id}: {log:?}");
        h.calls(2);
        let live = h.snap().live[d as usize];
        assert_eq!(
            live.as_slice()[live_index(d, ctrl)],
            value,
            "{id}: the engine did not take {value}"
        );
        checked.push(format!(
            "{id} -> dev {} sub {} [{ctrl}, {value}]",
            d.id(),
            sub_of(d)
        ));
    }
    println!("{}", checked.join("\n"));
    // every control of those kinds is in the list or in OTHER
    let listed: Vec<String> = param_controls().into_iter().map(|p| p.0).collect();
    for c in layout::controls() {
        if matches!(c.kind.as_str(), "knob" | "fader" | "hswitch" | "vswitch")
            && (c.section.starts_with("tb303")
                || ["tr808", "tr909", "delay", "dist", "comp", "pcf"].contains(&c.section.as_str()))
        {
            assert!(
                listed.contains(&c.id) || OTHER.contains(&c.id.as_str()),
                "{} has no entry",
                c.id
            );
        }
    }
}

#[test]
fn the_flam_knob_the_selector_and_the_flam_button() {
    let mut h = Harness::new();
    // The 909 flam knob is controller 29 of the 909.
    h.drag("tr909.flam", 10.0);
    let v = rb_ui::behavior::knob_send(h.app.panel.knob("tr909.flam"));
    assert_eq!(h.take_log(), vec![dev(Device::Tr909, 74, &[29, v as u8])]);
    // The selector knob and the Flam button are UI state: nothing is sent.
    h.drag("tr808.selector", 10.0);
    h.click("tr909.flam_button");
    assert_eq!(h.take_log(), vec![]);
}

#[test]
fn the_mixer_sends_msg_145_and_147() {
    let mut h = Harness::new();
    for c in 0..4u8 {
        let n = c + 1;
        // (control, field k of msg 147): mute 0, level 1, pan 2, delay send 3, dist 4
        h.take_log();
        h.click(&format!("mixer{n}.mute"));
        assert_eq!(
            h.take_log(),
            vec![dev(Device::Mixer, 147, &[c, 0, 0])],
            "mute {n}"
        );
        h.click(&format!("mixer{n}.mute"));
        h.take_log();
        // a press off the handle jumps it to the pointer, the drag then moves it: two values
        h.drag(&format!("mixer{n}.level"), 12.0);
        let v = rb_ui::behavior::fader_send(h.app.panel.fader(&format!("mixer{n}.level")));
        let log = h.take_log();
        assert_eq!(log.last(), Some(&dev(Device::Mixer, 147, &[c, 1, v as u8])));
        assert!(
            log.iter().all(
                |m| matches!(m, Msg::Device { dev: 0, sub: 147, data } if data[..2] == [c, 1])
            )
        );
        h.drag(&format!("mixer{n}.pan"), 20.0);
        let v = rb_ui::behavior::knob_send(h.app.panel.knob(&format!("mixer{n}.pan")));
        assert_eq!(
            h.take_log(),
            vec![dev(Device::Mixer, 147, &[c, 2, v as u8])]
        );
        h.drag(&format!("mixer{n}.delay"), 20.0);
        let v = rb_ui::behavior::knob_send(h.app.panel.knob(&format!("mixer{n}.delay")));
        assert_eq!(
            h.take_log(),
            vec![dev(Device::Mixer, 147, &[c, 3, v as u8])]
        );
        h.click(&format!("mixer{n}.dist"));
        assert_eq!(h.take_log(), vec![dev(Device::Mixer, 147, &[c, 4, 1])]);
        // PCF and Comp are exclusive ids: c + 1 and c + 2
        h.click(&format!("mixer{n}.pcf"));
        assert_eq!(h.take_log(), vec![dev(Device::Mixer, 145, &[2, c + 1])]);
        h.click(&format!("mixer{n}.comp"));
        assert_eq!(h.take_log(), vec![dev(Device::Mixer, 145, &[1, c + 2])]);
        h.calls(2);
        let m = h.snap().mixer;
        let ch = m.channels[c as usize];
        assert_eq!(
            (m.pcf_id, m.comp_id, ch.dist, ch.enabled),
            (c + 1, c + 2, 1, 1)
        );
        assert_eq!(
            (ch.level as i32, ch.pan as i32, ch.send as i32),
            (
                rb_ui::behavior::fader_send(h.app.panel.fader(&format!("mixer{n}.level"))),
                rb_ui::behavior::knob_send(h.app.panel.knob(&format!("mixer{n}.pan"))),
                rb_ui::behavior::knob_send(h.app.panel.knob(&format!("mixer{n}.delay"))),
            )
        );
        // the radio switches turn off with a second click
        h.click(&format!("mixer{n}.pcf"));
        assert_eq!(h.take_log(), vec![dev(Device::Mixer, 145, &[2, 0])]);
        h.click(&format!("mixer{n}.comp"));
        assert_eq!(h.take_log(), vec![dev(Device::Mixer, 145, &[1, 0])]);
    }
    // the master: the fader is id 0, its Comp switch the compressor id 1
    h.drag("master.fader", 10.0);
    let v = rb_ui::behavior::fader_send(h.app.panel.fader("master.fader"));
    assert_eq!(
        h.take_log().last(),
        Some(&dev(Device::Mixer, 145, &[0, v as u8]))
    );
    h.click("master.comp");
    assert_eq!(h.take_log(), vec![dev(Device::Mixer, 145, &[1, 1])]);
    h.calls(2);
    let m = h.snap().mixer;
    assert_eq!((m.master as i32, m.comp_id), (v, 1));
}

#[test]
fn the_effect_arrows_send_steps_and_the_pcf_wave() {
    let mut h = Harness::new();
    h.click("delay.steps_up");
    h.click("delay.steps_down");
    h.click("delay.steps_down");
    assert_eq!(
        h.take_log(),
        vec![
            dev(Device::Delay, 84, &[1, 4]),
            dev(Device::Delay, 84, &[1, 3]),
            dev(Device::Delay, 84, &[1, 2]),
        ]
    );
    // the panel shows the engine's wave (the new song's default, 24) and steps from there
    let wave = h.snap().live[Device::Pcf as usize].as_slice()[4];
    assert_eq!(h.app.panel.display("pcf.pattern"), Some(wave as i32));
    h.click("pcf.pattern_up");
    h.click("pcf.pattern_up");
    assert_eq!(
        h.take_log(),
        vec![
            dev(Device::Pcf, 114, &[4, wave + 1]),
            dev(Device::Pcf, 114, &[4, wave + 2])
        ]
    );
    h.calls(2);
    let s = h.snap();
    assert_eq!(s.live[Device::Delay as usize].as_slice()[1], 2);
    assert_eq!(s.live[Device::Pcf as usize].as_slice()[4], wave + 2);
    assert_eq!(h.app.panel.display("delay.steps"), Some(2));
    assert_eq!(h.app.panel.display("pcf.pattern"), Some(wave as i32 + 2));
}

#[test]
fn the_transport_sends_msg_35() {
    let mut h = Harness::new();
    h.click("transport.mode");
    h.click("transport.loop");
    h.click("transport.tempo_up");
    h.drag("transport.shuffle", 10.0);
    let shuffle = rb_ui::behavior::knob_send(h.app.panel.knob("transport.shuffle")) as u32;
    h.click("transport.loop_start_up");
    h.click("transport.loop_length_up");
    h.click("transport.ff");
    h.click("transport.bar_up");
    h.click("transport.record");
    h.click("transport.record");
    h.click("transport.play");
    h.calls(2);
    assert_eq!(
        h.take_log(),
        vec![
            global(0, 1),
            global(3, 1),
            global(4, 141_000),
            global(20, shuffle),
            // loop start up: {12, start} then {16, end} (the new song's loop is bar 1, one
            // bar long: the length is kept)
            global(12, BAR as u32),
            global(16, 2 * BAR as u32),
            // length up
            global(16, 3 * BAR as u32),
            // fast forward 10 bars, bar up
            global(8, 10 * BAR as u32),
            global(8, 11 * BAR as u32),
            global(2, 1),
            global(2, 0),
            global(1, 1),
        ]
    );
    let s = h.snap();
    assert!(s.playing && s.song_mode && s.loop_on);
    assert_eq!(
        (s.tempo, s.shuffle as u32, s.loop_start, s.loop_end),
        (141_000, shuffle, BAR as u32, 3 * BAR as u32)
    );
    // Stop while playing is {1, 0}; stopped, Stop locates ({8, t}) to the loop start, then to 0
    h.click("transport.stop");
    assert_eq!(h.take_log(), vec![global(1, 0)]);
    h.calls(2);
    assert!(!h.snap().playing);
    h.click("transport.stop");
    h.click("transport.stop");
    h.calls(2);
    let log = h.take_log();
    assert_eq!(log.len(), 2);
    assert!(matches!(log[0], Msg::Global { id: 8, .. }));
    assert_eq!(log[1], global(8, 0));
    assert_eq!(h.snap().tick, 0);
    assert_eq!(h.app.panel.display("transport.bar"), Some(1));
}

#[test]
fn the_pattern_sections_send_the_selection_the_header_and_the_switch() {
    let mut h = Harness::new();
    for (r, d) in [
        Device::Tb303_1,
        Device::Tb303_2,
        Device::Tr808,
        Device::Tr909,
    ]
    .into_iter()
    .enumerate()
    {
        let n = r + 1;
        let (hdr, base) = match r {
            0 | 1 => (42, 40),
            2 => (59, 57),
            _ => (76, 74),
        };
        h.take_log();
        // bank C, button 3: pattern 18; the bank click alone sends nothing
        h.click(&format!("pattern{n}.bankC"));
        assert_eq!(h.take_log(), vec![]);
        h.click(&format!("pattern{n}.pattern3"));
        assert_eq!(h.take_log(), vec![dev(d, base, &[1, 18])]);
        // Shuffle and Steps are the header of the selected pattern
        h.click(&format!("pattern{n}.shuffle"));
        h.click(&format!("pattern{n}.steps_down"));
        assert_eq!(
            h.take_log(),
            vec![dev(d, hdr, &[18, 0, 1]), dev(d, hdr, &[18, 1, 15])]
        );
        // on/off is controller 0
        h.click(&format!("pattern{n}.on"));
        assert_eq!(h.take_log(), vec![dev(d, base, &[0, 0])]);
        h.calls(2);
        let live = h.snap().live[d as usize];
        assert_eq!(&live.as_slice()[..2], &[0, 18]);
        // the engine holds the header
        let pat = h.engine_patterns(r);
        let len = if r < 2 { 98 } else { 194 };
        assert_eq!(&pat[18 * len..18 * len + 2], &[1, 15], "row {n}");
    }
}

#[test]
fn pattern_32_on_every_row_stopped_and_playing_does_not_panic() {
    // task-077: fill_views read 12-byte drum steps on the 303 rows and ran past pattern 31
    let mut h = Harness::new();
    for playing in [false, true] {
        if playing {
            h.click("transport.play");
        }
        for n in 1..=4 {
            // bank D, button 8: pattern 32 (index 31)
            h.click(&format!("pattern{n}.bankD"));
            h.click(&format!("pattern{n}.pattern8"));
            h.calls(2);
        }
        // edit a step of pattern 32 on each row
        h.click("tb303_1.key7");
        h.click("tb303_2.key7");
        let o = layout::TR808_ORIGIN;
        h.click_at(Pos2::new(o.0 + 16.0 + 8.0, o.1 + 91.0 + 8.0));
        let o = layout::TR909_ORIGIN;
        h.click_at(Pos2::new(o.0 + 29.0 + 10.0, o.1 + 86.0 + 8.0));
        h.calls(4);
        // the 808 step toggles (1, then 0); the 909 BD step cycles off, low (1), high (2)
        let want = [[7, 7, 1, 1], [7, 7, 0, 2]][playing as usize];
        for (r, &want) in want.iter().enumerate() {
            let len = if r < 2 { 98 } else { 194 };
            assert_eq!(h.app.panel.rows[r].pattern, 31, "row {r}");
            let pat = h.engine_patterns(r);
            assert_eq!(pat.len(), 32 * len);
            // 303: the note of step 1 at 2; drums: the BD column (1) of step 1 at 2 + 1
            let at = 31 * len + if r < 2 { 2 } else { 3 };
            assert_eq!(pat[at], want, "row {r} playing {playing}");
            assert_eq!(
                h.app.link.as_ref().unwrap().patterns()[r][at],
                want,
                "row {r} link"
            );
        }
    }
    h.click("transport.stop");
    h.calls(2);
}

#[test]
fn a_song_whose_pattern_select_is_past_31_opens_on_pattern_32() {
    // task-077: the select byte of a song file was stored unclamped
    for byte in [32u8, 255] {
        let mut h = Harness::new();
        let mut song =
            rb_ui::session::load_song(&root().join("tests/songs/shipped/Calm.rbs")).unwrap();
        song.tb303[0].params[1] = byte;
        song.tb303[1].params[1] = byte;
        song.tr808.params[1] = byte;
        song.tr909.params[1] = byte;
        let link = h.app.link.as_mut().unwrap();
        link.load(&song, Some(samples()), &mut h.app.panel).unwrap();
        for r in 0..4 {
            assert_eq!(h.app.panel.rows[r].pattern, 31, "row {r} byte {byte}");
        }
        h.calls(2);
        h.click("tb303_1.key7");
        h.click("tb303_2.key7");
        let o = layout::TR808_ORIGIN;
        h.click_at(Pos2::new(o.0 + 16.0 + 8.0, o.1 + 91.0 + 8.0));
        let o = layout::TR909_ORIGIN;
        h.click_at(Pos2::new(o.0 + 29.0 + 10.0, o.1 + 86.0 + 8.0));
        h.click("transport.play");
        h.calls(4);
    }
}

#[test]
fn the_303_step_editor_edits_the_pattern_as_the_original() {
    let mut h = Harness::new();
    // key 7 (G) on the edit step 1 of pattern 0: sub 45 {pattern, step, field 0 note, 7}
    h.click("tb303_1.key7");
    // (`tb303_1.accent` is both the accent knob and the accent button)
    h.click_at(Harness::center_of("trigger", "tb303_1.accent"));
    h.click("tb303_1.slide");
    h.click("tb303_1.up");
    h.click("tb303_1.down");
    h.click("tb303_1.note_pause");
    // a new song's steps are rests: Note/Pause flips "rest" to 0
    assert_eq!(
        h.take_log(),
        [
            [0, 0, 0, 7],
            [0, 0, 2, 1],
            [0, 0, 1, 1],
            [0, 0, 3, 1],
            [0, 0, 4, 1],
            [0, 0, 5, 0],
        ]
        .map(|d| dev(Device::Tb303_1, 45, &d))
    );
    // Step moves the edit step (nothing is sent); pitch mode on goes back to step 1 and a key
    // there advances
    h.click("tb303_1.step");
    assert_eq!(h.app.panel.display("tb303_1.edit_step"), Some(2));
    h.click_at(Harness::center("tb303_1.pitch_button"));
    assert_eq!(h.app.panel.display("tb303_1.edit_step"), Some(1));
    h.click("tb303_1.key2");
    h.click("tb303_1.key4");
    assert_eq!(
        h.take_log(),
        [[0, 0, 0, 2], [0, 1, 0, 4]].map(|d| dev(Device::Tb303_1, 45, &d))
    );
    h.calls(2);
    let pat = h.engine_patterns(0);
    // step 1: the flags set above, and the note 2 the pitch-mode key wrote over the 7
    assert_eq!(&pat[2..8], &[2, 1, 1, 1, 1, 0]);
    assert_eq!(&pat[8..14], &[4, 0, 0, 0, 0, 1], "step 2 key 4");
    // The other 303 is untouched
    assert_eq!(
        h.engine_patterns(1)[2..8],
        h.app.link.as_ref().unwrap().patterns()[1][2..8]
    );
    // Clear sends the whole pattern: sub 44 {pattern, 98 bytes}, steps note 0 and rest on
    h.click("tb303_1.clear");
    let log = h.take_log();
    assert_eq!(log.len(), 1);
    let Msg::Device {
        dev: 1,
        sub: 44,
        data,
    } = &log[0]
    else {
        panic!("{log:?}")
    };
    assert_eq!(data.len(), 1 + 98);
    assert_eq!(data[0], 0);
    for s in 0..16 {
        assert_eq!(&data[1 + 2 + 6 * s..1 + 8 + 6 * s], &[0, 0, 0, 0, 0, 1]);
    }
    h.calls(2);
    let pat = h.engine_patterns(0);
    assert!(pat[..98] == data[1..], "the engine has the cleared pattern");
}

#[test]
fn the_808_step_buttons_edit_the_selected_instrument_live() {
    let mut h = Harness::new();
    let o = layout::TR808_ORIGIN;
    let strip = |step: usize| Pos2::new(o.0 + 16.0 + 16.0 * step as f32 + 8.0, o.1 + 91.0 + 8.0);
    // BD is selected at start: instrument 1. Drag over steps 1..4 from an empty step lights all.
    h.move_to(strip(0));
    h.down(Modifiers::NONE);
    h.move_to(strip(3));
    h.up();
    assert_eq!(
        h.take_log(),
        (0..4)
            .map(|s| dev(Device::Tr808, 62, &[0, s, 1, 1]))
            .collect::<Vec<_>>()
    );
    // select the snare from the legend row (x 10 + 22 i) and set step 5
    h.click_at(Pos2::new(o.0 + 10.0 + 22.0 * 2.0 + 11.0, o.1 + 78.0));
    h.click_at(strip(4));
    assert_eq!(h.take_log(), vec![dev(Device::Tr808, 62, &[0, 4, 2, 1])]);
    // a click on a lit step erases it
    h.click_at(strip(0));
    assert_eq!(h.take_log(), vec![dev(Device::Tr808, 62, &[0, 0, 2, 1])]);
    h.click_at(strip(4));
    assert_eq!(h.take_log(), vec![dev(Device::Tr808, 62, &[0, 4, 2, 0])]);
    h.calls(2);
    let pat = h.engine_patterns(2);
    // step s, instrument i at 2 + 12 s + i
    for s in 0..4 {
        assert_eq!(pat[2 + 12 * s + 1], 1, "BD step {s}");
    }
    assert_eq!(pat[2 + 12 * 4 + 2], 0);
    assert_eq!(pat[2], 0);
}

#[test]
fn the_909_step_buttons_send_the_values_of_the_modifiers_and_clear_the_other_hihat() {
    let mut h = Harness::new();
    let o = layout::TR909_ORIGIN;
    let strip = |step: usize| Pos2::new(o.0 + 29.0 + 21.0 * step as f32 + 10.0, o.1 + 86.0 + 8.0);
    let click_mods = |h: &mut Harness, p: Pos2, m: Modifiers| {
        h.move_to(p);
        h.down(m);
        h.up();
    };
    // BD (selected at start): off -> low -> high -> off
    h.click_at(strip(0));
    h.click_at(strip(0));
    h.click_at(strip(0));
    assert_eq!(
        h.take_log(),
        [1, 2, 0].map(|v| dev(Device::Tr909, 79, &[0, 0, 1, v]))
    );
    // Ctrl: 1, Alt: 2, Ctrl+Alt: flam 3
    click_mods(&mut h, strip(1), Modifiers::CTRL);
    click_mods(&mut h, strip(2), Modifiers::ALT);
    click_mods(&mut h, strip(3), Modifiers::CTRL | Modifiers::ALT);
    assert_eq!(
        h.take_log(),
        [(1, 1), (2, 2), (3, 3)].map(|(s, v)| dev(Device::Tr909, 79, &[0, s, 1, v]))
    );
    // CH (instrument 8) on step 5, then OH (9) on the same step clears the CH: two messages
    h.click_at(Pos2::new(o.0 + 236.0 + 21.0 * 2.0 + 10.0, o.1 + 11.0));
    assert_eq!(h.app.panel.inst909, 8);
    h.click_at(strip(4));
    h.click_at(Pos2::new(o.0 + 236.0 + 21.0 * 3.0 + 10.0, o.1 + 11.0));
    assert_eq!(h.app.panel.inst909, 9);
    h.take_log();
    h.click_at(strip(4));
    assert_eq!(
        h.take_log(),
        vec![
            dev(Device::Tr909, 79, &[0, 4, 9, 1]),
            dev(Device::Tr909, 79, &[0, 4, 8, 0])
        ]
    );
    h.calls(2);
    let pat = h.engine_patterns(3);
    let at = |s: usize, i: usize| pat[2 + 12 * s + i];
    assert_eq!((at(1, 1), at(2, 1), at(3, 1)), (1, 2, 3));
    assert_eq!((at(4, 8), at(4, 9)), (0, 1));
}

#[test]
fn edits_are_heard_on_the_next_pass_of_the_pattern() {
    let mut h = Harness::new();
    // the empty song is silent
    h.click("transport.play");
    let silent = h.calls(40);
    assert!(silent.iter().all(|&s| s == 0), "an empty song is silent");
    // light BD steps 1, 5, 9, 13 of the 808 while it plays
    let o = layout::TR808_ORIGIN;
    for s in [0, 4, 8, 12] {
        h.click_at(Pos2::new(o.0 + 16.0 + 16.0 * s as f32 + 8.0, o.1 + 99.0));
    }
    // up to a pass of the pattern later (16 steps = one bar = 75 calls at 140 BPM)
    let heard = h.calls(80);
    assert!(heard.iter().any(|&s| s != 0), "the BD is heard");
    // the pattern lights follow playback: the running light is on a step
    let mut steps = vec![];
    for _ in 0..40 {
        h.calls(1);
        steps.push(h.app.panel.playing_step808);
    }
    assert!(steps.iter().any(|s| s.is_some()));
    assert!(steps.windows(2).any(|w| w[0] != w[1]), "it moves");
}

#[test]
fn a_new_pattern_starts_at_the_bar_line_but_the_panel_shows_it_at_once() {
    let mut h = Harness::new();
    let o = layout::TR808_ORIGIN;
    // pattern 0: BD on step 1; pattern 1 stays empty
    h.click_at(Pos2::new(o.0 + 16.0 + 8.0, o.1 + 99.0));
    h.click("transport.play");
    h.calls(30);
    assert_eq!(h.snap().patterns[2], Some(0));
    h.click("pattern3.pattern2");
    assert_eq!(h.app.panel.rows[2].pattern, 1);
    // the strip shows pattern 1 (empty) at once
    h.calls(1);
    assert!(!h.app.panel.steps808[1][0]);
    // still the old pattern for part of the bar, the new one after the bar line
    let mut change = None;
    for i in 0..120 {
        h.calls(1);
        let s = h.snap();
        if s.patterns[2] == Some(1) {
            change = Some((i, s.tick));
            break;
        }
    }
    let (i, tick) = change.expect("the pattern changes");
    println!("pattern change after {i} calls at tick {tick}");
    assert!(i > 0, "not at once");
    assert!(tick % BAR < 24, "at the bar line: tick {tick}");
    // and back to pattern 0: the strip shows its BD again
    h.click("pattern3.pattern1");
    h.calls(1);
    assert!(h.app.panel.steps808[1][0]);
}

// ---- songs ---------------------------------------------------------------------------------

/// The max |sample| of an output.
fn peak(o: &[i16]) -> i32 {
    o.iter().map(|&s| (s as i32).abs()).max().unwrap_or(0)
}

#[test]
fn a_shipped_song_plays_in_song_mode_and_the_panel_follows() {
    let mut h = Harness::with_song("tests/songs/shipped/Fast Forward.rbs");
    let s = h.snap();
    assert!(s.song_mode && !s.playing);
    assert!(h.app.panel.transport.song_mode);
    h.click("transport.play");
    let mut out = vec![];
    let mut max_bar = 0;
    let mut patterns_seen = std::collections::HashSet::new();
    let mut steps808 = std::collections::HashSet::new();
    let mut meter = 0;
    for _ in 0..600 {
        out.extend(h.calls(1));
        let s = h.snap();
        if s.frames <= h.app.link.as_ref().unwrap().adopts_after() {
            continue; // the call before the commands show (the panel keeps its own values)
        }
        // the panel shows the engine's position, bar and tempo
        assert_eq!(h.app.panel.transport.position, s.tick);
        assert_eq!(h.app.panel.display("transport.bar"), Some(s.tick / BAR + 1));
        assert_eq!(h.app.panel.transport.tempo, s.tempo as i32);
        assert!(h.app.panel.transport.playing);
        max_bar = max_bar.max(s.tick / BAR);
        for r in 0..4 {
            // the selected pattern of each section follows the song's live pattern
            let live = s.live[r + 1].as_slice()[1];
            assert_eq!(h.app.panel.rows[r].pattern, live, "row {r}");
            patterns_seen.insert((r, live));
        }
        steps808.insert(h.app.panel.playing_step808);
        meter = meter.max(h.app.panel.meters["master.meter_l"]);
    }
    assert!(peak(&out) > 1000, "the song is heard");
    assert!(max_bar >= 6, "bar {max_bar}");
    assert!(patterns_seen.len() > 4, "the pattern buttons moved");
    assert!(steps808.len() > 8, "the 808 running light moves");
    assert!(meter > 0, "the master meter moves");
    assert_eq!(h.snap().faulted_calls, 0);
    // Stop: the song stops; stopped, the position stays; Stop again goes to the loop start
    h.click("transport.stop");
    h.calls(2);
    assert!(!h.snap().playing);
    assert!(!h.app.panel.transport.playing);
}

#[test]
fn a_shipped_song_plays_in_pattern_mode_and_the_switch_follows() {
    let mut h = Harness::with_song("tests/songs/shipped/Calm.rbs");
    // to pattern mode before playing: the selected patterns loop
    h.click("transport.mode");
    assert!(!h.app.panel.transport.song_mode);
    h.click("transport.play");
    let out = h.calls(200);
    let s = h.snap();
    assert!(s.playing && !s.song_mode);
    assert!(peak(&out) > 1000, "the patterns are heard");
    assert_eq!(
        h.app.panel.display("transport.bar"),
        None,
        "disabled in pattern mode"
    );
    // a pattern chosen by hand plays after the bar line and stays
    let before = s.patterns[0];
    h.click("pattern1.pattern8");
    h.calls(100);
    assert_eq!(h.snap().patterns[0], Some(7));
    assert_ne!(before, Some(7));
    h.calls(100);
    assert_eq!(h.snap().patterns[0], Some(7));
}

#[test]
fn msg_35_widths_and_record_off_messages() {
    // ids 4, 8, 12, 16 carry a big-endian u32, the others one byte (docs/song_load.md section 2)
    assert_eq!(global(4, 141_000).encode(), [35, 4, 0, 2, 38, 200]);
    assert_eq!(global(8, 768).encode(), [35, 8, 0, 0, 3, 0]);
    assert_eq!(global(12, 768).encode(), [35, 12, 0, 0, 3, 0]);
    assert_eq!(global(16, 1536).encode(), [35, 16, 0, 0, 6, 0]);
    assert_eq!(global(1, 1).encode(), [35, 1, 1]);
    assert_eq!(global(20, 84).encode(), [35, 20, 84]);
    // rewind, fast forward, bar down and Stop turn record off: `{2, 0}` before the move
    let mut h = Harness::new();
    h.click("transport.mode");
    for (id, then) in [
        ("transport.rewind", global(8, 0)),
        ("transport.ff", global(8, 10 * BAR as u32)),
        ("transport.bar_down", global(8, 9 * BAR as u32)),
    ] {
        h.click("transport.record");
        h.take_log();
        h.click(id);
        assert_eq!(h.take_log(), vec![global(2, 0), then], "{id}");
    }
    h.click("transport.record");
    h.take_log();
    h.click("transport.stop");
    assert_eq!(h.take_log()[0], global(2, 0));
    // bar up keeps record on
    h.click("transport.record");
    h.take_log();
    h.click("transport.bar_up");
    assert_eq!(h.take_log().len(), 1);
}

#[test]
fn the_panel_ignores_snapshots_older_than_its_commands() {
    let mut h = Harness::new();
    h.calls(3);
    let before = h.snap().frames;
    h.click("delay.on");
    let link = h.app.link.as_ref().unwrap();
    // the call that has started when the commands are pushed does not drain them: its
    // snapshot (before + one call) is not taken, the one after it is
    assert_eq!(link.adopts_after(), before + CALL_FRAMES as u64);
    let on = h.app.panel.toggle_on("delay.on");
    h.calls(1);
    assert_eq!(h.app.panel.toggle_on("delay.on"), on);
    h.calls(2);
    assert_eq!(h.app.panel.toggle_on("delay.on"), on);
    assert_eq!(
        h.snap().live[Device::Delay as usize].as_slice()[0],
        on as u8
    );
}

#[test]
fn load_sends_the_events_the_panel_had_first() {
    let mut h = Harness::new();
    h.move_to(Harness::center("transport.loop"));
    h.down(Modifiers::NONE);
    // an event still in the panel when a song is loaded is sent, not dropped with the panel
    h.app.panel.events.push(rb_ui::model::Event::Set {
        id: "transport.shuffle".into(),
        value: 40,
    });
    h.take_log();
    let song = rb_ui::session::load_song(&root().join("tests/songs/shipped/Calm.rbs")).unwrap();
    let link = h.app.link.as_mut().unwrap();
    link.load(&song, Some(samples()), &mut h.app.panel).unwrap();
    assert_eq!(h.take_log(), vec![global(20, 40)]);
}

#[test]
fn the_mode_switch_changes_mode_while_playing_and_space_plays_and_stops() {
    // Calm has no PCF: the PCF's step after a return to pattern mode while playing differs from
    // the original's (task-069), so a song with a PCF is not used for this switch.
    let mut h = Harness::with_song("tests/songs/shipped/Calm.rbs");
    h.key(egui::Key::Space);
    assert_eq!(h.take_log(), vec![global(1, 1)]);
    let a = h.calls(50);
    assert!(h.snap().playing && h.snap().song_mode);
    assert!(peak(&a) > 0);
    h.click("transport.mode");
    h.calls(2);
    // pattern mode turns record off (`sub_42A058`, 0x42A4CC), whatever record was
    assert_eq!(h.take_log(), vec![global(0, 0), global(2, 0)]);
    let s = h.snap();
    assert!(s.playing && !s.song_mode && !h.app.panel.transport.song_mode);
    assert!(peak(&h.calls(50)) > 0, "the patterns play on");
    h.click("transport.mode");
    h.calls(50);
    assert!(h.snap().song_mode && h.app.panel.transport.song_mode);
    assert_eq!(h.snap().faulted_calls, 0);
    h.key(egui::Key::Space);
    h.calls(2);
    assert!(!h.snap().playing);
}

#[test]
fn a_song_loaded_while_another_plays_waits_for_play() {
    let mut h = Harness::with_song("tests/songs/shipped/Calm.rbs");
    h.click("transport.play");
    h.calls(20);
    let g = h.snap().generation;
    h.load("tests/songs/shipped/Fast Forward.rbs");
    let s = h.snap();
    assert!(s.pending && s.playing && s.generation == g);
    // the panel shows the new song (the commands address it); Play switches
    let tempo = h.app.panel.transport.tempo;
    h.click("transport.play");
    h.calls(3);
    let s = h.snap();
    assert!(!s.pending && s.generation == g + 1 && s.playing);
    assert_eq!(h.app.panel.transport.tempo, tempo);
    assert_eq!(s.tempo as i32, tempo);
}

#[test]
fn the_knobs_follow_the_songs_automation() {
    // Fast Forward automates controllers: the panel's knobs move with the song
    let mut h = Harness::with_song("tests/songs/shipped/Fast Forward.rbs");
    h.click("transport.play");
    let mut seen = std::collections::HashSet::new();
    for _ in 0..800 {
        h.calls(1);
        let s = h.snap();
        let live = s.live[Device::Tb303_1 as usize].as_slice()[3];
        assert_eq!(
            rb_ui::behavior::knob_send(h.app.panel.knob("tb303_1.cutoff")),
            live as i32
        );
        seen.insert(live);
    }
    assert!(seen.len() > 3, "the cutoff moved: {seen:?}");
}

#[test]
fn the_wind_bar_and_loop_arrows_send_nothing_in_pattern_mode() {
    // `sub_42A058` tests them inside the song-mode block (0x42A074); tempo and the loop switch
    // are outside it
    let mut h = Harness::new();
    for id in [
        "transport.rewind",
        "transport.ff",
        "transport.bar_up",
        "transport.bar_down",
        "transport.loop_start_up",
        "transport.loop_start_down",
        "transport.loop_length_up",
        "transport.loop_length_down",
    ] {
        h.click(id);
    }
    assert_eq!(h.take_log(), vec![]);
    let t = &h.app.panel.transport;
    assert_eq!((t.position, t.loop_start, t.loop_end), (0, 0, BAR));
    h.click("transport.tempo_up");
    h.click("transport.loop");
    assert_eq!(h.take_log(), vec![global(4, 141_000), global(3, 1)]);
    // song mode: they send their messages
    h.click("transport.mode");
    h.click("transport.ff");
    h.click("transport.loop_length_up");
    assert_eq!(
        h.take_log(),
        vec![
            global(0, 1),
            global(8, 10 * BAR as u32),
            global(16, 2 * BAR as u32)
        ]
    );
}
