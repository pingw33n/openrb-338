//! The menu (task-047), headless: saving a song gives back the file that was read, the edits
//! of the panel are saved, the export writes the file `rb render` writes, the mod and the
//! Vintage flag change and are saved, and the unsaved-changes prompt guards opening and closing.
//!
//! The drum machines play the built-in kit rb embeds (`assets/Standard.rbm`). The mod tests
//! need the recrypted shipped mods, which `rb-testkit` makes from the CD image (`RB_CD_ISO`);
//! they skip when it is not set.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2};
use rb_cli::mods::{Kit, ModOpts, STANDARD};
use rb_cli::render::{DEFAULT_IDLE_BLOCKS, DEFAULT_QUALITY, Renderer, load_song};
use rb_engine::samples::SampleSet;
use rb_engine::tables::Tables;
use rb_player::{Audio, CALL_FRAMES};
use rb_ui::app::App;
use rb_ui::browser::Mode;
use rb_ui::document::compose;
use rb_ui::keys::KeyMode;
use rb_ui::layout::{self, PANEL_H, PANEL_W};
use rb_ui::link::{self, Link};
use rb_ui::menu::{Answer, Cmd, MENU_H};
use rb_ui::model::Panel;
use rb_ui::session;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn tables() -> Arc<Tables> {
    static T: OnceLock<Arc<Tables>> = OnceLock::new();
    T.get_or_init(|| Arc::new(Tables::new())).clone()
}

/// The built-in kit, as the app has it.
fn samples() -> Arc<SampleSet> {
    rbm::standard::builtin_samples()
}

/// The standard mod rb embeds, as a file: `assets/Standard.rbm`.
fn standard_mod() -> PathBuf {
    root().join("assets/Standard.rbm")
}

/// A recrypted shipped mod (made from the CD image, `RB_CD_ISO`, by `rb-testkit`), or a skip.
fn mod_file(name: &str) -> Option<PathBuf> {
    rb_testkit::recrypted_mods().map(|d| d.join(name))
}

fn songs(dir: &str) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(root().join(dir))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "rbs"))
        .collect();
    v.sort();
    v
}

struct Harness {
    ctx: egui::Context,
    app: App,
    audio: Audio,
    time: f64,
    height: f32,
}

impl Harness {
    fn new() -> Self {
        let ctx = egui::Context::default();
        egui_extras::install_image_loaders(&ctx);
        let (link, audio) = Link::new(tables(), Some(samples()));
        let mut app = App::with_link(Panel::new(), link, None);
        app.samples = Some(samples());
        let mut h = Self {
            ctx,
            app,
            audio,
            time: 0.0,
            height: PANEL_H,
        };
        h.calls(1);
        h
    }

    /// With the menu bar, in a window as high as the panel and the bar.
    fn with_menu() -> Self {
        let mut h = Self::new();
        h.app.menu = true;
        h.height = PANEL_H + MENU_H;
        h.frame(vec![]);
        h
    }

    fn frame(&mut self, events: Vec<Event>) {
        self.frame_shapes(events);
    }

    /// A frame; the debug text of the shapes it painted (the labels drawn show in it).
    fn frame_shapes(&mut self, mut events: Vec<Event>) -> String {
        self.time += 0.016;
        events.insert(0, Event::ModifiersChanged(Modifiers::NONE));
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(
                Pos2::ZERO,
                Vec2::new(PANEL_W, self.height),
            )),
            time: Some(self.time),
            events,
            ..Default::default()
        };
        let app = &mut self.app;
        let mut out = self.ctx.run_ui(input, |ui| app.show(ui));
        out.textures_delta.clear();
        format!("{:?}", out.shapes)
    }

    /// `n` engine calls, then a UI frame that takes the snapshot.
    fn calls(&mut self, n: usize) -> Vec<i16> {
        let mut out = vec![0i16; 2 * CALL_FRAMES * n];
        for c in out.chunks_mut(2 * CALL_FRAMES) {
            self.audio.render_i16(c);
        }
        self.frame(vec![]);
        out
    }

    fn open(&mut self, path: &Path) {
        self.app.open_song(path);
        assert!(self.app.notice.is_none(), "{:?}", self.app.notice);
        self.calls(2);
    }

    fn click(&mut self, p: Pos2) {
        let button = |pressed| Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        self.frame(vec![Event::PointerMoved(p)]);
        self.frame(vec![button(true)]);
        self.frame(vec![button(false)]);
    }

    /// Waits for a running export.
    fn finish_export(&mut self) {
        for _ in 0..6000 {
            if !self.app.exporting() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
            self.frame(vec![]);
        }
        panic!("the export did not finish");
    }
}

fn read_rbs(path: &Path) -> rbs::Song {
    rbs::Song::parse(&std::fs::read(path).unwrap()).unwrap()
}

/// Each track's events as (tick, controller, value), sorted: a save and a reopen keep the
/// events but reverse the order of a tick's events, as the original does (the load inserts
/// each event before those of its tick, `docs/initial_investigation.md` section 5).
fn track_sets(h: &mut Harness) -> Vec<Vec<(i32, u8, u8)>> {
    let seq = &h.audio.current_mut().frame.seq;
    (0..seq.tracks.len())
        .map(|t| {
            let mut v: Vec<_> = seq
                .events(t)
                .iter()
                .map(|e| (e.tick, e.ctrl, e.value))
                .collect();
            v.sort_unstable();
            v
        })
        .collect()
}

/// task-057: a song recorded in song mode (knob moves while playing, then record off, which
/// merges them into the tracks) is modified, saves the engine's tracks, reopens with them, saves
/// back unchanged, and renders as the recorded engine state does.
#[test]
fn a_recorded_song_saves_its_tracks() {
    let src = root().join("tests/songs/diag/songs/303_saw_lowcut.rbs");
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("recorded.rbs");
    let mut h = Harness::new();
    h.open(&src);
    if !h.app.panel.transport.song_mode {
        h.app.panel.press("transport.mode", 0.0);
    }
    h.app.panel.press("transport.record", 0.0);
    h.app.panel.press("transport.play", 0.0);
    h.frame(vec![]);
    h.calls(20);
    assert!(h.app.panel.transport.recording);
    for (k, v) in [200, 700, 450].into_iter().enumerate() {
        h.app.panel.set_knob("tb303_1.cutoff", v);
        h.app.panel.set_knob("tb303_1.resonance", 1000 - v);
        h.frame(vec![]);
        h.calls(10 + k);
    }
    assert!(h.app.link.as_ref().unwrap().recorded_tracks().is_none());
    h.app.panel.press("transport.record", 0.0);
    h.frame(vec![]);
    h.calls(2);
    assert!(!h.app.panel.transport.recording);
    let link = h.app.link.as_ref().unwrap();
    let recorded = link.recorded_tracks().expect("the merge's tracks");
    assert!(recorded[1].iter().any(|e| e.tick > 0 && e.ctrl == 3));
    assert!(h.app.doc.modified, "recording is an edit");
    h.app.save_to(&out);
    let saved = read_rbs(&out);
    let a = read_rbs(&src);
    assert_ne!(saved.tracks, a.tracks);
    let live = track_sets(&mut h);

    let mut h2 = Harness::new();
    h2.open(&out);
    assert_eq!(track_sets(&mut h2), live, "the reopened tracks");
    let again = dir.path().join("again.rbs");
    h2.app.save_to(&again);
    assert_eq!(std::fs::read(&again).unwrap(), std::fs::read(&out).unwrap());

    // the saved song renders as the source song with the recorded tracks (in the engine's
    // list order) does
    let mut want = load_song(&src).unwrap_or_else(|_| panic!("source"));
    let seq = &h.audio.current_mut().frame.seq;
    for (t, tr) in seq.tracks.iter().enumerate() {
        want.tracks[t] = tr
            .events(&seq.pool)
            .iter()
            .map(|e| rb_engine::song::TrackEvent {
                tick: e.tick as u32,
                ctrl: e.ctrl,
                value: e.value,
            })
            .collect();
    }
    let got = load_song(&out).unwrap_or_else(|_| panic!("saved"));
    let mut r = Renderer::new(ModOpts::default());
    let render = |r: &mut Renderer, s| {
        r.render("x", s, DEFAULT_IDLE_BLOCKS)
            .unwrap_or_else(|rb_cli::Fail(_, m)| panic!("{m}"))
    };
    let (g, w) = (render(&mut r, &got), render(&mut r, &want));
    assert!(g == w, "the saved song renders otherwise");
    let plain = render(
        &mut r,
        &load_song(&src).unwrap_or_else(|_| panic!("source")),
    );
    assert!(g != plain, "the recording is not heard in the export");
}

#[test]
fn every_controller_has_a_panel_value() {
    let panel = Panel::new();
    for id in link::controls().keys() {
        assert!(panel.value(id).is_some(), "no value for {id}");
    }
    for id in [
        "master.fader",
        "transport.shuffle",
        "mixer1.mute",
        "mixer2.level",
        "mixer3.pan",
        "mixer4.delay",
        "mixer1.dist",
    ] {
        assert!(panel.value(id).is_some(), "no value for {id}");
    }
}

/// A song that is opened and saved without an edit is the file read.
#[test]
fn saving_an_unedited_corpus_song_gives_back_the_file() {
    let mut files = songs("tests/songs/diag/songs");
    files.extend(songs("tests/songs/shipped"));
    assert!(files.len() > 40, "{} songs", files.len());
    let mut different = Vec::new();
    let mut older = 0;
    for path in &files {
        let opened = session::open_song(path).unwrap();
        let (mut link, _audio) = Link::new(tables(), None);
        let mut panel = Panel::new();
        link.load(&opened.engine, None, &mut panel).unwrap();
        let song = compose(&opened.source, &panel, &link, &opened.engine.mod_name);
        let bytes = song.write().unwrap();
        let a = read_rbs(path);
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if (a.head.major, a.head.minor, a.head.revision) == (4, 2, 0) {
            // a 4.2 file is saved byte for byte
            if bytes != std::fs::read(path).unwrap() {
                different.push(format!("{name}: {}", first_difference(&a, &song)));
            }
        } else {
            // an older file is saved as a 4.2 file that loads to the same engine song (the
            // load converts old values; the saved ones are the converted)
            older += 1;
            let again = rbs::Song::parse(&bytes).unwrap();
            let (x, y) = (&opened.engine, rb_engine::load::convert(&again).unwrap());
            let field = if x.globals != y.globals {
                "globals"
            } else if x.mixer != y.mixer {
                "mixer"
            } else if x.delay != y.delay || x.pcf != y.pcf || x.dist != y.dist || x.comp != y.comp {
                "an effect"
            } else if x.tb303 != y.tb303 {
                "a 303"
            } else if x.tr808 != y.tr808 {
                "the 808"
            } else if x.tr909 != y.tr909 {
                "the 909"
            } else if x.tracks != y.tracks {
                "the tracks"
            } else if *x != y {
                "something else"
            } else {
                ""
            };
            if !field.is_empty() {
                different.push(format!("{name}: loads differently ({field})"));
            }
        }
    }
    assert!(
        different.is_empty(),
        "{} of {} songs differ:\n{}",
        different.len(),
        files.len(),
        different.join("\n")
    );
    eprintln!("{} songs, {older} of them older than 4.2", files.len());
}

/// What differs between two songs, for the message of a failing test.
fn first_difference(a: &rbs::Song, b: &rbs::Song) -> String {
    macro_rules! chk {
        ($($f:tt)+) => {
            if a.$($f)+ != b.$($f)+ {
                return stringify!($($f)+).to_string();
            }
        };
    }
    chk!(head);
    chk!(glob);
    chk!(usri);
    chk!(mixer);
    chk!(delay);
    chk!(pcf);
    chk!(dist);
    chk!(comp);
    chk!(tb303[0].params);
    chk!(tb303[1].params);
    chk!(tb303[0].patterns);
    chk!(tb303[1].patterns);
    chk!(tr808.params);
    chk!(tr808.patterns);
    chk!(tr909.params);
    chk!(tr909.patterns);
    chk!(tracks);
    "unknown chunks or the bytes".into()
}

#[test]
fn save_and_save_as_reopen_identically() {
    let src = root().join("tests/songs/diag/songs/303_saw_lowcut.rbs");
    let dir = tempfile::tempdir().unwrap();
    let mut h = Harness::new();
    h.open(&src);
    assert_eq!(h.app.doc.path.as_deref(), Some(src.as_path()));
    assert!(!h.app.doc.modified);

    // Save As: a copy, and it becomes the song's file
    let copy = dir.path().join("copy.rbs");
    h.app.command(Cmd::SaveAs);
    assert!(h.app.browsing());
    h.app.chosen(Mode::SaveSong, copy.clone());
    assert_eq!(h.app.doc.path.as_deref(), Some(copy.as_path()));
    assert_eq!(
        std::fs::read(&copy).unwrap(),
        std::fs::read(&src).unwrap(),
        "the copy differs from the source"
    );

    // an edit is saved by Save, over the same file, and re-opens as it was
    h.app.panel.set_knob("tb303_1.cutoff", 900);
    h.app.panel.set_fader("master.fader", 400);
    h.app.panel.set_switch("transport.mode", 0);
    h.frame(vec![]);
    assert!(h.app.doc.modified);
    h.app.command(Cmd::Save);
    assert!(!h.app.doc.modified);
    let saved = read_rbs(&copy);
    assert_eq!(saved.tb303[0].params[3], 114, "cutoff 900 -> 114");
    assert_eq!(saved.mixer.master, 51);
    assert_eq!(saved.glob.song_mode, 0);

    let mut h2 = Harness::new();
    h2.open(&copy);
    // (the knob's position is the engine's value 114 spread over 0..1000)
    assert_eq!(h2.app.panel.value("tb303_1.cutoff"), Some(114));
    assert_eq!(h2.app.panel.value("master.fader"), Some(51));
    assert!(!h2.app.panel.transport.song_mode);
    // and saving it again changes nothing
    let again = dir.path().join("again.rbs");
    h2.app.save_to(&again);
    assert_eq!(
        std::fs::read(&again).unwrap(),
        std::fs::read(&copy).unwrap()
    );
}

#[test]
fn save_without_a_file_asks_for_one_and_a_failed_write_is_shown() {
    let mut h = Harness::new();
    assert!(h.app.doc.path.is_none());
    h.app.command(Cmd::Save);
    assert!(h.app.browsing(), "Save on a new song is Save As");

    let dir = tempfile::tempdir().unwrap();
    let bad = dir.path().join("no/such/dir/x.rbs");
    h.app.chosen(Mode::SaveSong, bad);
    let n = h.app.notice.clone().unwrap();
    assert!(n.contains("cannot write"), "{n}");
    assert!(h.app.doc.path.is_none());

    // a new song saves too, and re-opens as the panel showed it
    let ok = dir.path().join("new.rbs");
    h.app.chosen(Mode::SaveSong, ok.clone());
    assert!(h.app.notice.is_none(), "{:?}", h.app.notice);
    let s = read_rbs(&ok);
    assert_eq!(s.head.version(), "4.2");
    assert_eq!(s.glob.mod_name, STANDARD);
    assert_eq!(s.tb303[0].patterns.len(), 32);
    let mut h2 = Harness::new();
    h2.open(&ok);
}

#[test]
fn an_edit_of_each_kind_is_saved() {
    let src = root().join("tests/songs/diag/songs/303_saw_lowcut.rbs");
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("edited.rbs");
    let mut h = Harness::new();
    h.open(&src);
    let p = &mut h.app.panel;
    p.set_knob("tr808.bd_level", 1000);
    p.set_knob("tr909.flam", 500);
    p.set_knob("delay.feedback", 250);
    p.set_fader("mixer2.level", 600);
    p.press("tb303_2.accent", 0.0);
    p.press("mixer3.mute", 0.0);
    h.frame(vec![]);
    h.app.save_to(&out);
    let a = read_rbs(&src);
    let b = read_rbs(&out);
    assert_eq!(b.tr808.params[3], 127);
    assert_eq!(b.tr909.params[29], 64);
    assert_eq!(b.delay.values[3], 32);
    assert_eq!(b.mixer.channels[1].level, 76);
    assert_eq!(b.mixer.channels[2].enabled, 1 - a.mixer.channels[2].enabled);
    // what the panel cannot change is as read
    assert_eq!(a.tracks, b.tracks);
    assert_eq!(a.usri, b.usri);
    assert_eq!(a.head, b.head);
}

#[test]
fn the_vintage_flag_toggles_the_sound_model_and_is_saved() {
    let src = root().join("tests/songs/diag/songs/303_saw_lowcut.rbs");
    let dir = tempfile::tempdir().unwrap();
    let mut h = Harness::new();
    h.open(&src);
    assert!(!h.app.panel.vintage);
    assert!(!h.audio.current_mut().frame.vintage);

    h.app.command(Cmd::ToggleVintage);
    h.frame(vec![]);
    h.calls(2);
    assert!(h.app.panel.vintage);
    assert!(
        h.audio.current_mut().frame.vintage,
        "the engine's sound model"
    );
    assert!(h.app.doc.modified);
    let out = dir.path().join("vintage.rbs");
    h.app.save_to(&out);
    assert_eq!(read_rbs(&out).glob.vintage, 1);

    // it opens vintage, and toggles back
    let mut h2 = Harness::new();
    h2.open(&out);
    assert!(h2.app.panel.vintage);
    assert!(h2.audio.current_mut().frame.vintage);
    h2.app.command(Cmd::ToggleVintage);
    h2.frame(vec![]);
    h2.calls(2);
    assert!(!h2.audio.current_mut().frame.vintage);
    let back = dir.path().join("back.rbs");
    h2.app.save_to(&back);
    assert_eq!(read_rbs(&back).glob.vintage, 0);
}

#[test]
fn the_keyboard_mode_is_set_from_the_menu() {
    let mut h = Harness::new();
    h.app.command(Cmd::SetKeyMode(KeyMode::Patterns));
    assert_eq!(h.app.key_mode, KeyMode::Patterns);
    h.app.command(Cmd::SetKeyMode(KeyMode::Synth));
    assert_eq!(h.app.key_mode, KeyMode::Synth);
}

/// Opens `src`, exports it to each format, and compares each file with what `rb render` writes
/// for the song as saved (and, unedited, for the source file).
#[test]
fn export_writes_the_file_rb_render_writes() {
    let dir = tempfile::tempdir().unwrap();
    // a 303 song and a song with the drum machines
    for src in [
        root().join("tests/songs/diag/songs/303_saw_lowcut.rbs"),
        root().join("tests/songs/shipped/Calm.rbs"),
    ] {
        let mut h = Harness::new();
        h.open(&src);
        assert!(h.app.panel.transport.song_mode, "{}", src.display());
        // an edit, so that the export differs from the source's render
        h.app.panel.set_knob("tb303_1.cutoff", 300);
        h.frame(vec![]);
        let saved = dir.path().join("saved.rbs");
        h.app.save_to(&saved);
        let song = load_song(&saved).unwrap_or_else(|_| panic!("saved song"));
        for ext in ["wav", "flac", "ogg"] {
            let got = dir.path().join(format!("ui.{ext}"));
            let want = dir.path().join(format!("cli.{ext}"));
            h.app.command(Cmd::Export);
            assert!(h.app.browsing());
            h.app.chosen(Mode::Export, got.clone());
            assert!(h.app.exporting());
            h.finish_export();
            assert!(h.app.notice.is_none(), "{:?}", h.app.notice);
            let format = match ext {
                "wav" => rb_cli::encode::Format::Wav,
                "flac" => rb_cli::encode::Format::Flac,
                _ => rb_cli::encode::Format::Ogg,
            };
            Renderer::new(ModOpts::default())
                .render_to_file(
                    "x",
                    &song,
                    DEFAULT_IDLE_BLOCKS,
                    format,
                    DEFAULT_QUALITY,
                    &want,
                )
                .unwrap_or_else(|rb_cli::Fail(_, m)| panic!("{m}"));
            let (a, b) = (std::fs::read(&got).unwrap(), std::fs::read(&want).unwrap());
            assert!(a.len() > 1000, "{ext} is empty");
            assert!(
                a == b,
                "{} as {ext} differs from rb render's",
                src.display()
            );
        }
    }
}

#[test]
fn export_is_not_offered_in_pattern_mode_and_its_errors_are_shown() {
    let mut h = Harness::new();
    h.app.command(Cmd::Export);
    assert!(!h.app.browsing());
    assert!(h.app.notice.as_deref().unwrap().contains("song mode"));
    h.app.notice = None;

    let src = root().join("tests/songs/diag/songs/303_saw_lowcut.rbs");
    h.open(&src);
    assert!(h.app.panel.transport.song_mode);
    // a wrong extension, and a folder that is not there
    h.app.chosen(Mode::Export, PathBuf::from("/tmp/x.mp3"));
    assert!(h.app.notice.as_deref().unwrap().contains(".wav"));
    let dir = tempfile::tempdir().unwrap();
    h.app.chosen(Mode::Export, dir.path().join("no/dir/x.wav"));
    h.finish_export();
    let n = h.app.notice.clone().unwrap();
    assert!(n.contains("cannot write"), "{n}");
}

#[test]
fn the_unsaved_prompt_guards_opening_another_song() {
    let a = root().join("tests/songs/diag/songs/303_saw_lowcut.rbs");
    let b = root().join("tests/songs/diag/songs/303_accent0.rbs");
    let dir = tempfile::tempdir().unwrap();
    let work = dir.path().join("work.rbs");
    std::fs::copy(&a, &work).unwrap();
    let mut h = Harness::new();
    h.open(&work);

    // unmodified: opens at once
    h.app.open_song(&b);
    assert!(!h.app.prompting());
    assert_eq!(h.app.doc.path.as_deref(), Some(b.as_path()));
    h.open(&work);

    // modified: asks. Cancel keeps the song and its edit.
    h.app.panel.set_knob("tb303_1.cutoff", 123);
    h.frame(vec![]);
    assert!(h.app.doc.modified);
    h.app.open_song(&b);
    assert!(h.app.prompting());
    assert_eq!(h.app.doc.path.as_deref(), Some(work.as_path()));
    h.app.answer(Answer::Cancel);
    assert!(!h.app.prompting());
    assert_eq!(h.app.doc.path.as_deref(), Some(work.as_path()));
    assert!(h.app.doc.modified);

    // Save: writes the file, then opens the other song
    h.app.open_song(&b);
    h.app.answer(Answer::Save);
    assert_eq!(h.app.doc.path.as_deref(), Some(b.as_path()));
    assert_eq!(read_rbs(&work).tb303[0].params[3], 16, "cutoff 123 -> 16");

    // Don't Save: opens it and drops the edit
    h.open(&work);
    h.app.panel.set_knob("tb303_1.cutoff", 1000);
    h.frame(vec![]);
    h.app.open_song(&b);
    h.app.answer(Answer::DontSave);
    assert_eq!(h.app.doc.path.as_deref(), Some(b.as_path()));
    assert_eq!(read_rbs(&work).tb303[0].params[3], 16);
}

#[test]
fn the_unsaved_prompt_guards_closing_and_save_as_for_a_new_song() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = Harness::new();
    // nothing to lose: closes
    assert!(h.app.request_close());
    h.app.panel.set_knob("tb303_1.cutoff", 700);
    h.frame(vec![]);
    assert!(h.app.doc.modified);
    assert!(!h.app.request_close());
    assert!(h.app.prompting());
    h.app.answer(Answer::Cancel);
    assert!(!h.app.close_allowed());

    // Save on a song with no file: the browser, and the close goes on after the write
    assert!(!h.app.request_close());
    h.app.answer(Answer::Save);
    assert!(h.app.browsing());
    assert!(!h.app.close_allowed());
    let out = dir.path().join("closing.rbs");
    h.app.chosen(Mode::SaveSong, out.clone());
    assert!(out.is_file());
    assert!(h.app.close_allowed());

    let mut h = Harness::new();
    h.app.panel.set_knob("tb303_1.cutoff", 700);
    h.frame(vec![]);
    assert!(!h.app.request_close());
    h.app.answer(Answer::DontSave);
    assert!(h.app.close_allowed());
}

#[test]
fn play_and_locate_are_not_edits() {
    let src = root().join("tests/songs/diag/songs/303_saw_lowcut.rbs");
    let mut h = Harness::new();
    h.open(&src);
    let c = layout::controls()
        .into_iter()
        .find(|c| c.id == "transport.play")
        .unwrap();
    h.app.panel.press("transport.play", 0.0);
    h.app.panel.press("transport.stop", 0.0);
    h.app.panel.press("transport.stop", 0.0);
    h.frame(vec![]);
    let _ = c;
    assert!(!h.app.doc.modified, "{:?}", h.app.panel.events);
}

#[test]
fn the_menu_bar_sits_above_the_panel_and_opens() {
    let mut h = Harness::with_menu();
    // the panel is below the bar: Play, MENU_H lower, is pressed
    let c = layout::controls()
        .into_iter()
        .find(|c| c.id == "transport.play")
        .unwrap();
    let pos = Pos2::new(c.x + c.w / 2.0, c.y + c.h / 2.0 + MENU_H);
    h.click(pos);
    assert!(h.app.panel.transport.playing);
    // File opens a menu
    h.click(Pos2::new(14.0, MENU_H / 2.0));
    assert!(egui::Popup::is_any_open(&h.ctx), "no menu opened");
}

#[test]
fn a_mod_changes_the_kit_and_is_saved() {
    let Some(pbe2) = mod_file("PBE2.rbm") else {
        return;
    };
    let src = root().join("tests/songs/diag/songs/drums_kit.rbs");
    let dir = tempfile::tempdir().unwrap();
    let play = |h: &mut Harness| {
        h.app.panel.press("transport.play", 0.0);
        // (the frame sends Play; the engine calls after it play)
        h.frame(vec![]);
        h.calls(40)
    };

    let mut std_h = Harness::new();
    std_h.open(&src);
    let std_out = play(&mut std_h);
    assert!(std_out.iter().any(|&s| s != 0), "the song is silent");

    let mut mod_h = Harness::new();
    // without the startup sounds of the open and the change (the song opened with the mod
    // below has it off too)
    mod_h.app.settings.startup_sound = false;
    mod_h.open(&src);
    mod_h.app.command(Cmd::UseKit(Kit::Mod(pbe2.clone())));
    assert!(mod_h.app.notice.is_none(), "{:?}", mod_h.app.notice);
    assert!(mod_h.app.doc.modified);
    let name = session::mod_info(&pbe2).unwrap().name;
    assert_eq!(mod_h.app.doc.mod_name, name);
    let mod_out = play(&mut mod_h);
    assert_ne!(
        std_out, mod_out,
        "the mod's kit sounds like the standard one"
    );

    // the save names it, and the song opens with it from the Mods folder
    let saved = dir.path().join("with_mod.rbs");
    mod_h.app.save_to(&saved);
    let s = read_rbs(&saved);
    assert_eq!(s.glob.mod_name, name);
    let mut open_h = Harness::new();
    open_h.app.settings.mods_dir = Some(pbe2.parent().unwrap().to_path_buf());
    open_h.app.settings.startup_sound = false;
    open_h.open(&saved);
    assert_eq!(open_h.app.doc.kit, Kit::Mod(pbe2.clone()));
    assert_eq!(play(&mut open_h), mod_out, "opened with its own mod");

    // back to the standard kit
    mod_h.app.command(Cmd::UseKit(Kit::Standard));
    assert_eq!(mod_h.app.doc.mod_name, STANDARD);
    mod_h.app.save_to(&saved);
    assert_eq!(read_rbs(&saved).glob.mod_name, STANDARD);
    let mut back = Harness::new();
    back.open(&saved);
    assert_eq!(play(&mut back), std_out);

    // the export uses the mod
    let wav = dir.path().join("m.wav");
    let mut e = Harness::new();
    e.open(&src);
    e.app.command(Cmd::UseKit(Kit::Mod(pbe2.clone())));
    e.app.chosen(Mode::Export, wav.clone());
    e.finish_export();
    assert!(e.app.notice.is_none(), "{:?}", e.app.notice);
    let song = load_song(&src).unwrap();
    let mods = ModOpts {
        mod_file: Some(pbe2),
        ..ModOpts::default()
    };
    let want = dir.path().join("w.wav");
    Renderer::new(mods)
        .render_to_file(
            "x",
            &song,
            DEFAULT_IDLE_BLOCKS,
            rb_cli::encode::Format::Wav,
            DEFAULT_QUALITY,
            &want,
        )
        .unwrap_or_else(|rb_cli::Fail(_, m)| panic!("{m}"));
    assert!(std::fs::read(&wav).unwrap() == std::fs::read(&want).unwrap());
}

/// `base` with the preview `audio` mixed into its calls from the start, as the player does.
fn with_preview(base: &[i16], audio: &[i16]) -> Vec<i16> {
    let mut out = base.to_vec();
    let mut pos = 0;
    for c in out.chunks_mut(2 * CALL_FRAMES) {
        pos = rb_player::mix_preview(c, audio, pos);
    }
    out
}

/// task-074: with "Play Mod Startup Sound" on (the default), a mod change plays the new mod's
/// startup sound after the song; off, it does not.
#[test]
fn a_mod_change_plays_the_startup_sound_when_the_preference_is_on() {
    let Some(pbe2) = mod_file("PBE2.rbm") else {
        return;
    };
    let src = root().join("tests/songs/diag/songs/drums_kit.rbs");
    let kit = rb_cli::render::load_mod(&pbe2, &samples()).unwrap_or_else(|e| panic!("{}", e.1));
    let audio = kit.get(rb_engine::samples::STARTUP_ID).audio();
    let calls = audio.len().div_ceil(CALL_FRAMES) + 2;
    let run = |on: bool| {
        let mut h = Harness::new();
        assert!(h.app.settings.startup_sound, "on by default");
        h.app.settings.startup_sound = on;
        h.open(&src);
        h.app.panel.press("transport.play", 0.0);
        h.frame(vec![]);
        h.calls(3);
        h.app.command(Cmd::UseKit(Kit::Mod(pbe2.clone())));
        assert!(h.app.notice.is_none(), "{:?}", h.app.notice);
        let out = h.calls(calls);
        (out, h.audio.preview_position())
    };
    let (off, pos_off) = run(false);
    let (on, pos_on) = run(true);
    assert_eq!((pos_off, pos_on), (None, None), "the sound ended");
    assert_ne!(on, off);
    assert_eq!(on, with_preview(&off, audio));
}

/// task-087: opening a song plays its mod's startup sound when the preference is on
/// (`sub_425C4D` -> `sub_426C67`, 0x426400), not when it is off: the built-in kit's for a song
/// on "Standard ReBirth", the mod's for a song on a mod of the Mods folder.
#[test]
fn a_song_open_plays_its_mods_startup_sound_when_the_preference_is_on() {
    let src = root().join("tests/songs/diag/songs/drums_kit.rbs");
    let run = |path: &Path, mods_dir: Option<&Path>, on: bool, calls: usize| {
        let mut h = Harness::new();
        h.app.settings.mods_dir = mods_dir.map(Path::to_path_buf);
        h.app.settings.startup_sound = on;
        h.app.open_song(path);
        assert!(h.app.notice.is_none(), "{:?}", h.app.notice);
        let out = h.calls(calls);
        (out, h.audio.preview_position())
    };
    let standard = samples();
    let audio = standard.get(rb_engine::samples::STARTUP_ID).audio();
    let calls = audio.len().div_ceil(CALL_FRAMES) + 2;
    let (off, pos_off) = run(&src, None, false, calls);
    let (on, pos_on) = run(&src, None, true, calls);
    assert_eq!((pos_off, pos_on), (None, None), "the sound ended");
    assert_ne!(on, off);
    assert_eq!(on, with_preview(&off, audio));

    // a song on a mod plays that mod's sound
    let Some(pbe2) = mod_file("PBE2.rbm") else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let saved = dir.path().join("pbe2.rbs");
    let mut h = Harness::new();
    h.app.settings.startup_sound = false;
    h.open(&src);
    h.app.command(Cmd::UseKit(Kit::Mod(pbe2.clone())));
    h.app.save_to(&saved);
    assert!(h.app.notice.is_none(), "{:?}", h.app.notice);
    let kit = rb_cli::render::load_mod(&pbe2, &standard).unwrap_or_else(|e| panic!("{}", e.1));
    let audio = kit.get(rb_engine::samples::STARTUP_ID).audio();
    assert_ne!(audio, standard.get(rb_engine::samples::STARTUP_ID).audio());
    let calls = audio.len().div_ceil(CALL_FRAMES) + 2;
    let folder = pbe2.parent().unwrap();
    let (off, _) = run(&saved, Some(folder), false, calls);
    let (on, pos_on) = run(&saved, Some(folder), true, calls);
    assert_eq!(pos_on, None, "the sound ended");
    assert_eq!(on, with_preview(&off, audio));
}

/// task-087: the start plays the startup sound once when the preference is on: of the new
/// empty song on "Standard ReBirth" without a song (`sub_40A50D` -> `sub_426CA8`, 0x426D48),
/// of the song's mod with one (the open); never when it is off.
#[test]
fn the_start_plays_the_startup_sound_when_the_preference_is_on() {
    let src = root().join("tests/songs/diag/songs/drums_kit.rbs");
    let standard = samples();
    let audio = standard.get(rb_engine::samples::STARTUP_ID).audio();
    let calls = audio.len().div_ceil(CALL_FRAMES) + 2;
    let run = |song: bool, on: bool| {
        let mut h = Harness::new();
        h.app.settings.startup_sound = on;
        let opened = song.then(|| (src.clone(), session::open_song(&src).unwrap()));
        h.app.start(opened);
        assert!(h.app.notice.is_none(), "{:?}", h.app.notice);
        assert_eq!(h.app.doc.path.as_deref(), song.then_some(src.as_path()));
        let out = h.calls(calls);
        (out, h.audio.preview_position())
    };
    for song in [false, true] {
        let (off, pos_off) = run(song, false);
        let (on, pos_on) = run(song, true);
        assert_eq!((pos_off, pos_on), (None, None), "the sound ended");
        assert_ne!(on, off, "song {song}");
        // once: a second preview would restart it
        assert_eq!(on, with_preview(&off, audio), "song {song}");
    }
}

/// task-074: a click on the logo plays the song's mod's startup sound, whatever the
/// preference (`sub_42A799` does not test it); elsewhere in the transport it does not.
#[test]
fn the_logo_plays_the_startup_sound() {
    let mut h = Harness::new();
    h.app.settings.startup_sound = false;
    h.click(Pos2::new(440.0, 25.0));
    h.calls(1);
    assert_eq!(h.audio.preview_position(), None);
    let before = h.calls(4);
    h.click(Pos2::new(540.0, 25.0));
    let out = h.calls(4);
    assert_eq!(h.audio.preview_position(), Some(4 * CALL_FRAMES));
    let audio = samples()
        .get(rb_engine::samples::STARTUP_ID)
        .audio()
        .to_vec();
    // the new empty song renders the same in every call while stopped
    assert_eq!(
        before[..2 * CALL_FRAMES],
        before[2 * CALL_FRAMES..4 * CALL_FRAMES]
    );
    assert_eq!(out, with_preview(&before, &audio));
}

#[test]
fn a_cancelled_save_as_from_the_prompt_ends_the_action() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = Harness::new();
    h.app.panel.set_knob("tb303_1.cutoff", 700);
    h.frame(vec![]);
    assert!(!h.app.request_close());
    h.app.answer(Answer::Save);
    assert!(h.app.browsing());
    // Esc in the browser cancels it
    h.frame(vec![Event::Key {
        key: egui::Key::Escape,
        physical_key: Some(egui::Key::Escape),
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    assert!(!h.app.browsing());
    // a plain Save As later does not close the window
    h.app.chosen(Mode::SaveSong, dir.path().join("later.rbs"));
    assert!(h.app.notice.is_none(), "{:?}", h.app.notice);
    assert!(!h.app.close_allowed());
    assert!(!h.app.doc.modified);
}

/// The panel follows the engine's mixer distortion insert and a pattern section's on switch
/// (two fixes of `Panel::adopt`).
#[test]
fn the_panel_takes_the_distortion_insert_and_the_section_switch_from_the_song() {
    let dist = root().join("tests/songs/diag/songs/mixer_dist_switch.rbs");
    let src = read_rbs(&dist);
    assert!(
        src.mixer.channels.iter().any(|c| c.dist != 0),
        "no insert on"
    );
    let mut h = Harness::new();
    h.open(&dist);
    for (c, ch) in src.mixer.channels.iter().enumerate() {
        let id = format!("mixer{}.dist", c + 1);
        assert_eq!(h.app.panel.toggle_on(&id), ch.dist != 0, "{id}");
        assert_eq!(h.app.panel.value(&id), Some(ch.dist), "{id}");
    }

    let off = root().join("tests/songs/shipped/A Taste of Haste.rbs");
    assert_eq!(
        read_rbs(&off).tb303[0].params[0],
        0,
        "the 303 section is on"
    );
    h.open(&off);
    assert!(!h.app.panel.toggle_on("pattern1.on"));
    assert_eq!(h.app.panel.value("pattern1.on"), Some(0));
    // and back on with a song that has it on
    h.open(&dist);
    let on = read_rbs(&dist).tb303[0].params[0];
    assert_eq!(h.app.panel.value("pattern1.on"), Some(on));
}

#[test]
fn the_mods_menu_lists_the_mods_folder() {
    let Some(pbe2) = mod_file("PBE2.rbm") else {
        return;
    };
    let mut h = Harness::new();
    h.app.settings.mods_dir = pbe2.parent().map(Path::to_path_buf);
    let list = h.app.list_mods().unwrap();
    assert!(list.iter().any(|e| e.path == pbe2), "{list:?}");
    assert!(list.len() >= 3);

    // a mod that is not found gives the built-in kit, a notice, and keeps the name
    // (an empty folder that exists: a missing one falls back to the binary's RB_MOD_DIR)
    let empty = tempfile::tempdir().unwrap();
    h.app.settings.mods_dir = Some(empty.path().to_path_buf());
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("x.rbs");
    let mut s = rbs::Song::empty();
    s.glob.mod_name = "No Such Mod".into();
    std::fs::write(&out, s.write().unwrap()).unwrap();
    h.app.open_song(&out);
    assert!(h.app.notice.as_deref().unwrap().contains("No Such Mod"));
    assert_eq!(h.app.doc.kit, Kit::Standard);
    assert_eq!(h.app.doc.mod_name, "No Such Mod");
}

/// task-101: the Mods menu's list is read at start, by Mods > Refresh List and when the Mods
/// folder changes; drawing the menu reads no folder, and a song open does not fill the list;
/// the refresh also clears the demo answer kept for the current mod file.
#[test]
fn refresh_list_reads_the_mods_folder_again() {
    let (Some(pbe2), Some(msm2)) = (mod_file("PBE2.rbm"), mod_file("MSM2.rbm")) else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("mods");
    std::fs::create_dir(&folder).unwrap();
    std::fs::copy(&pbe2, folder.join("PBE2.rbm")).unwrap();
    let names = |h: &Harness| -> Vec<std::path::PathBuf> {
        h.app
            .mods()
            .unwrap()
            .as_ref()
            .unwrap()
            .iter()
            .map(|e| e.path.clone())
            .collect()
    };

    let mut h = Harness::with_menu();
    assert!(h.app.mods().is_none());
    h.app.settings.mods_dir = Some(folder.clone());
    h.app.start(None);
    assert_eq!(names(&h), vec![folder.join("PBE2.rbm")]);

    // a mod added to the folder is not seen by drawing the Mods menu: find the "Mods" title
    // by clicking along the bar until the menu with "Refresh List" is open, close it, add the
    // mod, and open it again
    std::fs::copy(&msm2, folder.join("MSM2.rbm")).unwrap();
    let mut mods_at = None;
    for x in (10..200).step_by(6) {
        let p = Pos2::new(x as f32, MENU_H / 2.0);
        h.frame(vec![Event::PointerMoved(p)]);
        let down = |pressed| Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        h.frame(vec![down(true)]);
        h.frame(vec![down(false)]);
        let text = h.frame_shapes(vec![]);
        if text.contains("Refresh List") {
            mods_at = Some(p);
            break;
        }
    }
    let mods_at = mods_at.expect("the Mods menu did not open");
    // task-118: Choose file..., Refresh List, a separator, then Demo Song and Info...
    let drawn = h.frame_shapes(vec![]);
    let at = |t: &str| drawn.find(t).unwrap_or_else(|| panic!("{t} is not drawn"));
    assert!(at("Choose file...") < at("Refresh List"));
    assert!(at("Refresh List") < at("Demo Song"));
    assert!(at("Demo Song") < at("Info..."));
    // a separator is a line segment: one between Refresh List and Demo Song, none between
    // Choose file... and Refresh List (so the two form one group)
    let lines = |a: &str, b: &str| drawn[at(a)..at(b)].matches("LineSegment").count();
    assert_eq!(lines("Refresh List", "Demo Song"), 1, "{drawn}");
    assert_eq!(lines("Choose file...", "Refresh List"), 0);
    // the menu is open and drawn over a folder that changed: the list stays
    h.frame(vec![]);
    assert!(h.frame_shapes(vec![]).contains("Refresh List"));
    assert_eq!(names(&h), vec![folder.join("PBE2.rbm")]);
    // closed and opened again: still the list read at start
    h.click(mods_at);
    h.click(mods_at);
    assert!(h.frame_shapes(vec![]).contains("Refresh List"));
    assert_eq!(names(&h), vec![folder.join("PBE2.rbm")]);
    h.click(mods_at);
    // a song open resolves its mod in the folder but does not fill the list
    let song = root().join("tests/songs/diag/songs/drums_kit.rbs");
    h.app.doc.modified = false;
    h.open(&song);
    assert_eq!(names(&h), vec![folder.join("PBE2.rbm")]);
    // Refresh List sees it
    h.app.command(Cmd::RefreshMods);
    let mut got = names(&h);
    got.sort();
    assert_eq!(got, vec![folder.join("MSM2.rbm"), folder.join("PBE2.rbm")]);

    // the demo answer of the current mod is kept until the refresh
    h.app.use_kit(Kit::Mod(folder.join("PBE2.rbm")));
    assert!(h.app.has_demo());
    std::fs::remove_file(folder.join("PBE2.rbm")).unwrap();
    h.calls(2);
    assert!(h.app.has_demo());
    assert_eq!(names(&h).len(), 2);
    // a mod removed: Refresh List drops it, and the demo answer is read again
    h.app.command(Cmd::RefreshMods);
    assert_eq!(names(&h), vec![folder.join("MSM2.rbm")]);
    assert!(!h.app.has_demo());

    // a changed Mods folder setting is read by the browser path; here the refresh it calls
    let other = dir.path().join("other");
    std::fs::create_dir(&other).unwrap();
    h.app.settings.mods_dir = Some(other);
    assert_eq!(names(&h).len(), 1);
    h.app.refresh_mods();
    assert!(names(&h).is_empty());
}

/// task-118: Mods > Refresh List reports on the status line what it read (the count, the folder
/// and what changed), and a folder that cannot be read on the notice line.
#[test]
fn refresh_list_reports_its_result() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("mods");
    std::fs::create_dir(&folder).unwrap();
    let mut h = Harness::new();
    h.app.settings.mods_dir = Some(folder.clone());
    h.app.notice = Some("unrelated".into());
    h.app.command(Cmd::RefreshMods);
    let want = format!(
        "Mod list refreshed: 0 mods in {} (+0, -0)",
        folder.display()
    );
    assert_eq!(h.app.status.as_deref(), Some(want.as_str()));
    // an unrelated notice stays
    assert_eq!(h.app.notice.as_deref(), Some("unrelated"));

    if let (Some(pbe2), Some(msm2)) = (mod_file("PBE2.rbm"), mod_file("MSM2.rbm")) {
        std::fs::copy(&pbe2, folder.join("PBE2.rbm")).unwrap();
        h.app.command(Cmd::RefreshMods);
        let want = format!("Mod list refreshed: 1 mod in {} (+1, -0)", folder.display());
        assert_eq!(h.app.status.as_deref(), Some(want.as_str()));
        std::fs::copy(&msm2, folder.join("MSM2.rbm")).unwrap();
        std::fs::remove_file(folder.join("PBE2.rbm")).unwrap();
        h.app.command(Cmd::RefreshMods);
        let want = format!("Mod list refreshed: 1 mod in {} (+1, -1)", folder.display());
        assert_eq!(h.app.status.as_deref(), Some(want.as_str()));
    }

    // a folder that cannot be read: the notice, no success line
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let locked = dir.path().join("locked");
        std::fs::create_dir(&locked).unwrap();
        struct Restore(PathBuf);
        impl Drop for Restore {
            fn drop(&mut self) {
                let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o755));
            }
        }
        let _restore = Restore(locked.clone());
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
        // (running as root reads it anyway: then there is no error to show)
        if std::fs::read_dir(&locked).is_err() {
            h.app.settings.mods_dir = Some(locked.clone());
            h.app.command(Cmd::RefreshMods);
            assert_eq!(h.app.status, None);
            assert!(
                !h.app
                    .notice
                    .clone()
                    .expect("the read error is shown")
                    .is_empty()
            );
            assert!(matches!(h.app.mods(), Some(Err(_))));
            // a good read after it takes the error's notice away
            h.app.settings.mods_dir = Some(folder.clone());
            h.app.command(Cmd::RefreshMods);
            assert_eq!(h.app.notice, None);
            assert!(
                h.app
                    .status
                    .as_deref()
                    .unwrap()
                    .starts_with("Mod list refreshed")
            );
        }
    }
}

/// task-093: the Mods menu checks the current song's mod: by the song's mod name, following a
/// song open and a mod change; a mod that is not in the folder checks nothing.
#[test]
fn the_mods_menu_checks_the_songs_mod() {
    let Some(pbe2) = mod_file("PBE2.rbm") else {
        return;
    };
    let mut h = Harness::new();
    h.app.settings.mods_dir = pbe2.parent().map(Path::to_path_buf);
    let list = h.app.list_mods().unwrap();
    let pbe2_at = list.iter().position(|e| e.path == pbe2).unwrap();

    // the start: the standard kit
    assert_eq!(h.app.mod_checks(&list), (true, None));

    // a mod change
    h.app.use_kit(Kit::Mod(pbe2.clone()));
    assert_eq!(h.app.mod_checks(&list), (false, Some(pbe2_at)));
    h.app.use_kit(Kit::Standard);
    assert_eq!(h.app.mod_checks(&list), (true, None));

    // a song open: a song that names the mod, then one that names a mod that is missing
    let dir = tempfile::tempdir().unwrap();
    let mut s = rbs::Song::empty();
    s.glob.mod_name = list[pbe2_at].name.clone();
    let named = dir.path().join("named.rbs");
    std::fs::write(&named, s.write().unwrap()).unwrap();
    h.app.doc.modified = false; // the mod changes above asked for the unsaved prompt
    h.app.open_song(&named);
    assert_eq!(h.app.mod_checks(&list), (false, Some(pbe2_at)));
    s.glob.mod_name = "No Such Mod".into();
    let missing = dir.path().join("missing.rbs");
    std::fs::write(&missing, s.write().unwrap()).unwrap();
    h.app.open_song(&missing);
    assert_eq!(h.app.mod_checks(&list), (false, None));

    // a song that names the mod by its file name (`mods::find_in_dir`'s fallback) plays it
    s.glob.mod_name = "pbe2".into();
    let by_file = dir.path().join("by_file.rbs");
    std::fs::write(&by_file, s.write().unwrap()).unwrap();
    h.app.open_song(&by_file);
    assert_eq!(h.app.doc.kit, Kit::Mod(pbe2.clone()));
    assert_eq!(h.app.mod_checks(&list), (false, Some(pbe2_at)));

    // a mod file from outside the folder with a folder mod's INFO name checks nothing
    let outside = dir.path().join("Outside.rbm");
    std::fs::copy(&pbe2, &outside).unwrap();
    h.app.doc.modified = false;
    h.app.use_kit(Kit::Standard);
    h.app.use_kit(Kit::Mod(outside.clone()));
    assert_eq!(h.app.doc.kit, Kit::Mod(outside));
    assert_eq!(h.app.doc.mod_name, list[pbe2_at].name);
    assert_eq!(h.app.mod_checks(&list), (false, None));
}

/// task-088: a song the node pool refuses (more than 100000 events) opens no song: the alert's
/// text is shown, the open song, its document and the panel stay, and no startup sound plays
/// (sub_40B575's STOP alert, docs/song_load.md section 5.2).
#[test]
fn a_song_the_node_pool_refuses_is_not_opened() {
    let events = |n: u32, ctrls: [u8; 2]| -> Vec<rbs::Event> {
        (0..n)
            .map(|i| rbs::Event {
                pos: i / 2,
                ctrl: ctrls[(i % 2) as usize],
                value: (i / 2 % 2) as u8 + 1,
            })
            .collect()
    };
    let mut big = rbs::Song::empty();
    big.tracks[5].events = events(60_000, [3, 4]);
    big.tracks[6].events = events(41_000, [1, 2]);
    let dir = tempfile::tempdir().unwrap();
    let big_path = dir.path().join("big.rbs");
    std::fs::write(&big_path, big.write().unwrap()).unwrap();

    let src = root().join("tests/songs/diag/songs/drums_kit.rbs");
    let mut h = Harness::new();
    h.app.settings.startup_sound = true;
    h.open(&src);
    // let the startup sound of the first open end
    h.calls(2000);
    assert_eq!(h.audio.preview_position(), None);
    // a panel value the refused song's load would reset
    h.app.panel.transport.tempo = 77;
    let doc_path = h.app.doc.path.clone();
    h.app.open_song(&big_path);
    assert_eq!(
        h.app.notice.as_deref(),
        Some("There is not enough memory for opening the Song.")
    );
    assert_eq!(h.app.panel.transport.tempo, 77);
    h.calls(2);
    assert_eq!(h.audio.preview_position(), None, "no startup sound");
    assert_eq!(h.app.doc.path, doc_path);

    // the start with the refused song as the command-line song opens nothing either
    let mut s = Harness::new();
    s.app.settings.startup_sound = true;
    let opened = rb_ui::session::open_song(&big_path).unwrap();
    s.app.start(Some((big_path.clone(), opened)));
    assert!(s.app.notice.is_some());
    s.calls(2);
    assert_eq!(s.audio.preview_position(), None, "no startup sound");
    assert_eq!(s.app.doc.path, None);
}

/// task-090: opening a song while another plays stops the playback and leaves the new song
/// stopped at its start; an open that is cancelled (the unsaved prompt) or fails (unreadable
/// file, node pool refusal) leaves the song playing.
#[test]
fn opening_a_song_while_playing_stops_it_and_a_refused_open_does_not() {
    let a = root().join("tests/songs/diag/songs/drums_kit.rbs");
    let b = root().join("tests/songs/diag/songs/303_saw_lowcut.rbs");
    let play = |h: &mut Harness| {
        h.app.panel.press("transport.play", 0.0);
        h.frame(vec![]);
        h.calls(40);
        assert!(h.app.panel.transport.playing);
        assert!(h.app.panel.transport.position > 0);
    };

    // a successful open
    let mut h = Harness::new();
    h.open(&a);
    play(&mut h);
    h.app.open_song(&b);
    assert!(h.app.notice.is_none(), "{:?}", h.app.notice);
    h.calls(4);
    assert_eq!(h.app.doc.path.as_deref(), Some(b.as_path()));
    assert!(!h.app.panel.transport.playing, "stopped");
    assert_eq!(h.app.panel.transport.position, 0, "at its start");
    h.calls(40);
    assert!(!h.app.panel.transport.playing, "and it stays stopped");
    assert_eq!(h.app.panel.transport.position, 0);

    // a failed read: the song keeps playing
    let mut h = Harness::new();
    h.open(&a);
    play(&mut h);
    h.app
        .open_song(&root().join("tests/songs/diag/songs/no_such_song.rbs"));
    assert!(h.app.notice.is_some());
    h.calls(4);
    assert!(h.app.panel.transport.playing);
    assert_eq!(h.app.doc.path.as_deref(), Some(a.as_path()));

    // a cancelled unsaved prompt: the song keeps playing
    let mut h = Harness::new();
    h.open(&a);
    h.app.panel.set_knob("tb303_1.cutoff", 700);
    h.frame(vec![]);
    play(&mut h);
    h.app.open_song(&b);
    assert!(h.app.prompting());
    h.app.answer(Answer::Cancel);
    h.calls(4);
    assert!(h.app.panel.transport.playing);
    assert_eq!(h.app.doc.path.as_deref(), Some(a.as_path()));

    // a node pool refusal: the song keeps playing
    let events = |n: u32, ctrls: [u8; 2]| -> Vec<rbs::Event> {
        (0..n)
            .map(|i| rbs::Event {
                pos: i / 2,
                ctrl: ctrls[(i % 2) as usize],
                value: (i / 2 % 2) as u8 + 1,
            })
            .collect()
    };
    let mut big = rbs::Song::empty();
    big.tracks[5].events = events(60_000, [3, 4]);
    big.tracks[6].events = events(41_000, [1, 2]);
    let dir = tempfile::tempdir().unwrap();
    let big_path = dir.path().join("big.rbs");
    std::fs::write(&big_path, big.write().unwrap()).unwrap();
    let mut h = Harness::new();
    h.open(&a);
    play(&mut h);
    h.app.open_song(&big_path);
    assert!(h.app.notice.is_some());
    h.calls(4);
    assert!(h.app.panel.transport.playing);
    assert_eq!(h.app.doc.path.as_deref(), Some(a.as_path()));
}

/// task-089: the Options menu's "Play Mod Startup Sound" flips the preference and saves the
/// settings file at once; a save error goes to the notice line.
#[test]
fn the_startup_sound_checkbox_flips_the_preference_and_saves_it() {
    let dir = std::env::temp_dir().join(format!("rb-ui-startup-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let file = dir.join("ui.toml");
    let mut h = Harness::new();
    h.app.settings.file = Some(file.clone());
    assert!(h.app.settings.startup_sound);
    h.app.command(Cmd::ToggleStartupSound);
    assert!(!h.app.settings.startup_sound);
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("startup_sound = false\n"), "{text}");
    h.app.command(Cmd::ToggleStartupSound);
    assert!(h.app.settings.startup_sound);
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("startup_sound = true\n"), "{text}");
    assert!(h.app.notice.is_none());
    // a file that cannot be written: the error is on the notice line
    h.app.settings.file = Some(file.join("x"));
    h.app.command(Cmd::ToggleStartupSound);
    assert!(h.app.notice.is_some());
    let _ = std::fs::remove_dir_all(&dir);
}

/// task-091: File > New is the state of a start without a song: the same code builds both, and
/// a modified song asks the prompt first.
#[test]
fn file_new_gives_the_state_of_a_fresh_start() {
    let a = root().join("tests/songs/diag/songs/drums_kit.rbs");
    let state = |h: &Harness| {
        let d = &h.app.doc;
        // the panel's maps print in no fixed order: the pretty print's lines, sorted
        let mut lines: Vec<String> = format!("{:#?}", h.app.panel)
            .lines()
            .map(String::from)
            .collect();
        lines.sort();
        format!(
            "{lines:?}|{:?}|{}|{}|{:?}|{}",
            d.path,
            d.modified,
            d.mod_name,
            d.kit,
            d.source == rbs::Song::empty()
        )
    };

    // a fresh start without a song argument
    let mut start = Harness::new();
    let untouched = state(&start);
    start.app.start(None);
    start.calls(4);
    assert_eq!(
        state(&start),
        untouched,
        "the start builds the state the app is made in"
    );

    // a song that plays and was edited, then New through the prompt
    let mut h = Harness::new();
    h.app.settings.startup_sound = false;
    h.open(&a);
    h.app.panel.press("transport.play", 0.0);
    h.frame(vec![]);
    h.calls(40);
    h.app.panel.set_knob("tb303_1.cutoff", 700);
    h.frame(vec![]);
    assert!(h.app.doc.modified && h.app.panel.transport.playing);
    h.app.command(Cmd::New);
    assert!(h.app.prompting(), "unsaved changes ask first");
    h.app.answer(Answer::Cancel);
    assert_eq!(h.app.doc.path.as_deref(), Some(a.as_path()));
    h.app.command(Cmd::New);
    h.app.answer(Answer::DontSave);
    h.calls(4);
    assert_eq!(state(&h), untouched, "New equals the start");
    assert!(!h.app.panel.transport.playing);
    assert_eq!(h.app.panel.transport.position, 0);
    h.calls(40);
    assert!(!h.app.panel.transport.playing, "and it stays stopped");
    assert!(!h.app.settings.startup_sound, "the settings are kept");

    // unmodified: no prompt
    h.open(&a);
    h.app.command(Cmd::New);
    assert!(!h.app.prompting());
    h.calls(4);
    assert_eq!(state(&h), untouched);

    // a pending pattern edit at the moment of New does not leak into the new song
    h.open(&a);
    h.app.panel.set_step808(3, true);
    h.app.command(Cmd::New);
    h.calls(4);
    let fresh = Harness::new();
    let mirror = |h: &Harness| h.app.link.as_ref().unwrap().patterns().clone();
    assert_eq!(
        mirror(&h),
        mirror(&fresh),
        "the pattern mirror is the fresh start's"
    );
    assert_eq!(state(&h), untouched);

    // the shortcut
    h.open(&a);
    let key = Event::Key {
        key: egui::Key::N,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::COMMAND,
    };
    h.frame(vec![Event::ModifiersChanged(Modifiers::COMMAND), key]);
    h.calls(4);
    assert!(h.app.doc.path.is_none(), "Ctrl+N is New");
}

/// task-091: File > New plays the startup sound once when the preference is on, none when off.
/// task-099: it plays when the kit changes, so the song before it is on a mod.
#[test]
fn file_new_plays_the_startup_sound_when_the_preference_is_on() {
    let Some(pbe2) = mod_file("PBE2.rbm") else {
        return;
    };
    let a = root().join("tests/songs/diag/songs/drums_kit.rbs");
    let audio = samples()
        .get(rb_engine::samples::STARTUP_ID)
        .audio()
        .to_vec();
    let calls = audio.len().div_ceil(CALL_FRAMES) + 2;
    let run = |on: bool| {
        let mut h = Harness::new();
        h.app.settings.startup_sound = false;
        h.open(&a);
        h.app.use_kit(Kit::Mod(pbe2.clone()));
        h.calls(4);
        h.app.settings.startup_sound = on;
        h.app.command(Cmd::New);
        h.app.answer(Answer::DontSave);
        assert_eq!(h.app.doc.kit, Kit::Standard);
        let out = h.calls(calls);
        (out, h.audio.preview_position())
    };
    let (off, pos_off) = run(false);
    let (on, pos_on) = run(true);
    assert_eq!((pos_off, pos_on), (None, None), "the sound ended");
    assert_ne!(on, off);
    assert_eq!(on, with_preview(&off, &audio));
}

/// task-094: Mods > Demo Song is offered only for a mod that carries a `Demo.rbs`; it opens the
/// demo as a song that was never saved, on its mod, through the open path (the unsaved prompt,
/// the stop of the playing song), and a save never goes into the mod.
#[test]
fn the_demo_song_of_the_current_mod_opens_as_an_unsaved_song() {
    let Some(pbe2) = mod_file("PBE2.rbm") else {
        return;
    };
    let mut h = Harness::new();
    h.app.settings.startup_sound = false;

    // the standard kit has no demo, and "Demo Song" does nothing
    assert!(!h.app.has_demo());
    h.app.demo_song();
    assert!(h.app.notice.is_none());
    assert_eq!(h.app.doc.path, None);

    // a mod file without a Demo.rbs (the standard mod) is not offered either
    h.app.doc.kit = Kit::Mod(standard_mod());
    assert!(!h.app.has_demo());
    h.app.doc.kit = Kit::Standard;

    // a mod with one: enabled, and the bytes are a song that parses
    h.app.use_kit(Kit::Mod(pbe2.clone()));
    assert!(h.app.has_demo());
    let demo = session::mod_demo(&pbe2).unwrap();
    let want = rbs::Song::parse(&demo).unwrap();

    // the unsaved changes (the mod change) ask first; Cancel keeps the song
    assert!(h.app.doc.modified);
    h.app.demo_song();
    assert!(h.app.prompting());
    h.app.answer(Answer::Cancel);
    assert!(h.app.doc.modified);

    // Don't Save opens it; the playing song stops
    h.app.panel.press("transport.play", 0.0);
    h.frame(vec![]);
    h.calls(40);
    assert!(h.app.panel.transport.playing);
    h.app.demo_song();
    assert!(h.app.prompting());
    h.app.answer(Answer::DontSave);
    assert!(h.app.notice.is_none(), "{:?}", h.app.notice);
    h.calls(4);
    assert!(!h.app.panel.transport.playing, "stopped");
    assert_eq!(h.app.panel.transport.position, 0);
    assert_eq!(h.app.doc.path, None, "Untitled");
    assert!(!h.app.doc.modified);
    assert_eq!(h.app.doc.kit, Kit::Mod(pbe2.clone()));
    assert_eq!(h.app.doc.mod_name, session::mod_info(&pbe2).unwrap().name);
    assert!(h.app.has_demo());
    assert_eq!(h.app.panel.transport.tempo, want.glob.tempo as i32);

    // Save asks for a file, and the mod is untouched
    let before = std::fs::read(&pbe2).unwrap();
    h.app.save();
    assert!(h.app.browsing(), "Save goes to Save As");
    assert_eq!(
        std::fs::read(&pbe2).unwrap(),
        before,
        "the mod is untouched"
    );

    // unmodified: no prompt
    h.app.demo_song();
    assert!(!h.app.prompting());
}

/// task-095: the info window shows the open song, follows an open of another song, and closes
/// with Escape.
#[test]
fn the_info_window_follows_the_song_and_escape_closes_it() {
    let dir = root().join("tests/songs/diag/songs");
    let (a, b) = ("303_saw_lowcut.rbs", "303_saw_highcut.rbs");
    let mut h = Harness::with_menu();
    h.open(&dir.join(a));
    h.app.show_info();
    h.frame(vec![]);
    assert!(h.app.info_is_open());
    assert_eq!(h.app.info_shown().unwrap().name, a);
    h.open(&dir.join(b));
    h.frame(vec![]);
    assert!(h.app.info_is_open());
    assert_eq!(h.app.info_shown().unwrap().name, b);
    h.frame(vec![Event::Key {
        key: egui::Key::Escape,
        physical_key: Some(egui::Key::Escape),
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    assert!(!h.app.info_is_open());
}

/// task-100: File > Info opens the window centered in the app window, each open centers it
/// again, it can be dragged, and it stays inside a small window.
#[test]
fn the_info_window_opens_centered_and_stays_draggable() {
    let mut h = Harness::with_menu();
    let rect = |h: &Harness| {
        h.ctx
            .memory(|m| m.area_rect(h.app.info_id()))
            .expect("the window is drawn")
    };
    let screen = |h: &Harness| Rect::from_min_size(Pos2::ZERO, Vec2::new(PANEL_W, h.height));
    let esc = || Event::Key {
        key: egui::Key::Escape,
        physical_key: Some(egui::Key::Escape),
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    };

    h.app.show_info();
    for _ in 0..4 {
        h.frame(vec![]);
    }
    let r = rect(&h);
    let off = (r.center() - screen(&h).center()).abs();
    assert!(
        off.x < 3.0 && off.y < 3.0,
        "centered: {r:?} in {:?}",
        screen(&h)
    );

    // drag the title bar: the window moves and stays where it was dropped
    let grab = Pos2::new(r.center().x, r.min.y + 8.0);
    let to = grab + Vec2::new(-60.0, 40.0);
    let button = |pos, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    h.frame(vec![Event::PointerMoved(grab)]);
    h.frame(vec![button(grab, true)]);
    h.frame(vec![Event::PointerMoved(to)]);
    h.frame(vec![button(to, false)]);
    for _ in 0..3 {
        h.frame(vec![]);
    }
    let moved = rect(&h);
    assert!(
        (moved.min - (r.min + Vec2::new(-60.0, 40.0)))
            .abs()
            .max_elem()
            < 3.0,
        "dragged: {r:?} -> {moved:?}"
    );

    // File > Info on the open window leaves it where it was dropped
    h.app.show_info();
    for _ in 0..3 {
        h.frame(vec![]);
    }
    assert_eq!(rect(&h), moved, "an open window is not moved");

    // closed and opened again: centered again
    h.frame(vec![esc()]);
    assert!(!h.app.info_is_open());
    h.app.show_info();
    for _ in 0..4 {
        h.frame(vec![]);
    }
    let off = (rect(&h).center() - screen(&h).center()).abs();
    assert!(off.x < 3.0 && off.y < 3.0, "centered again: {:?}", rect(&h));

    // a small window: the window is inside it
    h.frame(vec![esc()]);
    h.height = 200.0;
    h.app.show_info();
    for _ in 0..4 {
        h.frame(vec![]);
    }
    let r = rect(&h);
    assert!(
        screen(&h).expand(0.5).contains_rect(r),
        "inside: {r:?} in {:?}",
        screen(&h)
    );
}

/// task-102: Help > About opens a window centered in the app window (each open again), shows
/// the logo, the version and the link, and Escape and the close button close it.
#[test]
fn the_about_window_opens_centered_and_closes() {
    let mut h = Harness::with_menu();
    let rect = |h: &Harness| h.ctx.memory(|m| m.area_rect(h.app.about_id()));
    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(PANEL_W, h.height));
    let esc = || Event::Key {
        key: egui::Key::Escape,
        physical_key: Some(egui::Key::Escape),
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    };
    assert!(!h.app.about_is_open());
    egui_extras::install_image_loaders(&h.ctx);
    // an image is a rect shape with a texture brush
    let images = |s: &str| s.matches("brush: Some(").count();
    let before = images(&h.frame_shapes(vec![]));

    h.app.command(Cmd::About);
    assert!(h.app.about_is_open());
    let mut shapes = String::new();
    for _ in 0..4 {
        shapes = h.frame_shapes(vec![]);
    }
    let r = rect(&h).expect("the window is drawn");
    let off = (r.center() - screen.center()).abs();
    assert!(off.x < 3.0 && off.y < 3.0, "centered: {r:?} in {screen:?}");
    assert!(shapes.contains("OpenRB-338"), "the name is drawn");
    assert!(
        shapes.contains(rb_ui::about::VERSION),
        "the version is drawn"
    );
    assert!(shapes.contains(rb_ui::about::URL), "the link is drawn");
    // the logo: one more image than without the window
    assert_eq!(images(&shapes), before + 1, "the logo image is drawn");
    // the version is `<time> (<git>)`
    let v = rb_ui::about::VERSION;
    assert!(
        v.len() > 21 && v.as_bytes()[19] == b' ' && v.ends_with(')'),
        "{v}"
    );

    // Escape closes it: its area is no longer drawn after the next frame
    h.frame(vec![esc()]);
    assert!(!h.app.about_is_open());

    // opened again: a new id, centered again; the close button closes it
    h.app.command(Cmd::About);
    for _ in 0..4 {
        h.frame(vec![]);
    }
    let r = rect(&h).expect("drawn again");
    let off = (r.center() - screen.center()).abs();
    assert!(off.x < 3.0 && off.y < 3.0, "centered again: {r:?}");
    // the close button is at the right end of the title bar
    let close = Pos2::new(r.max.x - 12.0, r.min.y + 10.0);
    h.click(close);
    h.frame(vec![]);
    assert!(!h.app.about_is_open(), "the close button closes it");
}

/// task-106: Mods > Info opens a centered window with the kit's splash at its physical width,
/// the INFO facts and the file facts; Escape and the close button close it; a song whose mod
/// is missing says so.
#[test]
fn the_mod_info_window_opens_centered_shows_the_kit_and_closes() {
    let mut h = Harness::with_menu();
    // a screen high enough for the whole window
    h.height = 900.0;
    let rect = |h: &Harness| h.ctx.memory(|m| m.area_rect(h.app.mod_info_id()));
    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(PANEL_W, h.height));
    let esc = || Event::Key {
        key: egui::Key::Escape,
        physical_key: Some(egui::Key::Escape),
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    };
    assert!(!h.app.mod_info_is_open());
    h.app.command(Cmd::ModInfo);
    assert!(h.app.mod_info_is_open());
    let mut shapes = String::new();
    for _ in 0..8 {
        shapes = h.frame_shapes(vec![]);
    }
    let r = rect(&h).expect("the window is drawn");
    let off = (r.center() - screen.center()).abs();
    assert!(off.x < 3.0 && off.y < 3.0, "centered: {r:?} in {screen:?}");
    for s in [
        "Standard ReBirth",
        "Embedded files",
        "Standard.rbm (built in)",
    ] {
        assert!(shapes.contains(s), "{s:?} is drawn in {r:?}");
    }
    assert!(!shapes.contains("is missing"));
    // the splash: 256 px, drawn at its physical width (this display's, or the fallback)
    let s = h.app.mod_info_splash().expect("the splash is drawn");
    let want = rb_ui::mod_info::physical_width(256.0, s.basis, h.ctx.pixels_per_point());
    assert_eq!(s.physical, want);
    if !s.scaled_down {
        assert_eq!(s.width, want);
    } else {
        assert!(s.width < want);
    }
    if let rb_ui::mod_info::Basis::Assumed(ppi) = s.basis {
        assert_eq!(ppi, rb_ui::mod_info::fallback_ppi());
    }

    // Escape closes it
    h.frame(vec![esc()]);
    assert!(!h.app.mod_info_is_open());

    // a song whose mod is missing: the name and the words; the built-in kit's facts
    h.app.doc.mod_name = "Gone Mod".into();
    h.app.command(Cmd::ModInfo);
    for _ in 0..4 {
        shapes = h.frame_shapes(vec![]);
    }
    assert!(shapes.contains("Gone Mod") && shapes.contains("is missing"));
    assert!(shapes.contains("Standard.rbm (built in)"));
    let r = rect(&h).expect("drawn again");
    let off = (r.center() - screen.center()).abs();
    assert!(off.x < 3.0 && off.y < 3.0, "centered again: {r:?}");
    // the close button is at the right end of the title bar
    h.click(Pos2::new(r.max.x - 12.0, r.min.y + 10.0));
    h.frame(vec![]);
    assert!(!h.app.mod_info_is_open(), "the close button closes it");
}

/// task-106: the window shows a real mod file's INFO, size and file count, and its splash.
#[test]
fn the_mod_info_window_shows_a_mod_files_facts() {
    let Some(path) = mod_file("PBE2.rbm") else {
        return;
    };
    let mut h = Harness::with_menu();
    // high enough for a long description
    h.height = 1400.0;
    h.app.use_kit(Kit::Mod(path.clone()));
    assert_eq!(h.app.doc.kit, Kit::Mod(path.clone()));
    h.app.command(Cmd::ModInfo);
    let mut shapes = String::new();
    for _ in 0..8 {
        shapes = h.frame_shapes(vec![]);
    }
    let d = rb_ui::session::ModDetails::of(&Kit::Mod(path.clone())).unwrap();
    assert_eq!(d.file_name, "PBE2.rbm");
    assert_eq!(d.size, std::fs::metadata(&path).unwrap().len());
    assert_eq!(d.name, h.app.doc.mod_name);
    assert!(d.files > 40, "{d:?}");
    for s in [
        d.name.as_str(),
        "PBE2.rbm",
        &rb_ui::mod_info::format_size(d.size),
        &d.files.to_string(),
    ] {
        assert!(shapes.contains(s), "{s:?} is drawn");
    }
    assert!(h.app.mod_info_splash().is_some(), "its splash is drawn");
}

/// task-106: the Mods menu lists "Info..." next to "Refresh List".
#[test]
fn the_mods_menu_has_an_info_item() {
    let mut h = Harness::with_menu();
    for x in (10..200).step_by(6) {
        let p = Pos2::new(x as f32, MENU_H / 2.0);
        h.frame(vec![Event::PointerMoved(p)]);
        let down = |pressed| Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        h.frame(vec![down(true)]);
        h.frame(vec![down(false)]);
        let text = h.frame_shapes(vec![]);
        if text.contains("Refresh List") {
            assert!(text.contains("Info..."), "the Mods menu has Info...");
            return;
        }
    }
    panic!("the Mods menu did not open");
}

/// task-105: Help > Keyboard Shortcuts opens a centered window (each open again) that lists the
/// keys by group with the current mode marked; Escape and the close button close it.
#[test]
fn the_key_help_window_opens_centered_marks_the_mode_and_closes() {
    let mut h = Harness::with_menu();
    let rect = |h: &Harness| h.ctx.memory(|m| m.area_rect(h.app.key_help_id()));
    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(PANEL_W, h.height));
    let esc = || Event::Key {
        key: egui::Key::Escape,
        physical_key: Some(egui::Key::Escape),
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    };
    assert!(!h.app.key_help_is_open());
    h.app.command(Cmd::KeyHelp);
    assert!(h.app.key_help_is_open());
    let mut shapes = String::new();
    for _ in 0..4 {
        shapes = h.frame_shapes(vec![]);
    }
    let r = rect(&h).expect("the window is drawn");
    let off = (r.center() - screen.center()).abs();
    assert!(off.x < 3.0 && off.y < 3.0, "centered: {r:?} in {screen:?}");
    assert!(shapes.contains("Keyboard Shortcuts"));
    assert!(shapes.contains("Menu shortcuts"));
    assert!(shapes.contains("Keyboard mode now: Program Synth"));

    // the other mode marks the other group
    h.app.key_mode = rb_ui::keys::KeyMode::Patterns;
    for _ in 0..2 {
        shapes = h.frame_shapes(vec![]);
    }
    assert!(shapes.contains("Keyboard mode now: Select Patterns"));

    // Escape closes it
    h.frame(vec![esc()]);
    assert!(!h.app.key_help_is_open());

    // opened again: a new id, centered again; the close button closes it
    h.app.command(Cmd::KeyHelp);
    for _ in 0..4 {
        h.frame(vec![]);
    }
    let r = rect(&h).expect("drawn again");
    let off = (r.center() - screen.center()).abs();
    assert!(off.x < 3.0 && off.y < 3.0, "centered again: {r:?}");
    let close = Pos2::new(r.max.x - 12.0, r.min.y + 10.0);
    h.click(close);
    h.frame(vec![]);
    assert!(!h.app.key_help_is_open(), "the close button closes it");
}

/// task-099: the startup sound plays only when the kit that plays after the change differs
/// from the one before: a song open on the same mod, Demo Song of the current mod and File >
/// New on the standard kit play nothing; a mod change and File > New from a mod song do.
#[test]
fn the_startup_sound_plays_only_when_the_kit_changes() {
    let Some(pbe2) = mod_file("PBE2.rbm") else {
        return;
    };
    let src = root().join("tests/songs/diag/songs/drums_kit.rbs");
    let mut h = Harness::new();
    let long = {
        let kit = rb_cli::render::load_mod(&pbe2, &samples()).unwrap_or_else(|e| panic!("{}", e.1));
        let a = kit.get(rb_engine::samples::STARTUP_ID).audio().len();
        let b = samples().get(rb_engine::samples::STARTUP_ID).audio().len();
        a.max(b).div_ceil(CALL_FRAMES) + 2
    };
    // runs `op`, lets one call render, and tells whether the sound started; then drains it
    let sounds = |h: &mut Harness, op: &dyn Fn(&mut Harness)| {
        op(h);
        assert!(h.app.notice.is_none(), "{:?}", h.app.notice);
        h.calls(1);
        let on = h.audio.preview_position().is_some();
        h.calls(long);
        assert_eq!(h.audio.preview_position(), None, "drained");
        on
    };

    // the first song plays (no previous kit); the same mod again does not
    assert!(sounds(&mut h, &|h| h.app.open_song(&src)));
    assert!(!sounds(&mut h, &|h| h.app.open_song(&src)));
    // File > New on the standard kit from a standard song: no sound
    assert!(!sounds(&mut h, &|h| h.app.new_now()));
    // a mod change plays; choosing the current mod again does not
    assert!(sounds(&mut h, &|h| h.app.use_kit(Kit::Mod(pbe2.clone()))));
    assert!(!sounds(&mut h, &|h| h.app.use_kit(Kit::Mod(pbe2.clone()))));
    // Demo Song opens the current mod's demo: the mod stays, no sound
    assert!(h.app.has_demo());
    assert!(!sounds(&mut h, &|h| h.app.open_demo_now(&pbe2)));
    assert_eq!(h.app.doc.kit, Kit::Mod(pbe2.clone()));
    // File > New from a mod song goes to the standard kit: sound; again: none
    assert!(sounds(&mut h, &|h| h.app.new_now()));
    assert_eq!(h.app.doc.kit, Kit::Standard);
    assert!(!sounds(&mut h, &|h| h.app.new_now()));
    // a failed mod change plays nothing and keeps the kit
    assert!(sounds(&mut h, &|h| h.app.use_kit(Kit::Mod(pbe2.clone()))));
    let bad = Kit::Mod(root().join("no/such/mod.rbm"));
    h.app.use_kit(bad);
    assert!(h.app.notice.is_some());
    h.calls(1);
    assert_eq!(h.audio.preview_position(), None);
    assert_eq!(h.app.doc.kit, Kit::Mod(pbe2.clone()));
    // this song names the standard kit: opened from a mod song it is a change, and plays
    // (the sound is judged on the resolved kit); the mod change left the song modified, so
    // the open asks first
    h.app.open_song(&src);
    assert!(h.app.prompting());
    h.app.answer(Answer::DontSave);
    h.calls(1);
    assert_eq!(h.app.doc.path.as_deref(), Some(src.as_path()));
    assert!(!h.app.doc.modified);
    assert_eq!(h.app.doc.kit, Kit::Standard);
    assert!(h.audio.preview_position().is_some());
    h.calls(long);
    // the preference still gates a changed kit
    h.app.settings.startup_sound = false;
    assert!(!sounds(&mut h, &|h| h.app.use_kit(Kit::Mod(pbe2.clone()))));
}

#[test]
fn the_window_title_is_the_app_name_the_song_and_the_modified_mark() {
    let mut h = Harness::new();
    h.frame(vec![]);
    assert_eq!(h.app.title, "OpenRB-338 - Untitled");
    h.app.panel.set_knob("tb303_1.cutoff", 900);
    h.frame(vec![]);
    assert!(h.app.doc.modified);
    assert_eq!(h.app.title, "OpenRB-338 - Untitled *");
}

/// A ReBirth 1.x song (a MIDI file, docs/song_load.md section 12) whose import gives the mixer
/// track out of tick order: MIDI track 2 (the 303 #1) sets the mixer's level and pan at time 8
/// (tick 192), then MIDI track 5 sets the PCF's channel (mixer controller 2) at time 2 (tick 48)
/// and the distortion's at time 8 (four mixer events at tick 192). The 909 buffer starts with
/// the import's `(0, 0, 0)`.
fn song_1x_unsorted() -> Vec<u8> {
    use rbs::rb1x::{BLOCK_LEN, Block, Event as MidiEvent, Kind, Song1x};
    let mut b = Box::new([0u8; BLOCK_LEN]);
    b[..5].copy_from_slice(&[0xBC, 3, 1, 0, 0]);
    b[5..9].copy_from_slice(b"TinY");
    b[9] = 90;
    b[14..18].copy_from_slice(&4u32.to_be_bytes());
    b[0x12] = 1;
    b[0x2A..0x2E].copy_from_slice(&128_000u32.to_be_bytes());
    for j in 0..3 {
        let r = 0x100 + 32 * j;
        b[r + 2..r + 5].copy_from_slice(&[100, 64, 20]);
        b[r + 7] = 0x3F;
        b[r + 0x17] = 1;
    }
    // a 303 #1 pattern with notes, so that the song plays
    for s in 0..16 {
        b[352 + 30 + 4 * s] = (s % 12) as u8 + 1;
    }
    let cc = |time, ctrl, value| MidiEvent {
        time,
        kind: Kind::Cc { ctrl, value },
    };
    Song1x {
        format: 1,
        division: 32,
        tracks: vec![
            vec![MidiEvent {
                time: 0,
                kind: Kind::Meta {
                    kind: 2,
                    data: b"File version 3.1.0".to_vec(),
                },
            }],
            vec![MidiEvent {
                time: 0,
                kind: Kind::Sysex(Block(b).encode()),
            }],
            vec![cc(8, 7, 80), cc(8, 10, 30), cc(16, 7, 120)],
            vec![],
            vec![],
            vec![cc(2, 31, 1), cc(8, 17, 0), cc(12, 31, 3)],
        ],
        version: (3, 1),
    }
    .write()
}

/// The engine's tracks of the open song, in list order.
fn track_lists(h: &mut Harness) -> Vec<Vec<(i32, u8, u8)>> {
    let seq = &h.audio.current_mut().frame.seq;
    (0..seq.tracks.len())
        .map(|t| {
            seq.events(t)
                .iter()
                .map(|e| (e.tick, e.ctrl, e.value))
                .collect()
        })
        .collect()
}

/// A list with the order of each tick's events reversed, the tick-0 events excepted: what a
/// save and a reopen make of a list whose tick-0 events all replace the song constructor's
/// (the load inserts each event before those of its tick, `sub_10014750`).
fn reopened_order(list: &[(i32, u8, u8)]) -> Vec<(i32, u8, u8)> {
    let mut out = Vec::new();
    for group in list.chunk_by(|a, b| a.0 == b.0) {
        if group[0].0 == 0 {
            out.extend_from_slice(group);
        } else {
            out.extend(group.iter().rev());
        }
    }
    out
}

/// task-125: Save As of a 1.x song whose import interleaves the events of several MIDI tracks
/// on one device writes the tracks the engine holds (sorted by tick, as the original's save
/// reads them back from the DLL), and the reopened file plays the same.
#[test]
fn a_1x_song_with_unsorted_imports_saves_and_reopens_the_same() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("old.rbs");
    std::fs::write(&src, song_1x_unsorted()).unwrap();
    // the import's mixer list is out of tick order
    let opened = session::open_song(&src).unwrap();
    let mixer: Vec<u32> = opened.engine.tracks[0].iter().map(|e| e.tick).collect();
    assert!(!mixer.is_sorted(), "{mixer:?}");

    let play = |h: &mut Harness| {
        h.app.panel.press("transport.play", 0.0);
        h.frame(vec![]);
        h.calls(60)
    };
    let mut h = Harness::new();
    h.app.settings.startup_sound = false;
    h.open(&src);
    let lists = track_lists(&mut h);
    let saved = dir.path().join("saved.rbs");
    h.app.save_to(&saved);
    assert!(h.app.notice.is_none(), "{:?}", h.app.notice);
    // the file holds the engine's lists as they are
    let s = read_rbs(&saved);
    for (t, list) in lists.iter().enumerate() {
        let got: Vec<_> = s.tracks[t]
            .events
            .iter()
            .map(|e| (e.pos as i32 * 24, e.ctrl, e.value))
            .collect();
        assert_eq!(&got, list, "track {t}");
    }
    // the mixer has the import's events, sorted, and the constructor's tick-0 events
    assert!(lists[0].contains(&(48, 2, 2)) && lists[0].contains(&(192, 6, 80)));
    assert!(lists[4].contains(&(0, 0, 0)));

    let mut h2 = Harness::new();
    h2.app.settings.startup_sound = false;
    h2.open(&saved);
    let reopened = track_lists(&mut h2);
    for (t, list) in lists.iter().enumerate() {
        assert_eq!(reopened[t], reopened_order(list), "track {t}");
    }
    let out = play(&mut h);
    assert!(out.iter().any(|&x| x != 0), "the song is silent");
    assert_eq!(play(&mut h2), out, "the reopened song plays differently");
    // and saving the reopened song again writes its lists
    let again = dir.path().join("again.rbs");
    h2.app.save_to(&again);
    assert!(h2.app.notice.is_none(), "{:?}", h2.app.notice);
}

/// task-125, gated on `RB_THIRDPARTY_SONGS_DIR` (the user's songs, read in place): every ReBirth
/// 1.x song of the folder saves (the composed song writes), and Razor Topz.RBS, the song of the
/// report, also after a change to Pitch Black Edition 2.0 (`PBE2.rbm`, recrypted from the CD
/// image by `rb-testkit`).
#[test]
fn every_1x_song_of_the_collection_saves() {
    let Some(dir) = rb_testkit::thirdparty_songs() else {
        return;
    };
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        // the rule of `crates/rb-cli/tests/thirdparty.rs` (a name like "x.rbs copy" counts)
        .filter(|p| {
            p.is_file()
                && p.file_name()
                    .is_some_and(|f| f.to_string_lossy().to_lowercase().contains(".rbs"))
        })
        .collect();
    paths.sort();
    let tmp = tempfile::tempdir().unwrap();
    let saved = tmp.path().join("saved.rbs");
    let mut h = Harness::new();
    let mut h2 = Harness::new();
    h.app.settings.startup_sound = false;
    h2.app.settings.startup_sound = false;
    let mut n = 0;
    for p in &paths {
        let bytes = std::fs::read(p).unwrap();
        if !rbs::rb1x::is_1x(&bytes) {
            continue;
        }
        h.open(p);
        h.app.save_to(&saved);
        assert!(
            h.app.notice.is_none(),
            "{}: {:?}",
            p.display(),
            h.app.notice
        );
        // the saved song reopens with the import's tracks, a tick's events reversed
        let lists = track_lists(&mut h);
        h2.open(&saved);
        let reopened = track_lists(&mut h2);
        for (t, list) in lists.iter().enumerate() {
            assert_eq!(
                reopened[t],
                reopened_order(list),
                "{}: track {t}",
                p.display()
            );
        }
        n += 1;
    }
    eprintln!("{n} 1.x songs save and reopen with their tracks");
    assert!(n > 0, "no 1.x song in {}", dir.display());

    let razor = dir.join("Razor Topz.RBS");
    if !razor.exists() {
        return;
    }
    // the song of the report: saved without the mod change, it plays as the import does
    h.open(&razor);
    h.app.save_to(&saved);
    h2.open(&saved);
    let play = |h: &mut Harness| {
        h.app.panel.press("transport.play", 0.0);
        h.frame(vec![]);
        h.calls(200)
    };
    let out = play(&mut h);
    assert!(out.iter().any(|&x| x != 0), "Razor Topz is silent");
    assert_eq!(play(&mut h2), out, "the saved Razor Topz plays differently");
    // and with the mod change of the report
    let Some(pbe2) = mod_file("PBE2.rbm") else {
        return;
    };
    h.open(&razor);
    h.app.command(Cmd::UseKit(Kit::Mod(pbe2)));
    assert!(h.app.notice.is_none(), "{:?}", h.app.notice);
    h.app.save_to(&saved);
    assert!(h.app.notice.is_none(), "{:?}", h.app.notice);
    assert_eq!(read_rbs(&saved).glob.mod_name, h.app.doc.mod_name);
}

/// A ReBirth 1.x song with a title, text, web page, 128 BPM, song mode and a loop (bars 2..6),
/// as `crates/rb-cli/tests/info.rs` builds one.
fn song_1x_with_info() -> Vec<u8> {
    use rbs::rb1x::{BLOCK_LEN, Block, Event as MidiEvent, Kind, Song1x};
    let mut b = Box::new([0u8; BLOCK_LEN]);
    b[..5].copy_from_slice(&[0xBC, 3, 1, 0, 0]);
    b[5..9].copy_from_slice(b"TinY");
    b[9] = 90;
    b[10..14].copy_from_slice(&2u32.to_be_bytes());
    b[14..18].copy_from_slice(&4u32.to_be_bytes());
    b[0x12] = 1;
    b[0x2A..0x2E].copy_from_slice(&128_000u32.to_be_bytes());
    b[0x2E] = 1;
    for j in 0..3 {
        let r = 0x100 + 32 * j;
        b[r + 2..r + 5].copy_from_slice(&[100, 64, 20]);
        b[r + 7] = 0x3F;
        b[r + 0x17] = 1;
    }
    let put = |b: &mut [u8], off: usize, s: &[u8]| b[off..off + s.len()].copy_from_slice(s);
    put(&mut b[..], 0x3160, b"Old song");
    put(&mut b[..], 0x3160 + 41, b"first\rsecond \xe9");
    put(&mut b[..], 0x3160 + 242, b"www.example.org");
    Song1x {
        format: 1,
        division: 32,
        tracks: vec![
            vec![MidiEvent {
                time: 0,
                kind: Kind::Meta {
                    kind: 2,
                    data: b"File version 3.1.0".to_vec(),
                },
            }],
            vec![MidiEvent {
                time: 0,
                kind: Kind::Sysex(Block(b).encode()),
            }],
        ],
        version: (3, 1),
    }
    .write()
}

/// task-123: File > Info of a ReBirth 1.x song shows the facts `rb info` prints for it (the
/// song's, not the empty stand-in's), and after a save, which makes an RBS 4.2 file, what that
/// file says.
#[test]
fn file_info_of_a_1x_song_shows_its_own_facts() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("old.rbs");
    std::fs::write(&src, song_1x_with_info()).unwrap();
    let expect = rb_cli::info::SongFacts::of_file(&src).unwrap();
    let mut h = Harness::new();
    h.app.settings.startup_sound = false;
    h.open(&src);
    h.app.show_info();
    h.frame(vec![]);
    let i = h.app.info_shown().unwrap().clone();
    assert_eq!(i.format, "ReBirth 1.x MIDI file (file version 3.1)");
    assert_eq!(i.version, "3.1");
    assert_eq!(i.window_title, "Old song");
    assert_eq!(i.text, "first\nsecond \u{e9}");
    assert_eq!(i.web_page, "www.example.org");
    assert_eq!(i.copyright, "");
    assert_eq!(i.bpm, 128.0);
    assert!(i.song_mode && i.looping && i.vintage);
    assert_eq!(i.shuffle, 64);
    // all of it is rb info's
    assert_eq!(i.window_title, expect.window_title);
    assert_eq!(i.text, expect.text);
    assert_eq!(i.web_page, expect.web_page);
    assert_eq!(i.copyright, expect.copyright);
    assert_eq!(i.version, expect.version);
    assert_eq!(
        (i.bpm, i.shuffle, i.vintage),
        (expect.bpm, expect.shuffle, expect.vintage)
    );
    assert_eq!((i.song_mode, i.looping), (expect.song_mode, expect.looping));

    // saved, the file is an RBS 4.2 song and the window says so
    let saved = dir.path().join("saved.rbs");
    h.app.save_to(&saved);
    assert!(h.app.notice.is_none(), "{:?}", h.app.notice);
    h.frame(vec![]);
    let after = h.app.info_shown().unwrap().clone();
    assert_eq!(after.format, "RBS 4.2");
    assert!(!after.rb1x);
    // the facts the window shows are the same before and after
    let facts = |i: &rb_ui::info::SongInfo| {
        (
            i.window_title.clone(),
            i.text.clone(),
            i.web_page.clone(),
            i.copyright.clone(),
            i.bpm,
            i.song_mode,
            i.looping,
            i.shuffle,
            i.vintage,
        )
    };
    assert_eq!(facts(&after), facts(&i));

    // the file keeps the song information, as the original's import copies it to the USRI
    // fields (`sub_430BA0` 0x431039..0x431078): a 0x0d is followed by a 0x0a (`sub_465D8D`)
    let s = read_rbs(&saved);
    assert_eq!(s.usri.window_title, "Old song");
    assert_eq!(s.usri.text, "first\r\nsecond \u{e9}");
    assert_eq!(s.usri.web_page, "www.example.org");
    assert_eq!(s.head.copyright, "");
    let mut h2 = Harness::new();
    h2.app.settings.startup_sound = false;
    h2.open(&saved);
    h2.app.show_info();
    h2.frame(vec![]);
    let reopened = h2.app.info_shown().unwrap();
    assert_eq!(reopened.format, "RBS 4.2");
    assert_eq!(facts(reopened), facts(&i));
}

/// The song information of a 1.x song is copied as the original does (`sub_465D8D`): at most 40,
/// 200 and 100 bytes, a 0x0a after each 0x0d that counts and is left out at the limit, and the
/// show-on-open byte is 0 or 1.
#[test]
fn song_information_of_a_1x_song_is_copied_like_the_original() {
    use rb_cli::info::Usri1x;
    use rbs::rb1x::{BLOCK_LEN, Block, Event as MidiEvent, Kind, Song1x};
    let mut b = Box::new([0u8; BLOCK_LEN]);
    b[..5].copy_from_slice(&[0xBC, 3, 1, 0, 0]);
    b[5..9].copy_from_slice(b"TinY");
    // 39 letters and a CR: the LF would be the 41st byte, so it is dropped
    b[0x3160..0x3160 + 39].fill(b'a');
    b[0x3160 + 39] = 0x0d;
    // 50 bytes of CR: the pairs fill 200
    b[0x3160 + 41..0x3160 + 41 + 150].fill(0x0d);
    // a full web page with no terminator in 100 bytes
    b[0x3160 + 242..0x3160 + 242 + 101].fill(b'w');
    b[0x3160 + 343] = 7;
    let song = Song1x {
        format: 1,
        division: 32,
        tracks: vec![vec![MidiEvent {
            time: 0,
            kind: Kind::Sysex(Block(b).encode()),
        }]],
        version: (3, 1),
    };
    let u = Usri1x::of(&song);
    assert_eq!(u.title, format!("{}\r", "a".repeat(39)));
    assert_eq!(u.text, "\r\n".repeat(100));
    assert_eq!(u.web_page, "w".repeat(100));
    assert_eq!(u.show_on_open, 1);
}

/// task-123: a mod change of a 1.x song updates the mod's web and ftp addresses File > Info
/// shows, as it does the saved file's.
#[test]
fn a_mod_change_of_a_1x_song_updates_the_infos_mod_addresses() {
    let Some(pbe2) = mod_file("PBE2.rbm") else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("old.rbs");
    std::fs::write(&src, song_1x_with_info()).unwrap();
    let mut h = Harness::new();
    h.app.settings.startup_sound = false;
    h.open(&src);
    h.app.use_kit(Kit::Mod(pbe2.clone()));
    assert!(h.app.notice.is_none(), "{:?}", h.app.notice);
    let m = session::mod_info(&pbe2).unwrap();
    assert!(
        !m.web.is_empty() || !m.ftp.is_empty(),
        "the mod has no addresses to test"
    );
    h.app.show_info();
    h.frame(vec![]);
    let i = h.app.info_shown().unwrap();
    assert_eq!(i.mod_name, m.name);
    assert_eq!(i.mod_www, m.web);
    assert_eq!(i.mod_ftp, m.ftp);
    assert_eq!(i.window_title, "Old song");
}
