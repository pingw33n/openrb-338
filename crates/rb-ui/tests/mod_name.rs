//! The mod name in the 808's mod window is centered in its strip, laid out by egui.

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Vec2};
use rb_ui::app::centered_in;

/// Lays out `text` in a real egui galley (wrapped at the strip's width) and returns the galley's
/// rectangle once placed by `centered_in`, and its number of lines.
fn placed(strip: Rect, text: &str) -> (Rect, usize) {
    let ctx = egui::Context::default();
    let mut out = None;
    let input = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(300.0, 300.0))),
        ..Default::default()
    };
    let mut frame = ctx.run_ui(input, |ui| {
        let galley = ui.painter().layout(
            text.into(),
            FontId::proportional(14.0),
            Color32::WHITE,
            strip.width(),
        );
        let pos = centered_in(strip, galley.size());
        out = Some((Rect::from_min_size(pos, galley.size()), galley.rows.len()));
    });
    // epaint panics in debug when a TexturesDelta is dropped with deltas unapplied
    frame.textures_delta.clear();
    out.expect("the frame ran")
}

fn strip() -> Rect {
    Rect::from_min_max(Pos2::new(10.0, 200.0), Pos2::new(110.0, 260.0))
}

#[test]
fn one_line_is_centered_both_ways() {
    let strip = strip();
    let (r, rows) = placed(strip, "Short");
    assert_eq!(rows, 1);
    assert!(r.height() > 0.0 && r.width() < strip.width());
    assert!((r.center().x - strip.center().x).abs() < 1e-3);
    assert!((r.center().y - strip.center().y).abs() < 1e-3);
    assert!(r.min.y > strip.min.y);
}

#[test]
fn wrapped_name_is_centered_as_a_block() {
    let strip = strip();
    let (one, _) = placed(strip, "Short");
    let (r, rows) = placed(strip, "A very long mod name that wraps");
    assert!(rows >= 2);
    assert!(r.height() > one.height() * 1.5);
    assert!((r.center().y - strip.center().y).abs() < 1e-3);
    assert!((r.center().x - strip.center().x).abs() < 1e-3);
    assert!(r.min.y >= strip.min.y && r.max.y <= strip.max.y);
}
