//! No texture the panel asks egui to upload may exceed the GPU's maximum texture side
//! (task-119): wgpu refuses a larger one and the app crashes.

use eframe::egui::{self, Pos2, RawInput, Rect, Vec2};
use orb_ui::app::App;
use orb_ui::layout::{PANEL_H, PANEL_W};
use orb_ui::menu::MENU_H;
use orb_ui::model::Panel;

/// wgpu's default `max_texture_dimension_2d` (the lowest limit a GPU reports).
const MAX_SIDE: usize = 8192;

/// The sizes of every image the panel uploads, drawing frames in a window of `size` points at
/// `ppp` pixels per point.
fn uploads(size: Vec2, ppp: f32) -> Vec<(String, [usize; 2])> {
    let ctx = egui::Context::default();
    egui_extras::install_image_loaders(&ctx);
    ctx.set_pixels_per_point(ppp);
    let mut app = App::new(Panel::demo(), None);
    app.menu = true;
    let mut sizes = vec![];
    for i in 0..6 {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
            time: Some(i as f64 / 60.0),
            // what eframe reports from the GPU's limits
            max_texture_side: Some(MAX_SIDE),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| app.show(ui));
        for (id, deltas) in &out.textures_delta.set {
            for d in deltas {
                // a partial update (`pos` set) writes inside an existing texture
                if d.pos.is_none() {
                    sizes.push((format!("{id:?}"), d.image.size()));
                }
            }
        }
        // epaint panics in debug when a TexturesDelta is dropped with deltas unapplied
        out.textures_delta.clear();
    }
    sizes
}

fn check(size: Vec2, ppp: f32) {
    let sizes = uploads(size, ppp);
    assert!(!sizes.is_empty(), "nothing was uploaded");
    for (id, s) in &sizes {
        assert!(
            s[0] <= MAX_SIDE && s[1] <= MAX_SIDE,
            "{id} is {}x{} at ppp {ppp}, window {size:?}",
            s[0],
            s[1]
        );
    }
}

#[test]
fn ppp_3_at_the_native_size() {
    check(Vec2::new(PANEL_W, PANEL_H + MENU_H), 3.0);
}

#[test]
fn big_window_at_high_density() {
    check(Vec2::new(3000.0, 2000.0), 3.0);
}
