//! `ui-art gen [DIR]` writes the art set (default `assets/ui`);
//! `ui-art check [DIR]` renders the committed set at 1x, 2x and 3x with resvg
//! and writes the panel and a sheet of all parts as PNGs to `DIR/preview`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use rb_ui_art::{SCALES, check_parts, contact_sheet, generate, parts, render};

fn default_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/ui")
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let cmd = args.next().unwrap_or_default();
    let dir = args.next().map(PathBuf::from).unwrap_or_else(default_dir);
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
        _ => {
            eprintln!("usage: ui-art gen|check [DIR]");
            ExitCode::from(2)
        }
    }
}
