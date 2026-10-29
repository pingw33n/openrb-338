//! The computer keyboard of the panel (task-070, docs/ui_spec.md section 10), headless: egui
//! `Key` events with a `physical_key` run through `App::show`, and what the panel did is read
//! from the model's events and, with a link, from the messages the link sends.

use std::sync::{Arc, OnceLock};

use eframe::egui::{self, Event, Key, Modifiers, Pos2, RawInput, Rect, Vec2};
use orb_engine::samples::SampleSet;
use orb_engine::song::Device;
use orb_engine::tables::Tables;
use orb_player::Audio;
use orb_ui::app::App;
use orb_ui::browser::Mode;
use orb_ui::keys::KeyMode;
use orb_ui::layout::{PANEL_H, PANEL_W};
use orb_ui::link::{Link, Msg};
use orb_ui::model::{BAR, Event as Out, Panel};

/// The built-in kit orb embeds.
fn samples() -> Arc<SampleSet> {
    rbm::standard::builtin_samples()
}

fn tables() -> Arc<Tables> {
    static T: OnceLock<Arc<Tables>> = OnceLock::new();
    T.get_or_init(|| Arc::new(Tables::new())).clone()
}

struct Harness {
    ctx: egui::Context,
    app: App,
    time: f64,
    _audio: Option<Audio>,
}

impl Harness {
    fn new() -> Self {
        let ctx = egui::Context::default();
        egui_extras::install_image_loaders(&ctx);
        let mut h = Self {
            ctx,
            app: App::new(Panel::new(), None),
            time: 0.0,
            _audio: None,
        };
        h.frame(vec![], Modifiers::NONE);
        h
    }

    /// A panel wired to a player with the original's drum samples.
    fn linked() -> Self {
        let mut h = Self::new();
        let (mut link, audio) = Link::new(tables(), Some(samples()));
        link.log = Some(vec![]);
        h.app.link = Some(link);
        h._audio = Some(audio);
        // one engine call, so the link has the song's pattern arrays
        let mut out = vec![0i16; 2 * orb_player::CALL_FRAMES];
        h._audio.as_mut().unwrap().render_i16(&mut out);
        h.frame(vec![], Modifiers::NONE);
        h.take_log();
        h
    }

    fn frame(&mut self, mut events: Vec<Event>, modifiers: Modifiers) {
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

    fn ev(key: Key, physical: Option<Key>, pressed: bool, mods: Modifiers) -> Event {
        Event::Key {
            key,
            physical_key: physical,
            pressed,
            repeat: false,
            modifiers: mods,
        }
    }

    /// Presses and releases a key (logical = physical) in two frames.
    fn key(&mut self, key: Key) {
        self.key_with(key, Some(key), Modifiers::NONE);
    }

    fn key_with(&mut self, key: Key, physical: Option<Key>, mods: Modifiers) {
        self.frame(vec![Self::ev(key, physical, true, mods)], mods);
        self.frame(vec![Self::ev(key, physical, false, mods)], Modifiers::NONE);
    }

    fn events(&mut self) -> Vec<Out> {
        std::mem::take(&mut self.app.panel.events)
    }

    fn take_log(&mut self) -> Vec<Msg> {
        std::mem::take(self.app.link.as_mut().unwrap().log.as_mut().unwrap())
    }

    fn focus(&mut self, row: usize) {
        self.app.panel.focus = row;
    }
}

fn step303(synth: usize, pattern: u8, step: usize, field: u8, value: u8) -> Out {
    Out::Step303 {
        synth,
        pattern,
        step,
        field,
        value,
    }
}

fn press(id: &str) -> Out {
    Out::Press { id: id.into() }
}

fn set(id: &str, value: i32) -> Out {
    Out::Set {
        id: id.into(),
        value,
    }
}

fn dev(d: Device, sub: u8, data: &[u8]) -> Msg {
    Msg::Device {
        dev: d.id(),
        sub,
        data: data.to_vec(),
    }
}

#[test]
fn the_transport_keys_follow_the_program() {
    let mut h = Harness::new();
    // a 808 row has the focus, so Enter is Play and `/` the loop switch
    h.focus(2);
    h.key(Key::Enter);
    assert_eq!(h.events(), vec![press("transport.play")]);
    assert!(h.app.panel.transport.playing);
    // Space stops a playing transport
    h.key(Key::Space);
    assert_eq!(h.events(), vec![press("transport.stop")]);
    h.key(Key::Slash);
    assert_eq!(h.events(), vec![set("transport.loop", 1)]);
    // Record works in song mode only (task-057, `sub_42A058`)
    h.key(Key::Insert);
    assert_eq!(h.events(), vec![]);
    h.app.panel.set_switch("transport.mode", 1);
    h.events();
    h.key(Key::Insert);
    assert_eq!(h.events(), vec![set("transport.record", 1)]);
    // numpad 5 / 4: fast forward / rewind by 10 bars (record goes off with the move)
    h.key(Key::Num5);
    assert_eq!(
        h.events(),
        vec![
            set("transport.record", 0),
            set("transport.position", 10 * BAR)
        ]
    );
    h.key(Key::Num4);
    assert_eq!(h.events(), vec![set("transport.position", 0)]);
    h.key(Key::Num8);
    assert_eq!(h.events(), vec![set("transport.bar", BAR)]);
    h.key(Key::Num7);
    assert_eq!(h.events(), vec![set("transport.bar", 0)]);
    // numpad 1 / 2: the loop start / end (the default loop is bars 1..5)
    h.key(Key::Num2);
    assert_eq!(h.events(), vec![set("transport.position", 4 * BAR)]);
    h.key(Key::Num1);
    assert_eq!(h.events(), vec![set("transport.position", 0)]);
    // numpad +: tempo up
    h.key(Key::Plus);
    assert_eq!(h.events(), vec![set("transport.tempo", 141_000)]);
    // numpad 0: Stop; stopped, it locates
    h.key(Key::Num0);
    assert_eq!(h.events(), vec![set("transport.position", 0)]);
    // in the patterns mode `-` is the numpad's tempo down
    h.app.key_mode = KeyMode::Patterns;
    h.key(Key::Minus);
    assert_eq!(h.events(), vec![set("transport.tempo", 140_000)]);
}

#[test]
fn up_and_down_move_the_focus_and_wrap() {
    let mut h = Harness::new();
    h.key(Key::ArrowUp);
    assert_eq!(h.app.panel.focus, 3);
    h.key(Key::ArrowDown);
    h.key(Key::ArrowDown);
    assert_eq!(h.app.panel.focus, 1);
}

#[test]
fn the_303_keys_edit_the_edit_step() {
    let mut h = Harness::new();
    // C, then the last pitch key `/` (high C), by position
    h.key(Key::C);
    h.key(Key::Slash);
    h.key(Key::J);
    assert_eq!(
        h.events(),
        vec![
            step303(0, 0, 0, 0, 0),
            step303(0, 0, 0, 0, 12),
            step303(0, 0, 0, 0, 6),
        ]
    );
    h.key(Key::P);
    h.key(Key::OpenBracket);
    h.key(Key::Semicolon);
    h.key(Key::Quote);
    h.key(Key::Equals);
    assert_eq!(
        h.events(),
        vec![
            step303(0, 0, 0, 2, 1),
            step303(0, 0, 0, 1, 1),
            step303(0, 0, 0, 4, 1),
            step303(0, 0, 0, 3, 1),
            step303(0, 0, 0, 5, 0),
        ]
    );
    // Enter is Step, Backspace is Back (a 303 has the focus)
    h.key(Key::Enter);
    h.key(Key::Enter);
    assert_eq!(h.app.panel.display("tb303_1.edit_step"), Some(3));
    h.key(Key::Backspace);
    assert_eq!(h.app.panel.display("tb303_1.edit_step"), Some(2));
    assert!(!h.app.panel.transport.playing, "Enter is not Play here");
    // the second 303 gets the keys when it has the focus
    h.focus(1);
    h.key(Key::M);
    assert_eq!(h.events(), vec![step303(1, 0, 0, 0, 7)]);
}

#[test]
fn minus_and_equals_by_position_whatever_the_layout() {
    // The key right of 0 on a German layout types a letter egui has no key for (sz); the
    // event then carries only the position. On a US keyboard it is `-`.
    let mut h = Harness::new();
    h.key_with(Key::Minus, Some(Key::Minus), Modifiers::NONE);
    h.key_with(Key::Equals, Some(Key::Equals), Modifiers::NONE);
    // a layout where the position types another named key: the logical key is ignored
    h.key_with(Key::Slash, Some(Key::Minus), Modifiers::NONE);
    h.key_with(Key::Num0, Some(Key::Equals), Modifiers::NONE);
    assert_eq!(
        h.events(),
        vec![step303(0, 0, 0, 5, 0), step303(0, 0, 0, 5, 1)]
            .into_iter()
            .cycle()
            .take(4)
            .collect::<Vec<_>>()
    );
    // the drums' AC key: tap on the playing step, the same two keys
    h.focus(3);
    h.app.panel.playing_step909 = Some(4);
    h.key_with(Key::Slash, Some(Key::Minus), Modifiers::NONE);
    h.key_with(Key::Num0, Some(Key::Equals), Modifiers::NONE);
    assert_eq!(
        h.events(),
        vec![Out::Step909 {
            instrument: 0,
            step: 4,
            value: 2
        }],
        "the second tap changes nothing"
    );
    // a Dvorak user: the key at C's position types L, the pitch is still C
    h.focus(0);
    h.key_with(Key::L, Some(Key::C), Modifiers::NONE);
    assert_eq!(h.events(), vec![step303(0, 0, 0, 0, 0)]);
    // no physical key reported: the logical key is the position
    h.key_with(Key::G, None, Modifiers::NONE);
    assert_eq!(h.events(), vec![step303(0, 0, 0, 0, 3)]);
}

#[test]
fn tab_taps_and_shift_tab_deletes_the_playing_step() {
    let mut h = Harness::new();
    h.key(Key::Tab);
    assert_eq!(h.events(), vec![], "a stopped 303 has no playing step");
    h.app.panel.playing_step303 = [Some(3), None];
    h.key(Key::Tab);
    assert_eq!(h.events(), vec![step303(0, 0, 3, 5, 0)]);
    h.key(Key::Tab);
    assert_eq!(h.events(), vec![], "already a note");
    h.key_with(Key::Tab, Some(Key::Tab), Modifiers::SHIFT);
    assert_eq!(h.events(), vec![step303(0, 0, 3, 5, 1)]);
    // the second 303 does not play
    h.focus(1);
    h.key(Key::Tab);
    assert_eq!(h.events(), vec![]);
}

#[test]
fn the_drum_tap_keys_set_instruments_on_the_playing_step() {
    let mut h = Harness::new();
    h.focus(2);
    h.app.panel.playing_step808 = Some(5);
    // S = SD (2), the `'` key = CH (11), the AC key
    h.key(Key::S);
    h.key(Key::Quote);
    h.key(Key::Minus);
    let on = |instrument| Out::Step808 {
        instrument,
        step: 5,
        on: true,
    };
    assert_eq!(h.events(), vec![on(2), on(11), on(0)]);
    // the 909: A = BD (1) with value 1; the keys do not select the instrument
    h.focus(3);
    h.app.panel.playing_step909 = Some(0);
    let sel = h.app.panel.inst909;
    h.key(Key::A);
    assert_eq!(
        h.events(),
        vec![Out::Step909 {
            instrument: 1,
            step: 0,
            value: 1
        }]
    );
    assert_eq!(h.app.panel.inst909, sel);
    // stopped: nothing
    h.app.panel.playing_step909 = None;
    h.key(Key::S);
    assert_eq!(h.events(), vec![]);
}

#[test]
fn ctrl_f_and_ctrl_g_switch_the_mode_and_the_patterns_mode_selects_patterns() {
    let mut h = Harness::new();
    assert_eq!(h.app.key_mode, KeyMode::Synth);
    h.key_with(Key::G, Some(Key::G), Modifiers::COMMAND);
    assert_eq!(h.app.key_mode, KeyMode::Patterns);
    assert_eq!(h.events(), vec![], "the shortcut is no pitch key");
    // 3 selects pattern 3 of row 1, `,` pattern 8 of row 4, Q pattern 1 of row 2: the
    // selection gives the row the focus
    h.key(Key::Num3);
    assert_eq!(h.app.panel.focus, 0);
    h.key(Key::Comma);
    assert_eq!(h.app.panel.focus, 3);
    h.key(Key::Q);
    assert_eq!(h.app.panel.focus, 1);
    assert_eq!(
        h.events(),
        vec![
            Out::Pattern { row: 0, pattern: 2 },
            Out::Pattern { row: 3, pattern: 7 },
            Out::Pattern { row: 1, pattern: 0 },
        ]
    );
    // the synth keys are off: P (Accent) does nothing
    h.key(Key::P);
    assert_eq!(h.events(), vec![]);
    // the shown bank is used
    h.app.panel.rows[2].bank = 2;
    h.key(Key::K);
    assert_eq!(
        h.events(),
        vec![Out::Pattern {
            row: 2,
            pattern: 23
        }]
    );
    h.key_with(Key::F, Some(Key::F), Modifiers::COMMAND);
    assert_eq!(h.app.key_mode, KeyMode::Synth);
}

#[test]
fn the_wind_buttons_repeat_while_the_key_is_held() {
    let mut h = Harness::new();
    h.focus(2);
    h.app.panel.set_switch("transport.mode", 1);
    h.events();
    h.frame(
        vec![Harness::ev(
            Key::Num5,
            Some(Key::Num5),
            true,
            Modifiers::NONE,
        )],
        Modifiers::NONE,
    );
    assert_eq!(h.events().len(), 1);
    // an auto-repeat event of the OS adds nothing by itself
    h.frame(
        vec![Event::Key {
            key: Key::Num5,
            physical_key: Some(Key::Num5),
            pressed: true,
            repeat: true,
            modifiers: Modifiers::NONE,
        }],
        Modifiers::NONE,
    );
    assert_eq!(h.events().len(), 0);
    // held 1 s: the schedule gives more steps (one per 100 ms from 400 ms)
    for _ in 0..62 {
        h.frame(vec![], Modifiers::NONE);
    }
    let held = h.events().len();
    assert!((5..=7).contains(&held), "{held}");
    h.frame(
        vec![Harness::ev(
            Key::Num5,
            Some(Key::Num5),
            false,
            Modifiers::NONE,
        )],
        Modifiers::NONE,
    );
    for _ in 0..40 {
        h.frame(vec![], Modifiers::NONE);
    }
    assert_eq!(h.events().len(), 0, "released");
}

#[test]
fn keys_with_ctrl_and_keys_while_a_browser_is_open_do_nothing() {
    let mut h = Harness::new();
    h.focus(2);
    h.key_with(Key::Space, Some(Key::Space), Modifiers::CTRL);
    assert_eq!(h.events(), vec![]);
    h.app.open_browser(Mode::OpenSong);
    h.frame(vec![], Modifiers::NONE);
    h.key(Key::Space);
    h.key(Key::Num0);
    h.key(Key::ArrowDown);
    assert_eq!(h.events(), vec![]);
    assert_eq!(h.app.panel.focus, 2);
    assert!(h.app.browsing());
}

#[test]
fn the_keys_send_the_messages_of_the_buttons() {
    let mut h = Harness::linked();
    // Play from a drum row, then Step-less Enter is Play; the 303 key and the pattern key send
    // what the mouse sends
    h.focus(2);
    h.key(Key::Enter);
    h.focus(0);
    h.key(Key::V);
    h.key(Key::Equals);
    h.app.key_mode = KeyMode::Patterns;
    h.key(Key::Num2);
    h.key(Key::Y);
    assert_eq!(
        h.take_log(),
        vec![
            Msg::Global { id: 1, value: 1 },
            dev(Device::Tb303_1, 45, &[0, 0, 0, 2]),
            dev(Device::Tb303_1, 45, &[0, 0, 5, 0]),
            dev(Device::Tb303_1, 40, &[1, 1]),
            dev(Device::Tb303_2, 40, &[1, 5]),
        ]
    );
}

#[test]
fn a_held_key_shows_its_button_pressed_and_shift_equals_is_tempo_up() {
    let mut h = Harness::new();
    h.focus(2);
    assert!(!h.app.key_held("transport.play"));
    let enter = |pressed| Harness::ev(Key::Enter, Some(Key::Enter), pressed, Modifiers::NONE);
    h.frame(vec![enter(true)], Modifiers::NONE);
    assert!(h.app.key_held("transport.play"));
    h.frame(vec![enter(false)], Modifiers::NONE);
    assert!(!h.app.key_held("transport.play"));
    h.events();
    // Shift+= is `+`: tempo up, and no Note/Pause alias
    h.focus(0);
    h.key_with(Key::Equals, Some(Key::Equals), Modifiers::SHIFT);
    assert_eq!(h.events(), vec![set("transport.tempo", 141_000)]);
}

#[test]
fn the_wind_bar_keys_do_nothing_in_pattern_mode() {
    // `sub_42A058` tests them inside the song-mode block (0x42A074); tempo is outside it
    let mut h = Harness::new();
    h.focus(2);
    assert!(!h.app.panel.transport.song_mode);
    for k in [Key::Num4, Key::Num5, Key::Num7, Key::Num8] {
        h.key(k);
        assert_eq!(h.events(), vec![], "{k:?}");
    }
    assert_eq!(h.app.panel.transport.position, 0);
    h.key(Key::Plus);
    assert_eq!(h.events(), vec![set("transport.tempo", 141_000)]);
    // song mode: they work
    h.app.panel.set_switch("transport.mode", 1);
    h.events();
    h.key(Key::Num5);
    assert_eq!(h.events(), vec![set("transport.position", 10 * BAR)]);
    h.key(Key::Num7);
    assert_eq!(h.events(), vec![set("transport.bar", 9 * BAR)]);
}
