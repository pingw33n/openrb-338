use std::fs;
use std::path::{Path, PathBuf};

use rb_ui_art::layout::{PANEL_H, PANEL_W, layout};
use rb_ui_art::parts::all_parts;
use rb_ui_art::{check_parts, generate};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The committed assets/ui equals what the generator writes (run
/// `cargo run -p rb-ui-art -- gen` after changing the art).
#[test]
fn committed_assets_match_the_generator() {
    let dir = root().join("assets/ui");
    let files = generate();
    for (name, content) in &files {
        let have = fs::read_to_string(dir.join(name)).unwrap_or_default();
        assert!(
            have == *content,
            "{name} is stale: run `cargo run -p rb-ui-art -- gen`"
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
