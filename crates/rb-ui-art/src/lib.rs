//! The panel's SVG art set. `generate` builds every file of `assets/ui`; the
//! render functions draw them with resvg, the renderer egui_extras uses, so
//! what the check shows is what the app will show.

pub mod font;
pub mod layout;
pub mod parts;
pub mod svg;

use resvg::{tiny_skia, usvg};

use font::{Anchor, text};
use svg::{Part, Svg, n};

/// The scales the art is checked at.
pub const SCALES: [u32; 3] = [1, 2, 3];

/// All files of `assets/ui` except the README: (file name, content).
pub fn generate() -> Vec<(String, String)> {
    let parts = parts::all_parts();
    let l = layout::layout(&parts);
    let mut files: Vec<(String, String)> = parts
        .iter()
        .map(|p| (format!("{}.svg", p.name), p.to_svg()))
        .collect();
    files.push(("panel.svg".into(), layout::compose(&parts, &l)));
    files.push(("layout.tsv".into(), layout::tsv(&l)));
    files
}

/// Renders an SVG document at `scale` times its native size.
pub fn render(svg: &str, scale: u32) -> Result<tiny_skia::Pixmap, String> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).map_err(|e| e.to_string())?;
    let size = tree.size();
    let w = (size.width() * scale as f32).ceil() as u32;
    let h = (size.height() * scale as f32).ceil() as u32;
    let mut pixmap = tiny_skia::Pixmap::new(w, h).ok_or("empty size")?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale as f32, scale as f32),
        &mut pixmap.as_mut(),
    );
    Ok(pixmap)
}

/// Renders a square SVG document scaled to `size` x `size` pixels.
pub fn render_size(svg: &str, size: u32) -> Result<tiny_skia::Pixmap, String> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).map_err(|e| e.to_string())?;
    let mut pixmap = tiny_skia::Pixmap::new(size, size).ok_or("empty size")?;
    let scale = size as f32 / tree.size().width();
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    Ok(pixmap)
}

/// The files of a macOS `.iconset` (task-110): name and pixel size, 16 to 1024.
pub fn iconset_files() -> Vec<(String, u32)> {
    let mut v = Vec::new();
    for base in [16, 32, 128, 256, 512] {
        v.push((format!("icon_{base}x{base}.png"), base));
        v.push((format!("icon_{base}x{base}@2x.png"), base * 2));
    }
    v
}

/// The number of pixels that are not fully transparent.
fn visible(pixmap: &tiny_skia::Pixmap) -> usize {
    pixmap.pixels().iter().filter(|p| p.alpha() > 0).count()
}

/// Renders every part at 1x, 2x and 3x and returns the problems found: a
/// parse error, a wrong pixel size, or an empty image.
pub fn check_parts(parts: &[Part]) -> Vec<String> {
    let mut problems = Vec::new();
    for p in parts {
        for scale in SCALES {
            match render(&p.to_svg(), scale) {
                Err(e) => problems.push(format!("{} at {scale}x: {e}", p.name)),
                Ok(px) => {
                    let (w, h) = ((p.w * scale as f32) as u32, (p.h * scale as f32) as u32);
                    if (px.width(), px.height()) != (w, h) {
                        problems.push(format!(
                            "{} at {scale}x: {}x{} pixels, want {w}x{h}",
                            p.name,
                            px.width(),
                            px.height()
                        ));
                    }
                    if visible(&px) == 0 {
                        problems.push(format!("{} at {scale}x: nothing drawn", p.name));
                    }
                }
            }
        }
    }
    problems
}

/// A sheet showing every part with its name, as one SVG document.
pub fn contact_sheet(parts: &[Part]) -> String {
    const W: f32 = 700.0;
    let mut order: Vec<&Part> = parts.iter().collect();
    order.sort_by(|a, b| b.h.total_cmp(&a.h).then(a.name.cmp(&b.name)));
    let mut s = Svg::new();
    let (mut x, mut y, mut row_h) = (4.0_f32, 4.0_f32, 0.0_f32);
    for p in order {
        let cell_w = p.w.max(font::text_width(3.0, &p.name)) + 6.0;
        let cell_h = p.h + 10.0;
        if x + cell_w > W && x > 4.0 {
            x = 4.0;
            y += row_h + 4.0;
            row_h = 0.0;
        }
        s.g_at(x, y);
        s.rect(-1.0, -1.0, p.w + 2.0, p.h + 2.0, 0.0, "#ff00ff30");
        s.rect(0.0, 0.0, p.w, p.h, 0.0, "#79808a");
        s.raw(&p.body);
        text(
            &mut s,
            0.0,
            p.h + 6.0,
            3.0,
            Anchor::Start,
            "#ffffff",
            &p.name,
        );
        s.end_g();
        x += cell_w;
        row_h = row_h.max(cell_h);
    }
    let h = y + row_h + 4.0;
    format!(
        concat!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" ",
            "viewBox=\"0 0 {} {}\"><rect width=\"{}\" height=\"{}\" fill=\"#2b2f36\"/>{}</svg>\n",
        ),
        n(W),
        n(h),
        n(W),
        n(h),
        n(W),
        n(h),
        s.s
    )
}
