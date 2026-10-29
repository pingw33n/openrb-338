use std::fs;
use std::path::{Path, PathBuf};

use orb_ui_art::layout::{PANEL_H, PANEL_W, layout};
use orb_ui_art::parts::all_parts;
use orb_ui_art::{check_parts, generate};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The committed assets/ui equals what the generator writes (run
/// `cargo run -p orb-ui-art -- gen` after changing the art).
#[test]
fn committed_assets_match_the_generator() {
    let dir = root().join("assets/ui");
    let files = generate();
    for (name, content) in &files {
        let have = fs::read_to_string(dir.join(name)).unwrap_or_default();
        assert!(
            have == *content,
            "{name} is stale: run `cargo run -p orb-ui-art -- gen`"
        );
    }
    for e in fs::read_dir(&dir).unwrap().flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if name.ends_with(".svg") || name == "layout.tsv" {
            assert!(
                files.iter().any(|(f, _)| *f == name),
                "{name} is not generated"
            );
        }
    }
}

/// Every part parses and renders at 1x, 2x and 3x at its exact size, is not
/// empty, and uses no `<text>` (labels are paths, so no font is needed).
#[test]
fn every_part_renders_at_1x_2x_3x() {
    let parts = all_parts();
    let problems = check_parts(&parts);
    assert!(problems.is_empty(), "{problems:#?}");
    for p in &parts {
        assert!(!p.to_svg().contains("<text"), "{} has text", p.name);
    }
    let mut names: Vec<&str> = parts.iter().map(|p| p.name.as_str()).collect();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), parts.len(), "duplicate part names");
}

/// Controls lie inside the panel.
#[test]
fn controls_are_inside_the_panel() {
    let l = layout(&all_parts());
    for c in &l.ctls {
        assert!(c.x >= 0.0 && c.y >= 0.0, "{}", c.id);
        assert!(c.x + c.w <= PANEL_W && c.y + c.h <= PANEL_H, "{}", c.id);
    }
}

/// The macOS iconset (task-110): the ten files `iconutil` expects, at their pixel sizes, and
/// `render_size` draws the app icon at exactly that size with visible pixels.
#[test]
fn iconset_has_the_ten_sizes() {
    let files = orb_ui_art::iconset_files();
    let expect = [
        ("icon_16x16.png", 16),
        ("icon_16x16@2x.png", 32),
        ("icon_32x32.png", 32),
        ("icon_32x32@2x.png", 64),
        ("icon_128x128.png", 128),
        ("icon_128x128@2x.png", 256),
        ("icon_256x256.png", 256),
        ("icon_256x256@2x.png", 512),
        ("icon_512x512.png", 512),
        ("icon_512x512@2x.png", 1024),
    ];
    assert_eq!(files.len(), 10);
    for (f, (name, size)) in files.iter().zip(expect) {
        assert_eq!((f.0.as_str(), f.1), (name, size));
    }
    let svg = fs::read_to_string(root().join("assets/ui/app_icon.svg")).unwrap();
    for (name, size) in files {
        let px = orb_ui_art::render_size(&svg, size).unwrap();
        assert_eq!((px.width(), px.height()), (size, size), "{name}");
        assert!(px.pixels().iter().any(|p| p.alpha() > 0), "{name} is empty");
    }
}

/// Reads the directory of an `.ico` file: (width, height, PNG bytes) per image; a width of 0
/// means 256.
fn ico_images(bytes: &[u8]) -> Vec<(u32, u32, Vec<u8>)> {
    assert_eq!(&bytes[..4], &[0, 0, 1, 0], "an icon file");
    let n = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
    (0..n)
        .map(|i| {
            let e = &bytes[6 + 16 * i..6 + 16 * (i + 1)];
            let dim = |b: u8| if b == 0 { 256 } else { b as u32 };
            let len = u32::from_le_bytes(e[8..12].try_into().unwrap()) as usize;
            let off = u32::from_le_bytes(e[12..16].try_into().unwrap()) as usize;
            (dim(e[0]), dim(e[1]), bytes[off..off + len].to_vec())
        })
        .collect()
}

/// The Windows icon (task-111): `ico` has one PNG per size, and the committed
/// `assets/app_icon.ico` (made by `ui-art ico`) has the same sizes. The bytes are not compared:
/// the pixels may differ in the last bit between CPUs.
#[test]
fn ico_has_the_sizes_and_the_committed_file_matches() {
    let svg = fs::read_to_string(root().join("assets/ui/app_icon.svg")).unwrap();
    let made = orb_ui_art::ico(&svg).unwrap();
    let committed = fs::read(root().join("assets/app_icon.ico")).unwrap();
    for bytes in [&made, &committed] {
        let images = ico_images(bytes);
        let sizes: Vec<u32> = images.iter().map(|i| i.0).collect();
        assert_eq!(sizes, orb_ui_art::ICO_SIZES);
        for (w, h, png) in images {
            assert_eq!(w, h);
            assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
            // the PNG header's width and height agree with the directory entry
            let pw = u32::from_be_bytes(png[16..20].try_into().unwrap());
            let ph = u32::from_be_bytes(png[20..24].try_into().unwrap());
            assert_eq!((pw, ph), (w, h));
        }
    }
}
