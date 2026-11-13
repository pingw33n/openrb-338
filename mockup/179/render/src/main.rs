//! `render IN.svg OUT.png SCALE FONT_DIR`: renders an SVG at SCALE times its size with resvg,
//! with the fonts of FONT_DIR only (no system fonts, so the result is the same on every machine).
//! Images the SVG refers to are read from the paths it gives.

use std::sync::Arc;

use resvg::{tiny_skia, usvg};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() != 5 {
        eprintln!("usage: render IN.svg OUT.png SCALE FONT_DIR");
        std::process::exit(2);
    }
    let scale: f32 = a[3].parse().expect("SCALE is a number");
    let data = std::fs::read(&a[1]).expect("read the SVG");
    let mut db = usvg::fontdb::Database::new();
    db.load_fonts_dir(&a[4]);
    let opt = usvg::Options {
        resources_dir: std::fs::canonicalize(&a[1])
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf())),
        fontdb: Arc::new(db),
        ..Default::default()
    };
    let tree = usvg::Tree::from_data(&data, &opt).expect("parse the SVG");
    let size = tree.size();
    let w = (size.width() * scale).ceil() as u32;
    let h = (size.height() * scale).ceil() as u32;
    let mut pixmap = tiny_skia::Pixmap::new(w, h).expect("a size above zero");
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap.save_png(&a[2]).expect("write the PNG");
    println!("{} {w}x{h}", a[2]);
}
