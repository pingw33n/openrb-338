//! Save in song mode writes the devices' pattern-mode copies, not the live values the tracks
//! and the panel set (task-159): the original's save asks each device for its whole block
//! (303 31, 808 48, 909 65, delay 83, distortion 91, PCF 113, compressor 101, mixer 138), and
//! in song mode the DLL replies with the pattern-mode copy (the mixer's with its live master and
//! channel enables; docs/song_load.md section 4.10).

use std::path::Path;
use std::sync::Arc;

use eframe::egui::{self, Pos2, RawInput, Rect, Vec2};
use orb_engine::tables::Tables;
use orb_player::{Audio, CALL_FRAMES};
use orb_ui::app::App;
use orb_ui::layout::{PANEL_H, PANEL_W};
use orb_ui::link::Link;
use orb_ui::model::Panel;

struct Harness {
    ctx: egui::Context,
    app: App,
    audio: Audio,
    t: f64,
}

impl Harness {
    /// The app with the song `bytes` open from a file in `dir`.
    fn open(dir: &Path, bytes: &[u8]) -> Self {
        let path = dir.join("song.rbs");
        std::fs::write(&path, bytes).unwrap();
        let ctx = egui::Context::default();
        egui_extras::install_image_loaders(&ctx);
        let samples = rbm::standard::builtin_samples();
        let (link, audio) = Link::new(Arc::new(Tables::new()), Some(samples.clone()));
        let mut app = App::with_link(Panel::new(), link, None);
        app.samples = Some(samples);
        app.open_song(&path);
        assert!(app.errors.is_empty(), "{:?}", app.errors);
        Self {
            ctx,
            app,
            audio,
            t: 0.0,
        }
    }

    /// `n` engine calls, each followed by a UI frame.
    fn frames(&mut self, n: usize) {
        let mut buf = vec![0i16; 2 * CALL_FRAMES];
        for _ in 0..n {
            self.audio.render_i16(&mut buf);
            self.t += 0.016;
            let input = RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(PANEL_W, PANEL_H))),
                time: Some(self.t),
                ..Default::default()
            };
            let app = &mut self.app;
            let mut out = self.ctx.run_ui(input, |ui| app.show(ui));
            out.textures_delta.clear();
        }
    }

    fn saved(&self) -> rbs::Song {
        rbs::Song::parse(&self.app.song_bytes().unwrap()).unwrap()
    }
}

/// tests/songs/diag/songs/303_accent0.rbs (song mode) with device chunks that differ from the
/// tracks' tick-0 events: the 303's cutoff 10 and waveform 1 against the events 50 and 0, the
/// mixer's PCF id, channel 1 level and channel 2 pan against the mixer track's, the delay's
/// feedback. `song_mode` sets the mode byte.
fn song(song_mode: u8) -> (rbs::Song, Vec<u8>) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let src = root.join("tests/songs/diag/songs/303_accent0.rbs");
    let mut s = rbs::Song::parse(&std::fs::read(src).unwrap()).unwrap();
    s.glob.song_mode = song_mode;
    let t0 = |s: &rbs::Song, track: usize, ctrl: u8| {
        s.tracks[track]
            .events
            .iter()
            .find(|e| e.pos == 0 && e.ctrl == ctrl)
            .map(|e| e.value)
    };
    // the 303's tick-0 events are cutoff 50 and waveform 0
    assert_eq!((t0(&s, 1, 3), t0(&s, 1, 8)), (Some(50), Some(0)));
    s.tb303[0].params[3] = 10;
    s.tb303[0].params[8] = 1;
    // the chunk's values differ from the mixer track's and the delay track's tick-0 events
    // (or the ctor's, for a controller the song does not set)
    let other = |v: Option<u8>, a: u8, b: u8| if v == Some(a) { b } else { a };
    s.mixer.pcf_id = other(t0(&s, 0, 2), 3, 2);
    s.mixer.channels[0].level = other(t0(&s, 0, 6), 33, 34);
    s.mixer.channels[1].pan = other(t0(&s, 0, 13), 7, 8);
    s.delay.values[3] = other(t0(&s, 5, 3), 11, 12);
    let bytes = s.write().unwrap();
    (s, bytes)
}

/// The task's case: opened, drawn, played and saved without an edit, a song-mode song is the
/// file read, byte for byte, while the panel shows the tracks' values.
#[test]
fn an_unedited_song_mode_song_saves_the_file_read_after_frames() {
    let (_, bytes) = song(1);
    let dir = tempfile::tempdir().unwrap();
    let mut h = Harness::open(dir.path(), &bytes);
    h.frames(4);
    assert_eq!(h.app.panel.value("tb303_1.cutoff"), Some(50));
    assert_eq!(h.app.panel.switch("tb303_1.wave"), 0);
    assert!(
        h.app.song_bytes().unwrap() == bytes,
        "the save after frames differs"
    );
    let t = h.t;
    h.app.panel.press("transport.play", t);
    h.frames(60);
    assert!(h.app.panel.transport.playing);
    assert!(
        h.app.song_bytes().unwrap() == bytes,
        "the save while playing differs"
    );
}

/// A pattern-mode edit goes with the song into song mode (slot 9 copies the live block into
/// the pattern copy) and is saved there; a song-mode edit without record changes only the live
/// value, which the save does not write; back in pattern mode the pattern copy is live again.
#[test]
fn the_pattern_mode_values_are_saved_in_song_mode() {
    let (_, bytes) = song(0);
    let dir = tempfile::tempdir().unwrap();
    let mut h = Harness::open(dir.path(), &bytes);
    h.frames(4);
    assert_eq!(h.app.panel.value("tb303_1.cutoff"), Some(10));
    // pattern mode: cutoff 127 and channel 1 level 127, then song mode
    h.app.panel.set_knob("tb303_1.cutoff", 1000);
    h.app.panel.set_fader("mixer1.level", 1000);
    h.frames(4);
    h.app.panel.set_switch("transport.mode", 1);
    h.frames(4);
    assert!(h.app.panel.transport.song_mode);
    // the panel shows the tracks' values, the save the pattern-mode ones
    assert_eq!(h.app.panel.value("tb303_1.cutoff"), Some(50));
    let s = h.saved();
    assert_eq!(s.glob.song_mode, 1);
    assert_eq!(s.tb303[0].params[3], 127);
    assert_eq!(s.mixer.channels[0].level, 127);
    // a song-mode edit: heard and shown, not saved
    h.app.panel.set_knob("tb303_1.cutoff", 0);
    h.app.panel.set_fader("mixer1.level", 0);
    h.frames(4);
    assert_eq!(h.app.panel.value("tb303_1.cutoff"), Some(0));
    let s = h.saved();
    assert_eq!(s.tb303[0].params[3], 127);
    assert_eq!(s.mixer.channels[0].level, 127);
    // back in pattern mode the pattern-mode values are live, shown and saved
    h.app.panel.set_switch("transport.mode", 0);
    h.frames(4);
    assert_eq!(h.app.panel.value("tb303_1.cutoff"), Some(127));
    let s = h.saved();
    assert_eq!(s.glob.song_mode, 0);
    assert_eq!(s.tb303[0].params[3], 127);
    assert_eq!(s.mixer.channels[0].level, 127);
}

/// The mixer's master and channel enables have no controller: in song mode the save writes
/// their live values (query 138 copies them into the pattern copy first, 0x10015635), so a
/// song-mode edit of them is saved.
#[test]
fn the_mixer_master_and_mutes_are_saved_live_in_song_mode() {
    let (s0, bytes) = song(1);
    let dir = tempfile::tempdir().unwrap();
    let mut h = Harness::open(dir.path(), &bytes);
    h.frames(4);
    h.app.panel.set_fader("master.fader", 0);
    h.app.panel.press("mixer2.mute", h.t);
    h.frames(4);
    let s = h.saved();
    assert_eq!(s.mixer.master, 0);
    assert_eq!(
        s.mixer.channels[1].enabled,
        1 - s0.mixer.channels[1].enabled
    );
    // the controllers stay the pattern-mode copy's
    assert_eq!(s.mixer.channels[0].level, s0.mixer.channels[0].level);
    assert_eq!(s.mixer.pcf_id, s0.mixer.pcf_id);
}
