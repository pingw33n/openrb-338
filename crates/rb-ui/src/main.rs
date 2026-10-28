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

fn main() -> eframe::Result {
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
                Err(e) => {
                    eprintln!("OpenRB-338: {e}");
                    std::process::exit(1);
                }
            }
        }
        match rb_player::output::start(audio) {
            Ok(s) => _stream = Some(s),
            Err(e) => eprintln!("OpenRB-338: audio output: {e}; the panel runs without sound"),
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
            app.start(startup);
            if let Some(mode) = browse {
                app.open_browser(mode);
            }
            Ok(Box::new(app))
        }),
    )
}
