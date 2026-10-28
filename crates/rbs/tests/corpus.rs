//! Round trip and model equality over the shipped songs and the diagnostic corpus.
//!
//! The model is compared with `tools/rbs.py json` (run through /usr/bin/python3 on each song).

use rbs::*;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn songs() -> Vec<PathBuf> {
    let mut v = Vec::new();
    for dir in ["tests/songs/shipped", "tests/songs/diag/songs"] {
        for e in std::fs::read_dir(root().join(dir)).unwrap() {
            let p = e.unwrap().path();
            if p.extension().is_some_and(|x| x == "rbs") {
                v.push(p);
            }
        }
    }
    v.sort();
    assert!(v.len() >= 160, "found only {} songs", v.len());
    v
}

fn params(names: &[&str], values: &[u8]) -> Value {
    Value::Object(
        names
            .iter()
            .zip(values)
            .map(|(n, v)| (n.to_string(), json!(v)))
            .collect(),
    )
}

fn to_json(s: &Song) -> Value {
    let pat303 = |p: &Pattern303| {
        json!({"shuffle": p.shuffle, "length": p.length, "steps": p.steps.iter().map(|st| json!({
            "note": st.note, "slide": st.slide, "accent": st.accent, "up": st.up, "down": st.down,
            "rest": st.rest, "flags_hi": st.flags_hi})).collect::<Vec<_>>()})
    };
    let drums = |d: &Drums, names: &[&str]| {
        json!({"params": params(names, &d.params), "patterns": d.patterns.iter().map(|p| json!({
            "shuffle": p.shuffle, "length": p.length, "steps": p.steps})).collect::<Vec<_>>()})
    };
    let plain = |p: &Params, names: &[&str]| json!({"values": params(names, &p.values)});
    let tracks: serde_json::Map<String, Value> = TRACK_NAMES
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let ev = s.tracks[i].events.iter().map(|e| {
                let ctrl = controller_name(i, e.ctrl).map_or(json!(e.ctrl), Value::String);
                json!({"pos": e.pos, "ctrl": ctrl, "value": e.value})
            });
            (n.to_string(), Value::Array(ev.collect()))
        })
        .collect();
    json!({
        "version": s.head.version(),
        "head": {"copyright": s.head.copyright},
        "glob": {"song_mode": s.glob.song_mode, "loop": s.glob.loop_on, "tempo": s.glob.tempo,
            "loop_start": s.glob.loop_start, "loop_end": s.glob.loop_end, "shuffle": s.glob.shuffle,
            "mod_name": s.glob.mod_name, "mod_ftp": s.glob.mod_ftp, "mod_www": s.glob.mod_www,
            "vintage": s.glob.vintage},
        "usri": {"window_title": s.usri.window_title, "text": s.usri.text,
            "web_page": s.usri.web_page, "show_on_open": s.usri.show_on_open},
        "mixer": {"master": s.mixer.master, "comp_id": s.mixer.comp_id, "pcf_id": s.mixer.pcf_id,
            "channels": s.mixer.channels.iter().map(|c| json!({
                "enabled": c.enabled, "level": c.level,
                "pan": c.pan, "delay_send": c.delay_send, "dist": c.dist
            })).collect::<Vec<_>>()},
        "delay": plain(&s.delay, &DELAY_PARAMS),
        "pcf": plain(&s.pcf, &PCF_PARAMS),
        "dist": plain(&s.dist, &DIST_PARAMS),
        "comp": plain(&s.comp, &COMP_PARAMS),
        "tb303": s.tb303.iter().map(|t| json!({"params": params(&TB303_PARAMS, &t.params),
            "patterns": t.patterns.iter().map(pat303).collect::<Vec<_>>()})).collect::<Vec<_>>(),
        "tr808": drums(&s.tr808, &TR808_PARAMS),
        "tr909": drums(&s.tr909, &TR909_PARAMS),
        "tracks": tracks,
    })
}

#[test]
fn round_trip_is_byte_identical() {
    for p in songs() {
        let data = std::fs::read(&p).unwrap();
        let song = Song::parse(&data).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
        assert!(
            song.write().unwrap() == data,
            "{} does not round-trip",
            p.display()
        );
    }
}

#[test]
fn model_equals_rbs_py_json() {
    for p in songs() {
        let out = Command::new("/usr/bin/python3")
            .arg(root().join("tools/rbs.py"))
            .arg("json")
            .arg(&p)
            .output()
            .expect("run /usr/bin/python3 tools/rbs.py");
        assert!(out.status.success(), "rbs.py failed on {}", p.display());
        let want: Value = serde_json::from_slice(&out.stdout).unwrap();
        let song = Song::parse(&std::fs::read(&p).unwrap()).unwrap();
        assert!(
            to_json(&song) == want,
            "{} differs from rbs.py json",
            p.display()
        );
    }
}

#[test]
fn modified_field_is_written_and_reparsed() {
    let data = std::fs::read(root().join("tests/songs/shipped/Silent Default Song.rbs")).unwrap();
    let mut song = Song::parse(&data).unwrap();
    song.glob.tempo = 133_000;
    song.usri.window_title = "hello".into();
    song.tracks[3].events.push(Event {
        pos: 41024,
        ctrl: 0,
        value: 1,
    });
    let again = Song::parse(&song.write().unwrap()).unwrap();
    assert_eq!(again.glob.tempo, 133_000);
    assert_eq!(again.usri.window_title, "hello");
    assert_eq!(again.tracks[3].events.last().unwrap().pos, 41024);
}

#[test]
fn unknown_chunk_is_preserved_and_errors_name_the_chunk() {
    let data = std::fs::read(root().join("tests/songs/shipped/Silent Default Song.rbs")).unwrap();
    let mut song = Song::parse(&data).unwrap();
    song.unknown.push(Unknown {
        index: 2,
        tag: *b"XTRA",
        body: vec![1, 2, 3],
    });
    let bytes = song.write().unwrap();
    let again = Song::parse(&bytes).unwrap();
    assert_eq!(again.unknown, song.unknown);
    assert!(again.write().unwrap() == bytes);

    let e = Song::parse(&data[..data.len() - 5]).unwrap_err();
    assert!(!e.chunk.is_empty(), "{e}");
    let mut bad = data.clone();
    bad[0] = b'X';
    assert!(Song::parse(&bad).is_err());
}
