//! The `.rbsx` song (`rbsx.rs`), headless with a real player: an `.rbsx` keeps the GENERATE
//! menu's settings with the song and gives them back when opened, an `.rbs` saved by the app
//! stays a plain RBS song, and Save asks before an `.rbs` with GENERATE settings loses them.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use eframe::egui::{self, Event, Modifiers, Pos2, RawInput, Rect, Vec2};
use orb_engine::samples::SampleSet;
use orb_engine::tables::Tables;
use orb_player::{Audio, CALL_FRAMES};
use orb_ui::app::App;
use orb_ui::browser::Mode;
use orb_ui::generate::{Field, Op};
use orb_ui::layout::{PANEL_H, PANEL_W};
use orb_ui::link::Link;
use orb_ui::menu::Cmd;
use orb_ui::model::Panel;
use orb_ui::rbsx::{self, FormatAnswer, GenState};
use orb_ui::scale::SCALES;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn tables() -> Arc<Tables> {
    static T: OnceLock<Arc<Tables>> = OnceLock::new();
    T.get_or_init(|| Arc::new(Tables::new())).clone()
}

fn samples() -> Arc<SampleSet> {
    rbm::standard::builtin_samples()
}

struct Harness {
    ctx: egui::Context,
    app: App,
    audio: Audio,
    time: f64,
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
        };
        h.calls(1);
        h
    }

    fn frame(&mut self) {
        self.time += 0.016;
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(PANEL_W, PANEL_H))),
            time: Some(self.time),
            events: vec![Event::ModifiersChanged(Modifiers::NONE)],
            ..Default::default()
        };
        let app = &mut self.app;
        let mut out = self.ctx.run_ui(input, |ui| app.show(ui));
        out.textures_delta.clear();
    }

    fn calls(&mut self, n: usize) {
        let mut out = vec![0i16; 2 * CALL_FRAMES * n];
        for c in out.chunks_mut(2 * CALL_FRAMES) {
            self.audio.render_i16(c);
        }
        self.frame();
    }

    fn open(&mut self, path: &Path) {
        self.app.open_song(path);
        assert!(self.app.errors.is_empty(), "{:?}", self.app.errors);
        self.calls(2);
    }

    /// BAZE 1 on C IONIAN, a RANDOM run of NOTE and ACCENT, MOVE's +1 chosen: settings to keep.
    fn set_up_generate(&mut self) {
        let ionian = SCALES.iter().position(|(n, _)| *n == "IONIAN").unwrap();
        self.app.press_field(0, Field::Scale);
        self.app.choose_item(ionian);
        self.app.press_field(0, Field::Key);
        self.app.choose_item(0);
        self.app.set_random_seed(3);
        self.app.press_op(0, Op::Random);
        self.app.press_op_param(0, Op::Random, orb_ui::random::NOTE);
        self.app.press_op_param(0, Op::Random, orb_ui::random::ACCENT);
        self.app.press_op(0, Op::Random);
        self.app.press_op(0, Op::Move);
        self.app.press_op_param(0, Op::Move, 3);
        self.calls(2);
    }
}

fn song() -> PathBuf {
    root().join("tests/songs/diag/songs/303_saw_lowcut.rbs")
}

#[test]
fn an_rbsx_keeps_the_generate_settings_and_its_song() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = Harness::new();
    h.open(&song());
    assert!(h.app.gen_state().is_empty(), "an .rbs opens with an empty menu");
    h.set_up_generate();
    let kept = h.app.gen_state();
    assert!(!kept.is_empty());
    let pattern = h.app.panel.pattern_303(0);

    // Save As with a name and no extension: an .rbsx
    h.app.command(Cmd::SaveAs);
    assert!(h.app.browsing());
    let out = dir.path().join("mine.rbsx");
    h.app.chosen(Mode::SaveSong, out.clone());
    assert!(h.app.errors.is_empty(), "{:?}", h.app.errors);
    assert!(!h.app.doc.modified);

    // the file is an RBS song with the OGEN chunk at its end
    let bytes = std::fs::read(&out).unwrap();
    let mut parsed = rbs::Song::parse(&bytes).unwrap();
    assert_eq!(rbsx::split(&mut parsed), Some(kept));

    // a new app opens it: the song and the menu come back
    let mut h2 = Harness::new();
    h2.open(&out);
    assert_eq!(h2.app.gen_state(), kept);
    assert_eq!(h2.app.panel.pattern_303(0), pattern);
    assert!(h2.app.row_done(0, 0) && h2.app.row_done(0, 1));
    // File > New empties the menu again
    h2.app.new_now();
    h2.calls(1);
    assert!(h2.app.gen_state().is_empty());
}

#[test]
fn an_rbs_saved_by_the_app_stays_a_plain_rbs_and_save_asks_first() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("old.rbs");
    std::fs::copy(song(), &src).unwrap();
    let mut h = Harness::new();
    h.open(&src);
    // no GENERATE settings: Save writes the .rbs without asking
    h.app.panel.set_knob("tb303_1.cutoff", 700);
    h.frame();
    h.app.command(Cmd::Save);
    assert!(!h.app.format_prompting());
    let plain = std::fs::read(&src).unwrap();
    let mut parsed = rbs::Song::parse(&plain).unwrap();
    assert_eq!(rbsx::split(&mut parsed), None);

    // with GENERATE settings, Save asks; Cancel saves nothing
    h.set_up_generate();
    assert!(h.app.doc.modified, "the menu's changes count as changes");
    h.app.command(Cmd::Save);
    assert!(h.app.format_prompting());
    h.app.format_answer(FormatAnswer::Cancel);
    assert!(!h.app.format_prompting());
    assert_eq!(std::fs::read(&src).unwrap(), plain);
    assert!(h.app.doc.modified);

    // ".rbs": the file is saved without the settings, a plain RBS song
    h.app.command(Cmd::Save);
    h.app.format_answer(FormatAnswer::Rbs);
    assert!(!h.app.doc.modified);
    let mut parsed = rbs::Song::parse(&std::fs::read(&src).unwrap()).unwrap();
    assert_eq!(rbsx::split(&mut parsed), None);
    assert!(parsed.unknown.is_empty());

    // ".rbsx": Save As opens on the same folder with the name as .rbsx
    h.app.panel.set_knob("tb303_1.cutoff", 800);
    h.frame();
    h.app.command(Cmd::Save);
    h.app.format_answer(FormatAnswer::Rbsx);
    assert!(h.app.browsing());
    let rbsx_path = dir.path().join("old.rbsx");
    h.app.chosen(Mode::SaveSong, rbsx_path.clone());
    assert_eq!(h.app.doc.path.as_deref(), Some(rbsx_path.as_path()));
    let mut parsed = rbs::Song::parse(&std::fs::read(&rbsx_path).unwrap()).unwrap();
    assert_eq!(rbsx::split(&mut parsed), Some(h.app.gen_state()));
    // the song is now an .rbsx: Save writes it without asking
    h.app.panel.set_knob("tb303_1.cutoff", 600);
    h.frame();
    h.app.command(Cmd::Save);
    assert!(!h.app.format_prompting());
    assert!(!h.app.doc.modified);
}

#[test]
fn every_shipped_rbs_song_still_opens_with_an_empty_menu() {
    let mut h = Harness::new();
    let dir = root().join("tests/songs/shipped");
    let mut n = 0;
    for e in std::fs::read_dir(&dir).unwrap().flatten() {
        let p = e.path();
        if p.extension().and_then(|x| x.to_str()) == Some("rbs") {
            // (a song on a mod we do not have plays the built-in kit and says so: not our test)
            h.app.open_song(&p);
            h.app.errors.retain(|e| !e.contains("was not found in the Mods folder"));
            assert!(h.app.errors.is_empty(), "{}: {:?}", p.display(), h.app.errors);
            h.calls(2);
            assert_eq!(h.app.doc.path.as_deref(), Some(p.as_path()));
            assert_eq!(h.app.gen_state(), GenState::default(), "{}", p.display());
            n += 1;
        }
    }
    assert!(n > 0);
}
