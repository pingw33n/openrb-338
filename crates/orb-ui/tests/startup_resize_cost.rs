//! What the panel's art costs at the start and during a resize drag (task-179), in a headless
//! context (no GPU, no window): the first frame, which rasterizes every part, and a drag of 40
//! frames that grows the window by 3 x 2 points a frame. Ignored by default (it only measures):
//! `cargo test --release -p orb-ui --test startup_resize_cost -- --ignored --nocapture`.

use std::time::Instant;

use eframe::egui::{self, Pos2, RawInput, Rect, Vec2};
use orb_ui::app::App;
use orb_ui::layout::{PANEL_H, PANEL_W};
use orb_ui::menu::MENU_H;
use orb_ui::model::Panel;

fn frame(ctx: &egui::Context, app: &mut App, size: Vec2, t: f64) -> f64 {
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
        time: Some(t),
        max_texture_side: Some(8192),
        ..Default::default()
    };
    let t0 = Instant::now();
    let mut out = ctx.run_ui(input, |ui| app.show(ui));
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    out.textures_delta.clear();
    ms
}

fn measure(ppp: f32) {
    let ctx = egui::Context::default();
    egui_extras::install_image_loaders(&ctx);
    ctx.set_pixels_per_point(ppp);
    let mut app = App::new(Panel::demo(), None);
    app.menu = true;
    let size = Vec2::new(PANEL_W, PANEL_H + MENU_H);
    let mut t = 0.0;
    // the first complete frame: since task-179 the window's very first frame rasterizes
    // nothing (eframe draws it before the window has its size), so it is the second
    let mut first = frame(&ctx, &mut app, size, t);
    t += 1.0 / 60.0;
    first += frame(&ctx, &mut app, size, t);
    let mut settle = 0.0;
    for _ in 0..3 {
        t += 1.0 / 60.0;
        settle += frame(&ctx, &mut app, size, t);
    }
    let mut drag = vec![];
    for i in 1..=40 {
        t += 1.0 / 60.0;
        let s = size + Vec2::new(3.0, 2.0) * i as f32;
        drag.push(frame(&ctx, &mut app, s, t));
    }
    // the drag ends: the window keeps its last size for a second
    let end = size + Vec2::new(120.0, 80.0);
    let mut after = 0.0;
    for _ in 0..60 {
        t += 1.0 / 60.0;
        after += frame(&ctx, &mut app, end, t);
    }
    let total: f64 = drag.iter().sum();
    let max = drag.iter().cloned().fold(0.0, f64::max);
    println!(
        "ppp {ppp}: first complete frame (2 frames) {first:.1} ms, next 3 frames {settle:.1} \
         ms; resize drag of 40 frames {total:.1} ms (mean {:.1}, max {max:.1}); the second \
         after it {after:.1} ms",
        total / 40.0
    );
}

#[test]
#[ignore = "a measurement: run it with --ignored --nocapture"]
fn startup_and_resize_cost() {
    measure(1.0);
    measure(2.0);
}
