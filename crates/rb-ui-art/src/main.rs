//! `ui-art gen [DIR]` writes the art set (default `assets/ui`);
//! `ui-art check [DIR]` renders the committed set at 1x, 2x and 3x with resvg
//! and writes the panel and a sheet of all parts as PNGs to `DIR/preview`;
//! `ui-art iconset DIR` renders the app icon at the sizes of a macOS `.iconset` (task-110).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use rb_ui_art::{
    SCALES, check_parts, contact_sheet, generate, iconset_files, parts, render, render_size,
};

fn default_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/ui")
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let cmd = args.next().unwrap_or_default();
    let explicit_dir = args.next().map(PathBuf::from);
    let dir = explicit_dir.clone().unwrap_or_else(default_dir);
    match cmd.as_str() {
        "gen" => {
            fs::create_dir_all(&dir).unwrap();
            let files = generate();
            // Remove parts that no longer exist.
            for e in fs::read_dir(&dir).unwrap().flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                if name.ends_with(".svg") && !files.iter().any(|(f, _)| *f == name) {
                    fs::remove_file(e.path()).unwrap();
                    println!("removed {name}");
                }
            }
            for (name, content) in &files {
                fs::write(dir.join(name), content).unwrap();
            }
            println!("wrote {} files to {}", files.len(), dir.display());
            ExitCode::SUCCESS
        }
        "check" => {
            let all = parts::all_parts();
            let problems = check_parts(&all);
            for p in &problems {
                eprintln!("{p}");
            }
            let preview = dir.join("preview");
            fs::create_dir_all(&preview).unwrap();
            let panel = fs::read_to_string(dir.join("panel.svg")).expect("panel.svg");
            let sheet = contact_sheet(&all);
            for scale in SCALES {
                for (name, svg) in [("panel", &panel), ("parts", &sheet)] {
                    let px = render(svg, scale).expect("render");
                    let path = preview.join(format!("{name}_{scale}x.png"));
                    px.save_png(&path).unwrap();
                    println!("{} ({}x{})", path.display(), px.width(), px.height());
                }
            }
            println!(
                "{} parts x {} scales checked, {} problems",
                all.len(),
                SCALES.len(),
                problems.len()
            );
            if problems.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        "iconset" => {
            // The macOS icon (task-110): `iconutil -c icns DIR` turns this folder into an .icns.
            // DIR is required: the default folder is assets/ui, which must not get PNGs.
            let Some(dir) = explicit_dir else {
                eprintln!("usage: ui-art iconset DIR");
                return ExitCode::from(2);
            };
            let svg = fs::read_to_string(default_dir().join("app_icon.svg")).expect("app_icon.svg");
            fs::create_dir_all(&dir).unwrap();
            for (name, size) in iconset_files() {
                let px = render_size(&svg, size).expect("render");
                let path = dir.join(name);
                px.save_png(&path).unwrap();
                println!("{} ({}x{})", path.display(), px.width(), px.height());
            }
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("usage: ui-art gen|check [DIR] | iconset DIR");
            ExitCode::from(2)
        }
    }
}
