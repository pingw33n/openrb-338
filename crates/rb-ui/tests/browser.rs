//! The in-app file browser (task-046), headless: key and text events are run through
//! `Browser::show` frame by frame over a temporary directory tree.

use std::fs;
use std::path::{Path, PathBuf};

use eframe::egui::{self, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2};
use rb_ui::app::App;
use rb_ui::browser::{Browser, ExportFormat, ModStatus, Mode, Poll};
use rb_ui::layout::{PANEL_H, PANEL_W};
use rb_ui::model::Panel;
use rb_ui::settings::Settings;

struct Harness {
    ctx: egui::Context,
    time: f64,
}

impl Harness {
    fn new() -> Self {
        Self {
            ctx: egui::Context::default(),
            time: 0.0,
        }
    }

    fn input(&mut self, events: Vec<Event>, size: Vec2) -> RawInput {
        self.time += 0.016;
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
            time: Some(self.time),
            events,
            ..Default::default()
        }
    }

    fn frame(&mut self, b: &mut Browser, s: &mut Settings, events: Vec<Event>) -> Poll {
        let input = self.input(events, Vec2::new(624.0, 510.0));
        let mut poll = Poll::Open;
        let ctx = self.ctx.clone();
        let mut out = self.ctx.run_ui(input, |_| poll = b.show(&ctx, s));
        out.textures_delta.clear();
        poll
    }

    fn key(&mut self, b: &mut Browser, s: &mut Settings, k: Key) -> Poll {
        self.key_mods(b, s, k, Modifiers::NONE)
    }

    fn key_mods(&mut self, b: &mut Browser, s: &mut Settings, k: Key, m: Modifiers) -> Poll {
        let down = Event::Key {
            key: k,
            physical_key: Some(k),
            pressed: true,
            repeat: false,
            modifiers: m,
        };
        let up = Event::Key {
            key: k,
            physical_key: Some(k),
            pressed: false,
            repeat: false,
            modifiers: m,
        };
        let p = self.frame(b, s, vec![Event::ModifiersChanged(m), down, up]);
        self.frame(b, s, vec![Event::ModifiersChanged(Modifiers::NONE)]);
        p
    }

    fn text(&mut self, b: &mut Browser, s: &mut Settings, t: &str) {
        self.frame(b, s, vec![Event::Text(t.to_string())]);
    }
}

fn tree() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    let p = d.path();
    fs::create_dir(p.join("sub")).unwrap();
    fs::create_dir(p.join("Zeta")).unwrap();
    fs::write(p.join("a.rbs"), b"x").unwrap();
    fs::write(p.join("c.RBS"), b"x").unwrap();
    fs::write(p.join("b.txt"), b"x").unwrap();
    fs::write(p.join(".hidden.rbs"), b"x").unwrap();
    fs::write(p.join("sub/inner.rbs"), b"x").unwrap();
    d
}

fn browser(mode: Mode, dir: &Path) -> (Browser, Settings) {
    let s = Settings::default();
    (Browser::new(mode, &s).with_dir(dir), s)
}

#[test]
fn lists_folders_first_filtered_by_extension() {
    let d = tree();
    let (mut b, _) = browser(Mode::OpenSong, d.path());
    assert_eq!(b.names(), ["sub", "Zeta", "a.rbs", "c.RBS"]);
    b.set_show_all(true);
    assert_eq!(b.names(), ["sub", "Zeta", "a.rbs", "b.txt", "c.RBS"]);
}

#[test]
fn keyboard_navigates_and_chooses() {
    let d = tree();
    let (mut b, mut s) = browser(Mode::OpenSong, d.path());
    let mut h = Harness::new();
    h.frame(&mut b, &mut s, vec![]);
    // Down selects the first row, a folder; Enter goes in.
    h.key(&mut b, &mut s, Key::ArrowDown);
    assert_eq!(b.selected_name(), Some("sub"));
    assert_eq!(h.key(&mut b, &mut s, Key::Enter), Poll::Open);
    assert_eq!(b.dir(), d.path().join("sub"));
    assert_eq!(b.names(), ["inner.rbs"]);
    // Backspace goes back up and selects the folder that was left.
    h.key(&mut b, &mut s, Key::Backspace);
    assert_eq!(b.dir(), d.path());
    assert_eq!(b.selected_name(), Some("sub"));
    // Type-ahead jumps to "a.rbs", Enter chooses it.
    h.text(&mut b, &mut s, "a");
    assert_eq!(b.selected_name(), Some("a.rbs"));
    let p = h.key(&mut b, &mut s, Key::Enter);
    assert_eq!(p, Poll::Chosen(d.path().join("a.rbs")));
    // The folder is now the first recent folder.
    assert_eq!(s.recent, [d.path().to_path_buf()]);
}

#[test]
fn up_down_clamp_and_escape_cancels() {
    let d = tree();
    let (mut b, mut s) = browser(Mode::OpenSong, d.path());
    let mut h = Harness::new();
    h.key(&mut b, &mut s, Key::ArrowUp);
    assert_eq!(b.selected_name(), Some("c.RBS"));
    h.key(&mut b, &mut s, Key::ArrowDown);
    assert_eq!(b.selected_name(), Some("c.RBS"));
    h.key(&mut b, &mut s, Key::Home);
    assert_eq!(b.selected_name(), Some("sub"));
    h.key(&mut b, &mut s, Key::End);
    assert_eq!(b.selected_name(), Some("c.RBS"));
    assert_eq!(h.key(&mut b, &mut s, Key::Escape), Poll::Cancelled);
    assert!(s.recent.is_empty());
}

#[test]
fn open_without_selection_is_an_error_shown_in_the_dialog() {
    let d = tree();
    let (mut b, mut s) = browser(Mode::OpenSong, d.path());
    let mut h = Harness::new();
    assert_eq!(h.key(&mut b, &mut s, Key::Enter), Poll::Open);
    assert_eq!(b.error(), Some("Select a file first."));
}

#[test]
fn save_asks_before_replacing_a_file() {
    let d = tree();
    let (mut b, mut s) = browser(Mode::SaveSong, d.path());
    let mut h = Harness::new();
    h.frame(&mut b, &mut s, vec![]);
    b.set_name("a");
    // The name gets the extension; "a.rbs" exists, so the dialog asks.
    assert_eq!(b.target().unwrap(), d.path().join("a.rbs"));
    assert_eq!(b.accept(&mut s), Poll::Open);
    assert_eq!(
        b.pending_overwrite(),
        Some(d.path().join("a.rbs").as_path())
    );
    // Esc answers no and keeps the dialog open.
    assert_eq!(h.key(&mut b, &mut s, Key::Escape), Poll::Open);
    assert!(b.pending_overwrite().is_none());
    // Enter is the safe answer too: it goes back, it does not replace.
    b.accept(&mut s);
    assert_eq!(h.key(&mut b, &mut s, Key::Enter), Poll::Open);
    assert!(b.pending_overwrite().is_none());
    // N goes back and its letter does not reach the Name field.
    b.accept(&mut s);
    h.frame(&mut b, &mut s, vec![]);
    h.key(&mut b, &mut s, Key::N);
    h.text(&mut b, &mut s, "n");
    assert!(b.pending_overwrite().is_none());
    assert_eq!(b.name(), "a");
    // Only Y replaces.
    b.accept(&mut s);
    assert_eq!(
        h.key(&mut b, &mut s, Key::Y),
        Poll::Chosen(d.path().join("a.rbs"))
    );
}

/// The center of a widget after the layout has settled (the first frame of a modal is a
/// sizing pass).
fn at(h: &mut Harness, b: &mut Browser, s: &mut Settings, name: &str) -> Pos2 {
    // A pause, so that the next click is not the second of a double click.
    h.time += 2.0;
    h.frame(b, s, vec![]);
    h.frame(b, s, vec![]);
    b.hit(name).unwrap_or_else(|| panic!("no {name}")).center()
}

/// One click: the pointer moves there, presses and releases, a frame each.
fn click(h: &mut Harness, b: &mut Browser, s: &mut Settings, pos: Pos2) -> Poll {
    let button = |pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    h.frame(b, s, vec![Event::PointerMoved(pos)]);
    // Two calls within a second at one spot are a double click; the caller does that on
    // purpose, so only `fresh` gaps are added by the callers that need them.
    let a = h.frame(b, s, vec![button(true)]);
    let c = h.frame(b, s, vec![button(false)]);
    if a != Poll::Open { a } else { c }
}

#[test]
fn mouse_click_and_double_click_on_rows_and_places() {
    let d = tree();
    let (mut b, mut s) = browser(Mode::OpenSong, d.path());
    let mut h = Harness::new();
    h.frame(&mut b, &mut s, vec![]);
    // A click selects, a double click on a folder enters it.
    let sub = at(&mut h, &mut b, &mut s, "row:sub");
    click(&mut h, &mut b, &mut s, sub);
    assert_eq!(b.selected_name(), Some("sub"));
    assert_eq!(b.dir(), d.path());
    click(&mut h, &mut b, &mut s, sub);
    assert_eq!(b.dir(), d.path().join("sub"));
    // A double click on a file chooses it.
    let inner = at(&mut h, &mut b, &mut s, "row:inner.rbs");
    click(&mut h, &mut b, &mut s, inner);
    assert_eq!(b.selected_name(), Some("inner.rbs"));
    assert_eq!(
        click(&mut h, &mut b, &mut s, inner),
        Poll::Chosen(d.path().join("sub/inner.rbs"))
    );
    // The folder is now a recent folder: clicking it in the sidebar goes there.
    assert_eq!(s.recent, [d.path().join("sub")]);
    let (mut b, _) = browser(Mode::OpenSong, d.path().join("sub").as_path());
    h.frame(&mut b, &mut s, vec![]);
    let recent = at(&mut h, &mut b, &mut s, "recent:sub");
    b.go(d.path().to_path_buf());
    h.frame(&mut b, &mut s, vec![]);
    click(&mut h, &mut b, &mut s, recent);
    assert_eq!(b.dir(), d.path().join("sub"));
    // A Place: Documents.
    let docs = at(&mut h, &mut b, &mut s, "place:Documents");
    click(&mut h, &mut b, &mut s, docs);
    assert_eq!(b.dir(), rb_ui::settings::documents_dir());
}

#[test]
fn mouse_replace_and_back_buttons() {
    let d = tree();
    let (mut b, mut s) = browser(Mode::SaveSong, d.path());
    let mut h = Harness::new();
    b.set_name("a.rbs");
    b.accept(&mut s);
    h.frame(&mut b, &mut s, vec![]);
    let back = at(&mut h, &mut b, &mut s, "button:Back");
    assert_eq!(click(&mut h, &mut b, &mut s, back), Poll::Open);
    assert!(b.pending_overwrite().is_none());
    b.accept(&mut s);
    h.frame(&mut b, &mut s, vec![]);
    let replace = at(&mut h, &mut b, &mut s, "button:Replace");
    assert_eq!(
        click(&mut h, &mut b, &mut s, replace),
        Poll::Chosen(d.path().join("a.rbs"))
    );
}

#[test]
fn go_to_text_on_a_file_in_a_writing_mode_says_so() {
    let d = tree();
    let (mut b, mut s) = browser(Mode::SaveSong, d.path());
    let mut h = Harness::new();
    h.frame(&mut b, &mut s, vec![]);
    h.key_mods(&mut b, &mut s, Key::L, Modifiers::COMMAND);
    h.key_mods(&mut b, &mut s, Key::A, Modifiers::COMMAND);
    h.text(
        &mut b,
        &mut s,
        &d.path().join("a.rbs").display().to_string(),
    );
    h.key(&mut b, &mut s, Key::Enter);
    assert!(b.error().unwrap().contains("is a file"), "{:?}", b.error());
}

#[test]
fn a_panel_control_does_not_react_while_the_browser_is_open() {
    let mut app = App::new(Panel::new(), None);
    let ctx = egui::Context::default();
    egui_extras::install_image_loaders(&ctx);
    let mut time = 0.0;
    let mut run = |app: &mut App, events: Vec<Event>| {
        time += 0.016;
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(PANEL_W, PANEL_H))),
            time: Some(time),
            events,
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| app.show(ui));
        out.textures_delta.clear();
    };
    let c = rb_ui::layout::controls()
        .into_iter()
        .find(|c| c.id == "transport.play")
        .unwrap();
    let pos = Pos2::new(c.x + c.w / 2.0, c.y + c.h / 2.0);
    let button = |pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    let press = |app: &mut App, run: &mut dyn FnMut(&mut App, Vec<Event>)| {
        run(app, vec![Event::PointerMoved(pos)]);
        run(app, vec![button(true)]);
        run(app, vec![button(false)]);
    };
    run(&mut app, vec![]);
    app.open_browser(Mode::OpenSong);
    run(&mut app, vec![]);
    press(&mut app, &mut run);
    // The click on the backdrop goes to the browser (it closes it), not to the Play button.
    assert!(app.panel.events.is_empty(), "{:?}", app.panel.events);
    assert!(!app.browsing());
    // A click inside the dialog does not dismiss the app's notice line.
    app.open_browser(Mode::OpenSong);
    run(&mut app, vec![]);
    app.notice = Some("x".into());
    let mid = Pos2::new(PANEL_W / 2.0, PANEL_H / 2.0);
    let button = |pressed| Event::PointerButton {
        pos: mid,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    run(&mut app, vec![Event::PointerMoved(mid)]);
    run(&mut app, vec![button(true)]);
    run(&mut app, vec![button(false)]);
    assert!(app.browsing());
    assert!(app.notice.is_some());
    assert!(app.panel.events.is_empty());
}

#[test]
fn save_new_name_needs_no_confirmation_and_bad_names_are_refused() {
    let d = tree();
    let (mut b, mut s) = browser(Mode::SaveSong, d.path());
    b.set_name("new.song.rbs");
    assert_eq!(
        b.accept(&mut s),
        Poll::Chosen(d.path().join("new.song.rbs"))
    );
    b.set_name("");
    assert_eq!(b.accept(&mut s), Poll::Open);
    assert_eq!(b.error(), Some("Type a file name."));
    b.set_name("a/b");
    assert_eq!(b.accept(&mut s), Poll::Open);
    assert!(b.error().unwrap().contains("cannot contain"));
    b.set_name("sub.rbs");
    fs::create_dir(d.path().join("sub.rbs")).unwrap();
    assert_eq!(b.accept(&mut s), Poll::Open);
    assert!(b.error().unwrap().contains("is a folder"));
}

#[test]
fn export_formats_follow_the_extension() {
    let d = tree();
    fs::write(d.path().join("x.flac"), b"x").unwrap();
    let (mut b, mut s) = browser(Mode::Export, d.path());
    b = b.with_name("mix.flac");
    assert_eq!(b.format(), ExportFormat::Flac);
    // The list shows the files of the chosen format only.
    assert_eq!(b.names(), ["sub", "Zeta"]);
    b.set_format(ExportFormat::Ogg);
    assert_eq!(b.name(), "mix.ogg");
    b.set_name("mix");
    assert_eq!(b.target().unwrap(), d.path().join("mix.ogg"));
    b.set_name("mix.mp3");
    assert_eq!(b.accept(&mut s), Poll::Open);
    assert_eq!(
        b.error(),
        Some("The extension must be .wav, .flac or .ogg.")
    );
    b.set_name("MIX.WAV");
    let Poll::Chosen(p) = b.accept(&mut s) else {
        panic!("not chosen");
    };
    assert_eq!(ExportFormat::of_path(&p), Some(ExportFormat::Wav));
    // Choosing an existing file in the list takes its name and format.
    b.set_format(ExportFormat::Flac);
    b.select(b.names().iter().position(|n| *n == "x.flac").unwrap());
    assert_eq!(b.name(), "x.flac");
    b.accept(&mut s);
    assert!(b.pending_overwrite().is_some());
}

#[test]
fn mods_that_are_not_recrypted_are_listed_and_refused() {
    let d = tempfile::tempdir().unwrap();
    // The smallest valid container: an empty CAT of type PRBM. It parses but has no rb key
    // window, so it is not recrypted.
    let mut cat = b"CAT ".to_vec();
    cat.extend_from_slice(&4u32.to_be_bytes());
    cat.extend_from_slice(b"PRBM");
    fs::write(d.path().join("old.rbm"), &cat).unwrap();
    fs::write(d.path().join("junk.rbm"), b"hello").unwrap();
    assert_eq!(
        ModStatus::of(&d.path().join("old.rbm")),
        ModStatus::NeedsRecrypt
    );
    assert!(matches!(
        ModStatus::of(&d.path().join("junk.rbm")),
        ModStatus::Invalid(_)
    ));

    let (mut b, mut s) = browser(Mode::ChooseMod, d.path());
    assert_eq!(b.names(), ["junk.rbm", "old.rbm"]);
    let mut h = Harness::new();
    h.frame(&mut b, &mut s, vec![]);
    b.select(1);
    assert_eq!(h.key(&mut b, &mut s, Key::Enter), Poll::Open);
    assert!(
        b.error().unwrap().contains("rb mod recrypt"),
        "{:?}",
        b.error()
    );
    b.select(0);
    b.accept(&mut s);
    assert!(b.error().unwrap().contains("junk.rbm"));
    assert!(s.recent.is_empty());
}

#[test]
fn unreadable_folders_and_missing_paths_are_errors() {
    let d = tree();
    let (mut b, _) = browser(Mode::OpenSong, d.path());
    b.go(d.path().join("nope"));
    assert!(b.error().unwrap().contains("cannot open"));
    assert_eq!(b.dir(), d.path());
}

#[test]
fn settings_round_trip_and_recent_folders_are_capped() {
    let d = tempfile::tempdir().unwrap();
    let file = d.path().join("cfg/ui.toml");
    let mut s = Settings::load_from(&file);
    assert_eq!(s.recent.len(), 0);
    s.songs_dir = Some(PathBuf::from("/songs"));
    s.mods_dir = Some(PathBuf::from("/mods"));
    for i in 0..12 {
        s.note_recent(Path::new(&format!("/r{i}")));
    }
    s.note_recent(Path::new("/r5"));
    assert_eq!(s.recent.len(), 8);
    assert_eq!(s.recent[0], Path::new("/r5"));
    s.save().unwrap();
    let t = Settings::load_from(&file);
    assert_eq!(t, s);
}

#[test]
fn recent_folders_and_the_configured_folder_are_the_starting_points() {
    let d = tree();
    let mut s = Settings {
        songs_dir: Some(d.path().join("sub")),
        ..Settings::default()
    };
    let b = Browser::new(Mode::OpenSong, &s);
    assert_eq!(b.dir(), d.path().join("sub"));
    // The mods folder is separate; a missing one falls back to the documents folder.
    s.mods_dir = Some(d.path().join("missing"));
    let b = Browser::new(Mode::ChooseMod, &s);
    assert_eq!(b.dir(), rb_ui::settings::documents_dir());
}

#[test]
fn path_bar_goes_to_a_folder_or_a_file() {
    let d = tree();
    let (mut b, mut s) = browser(Mode::OpenSong, d.path());
    let mut h = Harness::new();
    h.frame(&mut b, &mut s, vec![]);
    h.key_mods(&mut b, &mut s, Key::L, Modifiers::COMMAND);
    // Select all and type a new path into the focused bar.
    h.key_mods(&mut b, &mut s, Key::A, Modifiers::COMMAND);
    h.text(&mut b, &mut s, &d.path().join("sub").display().to_string());
    h.key(&mut b, &mut s, Key::Enter);
    assert_eq!(b.dir(), d.path().join("sub"));
}

#[test]
fn the_browser_draws_at_small_sizes() {
    let d = tree();
    let (mut b, mut s) = browser(Mode::Export, d.path());
    let mut h = Harness::new();
    for size in [
        Vec2::new(312.0, 255.0),
        Vec2::new(624.0, 510.0),
        Vec2::new(1400.0, 900.0),
    ] {
        let input = h.input(vec![], size);
        let ctx = h.ctx.clone();
        let mut out = h.ctx.run_ui(input, |_| {
            b.show(&ctx, &mut s);
        });
        out.textures_delta.clear();
        assert!(!out.shapes.is_empty());
    }
}

#[test]
fn the_app_opens_the_browser_with_ctrl_o_and_the_panel_ignores_keys_meanwhile() {
    let mut app = App::new(Panel::new(), None);
    let ctx = egui::Context::default();
    egui_extras::install_image_loaders(&ctx);
    let mut time = 0.0;
    let mut run = |app: &mut App, events: Vec<Event>| {
        time += 0.016;
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(624.0, 510.0))),
            time: Some(time),
            events,
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| app.show(ui));
        out.textures_delta.clear();
    };
    let k = |key, m| {
        vec![
            Event::ModifiersChanged(m),
            Event::Key {
                key,
                physical_key: Some(key),
                pressed: true,
                repeat: false,
                modifiers: m,
            },
            Event::Key {
                key,
                physical_key: Some(key),
                pressed: false,
                repeat: false,
                modifiers: m,
            },
        ]
    };
    run(&mut app, vec![]);
    run(&mut app, k(Key::O, Modifiers::COMMAND));
    assert!(app.browsing());
    // Space would press Play on the panel.
    run(&mut app, k(Key::Space, Modifiers::NONE));
    assert!(app.panel.events.is_empty(), "{:?}", app.panel.events);
    run(&mut app, k(Key::Escape, Modifiers::NONE));
    run(&mut app, vec![]);
    assert!(!app.browsing());
    run(&mut app, k(Key::Space, Modifiers::NONE));
    assert!(!app.panel.events.is_empty());
}
