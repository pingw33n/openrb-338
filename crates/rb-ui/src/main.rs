//! `rb-ui`: the panel window, wired to the real-time player (task-045).
//!
//! ```text
//! rb-ui [SONG.rbs] [--standard-mod FILE] [--demo] [--size WxH] [--ppp S] [--screenshot OUT.png]
//! ```
//! With a song it opens that song (the menu that opens others is task-047); without one the
//! panel starts on a new empty song. The drum machines play the built-in kit of the standard mod
//! (`--standard-mod`, `RB_STANDARD_MOD`, or `Standard.rbm` next to the binary; make it with
//! `rb standard-mod`); without it they are silent. `--demo` starts the panel in a lively state
//! with no player (a picture). `--size` is the inner window size in points (default 624x510),
//! `--ppp` overrides the pixel density. `--screenshot` saves the window to a PNG after a few
//! frames and quits.

use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui;
use rb_engine::tables::Tables;
use rb_ui::app::{App, Shot};
use rb_ui::layout::{PANEL_H, PANEL_W};
use rb_ui::link::Link;
use rb_ui::model::Panel;
use rb_ui::session;

fn usage() -> ! {
    eprintln!(
        "usage: rb-ui [SONG.rbs] [--standard-mod FILE] [--demo] [--size WxH] [--ppp S] \
         [--screenshot OUT.png]"
    );
    std::process::exit(2);
}

fn main() -> eframe::Result {
    let mut demo = false;
    let mut size = [PANEL_W, PANEL_H];
    let mut ppp: Option<f32> = None;
    let mut shot: Option<PathBuf> = None;
    let mut song: Option<PathBuf> = None;
    let mut standard: Option<PathBuf> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--demo" => demo = true,
            "--standard-mod" => standard = Some(args.next().unwrap_or_else(|| usage()).into()),
            "--size" => {
                let v = args.next().unwrap_or_else(|| usage());
                let (w, h) = v.split_once('x').unwrap_or_else(|| usage());
                size = [
                    w.parse().unwrap_or_else(|_| usage()),
                    h.parse().unwrap_or_else(|_| usage()),
                ];
            }
            "--ppp" => {
                ppp = Some(
                    args.next()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or_else(|| usage()),
                )
            }
            "--screenshot" => shot = Some(args.next().unwrap_or_else(|| usage()).into()),
            s if !s.starts_with("--") && song.is_none() => song = Some(s.into()),
            _ => usage(),
        }
    }

    let mut panel = if demo { Panel::demo() } else { Panel::new() };
    let mut link = None;
    // keeps the audio stream alive for the whole run
    let mut _stream = None;
    if !demo {
        let samples = match session::load_standard(standard.as_deref()) {
            Ok(s) => Some(Arc::new(s)),
            Err(e) => {
                eprintln!("rb-ui: no drum samples ({e}); the drum machines are silent");
                None
            }
        };
        let (mut l, audio) = Link::new(Arc::new(Tables::new()), samples.clone());
        if let Some(path) = &song {
            match session::load_song(path) {
                Ok(s) => l.load(&s, samples, &mut panel),
                Err(e) => {
                    eprintln!("rb-ui: {e}");
                    std::process::exit(1);
                }
            }
        }
        match rb_player::output::start(audio) {
            Ok(s) => _stream = Some(s),
            Err(e) => eprintln!("rb-ui: audio output: {e}; the panel runs without sound"),
        }
        link = Some(l);
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("rb")
            .with_inner_size(size)
            // The minimum is half the native panel size (decision task-040).
            .with_min_inner_size([PANEL_W / 2.0, PANEL_H / 2.0]),
        ..Default::default()
    };
    eframe::run_native(
        "rb-ui",
        options,
        Box::new(move |cc| {
            if let Some(ppp) = ppp {
                cc.egui_ctx.set_pixels_per_point(ppp);
            }
            let shot = shot.map(Shot::new);
            Ok(Box::new(match link {
                Some(l) => App::with_link(panel, l, shot),
                None => App::new(panel, shot),
            }))
        }),
    )
}
