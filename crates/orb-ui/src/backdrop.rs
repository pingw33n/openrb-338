//! The desk around the unit (task-179): in a window whose shape is not the panel's, the free
//! space left and right (or above and below) of the panel is a dark matte desk with the
//! unit's soft cast shadow on it, in place of a flat letterbox color.
//!
//! Nothing of it is rasterized per window size, so a resize drag costs no SVG work: the
//! desk's vertical gradient, the darkening toward the window's edges and the shadows are
//! meshes with colored vertices, and the speckle and the brushing are one tile of 256 x 256
//! points (`desk_tile`, rasterized once, at one pixel per point) repeated over the window. The
//! numbers are the approved mockup's (`backdrop()` of mockup/179/mock.py, removed with
//! task-179): the gradient `#1d2023` to `#131517`, the vignette's radial gradient (clear to
//! 62 % of the radius, then to 50 % black), and the shadows: a band of 12 panel pixels
//! from 5 left of the unit at 55 % and one of 20 from 8 inside its right edge at 85 %, both
//! blurred with a deviation of 5; the same above and below in a tall window.

use eframe::egui::{Color32, Mesh, Pos2, Rect, Shape, TextureOptions, Ui, Vec2};

use crate::app::{View, part_texture};
use crate::art;
use crate::layout::{PANEL_H, PANEL_W};

/// The desk's gradient, top and bottom.
const DESK_TOP: Color32 = Color32::from_rgb(0x1d, 0x20, 0x23);
const DESK_BOTTOM: Color32 = Color32::from_rgb(0x13, 0x15, 0x17);
/// The panel's dark seams, painted under the panel so no desk shows between its parts.
pub const SEAM: Color32 = Color32::from_rgb(0x14, 0x18, 0x1b);
/// The side of the desk's tile in points (`DESK_TILE` of orb-ui-art).
pub const TILE: f32 = 256.0;

/// Paints the desk over `avail` (the window below the menu bar) and the panel's seam color
/// under the panel of `view`.
pub fn paint(ui: &Ui, avail: Rect, view: &View) {
    let painter = ui.painter();
    let mut m = Mesh::default();
    for (p, c) in [
        (avail.left_top(), DESK_TOP),
        (avail.right_top(), DESK_TOP),
        (avail.right_bottom(), DESK_BOTTOM),
        (avail.left_bottom(), DESK_BOTTOM),
    ] {
        m.colored_vertex(p, c);
    }
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    painter.add(Shape::mesh(m));
    let unit = view.rect(0.0, 0.0, PANEL_W, PANEL_H);
    let wide = unit.width() < avail.width() - 0.5;
    let tall = unit.height() < avail.height() - 0.5;
    // The tile only where the desk shows, and at one pixel per point whatever the density: its
    // noise is the start's most expensive raster (about 200 ms at 2x on an M-series Mac), and
    // a soft speckle at a tenth of opacity on a dark desk does not need more.
    if let Some(part) = art::part("desk_tile").filter(|_| wide || tall) {
        let px = TILE;
        let tex = part_texture(
            ui.ctx(),
            part.uri,
            part.svg,
            Vec2::splat(px),
            Vec2::splat(TILE),
            TextureOptions::LINEAR_REPEAT,
        );
        if let Some(tex) = tex {
            let mut m = Mesh::with_texture(tex.id);
            let uv = Rect::from_min_max(Pos2::ZERO, (avail.size() / TILE).to_pos2());
            m.add_rect_with_uv(avail, uv, Color32::WHITE);
            painter.add(Shape::mesh(m));
        }
    }
    painter.add(Shape::mesh(vignette(avail)));
    let mut shadow = Mesh::default();
    let s = view.scale;
    if wide {
        blurred_rect(
            &mut shadow,
            view.rect(-5.0, 0.0, 12.0, PANEL_H),
            5.0 * s,
            0.55,
        );
        blurred_rect(
            &mut shadow,
            view.rect(616.0, 0.0, 20.0, PANEL_H),
            5.0 * s,
            0.85,
        );
    }
    if tall {
        blurred_rect(
            &mut shadow,
            view.rect(0.0, -5.0, PANEL_W, 12.0),
            5.0 * s,
            0.55,
        );
        blurred_rect(
            &mut shadow,
            view.rect(0.0, 502.0, PANEL_W, 20.0),
            5.0 * s,
            0.85,
        );
    }
    if !shadow.is_empty() {
        painter.add(Shape::mesh(shadow));
    }
    painter.rect_filled(unit, 0.0, SEAM);
}

/// The window's edges darkened: the mockup's radial gradient over the whole area (an ellipse
/// of 0.75 of the area's width and height), as a grid of colored vertices.
fn vignette(r: Rect) -> Mesh {
    const N: u32 = 16;
    let mut m = Mesh::default();
    for j in 0..=N {
        for i in 0..=N {
            let (u, v) = (i as f32 / N as f32, j as f32 / N as f32);
            let t = ((u - 0.5).powi(2) + (v - 0.5).powi(2)).sqrt() / 0.75;
            let a = if t < 0.62 {
                0.0
            } else {
                0.5 * ((t - 0.62) / 0.38).min(1.0)
            };
            let p = r.min + Vec2::new(u * r.width(), v * r.height());
            m.colored_vertex(p, Color32::from_black_alpha((a * 255.0).round() as u8));
        }
    }
    grid(&mut m, 0, N + 1, N + 1);
    m
}

/// The triangles of a grid of `cols` x `rows` vertices added from `base` on, row by row.
fn grid(m: &mut Mesh, base: u32, cols: u32, rows: u32) {
    for j in 0..rows - 1 {
        for i in 0..cols - 1 {
            let a = base + j * cols + i;
            m.add_triangle(a, a + 1, a + cols + 1);
            m.add_triangle(a, a + cols + 1, a + cols);
        }
    }
}

/// The error function (Abramowitz and Stegun 7.1.26, error below 1.5e-7).
fn erf(x: f32) -> f32 {
    let t = 1.0 / (1.0 + 0.327_591_1 * x.abs());
    let y = 1.0
        - (((((1.061_405_4 * t - 1.453_152_1) * t) + 1.421_413_8) * t - 0.284_496_74) * t
            + 0.254_829_6)
            * t
            * (-x * x).exp();
    y.copysign(x)
}

/// How much of a band from `a` to `b` a Gaussian blur of deviation `sigma` leaves at `x`.
fn band(x: f32, a: f32, b: f32, sigma: f32) -> f32 {
    let k = std::f32::consts::SQRT_2 * sigma;
    0.5 * (erf((x - a) / k) - erf((x - b) / k))
}

/// The positions a blurred edge from `a` to `b` is sampled at: across each edge, 3 deviations
/// to either side.
fn samples(a: f32, b: f32, sigma: f32) -> Vec<f32> {
    let mut v = Vec::new();
    for e in [a, b] {
        for k in 0..=8 {
            v.push(e - 3.0 * sigma + 6.0 * sigma * k as f32 / 8.0);
        }
    }
    v.sort_by(f32::total_cmp);
    v.dedup_by(|x, y| (*x - *y).abs() < 0.01);
    v
}

/// A black rectangle `r` blurred with a Gaussian of deviation `sigma` at `opacity`, as a grid
/// of colored vertices (the mockup's `feGaussianBlur` shadow).
fn blurred_rect(m: &mut Mesh, r: Rect, sigma: f32, opacity: f32) {
    if sigma <= 0.0 {
        return;
    }
    let xs = samples(r.min.x, r.max.x, sigma);
    let ys = samples(r.min.y, r.max.y, sigma);
    let base = m.vertices.len() as u32;
    for &y in &ys {
        let fy = band(y, r.min.y, r.max.y, sigma);
        for &x in &xs {
            let a = opacity * fy * band(x, r.min.x, r.max.x, sigma);
            m.colored_vertex(
                Pos2::new(x, y),
                Color32::from_black_alpha((a * 255.0).round().clamp(0.0, 255.0) as u8),
            );
        }
    }
    grid(m, base, xs.len() as u32, ys.len() as u32);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blurred_band() {
        // the middle of a wide band is full, its edges half, far outside nothing
        assert!((band(50.0, 0.0, 100.0, 5.0) - 1.0).abs() < 1e-4);
        assert!((band(0.0, 0.0, 100.0, 5.0) - 0.5).abs() < 1e-4);
        assert!(band(-20.0, 0.0, 100.0, 5.0) < 1e-4);
        assert!((erf(1.0) - 0.842_700_8).abs() < 1e-5);
    }

    #[test]
    fn the_tile_is_the_art_generators() {
        assert_eq!(TILE, orb_ui_art::parts::DESK_TILE);
        let p = art::part("desk_tile").unwrap();
        assert_eq!((p.w, p.h, p.bleed), (TILE, TILE, 0.0));
    }
}
