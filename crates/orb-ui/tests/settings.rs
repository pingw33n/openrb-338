//! The settings file (task-046, task-107): TOML round trip, damaged files, unknown keys,
//! atomic writes and `ORB_UI_CONFIG`.

use std::fs;
use std::path::PathBuf;

use orb_ui::settings::{MAX_RECENT, Settings};

fn p(s: &str) -> PathBuf {
    PathBuf::from(s)
}

#[test]
fn the_settings_round_trip_through_toml() {
    let d = tempfile::tempdir().unwrap();
    let file = d.path().join("OpenRB-338").join("ui.toml");
    let mut s = Settings::load_from(&file);
    assert!(s.startup_sound);
    s.songs_dir = Some(p("/songs"));
    s.mods_dir = Some(p("/mods dir/with \"quotes\" and \\"));
    s.recent = vec![p("/b"), p("/a")];
    s.startup_sound = false;
    s.save().unwrap();
    // It is TOML: it parses and holds the documented keys.
    let text = fs::read_to_string(&file).unwrap();
    let t: toml::Table = text.parse().unwrap();
    assert_eq!(t["songs_dir"].as_str(), Some("/songs"));
    assert_eq!(t["startup_sound"].as_bool(), Some(false));
    assert_eq!(t["recent"].as_array().unwrap().len(), 2);
    let r = Settings::load_from(&file);
    assert_eq!(r, s);
    // Unset paths are absent from the file and come back unset.
    let mut s2 = Settings::load_from(&file);
    s2.songs_dir = None;
    s2.mods_dir = None;
    s2.save().unwrap();
    let t: toml::Table = fs::read_to_string(&file).unwrap().parse().unwrap();
    assert!(!t.contains_key("songs_dir") && !t.contains_key("mods_dir"));
    assert_eq!(Settings::load_from(&file), s2);
    // Saving leaves no temporary file behind.
    assert_eq!(fs::read_dir(file.parent().unwrap()).unwrap().count(), 1);
}

#[test]
fn a_damaged_settings_file_gives_the_defaults_and_stays_until_saved() {
    let d = tempfile::tempdir().unwrap();
    let file = d.path().join("ui.toml");
    let defaults = Settings {
        file: Some(file.clone()),
        ..Settings::default()
    };
    for bad in [
        &[0xff, 0xfe, b'=', 0x80, b'\n', b's'][..],
        b"songs_dir = \"/x\nrecent = [",
        b"[[[",
        b"",
    ] {
        fs::write(&file, bad).unwrap();
        assert_eq!(Settings::load_from(&file), defaults);
        // Loading does not touch the file.
        assert_eq!(fs::read(&file).unwrap(), bad);
    }
    // The next save replaces it.
    let mut s = Settings::load_from(&file);
    s.songs_dir = Some(p("/x"));
    s.save().unwrap();
    assert_eq!(Settings::load_from(&file).songs_dir, Some(p("/x")));
    // A key of the wrong type is skipped, the others are kept.
    fs::write(
        &file,
        "songs_dir = 3\nmods_dir = \"/m\"\nstartup_sound = \"x\"\nrecent = [\"/r\", 4, \"/r\"]\n",
    )
    .unwrap();
    let s = Settings::load_from(&file);
    assert_eq!(s.songs_dir, None);
    assert_eq!(s.mods_dir, Some(p("/m")));
    assert!(s.startup_sound);
    assert_eq!(s.recent, vec![p("/r")]);
    // At most MAX_RECENT recent folders are read.
    let many: Vec<String> = (0..MAX_RECENT + 3).map(|i| format!("\"/r{i}\"")).collect();
    fs::write(&file, format!("recent = [{}]\n", many.join(", "))).unwrap();
    assert_eq!(Settings::load_from(&file).recent.len(), MAX_RECENT);
}

#[cfg(unix)]
#[test]
fn paths_that_are_not_utf8_are_left_out_of_a_save() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    let d = tempfile::tempdir().unwrap();
    let file = d.path().join("ui.toml");
    let bad = PathBuf::from(OsStr::from_bytes(b"/bad/\xff\xfe"));
    let mut s = Settings::load_from(&file);
    s.songs_dir = Some(bad.clone());
    s.mods_dir = Some(p("/mods"));
    s.recent = vec![p("/a"), bad, p("/b")];
    s.save().unwrap();
    let r = Settings::load_from(&file);
    assert_eq!(r.songs_dir, None);
    assert_eq!(r.mods_dir, Some(p("/mods")));
    assert_eq!(r.recent, vec![p("/a"), p("/b")]);
}

#[test]
fn a_failed_rename_leaves_no_temporary_file() {
    let d = tempfile::tempdir().unwrap();
    let file = d.path().join("ui.toml");
    // A directory where the file should go: the rename onto it fails.
    fs::create_dir(&file).unwrap();
    let mut s = Settings::load_from(&file);
    s.songs_dir = Some(p("/x"));
    assert!(s.save().is_err());
    let names: Vec<_> = fs::read_dir(d.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names, vec![std::ffi::OsString::from("ui.toml")]);
}

#[test]
fn unknown_keys_are_ignored() {
    let d = tempfile::tempdir().unwrap();
    let file = d.path().join("ui.toml");
    fs::write(
        &file,
        "future = 1\nsongs_dir = \"/s\"\n[section]\nx = true\n",
    )
    .unwrap();
    let s = Settings::load_from(&file);
    assert_eq!(s.songs_dir, Some(p("/s")));
    assert!(s.startup_sound);
}

#[test]
fn an_existing_ui_conf_is_ignored() {
    let d = tempfile::tempdir().unwrap();
    let old = d.path().join("ui.conf");
    let old_text = "songs=/old\nmods=/oldmods\nrecent=/r\nstartup_sound=0\n";
    fs::write(&old, old_text).unwrap();
    // The default file is ui.toml, next to nothing of the old one.
    let file = d.path().join("ui.toml");
    let mut s = Settings::load_from(&file);
    assert_eq!(s.songs_dir, None);
    assert!(s.recent.is_empty());
    assert!(s.startup_sound);
    s.songs_dir = Some(p("/new"));
    s.save().unwrap();
    // The old file is neither read nor deleted nor changed.
    assert_eq!(fs::read_to_string(&old).unwrap(), old_text);
    assert_eq!(Settings::load_from(&file).songs_dir, Some(p("/new")));
}

#[test]
fn orb_ui_config_names_the_settings_file() {
    let d = tempfile::tempdir().unwrap();
    let file = d.path().join("here.toml");
    // SAFETY: this is the only test in this binary that touches the variable.
    unsafe { std::env::set_var("ORB_UI_CONFIG", &file) };
    assert_eq!(orb_ui::settings::default_file(), Some(file.clone()));
    let mut s = Settings::load();
    s.mods_dir = Some(p("/mods"));
    s.save().unwrap();
    assert_eq!(Settings::load().mods_dir, Some(p("/mods")));
    // An empty value counts as unset.
    unsafe { std::env::set_var("ORB_UI_CONFIG", "") };
    assert_ne!(orb_ui::settings::default_file(), Some(PathBuf::new()));
    unsafe { std::env::remove_var("ORB_UI_CONFIG") };
    // Without the variable the file is OpenRB-338/ui.toml (there is none when no home is known).
    if let Some(default) = orb_ui::settings::default_file() {
        assert_eq!(default.file_name().unwrap(), "ui.toml");
        assert_eq!(default.parent().unwrap().file_name().unwrap(), "OpenRB-338");
    }
}
