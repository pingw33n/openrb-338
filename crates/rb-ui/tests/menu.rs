//! The menu (task-047), headless: saving a song gives back the file that was read, the edits
//! of the panel are saved, the export writes the file `rb render` writes, the mod and the
//! Vintage flag change and are saved, and the unsaved-changes prompt guards opening and closing.
//!
//! The export and mod tests need the standard mod (`RB_STANDARD_MOD`) and, for mods, a folder
//! of recrypted mods (`RB_MOD_DIR`); they skip when those are not set.

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

fn samples() -> Arc<SampleSet> {
    static S: OnceLock<Arc<SampleSet>> = OnceLock::new();
    S.get_or_init(|| {
        let dll = std::fs::read(root().join("original/ReBirth.dll")).unwrap();
        Arc::new(SampleSet::from_dll(&dll).unwrap())
    })
    .clone()
}

/// A path from the environment; a relative one is relative to the workspace root (cargo runs
/// the tests in the crate's directory).
fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|v| !v.is_empty())
        .map(|v| root().join(v))
}

/// The standard mod, or a skip.
fn standard_mod() -> Option<PathBuf> {
    let p = env_path("RB_STANDARD_MOD");
    if p.is_none() {
        eprintln!("skipped: RB_STANDARD_MOD is not set");
    }
    p
}

/// A recrypted mod of `RB_MOD_DIR`, or a skip.
fn mod_file(name: &str) -> Option<PathBuf> {
    let p = env_path("RB_MOD_DIR").map(|d| d.join(name));
    if p.is_none() {
        eprintln!("skipped: RB_MOD_DIR is not set");
    }
    p
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
        app.standard_path = env_path("RB_STANDARD_MOD");
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

    fn frame(&mut self, mut events: Vec<Event>) {
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
    let Some(std_mod) = standard_mod() else {
        return;
    };
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
    let mut r = Renderer::new(Some(&std_mod), ModOpts::default());
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
    let Some(std_mod) = standard_mod() else {
        return;
    };
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
            Renderer::new(Some(&std_mod), ModOpts::default())
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
    let (Some(_), Some(pbe2)) = (standard_mod(), mod_file("PBE2.rbm")) else {
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
    Renderer::new(standard_mod().as_deref(), mods)
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
    let (Some(_), Some(pbe2)) = (standard_mod(), mod_file("PBE2.rbm")) else {
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
    let (Some(_), Some(pbe2)) = (standard_mod(), mod_file("PBE2.rbm")) else {
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
    let file = dir.join("settings.txt");
    let mut h = Harness::new();
    h.app.settings.file = Some(file.clone());
    assert!(h.app.settings.startup_sound);
    h.app.command(Cmd::ToggleStartupSound);
    assert!(!h.app.settings.startup_sound);
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("startup_sound=0\n"), "{text}");
    h.app.command(Cmd::ToggleStartupSound);
    assert!(h.app.settings.startup_sound);
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("startup_sound=1\n"), "{text}");
    assert!(h.app.notice.is_none());
    // a file that cannot be written: the error is on the notice line
    h.app.settings.file = Some(file.join("x"));
    h.app.command(Cmd::ToggleStartupSound);
    assert!(h.app.notice.is_some());
    let _ = std::fs::remove_dir_all(&dir);
}
