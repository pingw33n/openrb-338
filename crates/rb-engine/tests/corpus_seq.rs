//! The song load's sequencer and mixer state, and the export's transport, over every song of the
//! reference corpus (task-022).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use rb_engine::frame::export::{self, ExportRange};
use rb_engine::frame::{Devices, Engine, MixerParams};
use rb_engine::song::{Mixer, Song};
use rb_engine::tables::Tables;
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn songs() -> Vec<(String, Song)> {
    let p = root().join("tests/refs/manifest.json");
    let m: Value = serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap();
    let mut out = Vec::new();
    for set in ["diag", "shipped"] {
        for (name, e) in m[set].as_object().unwrap() {
            let p = root().join(e["song"].as_str().unwrap());
            let data = std::fs::read(&p).unwrap();
            let s = rb_engine::load::convert(&rbs::Song::parse(&data).unwrap()).unwrap();
            out.push((format!("{set}/{name}"), s));
        }
    }
    out
}

/// The mixer's live parameters after the load, derived from the song independently of the
/// sequencer: in pattern mode the chunk; in song mode the ctor values, the chunk's master and
/// enables, and each controller's last tick-0 event of track 0 (the msg 32 chase).
fn expected_mixer(s: &Song) -> MixerParams {
    let chunk = MixerParams::from(&s.mixer);
    if s.globals.song_mode == 0 {
        return chunk;
    }
    let mut p = MixerParams {
        master: chunk.master,
        ..MixerParams::default()
    };
    for c in 0..4 {
        p.channels[c].enabled = chunk.channels[c].enabled;
    }
    for e in s.tracks[0].iter().filter(|e| e.tick == 0) {
        match e.ctrl {
            Mixer::CTRL_COMP_ID => p.comp_id = e.value,
            Mixer::CTRL_PCF_ID => p.pcf_id = e.value,
            c => {
                let ch = &mut p.channels[(c as usize - 6) / 6];
                match (c - 6) % 6 {
                    0 => ch.level = e.value,
                    1 => ch.pan = e.value,
                    2 => ch.send = e.value,
                    _ => ch.dist = e.value,
                }
            }
        }
    }
    p
}

#[test]
fn load_state_over_the_corpus() {
    let tables = Arc::new(Tables::new());
    let mut song_mode = 0;
    for (name, s) in songs() {
        let mut e = Engine::new(tables.clone(), Devices::placeholders());
        e.load(&s);
        let q = &e.frame.seq;
        let g = &q.globals;
        assert_eq!(
            (g.song_mode, g.play, g.loop_on, g.tempo, g.shuffle),
            (
                s.globals.song_mode,
                0,
                s.globals.loop_on,
                s.globals.tempo,
                s.globals.shuffle
            ),
            "{name}"
        );
        assert_eq!(
            (g.loop_start, g.loop_end),
            (s.globals.loop_start, s.globals.loop_end)
        );
        assert_eq!((q.pos, q.last_tick), (0.0, 0), "{name}");
        // every event was accepted by the insert (all ticks are below 767232 and distinct per
        // tick and controller), and in song mode the pointer is past the tick-0 events
        for (i, tr) in q.tracks.iter().enumerate() {
            assert_eq!(
                tr.events(&q.pool).len(),
                s.tracks[i].len(),
                "{name} track {i}"
            );
            let at0 = s.tracks[i].iter().filter(|e| e.tick == 0).count();
            let want = if s.globals.song_mode != 0 { at0 } else { 0 };
            assert_eq!(tr.next(), want, "{name} track {i}");
        }
        assert_eq!(e.frame.devices.mixer.live, expected_mixer(&s), "{name}");
        song_mode += (s.globals.song_mode != 0) as u32;
    }
    assert!(song_mode > 0);
}

#[test]
fn export_transport_over_the_corpus() {
    let tables = Arc::new(Tables::new());
    for (name, s) in songs().into_iter().take(40) {
        let mut e = Engine::new(tables.clone(), Devices::placeholders());
        let r = ExportRange::from(&s.globals);
        let out = export::export_song(&mut e, &s, 16);
        assert_eq!(out.len(), 2 * export::frame_count(r), "{name}");
        // the export ends stopped, at the start of a bar, after the loop wrapped (song mode with
        // the loop on) or ran past the loop end
        let q = &e.frame.seq;
        assert_eq!(q.globals.play, 0, "{name}");
        assert_eq!(q.pos % 768.0, 0.0, "{name}");
        let start = s.globals.loop_start as f64;
        if s.globals.song_mode != 0 && s.globals.loop_on != 0 {
            assert_eq!(q.pos, start, "{name}");
        } else {
            assert!(q.pos >= s.globals.loop_end as f64 - 768.0, "{name}");
        }
    }
}
