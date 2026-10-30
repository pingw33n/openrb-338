//! What one frame of the panel costs (task-153): the shapes `App::show` emits, the meshes and
//! vertices egui tessellates them into, the textures they bind, and the CPU time of the two, in
//! a headless context (no GPU). `cargo test --release -p orb-ui --test frame_cost -- --nocapture`
//! prints the numbers (a debug run draws 10 frames and checks only the counts' upper bounds).

use std::collections::HashSet;
use std::time::Instant;

use eframe::egui::{self, Pos2, RawInput, Rect, Vec2, epaint};
use orb_ui::app::App;
use orb_ui::layout::{PANEL_H, PANEL_W};
use orb_ui::menu::MENU_H;
use orb_ui::model::Panel;

/// The cost of a frame, drawn in a window of `size` points at `ppp` pixels per point.
struct Cost {
    shapes: usize,
    meshes: usize,
    vertices: usize,
    textures: usize,
    show_ms: f64,
    tess_ms: f64,
}

fn count_shapes(s: &epaint::Shape) -> usize {
    match s {
        epaint::Shape::Vec(v) => v.iter().map(count_shapes).sum(),
        _ => 1,
    }
}

fn cost(size: Vec2, ppp: f32) -> Cost {
    let ctx = egui::Context::default();
    egui_extras::install_image_loaders(&ctx);
    ctx.set_pixels_per_point(ppp);
    let mut app = App::new(Panel::demo(), None);
    app.menu = true;
    const WARM: usize = 8;
    // the timing needs many frames; a debug run only checks the counts
    const RUNS: usize = if cfg!(debug_assertions) { 2 } else { 200 };
    let mut c = Cost {
        shapes: 0,
        meshes: 0,
        vertices: 0,
        textures: 0,
        show_ms: 0.0,
        tess_ms: 0.0,
    };
    for i in 0..WARM + RUNS {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
            time: Some(i as f64 / 60.0),
            max_texture_side: Some(8192),
            ..Default::default()
        };
        let t0 = Instant::now();
        let mut out = ctx.run_ui(input, |ui| app.show(ui));
        let t1 = Instant::now();
        out.textures_delta.clear();
        let shapes: usize = out.shapes.iter().map(|s| count_shapes(&s.shape)).sum();
        let prims = ctx.tessellate(out.shapes, out.pixels_per_point);
        let t2 = Instant::now();
        if i < WARM {
            continue;
        }
        c.show_ms += (t1 - t0).as_secs_f64() * 1000.0 / RUNS as f64;
        c.tess_ms += (t2 - t1).as_secs_f64() * 1000.0 / RUNS as f64;
        c.shapes = shapes;
        let mut tex = HashSet::new();
        c.meshes = 0;
        c.vertices = 0;
        for p in &prims {
            if let epaint::Primitive::Mesh(m) = &p.primitive {
                c.meshes += 1;
                c.vertices += m.vertices.len();
                tex.insert(m.texture_id);
            }
        }
        c.textures = tex.len();
    }
    c
}

fn report(what: &str, size: Vec2, ppp: f32) -> Cost {
    let c = cost(size, ppp);
    println!(
        "{what}: {} shapes, {} meshes, {} vertices, {} textures; \
         show {:.3} ms, tessellate {:.3} ms",
        c.shapes, c.meshes, c.vertices, c.textures, c.show_ms, c.tess_ms
    );
    c
}

#[test]
fn a_frame_of_the_panel() {
    let native = report("native 1x", Vec2::new(PANEL_W, PANEL_H + MENU_H), 1.0);
    let big = report("1800x1100 at 2x", Vec2::new(1800.0, 1100.0), 2.0);
    // the demo panel drew 559 shapes, 524 and 534 meshes, 14112 and 16412 vertices
    // (2026-10-30): a change that makes a frame much bigger fails here
    for (c, max_vertices) in [(&native, 17_000), (&big, 20_000)] {
        assert!(c.shapes > 0 && c.meshes > 0 && c.vertices > 0);
        assert!(c.shapes <= 680, "{} shapes", c.shapes);
        assert!(c.meshes <= 650, "{} meshes", c.meshes);
        assert!(c.vertices <= max_vertices, "{} vertices", c.vertices);
    }
}
