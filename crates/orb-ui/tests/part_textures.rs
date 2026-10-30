//! The panel's part textures (`PartTextures`, task-153): a frame never draws a texture egui has
//! freed, also after the window changes size and comes back, and the textures of a size left
//! behind are dropped after `PART_TEXTURE_FRAMES` frames.

use std::collections::HashSet;

use eframe::egui::{self, Pos2, RawInput, Rect, TextureId, Vec2, epaint};
use orb_ui::app::{App, PART_TEXTURE_FRAMES, PartTextures};
use orb_ui::model::Panel;

/// One frame in a window of `size`: the meshes and those that use a freed texture. `live` is
/// the set of textures egui has allocated and not freed, kept up to date.
fn frame(
    ctx: &egui::Context,
    app: &mut App,
    size: Vec2,
    t: f64,
    live: &mut HashSet<TextureId>,
) -> (usize, usize) {
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
        time: Some(t),
        max_texture_side: Some(8192),
        ..Default::default()
    };
    let mut out = ctx.run_ui(input, |ui| app.show(ui));
    // what the renderer has while it draws this frame: the uploads are applied before the
    // drawing, the frees after it
    live.extend(out.textures_delta.set.keys().copied());
    let prims = ctx.tessellate(std::mem::take(&mut out.shapes), out.pixels_per_point);
    let (mut meshes, mut dead) = (0, 0);
    for p in &prims {
        if let epaint::Primitive::Mesh(m) = &p.primitive {
            meshes += 1;
            if !live.contains(&m.texture_id) {
                dead += 1;
            }
        }
    }
    for id in &out.textures_delta.free {
        live.remove(id);
    }
    out.textures_delta.clear();
    (meshes, dead)
}

#[test]
fn a_resize_and_back_draws_no_freed_texture() {
    let ctx = egui::Context::default();
    egui_extras::install_image_loaders(&ctx);
    let mut app = App::new(Panel::demo(), None);
    let a = Vec2::new(624.0, 510.0);
    let b = Vec2::new(700.0, 600.0);
    let mut live = HashSet::new();
    let mut t = 0.0;
    for (size, n) in [(a, 3), (b, 4), (a, 6), (b, 2), (a, 3)] {
        for _ in 0..n {
            let (meshes, dead) = frame(&ctx, &mut app, size, t, &mut live);
            assert!(meshes > 0);
            assert_eq!(dead, 0, "{size:?} at {t:.2}: {dead} of {meshes} meshes");
            t += 0.05;
        }
    }
}

#[test]
fn the_textures_of_a_size_left_behind_are_dropped() {
    let ctx = egui::Context::default();
    egui_extras::install_image_loaders(&ctx);
    let mut app = App::new(Panel::demo(), None);
    let a = Vec2::new(624.0, 510.0);
    let b = Vec2::new(700.0, 600.0);
    let mut live = HashSet::new();
    let mut t = 0.0;
    let mut run = |size, n: u64, app: &mut App| {
        for _ in 0..n {
            let (_, dead) = frame(&ctx, app, size, t, &mut live);
            assert_eq!(dead, 0);
            t += 0.05;
        }
    };
    run(a, 3, &mut app);
    let one_size = PartTextures::count(&ctx);
    run(b, 3, &mut app);
    assert!(
        PartTextures::count(&ctx) > one_size,
        "the second size adds textures"
    );
    run(a, 2 * PART_TEXTURE_FRAMES + 2, &mut app);
    assert_eq!(
        PartTextures::count(&ctx),
        one_size,
        "the second size's are gone"
    );
}
