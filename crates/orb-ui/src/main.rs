//! `OpenRB-338`: the panel window, wired to the real-time player (task-045). The binary has this
//! name (crate `orb-ui`) because the macOS Dock and Activity Monitor show the executable's name.
//!
//! ```text
//! OpenRB-338 [SONG]
//! ```
//! `--help` shows only this. The options below are for developers, tests and screenshots
//! (task-137): hidden from `--help` and the usage text, but they all work (docs/building.md,
//! "OpenRB-338 developer options"):
//!
//! ```text
//! --demo  --size WxH  --ppp S  --browse open|save|export|mod  --about  --info
//! --screenshot OUT.png
//! ```
//!
//! `-h` is `--help`; an argument `-psn_...` (Finder on old macOS) is ignored; a bad option prints
//! the usage and exits 2.
//!
//! With a song it opens that song, with the mod it names (looked up in the Mods folder of the
//! settings, `ORB_MOD_DIR`, or the default places of `orb render`); without one the panel starts on
//! a new empty song. The menu bar (task-047, docs/ui_spec.md section 18) opens, saves and exports
//! songs, chooses the mod and sets the Vintage flag and the keyboard mode. The drum machines play
//! the built-in kit "Standard ReBirth", the standard mod orb embeds (`assets/Standard.rbm`), or
//! the song's mod laid over it. `--demo`
//! starts the panel in a lively state with no player (a picture). The window starts maximized;
//! `--size` opens it, not maximized, with that inner size of the panel in points (624x510 is
//! the native size; the window is as high as the menu bar more, for screenshots), `--ppp`
//! overrides the pixel density. `--screenshot` saves the window to a PNG after a few frames and
//! quits. Ctrl/Cmd+O opens the in-app file browser (task-046) to open a song, Ctrl/Cmd+S saves;
//! `--about` starts with the About window open (for screenshots, task-115), `--info` with File >
//! Info's Song Information window (task-117).
//! `--browse MODE` starts with the browser open for a mode. The browser's folders are kept in the
//! settings file (`OpenRB-338/ui.toml` in the configuration folder, or `$ORB_UI_CONFIG`).

// The Windows release build is a GUI-subsystem program: no console window opens behind the
// panel (task-111). `orb` is not affected (this attribute is in the UI binary only). A debug
// build keeps the console, for `dbg!` and panics.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui;
use orb_engine::tables::Tables;
use orb_ui::app::{App, Shot};
use orb_ui::browser::Mode;
use orb_ui::layout::{PANEL_H, PANEL_W};
use orb_ui::link::Link;
use orb_ui::menu::MENU_H;
use orb_ui::model::Panel;
use orb_ui::session;
use orb_ui::settings::Settings;

/// The visible help: only the song. The developer options are hidden (task-137).
const USAGE: &str = "usage: OpenRB-338 [SONG]\n\
                     SONG is a ReBirth song (.rbs, RBS 4.x or ReBirth 1.x) to open";

/// The command line, parsed.
#[derive(Debug, PartialEq)]
struct Args {
    demo: bool,
    about: bool,
    info: bool,
    size: [f32; 2],
    sized: bool,
    ppp: Option<f32>,
    shot: Option<PathBuf>,
    song: Option<PathBuf>,
    browse: Option<Mode>,
}

/// What the command line asks instead of running the app.
#[derive(Debug, PartialEq)]
enum Exit {
    /// `--help`: print the usage, exit 0.
    Help,
    /// A bad option or value: print the usage, exit 2.
    Bad,
}

/// Parses the arguments after the program name. `-psn_...` (Finder on old macOS) is ignored.
fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Args, Exit> {
    let mut r = Args {
        demo: false,
        about: false,
        info: false,
        size: [PANEL_W, PANEL_H],
        sized: false,
        ppp: None,
        shot: None,
        song: None,
        browse: None,
    };
    let mut args = args.into_iter();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--help" | "-h" => return Err(Exit::Help),
            "--demo" => r.demo = true,
            "--about" => r.about = true,
            "--info" => r.info = true,
            "--size" => {
                let v = args.next().ok_or(Exit::Bad)?;
                let (w, h) = v.split_once('x').ok_or(Exit::Bad)?;
                r.size = [
                    w.parse().map_err(|_| Exit::Bad)?,
                    h.parse().map_err(|_| Exit::Bad)?,
                ];
                r.sized = true;
            }
            "--ppp" => r.ppp = Some(args.next().and_then(|v| v.parse().ok()).ok_or(Exit::Bad)?),
            "--screenshot" => r.shot = Some(args.next().ok_or(Exit::Bad)?.into()),
            "--browse" => {
                r.browse = Some(match args.next().as_deref() {
                    Some("open") => Mode::OpenSong,
                    Some("save") => Mode::SaveSong,
                    Some("export") => Mode::Export,
                    Some("mod") => Mode::ChooseMod,
                    _ => return Err(Exit::Bad),
                })
            }
            s if s.starts_with("-psn_") => {}
            s if !s.starts_with('-') && r.song.is_none() => r.song = Some(s.into()),
            _ => return Err(Exit::Bad),
        }
    }
    Ok(r)
}

fn usage(e: Exit) -> ! {
    match e {
        Exit::Help => {
            println!("{USAGE}");
            std::process::exit(0);
        }
        Exit::Bad => {
            eprintln!("{USAGE}");
            std::process::exit(2);
        }
    }
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
    let Args {
        demo,
        about,
        info,
        size,
        sized,
        ppp,
        shot,
        song,
        browse,
    } = parse_args(std::env::args().skip(1)).unwrap_or_else(|e| usage(e));

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
        match orb_player::output::start(audio) {
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
        .with_icon(Arc::new(orb_ui::art::app_icon(256)))
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
            // ORB_GPU_INFO=1 prints the adapter wgpu chose (scripts/appimage-smoke.sh)
            if std::env::var_os("ORB_GPU_INFO").is_some_and(|v| v == "1")
                && let Some(rs) = &cc.wgpu_render_state
            {
                let i = rs.adapter.get_info();
                eprintln!("gpu: {:?} {:?} {}", i.backend, i.device_type, i.name);
            }
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
            app.enable_logo_anim();
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
            if about {
                app.show_about();
            }
            if info {
                app.show_info();
            }
            Ok(Box::new(app))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(a: &[&str]) -> Result<Args, Exit> {
        parse_args(a.iter().map(|s| s.to_string()))
    }

    #[test]
    fn no_arguments_is_the_new_song() {
        let a = parse(&[]).unwrap();
        assert_eq!(a.song, None);
        assert!(!a.demo && !a.about && !a.info && !a.sized);
        assert_eq!(a.size, [PANEL_W, PANEL_H]);
    }

    #[test]
    fn help_is_exit_0_and_a_bad_option_is_exit_2() {
        assert_eq!(parse(&["--help"]), Err(Exit::Help));
        assert_eq!(parse(&["-h"]), Err(Exit::Help));
        assert_eq!(parse(&["song.rbs", "--help"]), Err(Exit::Help));
        for bad in [
            &["--nope"][..],
            &["-x"],
            &["--size"],
            &["--size", "10"],
            &["--size", "axb"],
            &["--ppp", "x"],
            &["--screenshot"],
            &["--browse", "other"],
            &["a.rbs", "b.rbs"],
        ] {
            assert_eq!(parse(bad), Err(Exit::Bad), "{bad:?}");
        }
    }

    #[test]
    fn the_hidden_options_still_parse() {
        let a = parse(&[
            "--demo",
            "--size",
            "300x200",
            "--ppp",
            "2",
            "--browse",
            "export",
            "--about",
            "--info",
            "--screenshot",
            "out.png",
        ])
        .unwrap();
        assert!(a.demo && a.about && a.info && a.sized);
        assert_eq!(a.size, [300.0, 200.0]);
        assert_eq!(a.ppp, Some(2.0));
        assert_eq!(a.browse, Some(Mode::Export));
        assert_eq!(a.shot, Some("out.png".into()));
    }

    #[test]
    fn the_song_is_a_path_with_spaces_and_psn_is_ignored() {
        let a = parse(&["-psn_0_12345", "/my songs/a b.rbs"]).unwrap();
        assert_eq!(a.song, Some("/my songs/a b.rbs".into()));
        let a = parse(&["-psn_0_12345"]).unwrap();
        assert_eq!(a.song, None);
    }

    #[test]
    fn the_usage_shows_only_the_song() {
        assert!(USAGE.contains("[SONG]"));
        for hidden in [
            "--demo",
            "--size",
            "--ppp",
            "--browse",
            "--screenshot",
            "--about",
            "--info",
        ] {
            assert!(!USAGE.contains(hidden), "{hidden}");
        }
    }
}
