//! The error message box (task-130), headless: a failed open shows it, it blocks the panel, the
//! menu, the shortcuts and the other windows, only OK closes it (not Escape, not a click outside),
//! errors queue, and information stays on the status line.

use std::path::Path;
use std::sync::Arc;

use eframe::egui::{self, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2};
use orb_engine::tables::Tables;
use orb_ui::app::App;
use orb_ui::browser::Mode;
use orb_ui::layout::{self, PANEL_H, PANEL_W};
use orb_ui::link::Link;
use orb_ui::menu::{Cmd, MENU_H};
use orb_ui::model::Panel;

struct Harness {
    ctx: egui::Context,
    app: App,
    time: f64,
    /// The player's audio side, kept alive for the link.
    _audio: Option<orb_player::Audio>,
    /// What the frames asked to put on the clipboard (`OutputCommand::CopyText`).
    copied: Vec<String>,
}

impl Harness {
    /// An app with the menu bar and no player, in a window as high as the panel and the bar.
    fn new() -> Self {
        let ctx = egui::Context::default();
        egui_extras::install_image_loaders(&ctx);
        let mut app = App::new(Panel::new(), None);
        app.menu = true;
        let mut h = Self {
            ctx,
            app,
            time: 0.0,
            _audio: None,
            copied: Vec::new(),
        };
        h.frame(vec![]);
        h
    }

    /// The same with the player the panel drives, as OpenRB-338 runs.
    fn with_player() -> Self {
        let mut h = Self::new();
        let tables = Arc::new(Tables::new());
        let (link, audio) = Link::new(tables, Some(rbm::standard::builtin_samples()));
        h.app.link = Some(link);
        h.app.samples = Some(rbm::standard::builtin_samples());
        h._audio = Some(audio);
        h.settle();
        h
    }

    /// A frame; the debug text of the shapes it painted (the labels drawn show in it).
    fn frame(&mut self, mut events: Vec<Event>) -> String {
        self.time += 0.016;
        events.insert(0, Event::ModifiersChanged(Modifiers::NONE));
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(
                Pos2::ZERO,
                Vec2::new(PANEL_W, PANEL_H + MENU_H),
            )),
            time: Some(self.time),
            events,
            ..Default::default()
        };
        let app = &mut self.app;
        let mut out = self.ctx.run_ui(input, |ui| app.show(ui));
        for c in &out.platform_output.commands {
            if let egui::OutputCommand::CopyText(t) = c {
                self.copied.push(t.clone());
            }
        }
        out.textures_delta.clear();
        format!("{:?}", out.shapes)
    }

    /// A few frames, so that the layers of the last one are known to the hit tests.
    fn settle(&mut self) {
        for _ in 0..3 {
            self.frame(vec![]);
        }
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

    fn key(&mut self, key: Key, modifiers: Modifiers) {
        let ev = |pressed| Event::Key {
            key,
            physical_key: Some(key),
            pressed,
            repeat: false,
            modifiers,
        };
        self.frame(vec![ev(true)]);
        self.frame(vec![ev(false)]);
    }

    /// The center of the control `id` (the panel is below the menu bar).
    fn center(id: &str) -> Pos2 {
        let c = layout::controls().into_iter().find(|c| c.id == id).unwrap();
        Pos2::new(c.x + c.w / 2.0, c.y + c.h / 2.0 + MENU_H)
    }

    fn click_ok(&mut self) {
        let ok = self.app.error_ok_rect().expect("the box is drawn");
        self.click(ok.center());
    }
}

/// What reaches the panel of the app: the events its controls and keys made.
fn panel_events(h: &Harness) -> usize {
    h.app.panel.events.len()
}

/// The box the keys and the clicks are tested against, with a control run first: the same input
/// does reach the panel when the box is not there.
fn open_box(h: &mut Harness) {
    h.app.open_song(Path::new("/no/such/dir/song.rbs"));
    h.settle();
}

#[test]
fn a_failed_open_shows_the_box_with_the_message() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = Harness::with_player();
    assert!(!h.app.error_open());
    assert!(h.app.error_ok_rect().is_none());

    // a file that is not there
    let missing = dir.path().join("missing.rbs");
    h.app.open_song(&missing);
    assert!(h.app.error_open());
    h.settle();
    let shapes = h.frame(vec![]);
    let message = h.app.error().unwrap().to_owned();
    assert!(message.contains("missing.rbs"), "{message}");
    assert!(shapes.contains("Error"), "the title");
    assert!(
        shapes.contains("missing.rbs"),
        "the message is drawn: {shapes}"
    );
    assert!(shapes.contains("OK"), "the button");
    assert!(h.app.error_ok_rect().is_some());
    assert_eq!(h.app.status, None, "not on the status line");
    h.click_ok();
    assert!(!h.app.error_open());

    // a file that is not a song: the same box, with the reader's message
    let bad = dir.path().join("bad.rbs");
    std::fs::write(&bad, b"this is not a song").unwrap();
    h.app.open_song(&bad);
    let message = h.app.error().unwrap().to_owned();
    assert!(message.contains("bad.rbs"), "{message}");
    h.settle();
    let shapes = h.frame(vec![]);
    assert!(shapes.contains("bad.rbs"), "{shapes}");
    // the song open as it was: the new empty one, still untitled
    assert_eq!(h.app.doc.path, None);

    // without a player there is nothing to open the song in: the same box
    let mut h = Harness::new();
    h.app.open_song(&missing);
    assert_eq!(
        h.app.error(),
        Some("There is no player to open the song in.")
    );
}

#[test]
fn the_box_blocks_the_panel_the_menu_and_the_shortcuts() {
    // the control: without the box the same input reaches the panel, the menu and the browser
    let mut h = Harness::new();
    h.settle();
    h.click(Harness::center("transport.play"));
    assert!(panel_events(&h) > 0, "a click on Play reaches the panel");
    h.app.panel.events.clear();
    let focus = h.app.panel.focus;
    h.key(Key::ArrowDown, Modifiers::NONE);
    assert_ne!(h.app.panel.focus, focus, "Down moves the focus row");
    h.click(Pos2::new(14.0, MENU_H / 2.0));
    assert!(egui::Popup::is_any_open(&h.ctx), "the File menu opens");
    h.key(Key::Escape, Modifiers::NONE);
    h.key(
        Key::O,
        Modifiers {
            command: true,
            ctrl: true,
            ..Modifiers::NONE
        },
    );
    assert!(h.app.browsing(), "Ctrl+O opens the browser");

    // with the box: none of it
    let mut h = Harness::new();
    open_box(&mut h);
    h.app.panel.events.clear();
    h.click(Harness::center("transport.play"));
    let focus = h.app.panel.focus;
    h.key(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(h.app.panel.focus, focus, "the focus row stays");
    assert_eq!(panel_events(&h), 0, "{:?}", h.app.panel.events);
    h.click(Pos2::new(14.0, MENU_H / 2.0));
    assert!(!egui::Popup::is_any_open(&h.ctx), "the menu stays shut");
    let ctrl = Modifiers {
        command: true,
        ctrl: true,
        ..Modifiers::NONE
    };
    h.key(Key::O, ctrl);
    assert!(!h.app.browsing(), "no shortcut works");
    assert!(h.app.error_open(), "none of it closed the box");

    // the other windows: an open window takes no key and no click either
    let mut h = Harness::new();
    h.app.command(Cmd::KeyHelp);
    h.settle();
    assert!(h.app.key_help_is_open());
    open_box(&mut h);
    // Escape closes the box and no more: the window under it stays
    h.key(Key::Escape, Modifiers::NONE);
    assert!(!h.app.error_open());
    assert!(h.app.key_help_is_open(), "Escape does not reach the window");
    h.key(Key::Escape, Modifiers::NONE);
    assert!(
        !h.app.key_help_is_open(),
        "after the box the window takes its keys again"
    );
}

#[test]
fn the_unsaved_prompt_gets_no_key_while_the_box_is_open() {
    let mut h = Harness::new();
    h.app.doc.modified = true;
    assert!(!h.app.request_close());
    assert!(h.app.prompting());
    open_box(&mut h);
    // S would save
    h.key(Key::S, Modifiers::NONE);
    assert!(h.app.prompting());
    assert!(!h.app.browsing());
    assert!(h.app.error_open());
    // Escape closes the box and cancels nothing, Enter would save
    h.key(Key::Escape, Modifiers::NONE);
    assert!(!h.app.error_open());
    assert!(h.app.prompting(), "the prompt is as it was");
    assert!(!h.app.browsing());
    h.key(Key::Escape, Modifiers::NONE);
    assert!(!h.app.prompting(), "now Escape cancels it");
}

#[test]
fn ok_the_close_button_escape_and_enter_close_the_box_a_click_outside_does_not() {
    let mut h = Harness::new();
    open_box(&mut h);
    assert!(h.app.error_open());
    // a click outside it: the panel, the menu bar, a corner
    for p in [
        Pos2::new(3.0, MENU_H + 3.0),
        Pos2::new(PANEL_W - 3.0, PANEL_H),
        Pos2::new(14.0, MENU_H / 2.0),
        Harness::center("transport.play"),
    ] {
        h.click(p);
        assert!(h.app.error_open(), "a click at {p:?} does not close it");
    }
    // and nothing came of those clicks on the panel
    assert_eq!(panel_events(&h), 0, "{:?}", h.app.panel.events);
    let ok = h.app.error_ok_rect().unwrap();
    let x = h
        .app
        .error_close_rect()
        .expect("the title bar has a close button");
    assert!(ok.width() > 0.0 && x.width() > 0.0);
    assert!(
        x.center().y < ok.center().y,
        "the close button is in the title bar"
    );

    // each way to close it closes the first box only
    for way in ["ok", "x", "escape", "enter", "space"] {
        let mut h = Harness::new();
        h.app.fail("one");
        h.app.fail("two");
        h.settle();
        match way {
            "ok" => h.click_ok(),
            "x" => {
                let x = h.app.error_close_rect().unwrap();
                h.click(x.center());
            }
            "escape" => h.key(Key::Escape, Modifiers::NONE),
            "enter" => h.key(Key::Enter, Modifiers::NONE),
            _ => h.key(Key::Space, Modifiers::NONE),
        }
        assert_eq!(
            h.app.error(),
            Some("two"),
            "{way} closes the first box only"
        );
        h.settle();
        h.key(Key::Enter, Modifiers::NONE);
        assert!(!h.app.error_open(), "{way}");
        assert!(h.app.error_ok_rect().is_none() && h.app.error_close_rect().is_none());
        // the keys that closed it did not reach the panel (Enter and Space are Play)
        assert_eq!(panel_events(&h), 0, "{way}: {:?}", h.app.panel.events);
    }
}

#[test]
fn enter_closes_the_box_whatever_has_the_focus() {
    let mut h = Harness::new();
    open_box(&mut h);
    // Tab moves the focus off OK (to the close button)
    h.key(Key::Tab, Modifiers::NONE);
    assert!(h.app.error_open());
    h.key(Key::Enter, Modifiers::NONE);
    assert!(!h.app.error_open());
}

#[test]
fn errors_queue_one_box_at_a_time_in_order() {
    let mut h = Harness::new();
    h.app.fail("first error");
    h.app.fail("second error");
    h.settle();
    assert_eq!(h.app.error(), Some("first error"));
    let shapes = h.frame(vec![]);
    assert!(shapes.contains("first error"));
    assert!(!shapes.contains("second error"), "one box at a time");
    assert!(shapes.contains("1 more"), "the queue is shown");
    h.click_ok();
    assert_eq!(h.app.error(), Some("second error"));
    let shapes = h.frame(vec![]);
    assert!(shapes.contains("second error"));
    assert!(!shapes.contains("first error"));
    assert!(!shapes.contains("more"));
    h.click_ok();
    assert!(!h.app.error_open());
    let shapes = h.frame(vec![]);
    assert!(!shapes.contains("second error"));
    // an error that comes while one is shown waits for it
    h.app.fail("third error");
    h.app.fail("fourth error");
    h.settle();
    h.click_ok();
    assert_eq!(h.app.error(), Some("fourth error"));
}

#[test]
fn information_stays_on_the_status_line() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("mods");
    std::fs::create_dir(&folder).unwrap();
    let mut h = Harness::new();
    h.app.settings.mods_dir = Some(folder.clone());
    h.app.command(Cmd::RefreshMods);
    let want = format!(
        "Mod list refreshed: 0 mods in {} (+0, -0)",
        folder.display()
    );
    assert_eq!(h.app.status.as_deref(), Some(want.as_str()));
    let shapes = h.frame(vec![]);
    assert!(
        shapes.contains("Mod list refreshed"),
        "the status line shows it"
    );
    assert!(!h.app.error_open(), "no box");
    assert!(h.app.error_ok_rect().is_none());
    // a click dismisses the information, as before
    h.settle();
    h.click(Pos2::new(PANEL_W / 2.0, MENU_H + 3.0));
    assert_eq!(h.app.status, None);

    // while a box is open, a click does not dismiss the status line under it
    h.app.status = Some("kept".into());
    h.app.fail("an error");
    h.settle();
    h.click(Pos2::new(PANEL_W / 2.0, PANEL_H / 2.0));
    assert_eq!(h.app.status.as_deref(), Some("kept"));
    assert!(h.app.error_open());
    h.click_ok();
    assert_eq!(h.app.status.as_deref(), Some("kept"));
}

#[test]
fn every_failure_of_the_menu_commands_goes_to_the_box() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = Harness::with_player();
    // Save As to a folder that is not there
    h.app
        .chosen(Mode::SaveSong, dir.path().join("no/such/dir/x.rbs"));
    assert!(h.app.error().unwrap().contains("cannot write"));
    h.app.dismiss_error();
    // Export in pattern mode
    h.app.command(Cmd::Export);
    assert!(h.app.error().unwrap().contains("song mode"));
    h.app.dismiss_error();
    // a mod that is not there
    h.app
        .use_kit(orb_cli::mods::Kit::Mod(dir.path().join("no/such/mod.rbm")));
    assert!(h.app.error_open());
    h.app.dismiss_error();
    assert!(!h.app.error_open());
    assert_eq!(h.app.status, None);
}

/// task-130: an error that can repeat (the audio output) never opens a box: the status line
/// shows the latest one, one notice per kind, with a count.
#[test]
fn a_repeated_audio_error_stays_on_the_status_line_and_opens_no_box() {
    let mut h = Harness::new();
    for _ in 0..50 {
        h.app.notify("audio", "Audio output: device lost");
        h.frame(vec![]);
    }
    assert!(!h.app.error_open(), "{:?}", h.app.errors);
    assert_eq!(
        h.app.status.as_deref(),
        Some("Audio output: device lost (repeated 50 times)")
    );
    let shapes = h.frame(vec![]);
    assert!(shapes.contains("device lost"), "the status line shows it");
    assert!(h.app.error_ok_rect().is_none());
    // another kind has its own count
    h.app.notify("stream", "Stream error");
    assert_eq!(h.app.status.as_deref(), Some("Stream error"));
    // the first one is shown plainly
    let mut h = Harness::new();
    h.app.notify("audio", "Audio output: no device");
    assert_eq!(h.app.status.as_deref(), Some("Audio output: no device"));
}

/// task-130: the message can be selected with the mouse and copied with the copy shortcut (the
/// platform's Copy event) while the box blocks everything else; the Copy button copies it all;
/// neither closes the box.
#[test]
fn the_message_is_selectable_and_copyable() {
    let mut h = Harness::new();
    h.app.fail("Cannot write /x/y.rbs: disk full");
    h.settle();
    // select the text with the mouse: press at its start, drag past its end
    let t = h.app.error_text_rect().expect("the text is drawn");
    let from = t.left_center() + Vec2::new(1.0, 0.0);
    let to = t.right_center() + Vec2::new(20.0, 0.0);
    let button = |pos, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    h.frame(vec![Event::PointerMoved(from)]);
    h.frame(vec![button(from, true)]);
    h.frame(vec![Event::PointerMoved(to)]);
    h.frame(vec![button(to, false)]);
    assert!(h.app.error_open(), "selecting does not close the box");
    // the copy shortcut (Ctrl/Cmd+C: the integration sends Event::Copy, and the key itself)
    let c = Modifiers {
        command: true,
        ctrl: true,
        ..Modifiers::NONE
    };
    h.frame(vec![
        Event::Copy,
        Event::Key {
            key: Key::C,
            physical_key: Some(Key::C),
            pressed: true,
            repeat: false,
            modifiers: c,
        },
    ]);
    assert_eq!(h.copied, ["Cannot write /x/y.rbs: disk full"]);
    assert!(
        h.app.error_open(),
        "the copy shortcut does not close the box"
    );

    // the Copy button copies the whole message, and does not close the box either
    let mut h = Harness::new();
    h.app.fail("one\ntwo");
    h.settle();
    let b = h.app.error_copy_rect().expect("the Copy button is drawn");
    h.click(b.center());
    assert_eq!(h.copied, ["one\ntwo"]);
    assert!(h.app.error_open());
    // Escape closes it, as ever
    h.key(Key::Escape, Modifiers::NONE);
    assert!(!h.app.error_open());
}

/// task-130: one Open that fails twice (the mod is missing, and the engine refuses the song) is
/// one box with both messages, not two boxes.
#[test]
fn one_open_with_two_failures_is_one_box() {
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
    big.glob.mod_name = "No Such Mod".into();
    big.tracks[5].events = events(60_000, [3, 4]);
    big.tracks[6].events = events(41_000, [1, 2]);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("big.rbs");
    std::fs::write(&path, big.write().unwrap()).unwrap();
    // an empty Mods folder that exists
    let mods = dir.path().join("mods");
    std::fs::create_dir(&mods).unwrap();

    let mut h = Harness::with_player();
    h.app.settings.mods_dir = Some(mods);
    h.app.open_song(&path);
    assert_eq!(h.app.errors.len(), 1, "{:?}", h.app.errors);
    let m = h.app.error().unwrap();
    assert!(m.contains("No Such Mod"), "{m}");
    assert!(m.contains("not enough memory"), "{m}");
    assert_eq!(m.lines().count(), 2, "{m}");
    h.settle();
    h.click_ok();
    assert!(!h.app.error_open());
}

/// task-130: a failed settings write of the browser ("Use as songs folder") is a box.
#[test]
fn a_browser_settings_failure_gives_a_box() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = Harness::new();
    // a settings file that cannot be written: its parent is a file
    let blocker = dir.path().join("blocker");
    std::fs::write(&blocker, b"x").unwrap();
    h.app.settings.file = Some(blocker.join("ui.toml"));
    h.app.open_browser(Mode::OpenSong);
    h.settle();
    let b = h
        .app
        .browser_hit("button:Use as songs folder")
        .expect("the button is drawn");
    h.click(b.center());
    assert!(h.app.error_open(), "the write failed");
    assert!(
        h.app.error().unwrap().contains("blocker"),
        "{:?}",
        h.app.error()
    );
    assert_eq!(h.app.errors.len(), 1);
}

/// task-130: the one audio failure of the launch is a box (with the start's other errors in the
/// same one); a later, repeating one is a notification.
#[test]
fn a_start_up_audio_failure_gives_a_box() {
    let mut h = Harness::new();
    h.app.action(|app| {
        app.start(None);
        app.fail("Audio output: no device; the panel runs without sound");
    });
    assert_eq!(h.app.errors.len(), 1);
    assert!(h.app.error().unwrap().contains("Audio output"));
    h.settle();
    h.click_ok();
    // the later ones repeat: a notification, no box
    h.app.notify("audio", "Audio output: stream error");
    h.app.notify("audio", "Audio output: stream error");
    assert!(!h.app.error_open());
}

/// task-130: a menu that is open when the error comes is closed, and the paste and cut of
/// the frame do not reach what is under the box.
#[test]
fn an_open_menu_is_closed_by_the_box() {
    let mut h = Harness::new();
    h.settle();
    h.click(Pos2::new(14.0, MENU_H / 2.0));
    assert!(egui::Popup::is_any_open(&h.ctx), "the File menu is open");
    h.app.fail("an error");
    h.settle();
    assert!(!egui::Popup::is_any_open(&h.ctx), "the box closed the menu");
    assert!(h.app.error_open());
    h.frame(vec![Event::Paste("x".into()), Event::Cut]);
    assert!(h.app.error_open());
}

/// task-130: a burst of errors from one action is one box that lists them; the same message
/// is never queued twice.
#[test]
fn a_burst_from_one_action_gives_one_box() {
    let mut h = Harness::new();
    let burst = (0..40)
        .map(|i| {
            if i % 2 == 0 {
                "a.rbm: not recrypted".to_string()
            } else {
                format!("b{i}.rbm: cannot read")
            }
        })
        .chain(std::iter::repeat_n("c.rbm: not a mod".to_string(), 5));
    h.app.fail_many(burst);
    assert_eq!(h.app.errors.len(), 1, "{:?}", h.app.errors);
    let m = h.app.error().unwrap();
    assert!(m.contains("a.rbm: not recrypted (20 times)"), "{m}");
    assert!(m.contains("c.rbm: not a mod (5 times)"), "{m}");
    assert_eq!(m.lines().count(), 22, "{m}");
    // the same message, again and again, queues once
    for _ in 0..10 {
        h.app.fail("same");
    }
    assert_eq!(h.app.errors.len(), 2);
    h.settle();
    h.click_ok();
    h.settle();
    h.click_ok();
    assert!(!h.app.error_open());
}
