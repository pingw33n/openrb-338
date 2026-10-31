//! task-167: the Mods menu's top-level entries are ordered by their INFO name, ASCII case
//! folded, lowercase first on a tie (`menu::mod_name_order`), not by file name.

use orb_ui::app::App;
use orb_ui::model::Panel;

/// A mod with no files whose INFO menu name is `name` (the INFO text is Latin-1 at +1003).
fn empty_mod(name: &str) -> Vec<u8> {
    let mut b = b"CAT \0\0\0\0PRBM".to_vec();
    b.extend(b"INFO");
    b.extend(1280u32.to_be_bytes());
    let mut info = vec![0u8; 1280];
    info[1003..1003 + name.len()].copy_from_slice(name.as_bytes());
    b.extend(info);
    let size = (b.len() - 8) as u32;
    b[4..8].copy_from_slice(&size.to_be_bytes());
    b
}

#[test]
fn list_mods_orders_the_entries_by_folded_name_lowercase_first() {
    let dir = tempfile::tempdir().unwrap();
    // the file names are in an order of their own (byte order: a, b, c, ...), the INFO names
    // are what the menu shows
    let files = [
        ("a.rbm", "beta"),
        ("b.rbm", "Alpha"),
        ("c.rbm", "alpha2"),
        ("d.rbm", "ALPHA2x"),
        ("e.rbm", "Zed"),
        ("f.rbm", "_x"),
        ("g.rbm", "Ab"),
        ("h.rbm", "ac"),
        // a shared name (ASCII case ignored): one entry, owned by the first file in byte order,
        // with its variants in the same order
        ("i.rbm", "Shared"),
        ("j.rbm", "SHARED"),
    ];
    for (f, n) in files {
        std::fs::write(dir.path().join(f), empty_mod(n)).unwrap();
    }
    let mut app = App::new(Panel::new(), None);
    app.settings.mods_dir = Some(dir.path().to_path_buf());
    let list = app.list_mods().unwrap();
    let names: Vec<&str> = list.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "_x", "Ab", "ac", "Alpha", "alpha2", "ALPHA2x", "beta", "Shared", "Zed"
        ]
    );
    let shared = list.iter().find(|e| e.name == "Shared").unwrap();
    assert_eq!(shared.path, dir.path().join("i.rbm"));
    let variants: Vec<_> = shared.variants.iter().map(|v| v.path.clone()).collect();
    assert_eq!(
        variants,
        [dir.path().join("i.rbm"), dir.path().join("j.rbm")]
    );
}
