//! `rb-ui`: the panel window. Controls are not yet wired to the audio engine (task-045).
//!
//! ```text
//! rb-ui [--demo] [--size WxH] [--ppp S] [--screenshot OUT.png]
//! ```
//! `--demo` starts in a lively state (song mode, playing, meters). `--size` is the inner
//! window size in points (default 624x510), `--ppp` overrides the pixel density.
//! `--screenshot` saves the window to a PNG after a few frames and quits.

use std::path::PathBuf;

use eframe::egui;
use rb_ui::app::{App, Shot};
use rb_ui::layout::{PANEL_H, PANEL_W};
use rb_ui::model::Panel;

fn usage() -> ! {
    eprintln!("usage: rb-ui [--demo] [--size WxH] [--ppp S] [--screenshot OUT.png]");
    std::process::exit(2);
}

fn main() -> eframe::Result {
    let mut demo = false;
    let mut size = [PANEL_W, PANEL_H];
    let mut ppp: Option<f32> = None;
    let mut shot: Option<PathBuf> = None;
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
            }
            "--ppp" => {
                ppp = Some(
                    args.next()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or_else(|| usage()),
                )
            }
            "--screenshot" => shot = Some(args.next().unwrap_or_else(|| usage()).into()),
            _ => usage(),
        }
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
            let panel = if demo { Panel::demo() } else { Panel::new() };
            Ok(Box::new(App::new(panel, shot.map(Shot::new))))
        }),
    )
}
