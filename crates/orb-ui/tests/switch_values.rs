//! A song whose switch controllers hold values out of 0..=1 (task-156: RB-AM.rbm's demo song
//! has a 303 waveform of 127, which closed the app on a missing art part) opens, plays and
//! draws frames: each switch shows the position the engine plays, keeps the song's value, and
//! no part is missing.

use std::path::Path;
use std::sync::Arc;

use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2};
use orb_engine::tables::Tables;
use orb_player::{Audio, CALL_FRAMES};
use orb_ui::app::App;
use orb_ui::art;
use orb_ui::layout::{self, PANEL_H, PANEL_W};
use orb_ui::link::Link;
use orb_ui::model::Panel;

/// The values written into the song, with the position each switch must show.
const SWITCHES: [(&str, u8, u8); 9] = [
    ("tb303_1.wave", 127, 1),
    ("tb303_2.wave", 2, 1),
    ("tr808.slot3_switch", 255, 1),
    ("tr808.slot4_switch", 64, 1),
    ("tr808.slot5_switch", 0, 0),
    ("tr808.slot6_switch", 200, 1),
    ("tr808.slot7_switch", 9, 1),
    // the PCF's low-pass is the value 1 only (`cmp dl, 1` at 0x10016921)
    ("pcf.mode", 2, 0),
    ("delay.triplet", 127, 1),
];

/// The song with the values of `SWITCHES` and 909 steps 0, 4, 7, 255, in pattern mode (0) or
/// song mode (1).
fn song_bytes(song_mode: u8) -> Vec<u8> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let src = root.join("tests/songs/diag/songs/303_accent0.rbs");
    let mut s = rbs::Song::parse(&std::fs::read(src).unwrap()).unwrap();
    // the devices' values and the tracks' tick-0 events (in pattern mode the panel keeps the
    // devices' values; in song mode it follows the tracks)
    let set = |s: &mut rbs::Song, track: usize, ctrl: u8, v: u8| {
        for e in &mut s.tracks[track].events {
            if e.ctrl == ctrl {
                e.value = v;
            }
        }
        match track {
            1 | 2 => s.tb303[track - 1].params[ctrl as usize] = v,
            3 => s.tr808.params[ctrl as usize] = v,
            5 => s.delay.values[ctrl as usize] = v,
            7 => s.pcf.values[ctrl as usize] = v,
            _ => unreachable!(),
        }
    };
    set(&mut s, 1, 8, 127);
    set(&mut s, 2, 8, 2);
    for (c, v) in [(11, 255), (14, 64), (17, 0), (19, 200), (21, 9)] {
        set(&mut s, 3, c, v);
    }
    set(&mut s, 7, 6, 2);
    set(&mut s, 5, 2, 127);
    s.glob.song_mode = song_mode;
    // 909 step bytes above the art's frames 0..=3
    for p in &mut s.tr909.patterns {
        for (k, step) in p.steps.iter_mut().enumerate() {
            step[0] = [0, 4, 7, 255][k % 4];
        }
    }
    s.write().unwrap()
}

fn frame(ctx: &egui::Context, app: &mut App, t: f64, events: Vec<Event>) {
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(PANEL_W, PANEL_H))),
        time: Some(t),
        events,
        ..Default::default()
    };
    let mut out = ctx.run_ui(input, |ui| app.show(ui));
    out.textures_delta.clear();
}

/// The app with the song `bytes` open from a file in `dir`.
fn open(dir: &Path, bytes: &[u8]) -> (egui::Context, App, Audio) {
    let path = dir.join("switches.rbs");
    std::fs::write(&path, bytes).unwrap();
    let ctx = egui::Context::default();
    egui_extras::install_image_loaders(&ctx);
    let samples = rbm::standard::builtin_samples();
    let (link, audio) = Link::new(Arc::new(Tables::new()), Some(samples.clone()));
    let mut app = App::with_link(Panel::new(), link, None);
    app.samples = Some(samples);
    app.open_song(&path);
    assert!(app.errors.is_empty(), "{:?}", app.errors);
    (ctx, app, audio)
}

#[test]
fn out_of_range_switch_values_open_play_and_draw() {
    let dir = tempfile::tempdir().unwrap();
    let (ctx, mut app, mut audio) = open(dir.path(), &song_bytes(0));

    let mut buf = vec![0i16; 2 * CALL_FRAMES];
    let mut t = 0.0;
    for _ in 0..4 {
        audio.render_i16(&mut buf);
        t += 0.016;
        frame(&ctx, &mut app, t, vec![]);
    }
    let controls = layout::controls();
    for (id, v, pos) in SWITCHES {
        assert_eq!(app.panel.switch(id), v, "{id} keeps the song's value");
        assert_eq!(app.panel.switch_position(id), pos, "{id} shows");
        let c = controls.iter().find(|c| c.id == id).unwrap();
        assert_eq!(
            app.panel.part_name(c, false, t),
            format!("{}{pos}", c.family())
        );
    }

    // play, show the 909 (its steps 0, 4, 7, 255 of the bass drum) and draw some frames
    app.panel.inst909 = 1;
    app.panel.press("transport.play", t);
    for _ in 0..30 {
        audio.render_i16(&mut buf);
        t += 0.016;
        frame(&ctx, &mut app, t, vec![]);
    }

    // a click flips the position shown: 127 (square) goes to 0 (saw)
    let c = controls.iter().find(|c| c.id == "tb303_1.wave").unwrap();
    let p = Pos2::new(c.x + c.w / 2.0, c.y + c.h / 2.0);
    let button = |pressed| Event::PointerButton {
        pos: p,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    for e in [Event::PointerMoved(p), button(true), button(false)] {
        t += 0.016;
        frame(&ctx, &mut app, t, vec![e]);
    }
    assert_eq!(app.panel.switch("tb303_1.wave"), 0);

    assert!(
        art::missing().is_empty(),
        "missing parts: {:?}",
        art::missing()
    );
}

/// Opened and saved without an edit, the song is the file read: the out-of-range switch values
/// (303 waveforms 127 and 2, 808 selectors 255, 64, 200, 9, PCF mode 2, delay mode 127) and
/// the 909 step bytes 4, 7, 255 are written back as the song holds them. The save comes before
/// the first frame, after the panel has drawn and followed the engine, and while the song
/// plays. In song mode the panel shows the tracks' values, which the load's range check makes
/// in-range defaults for these switches (waveform 2 becomes 0, the 808 selector 255 becomes
/// 0, the PCF mode 2 becomes 1, the delay mode 127 becomes 1), and the save still writes the
/// devices' pattern-mode copies, the file's bytes, as the original's does (task-159).
#[test]
fn out_of_range_switch_values_save_back_unchanged() {
    for song_mode in [0, 1] {
        let bytes = song_bytes(song_mode);
        let dir = tempfile::tempdir().unwrap();
        let (ctx, mut app, mut audio) = open(dir.path(), &bytes);
        let saved = app.song_bytes().unwrap();
        assert!(
            saved == bytes,
            "song mode {song_mode}: the save before a frame differs"
        );
        let mut buf = vec![0i16; 2 * CALL_FRAMES];
        let mut t = 0.0;
        for i in 0..40 {
            if i == 4 {
                let saved = app.song_bytes().unwrap();
                assert!(
                    saved == bytes,
                    "song mode {song_mode}: the save after frames differs"
                );
                app.panel.press("transport.play", t);
            }
            audio.render_i16(&mut buf);
            t += 0.016;
            frame(&ctx, &mut app, t, vec![]);
        }
        assert!(app.panel.transport.playing);
        if song_mode == 1 {
            // the panel follows the tracks (the switch events the load accepted, the defaults
            // for the ones it dropped)
            assert_eq!(app.panel.switch("tb303_2.wave"), 0);
            assert_eq!(app.panel.switch("pcf.mode"), 1);
        }
        let saved = app.song_bytes().unwrap();
        assert!(
            saved == bytes,
            "song mode {song_mode}: the save while playing differs"
        );
    }
}
