//! The settings file (task-046): damaged files, atomic writes and `RB_UI_CONFIG`.

use std::fs;
use std::path::PathBuf;

use rb_ui::settings::Settings;

#[test]
fn a_corrupt_settings_file_gives_the_defaults() {
    let d = tempfile::tempdir().unwrap();
    let file = d.path().join("ui.conf");
    fs::write(&file, [0xff, 0xfe, b'=', 0x80, b'\n', b's']).unwrap();
    let s = Settings::load_from(&file);
    assert_eq!(s.songs_dir, None);
    assert!(s.recent.is_empty());
    // Garbage lines are skipped, good ones kept.
    fs::write(&file, "junk\nsongs=/x\n=\nrecent=\nmods=/m\n").unwrap();
    let s = Settings::load_from(&file);
    assert_eq!(s.songs_dir, Some(PathBuf::from("/x")));
    assert_eq!(s.mods_dir, Some(PathBuf::from("/m")));
    // Saving leaves no temporary file behind.
    s.save().unwrap();
    assert_eq!(fs::read_dir(d.path()).unwrap().count(), 1);
    // the startup sound preference (task-074): on by default, and kept when turned off
    assert!(s.startup_sound);
    let mut s = Settings::load_from(&file);
    s.startup_sound = false;
    s.save().unwrap();
    assert!(!Settings::load_from(&file).startup_sound);
    fs::write(&file, "startup_sound=x\n").unwrap();
    assert!(Settings::load_from(&file).startup_sound);
}

#[test]
fn rb_ui_config_names_the_settings_file() {
    let d = tempfile::tempdir().unwrap();
    let file = d.path().join("here.conf");
    // SAFETY: this is the only test in this binary, and it is the only one that touches the
    // variable.
    unsafe { std::env::set_var("RB_UI_CONFIG", &file) };
    assert_eq!(rb_ui::settings::default_file(), Some(file.clone()));
    let mut s = Settings::load();
    s.mods_dir = Some(PathBuf::from("/mods"));
    s.save().unwrap();
    assert_eq!(Settings::load().mods_dir, Some(PathBuf::from("/mods")));
    unsafe { std::env::remove_var("RB_UI_CONFIG") };
}
