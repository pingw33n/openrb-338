//! `OpenRB-338`: the panel window, wired to the real-time player (task-045). The binary has this
//! name (crate `rb-ui`) because the macOS Dock and Activity Monitor show the executable's name.
//!
//! ```text
//! OpenRB-338 [SONG.rbs] [--demo] [--size WxH] [--ppp S] [--screenshot OUT.png]
//! ```
//! With a song it opens that song, with the mod it names (looked up in the Mods folder of the
//! settings, `RB_MOD_DIR`, or the default places of `rb render`); without one the panel starts on
//! a new empty song. The menu bar (task-047, docs/ui_spec.md section 18) opens, saves and exports
//! songs, chooses the mod and sets the Vintage flag and the keyboard mode. The drum machines play
//! the built-in kit "Standard ReBirth", the standard mod rb embeds (`assets/Standard.rbm`), or
//! the song's mod laid over it. `--demo`
//! starts the panel in a lively state with no player (a picture). The window starts maximized;
//! `--size` opens it, not maximized, with that inner size of the panel in points (624x510 is
//! the native size; the window is as high as the menu bar more, for screenshots), `--ppp`
//! overrides the pixel density. `--screenshot` saves the window to a PNG after a few frames and
//! quits. Ctrl/Cmd+O opens the in-app file browser (task-046) to open a song, Ctrl/Cmd+S saves;
//! `--browse MODE` starts with the browser open for a mode. The browser's folders are kept in the
//! settings file (`rb/ui.toml` in the configuration folder, or `$RB_UI_CONFIG`).

// The Windows release build is a GUI-subsystem program: no console window opens behind the
// panel (task-111). `rb` is not affected (this attribute is in the UI binary only). A debug
// build keeps the console, for `dbg!` and panics.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui;
use rb_engine::tables::Tables;
use rb_ui::app::{App, Shot};
use rb_ui::browser::Mode;
use rb_ui::layout::{PANEL_H, PANEL_W};
use rb_ui::link::Link;
use rb_ui::menu::MENU_H;
use rb_ui::model::Panel;
use rb_ui::session;
use rb_ui::settings::Settings;

fn usage() -> ! {
    eprintln!(
        "usage: OpenRB-338 [SONG.rbs] [--demo] [--size WxH] [--ppp S] \
         [--browse open|save|export|mod] [--screenshot OUT.png]\n\
         the window starts maximized; --size WxH opens it at that panel size instead"
    );
    std::process::exit(2);
}

/// Gives a GUI-subsystem process the console of the program that started it (task-111).
///
/// Such a process has no console, so `eprintln!` (the usage text, a song that does not open, the
/// audio error) goes nowhere. When it is started from `cmd` or PowerShell, this attaches to that
/// console and points the standard output and error at it, unless they were redirected (a file
/// or a pipe stays as it is). Started from the Start menu or Explorer there is no parent
/// console and nothing happens. What stays lost: `cmd` does not wait for a GUI program, so the
/// prompt comes back at once and the text appears over it; `start /wait OpenRB-338.exe` waits.
#[cfg(all(windows, not(debug_assertions)))]
fn attach_parent_console() {
    use std::fs::OpenOptions;
    use std::os::windows::io::IntoRawHandle;

    type Handle = *mut core::ffi::c_void;
    const ATTACH_PARENT_PROCESS: u32 = u32::MAX;
    const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;
    const STD_ERROR_HANDLE: u32 = -12i32 as u32;
    unsafe extern "system" {
        fn AttachConsole(process_id: u32) -> i32;
        fn GetStdHandle(std_handle: u32) -> Handle;
        fn SetStdHandle(std_handle: u32, handle: Handle) -> i32;
    }
    // SAFETY: plain Win32 calls; the handle given to SetStdHandle is leaked on purpose, it must
    // live as long as the process.
    unsafe {
        if AttachConsole(ATTACH_PARENT_PROCESS) == 0 {
            return;
        }
        for id in [STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
            let h = GetStdHandle(id);
            // none (null) or invalid (-1): not redirected, so use the console
            if (h.is_null() || h as isize == -1)
                && let Ok(f) = OpenOptions::new().write(true).open("CONOUT$")
            {
                SetStdHandle(id, f.into_raw_handle());
            }
        }
    }
}

fn main() -> eframe::Result {
    #[cfg(all(windows, not(debug_assertions)))]
    attach_parent_console();
    let mut demo = false;
    let mut size = [PANEL_W, PANEL_H];
    let mut sized = false;
    let mut ppp: Option<f32> = None;
    let mut shot: Option<PathBuf> = None;
    let mut song: Option<PathBuf> = None;
    let mut browse: Option<Mode> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--demo" => demo = true,
            "--size" => {
                let v = args.next().unwrap_or_else(|| usage());
                let (w, h) = v.split_once('x').unwrap_or_else(|| usage());
                size = [
                    w.parse().unwrap_or_else(|_| usage()),
                    h.parse().unwrap_or_else(|_| usage()),
                ];
                sized = true;
            }
            "--ppp" => {
                ppp = Some(
                    args.next()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or_else(|| usage()),
                )
            }
            "--screenshot" => shot = Some(args.next().unwrap_or_else(|| usage()).into()),
            "--browse" => {
                browse = Some(match args.next().as_deref() {
                    Some("open") => Mode::OpenSong,
                    Some("save") => Mode::SaveSong,
                    Some("export") => Mode::Export,
                    Some("mod") => Mode::ChooseMod,
                    _ => usage(),
                })
            }
            s if !s.starts_with("--") && song.is_none() => song = Some(s.into()),
            _ => usage(),
        }
    }

    let panel = if demo { Panel::demo() } else { Panel::new() };
    let mut link = None;
    let mut app_samples = None;
    let mut startup = None;
    // errors of the start, shown in the message box once the app is up (task-130)
    let mut start_errors: Vec<String> = Vec::new();
    // keeps the audio stream alive for the whole run
    let mut _stream = None;
    if !demo {
        let samples = Some(rbm::standard::builtin_samples());
        app_samples = samples.clone();
        let (l, audio) = Link::new(Arc::new(Tables::new()), samples.clone());
        // The song named on the command line is opened by the app (with its mod).
        if let Some(path) = &song {
            match session::open_song(path) {
                Ok(o) => startup = Some((path.clone(), o)),
                // the app starts on the new song, with the error in its message box
                Err(e) => {
                    eprintln!("OpenRB-338: {e}");
                    start_errors.push(e);
                }
            }
        }
        match rb_player::output::start(audio) {
            Ok(s) => _stream = Some(s),
            Err(e) => {
                eprintln!("OpenRB-338: audio output: {e}; the panel runs without sound");
                // the one failure at the launch is an error the user expects to hear of: the
                // box. Later audio errors repeat: `App::notify`.
                start_errors.push(format!("Audio output: {e}; the panel runs without sound"));
            }
        }
        link = Some(l);
    }

    // The window starts maximized; an explicit --size opens at that size instead.
    let viewport = egui::ViewportBuilder::default()
        .with_title("OpenRB-338")
        .with_app_id("OpenRB-338")
        .with_icon(Arc::new(rb_ui::art::app_icon(256)))
        // the menu bar is above the panel
        .with_inner_size([size[0], size[1] + MENU_H])
        // The minimum is half the native panel size (decision task-040).
        .with_min_inner_size([PANEL_W / 2.0, PANEL_H / 2.0 + MENU_H])
        .with_maximized(!sized);
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "OpenRB-338",
        options,
        Box::new(move |cc| {
            // always dark, whatever the system theme (task-127; `App::show` keeps it)
            cc.egui_ctx.set_theme(egui::ThemePreference::Dark);
            if let Some(ppp) = ppp {
                cc.egui_ctx.set_pixels_per_point(ppp);
            }
            let shot = shot.map(Shot::new);
            let mut app = match link {
                Some(l) => App::with_link(panel, l, shot),
                None => App::new(panel, shot),
            };
            app.samples = app_samples;
            app.menu = true;
            app.settings = Settings::load();
            // the song (or the new empty song) and its mod's startup sound; nothing in the demo,
            // which has no player
            // (the errors of the start are one box)
            app.action(|app| {
                app.start(startup);
                for e in start_errors {
                    app.fail(e);
                }
            });
            if let Some(mode) = browse {
                app.open_browser(mode);
            }
            Ok(Box::new(app))
        }),
    )
}
