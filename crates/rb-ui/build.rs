//! Embeds every SVG part of `assets/ui` (except the composed demo `panel.svg`) in the
//! binary: writes `$OUT_DIR/parts_gen.rs` with `PARTS: &[(name, uri, bytes)]`, sorted by name.

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let dir = manifest.join("../../assets/ui").canonicalize().unwrap();
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut names: Vec<String> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| {
            let name = e.unwrap().file_name().into_string().unwrap();
            let stem = name.strip_suffix(".svg")?;
            (stem != "panel").then(|| stem.to_string())
        })
        .collect();
    names.sort();
    let mut out = String::from("pub static PARTS: &[(&str, &str, &[u8])] = &[\n");
    for n in &names {
        let path = dir.join(format!("{n}.svg"));
        writeln!(
            out,
            "    ({n:?}, \"bytes://rb-ui/{n}.svg\", include_bytes!({:?})),",
            path.display().to_string()
        )
        .unwrap();
    }
    out.push_str("];\n");
    fs::write(
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("parts_gen.rs"),
        out,
    )
    .unwrap();
}
