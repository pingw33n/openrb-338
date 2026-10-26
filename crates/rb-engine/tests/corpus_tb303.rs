//! The TB-303s against the original (task-023).
//!
//! Every song of the diagnostic corpus has the delay enabled, and an enabled delay adds its
//! output (about 1e-6 at idle, plus 1e-6 × the channels D samples later) into the master, so the
//! corpus references of the TB-303 songs depend on the delay (task-026). Until it exists the 303s
//! are checked two ways: on the songs with the delay disabled against rbhost renders of those
//! variants (`tests/fixtures/tb303_nodelay.json`, made with `tools/nodelay.py`), and on the songs
//! as they are against their corpus references with a test-only model of the delay
//! ([`common::TestDelay`]). With the engine's own delay, the references are task-049.

mod common;

use std::sync::Arc;

use common::without_delay;
use common::{json, manifest, render_with, render_with_test_delay, root, sha256, song};
use rb_engine::frame::Devices;
use rb_engine::song::Song;
use rb_engine::tables::Tables;

/// The WAV file of the export of `s` after `idle` idle blocks, with the devices implemented so
/// far (no drum samples: these songs play no drum step).
fn render(tables: &Arc<Tables>, s: &Song, idle: u32) -> Vec<u8> {
    render_with(tables, Devices::implemented(None), s, idle)
}

/// The diagnostic songs whose sound comes from the TB-303s and the mixer only: no step is set in
/// any 808 or 909 pattern, and no insert is routed (mixer compressor and PCF ids 0, no channel's
/// distortion on, in the chunk or the mixer track). Derived from the `.rbs` files with
/// `tools/rbs.py` (task-023). All of them have the delay enabled.
const TB303_SONGS: &[&str] = &[
    // the ReBirth 2.0 sound
    "303_accent0",
    "303_accent127",
    "303_env0",
    "303_env127_decay0",
    "303_env127_decay127",
    "303_res0",
    "303_res127",
    "303_res64",
    "303_saw_highcut",
    "303_saw_lowcut",
    "303_slide",
    "303_slide_accent",
    "303_square_highcut",
    "303_square_lowcut",
    "303_tune_hi",
    "303_tune_lo",
    "303_two_voices",
    // the Vintage sound
    "303_vintage_accent",
    "303_vintage_highcut",
    "303_vintage_lowcut",
    "303_vintage_res127",
    "303_vintage_saw",
    "303_vintage_slide",
    "303_vintage_square",
    // the sequencer and the mixer around a 303
    "delay_303",
    "loop_bar1",
    "loop_bar2",
    "loop_events_inside",
    "mixer_level_sweep_303",
    "mixer_master_low",
    "mixer_pan_sweep_303",
    "shuffle_303_g0",
    "shuffle_303_g127",
    "shuffle_303_g64",
    "shuffle_flag_off",
    "silence",
];

/// AC #1 and #2: every TB-303 song, with the delay disabled, renders byte-identical to rbhost's
/// render of the same variant (the WAV's sha256, header included), in both sound models.
#[test]
fn tb303_songs_without_the_delay_match_rbhost() {
    let m = manifest();
    let fx = json(&root().join("crates/rb-engine/tests/fixtures/tb303_nodelay.json"));
    let idle = fx["idle_blocks"].as_u64().unwrap() as u32;
    let tables = Arc::new(Tables::new());
    let mut bad = Vec::new();
    let mut vintage = 0;
    for name in TB303_SONGS {
        let s = without_delay(song(&m["diag"][*name]));
        vintage += s.globals.is_vintage() as usize;
        let wav = render(&tables, &s, idle);
        assert_eq!(wav.len(), 44 + 4 * fx["frames"].as_u64().unwrap() as usize);
        if sha256(&wav) != fx["wav_sha256"][*name].as_str().unwrap() {
            bad.push(*name);
        }
    }
    assert_eq!((TB303_SONGS.len(), vintage), (36, 7));
    assert!(bad.is_empty(), "differ from rbhost: {bad:?}");
}

/// The difference to the corpus references is the delay's: with the test-only delay model in
/// place of the placeholder, every TB-303 song equals its corpus reference.
#[test]
fn tb303_songs_with_a_test_delay_match_their_references() {
    let m = manifest();
    let tables = Arc::new(Tables::new());
    let mut bad = Vec::new();
    for name in TB303_SONGS {
        let e = &m["diag"][*name];
        let idle = e["idle_blocks"].as_u64().unwrap() as u32;
        let wav = render_with_test_delay(&tables, Devices::implemented(None), &song(e), idle);
        if sha256(&wav) != e["wav_sha256"].as_str().unwrap() {
            bad.push(*name);
        }
    }
    assert!(bad.is_empty(), "differ from their references: {bad:?}");
}

/// The same songs against their corpus references with the engine's devices, which need the
/// engine's delay (task-026); task-049 removes the `ignore`. `silence` already matches.
#[test]
#[ignore = "the references include the enabled delay's output (task-026, task-049)"]
fn tb303_songs_match_their_references() {
    let m = manifest();
    let tables = Arc::new(Tables::new());
    let mut bad = Vec::new();
    for name in TB303_SONGS {
        let e = &m["diag"][*name];
        let idle = e["idle_blocks"].as_u64().unwrap() as u32;
        if sha256(&render(&tables, &song(e), idle)) != e["wav_sha256"].as_str().unwrap() {
            bad.push(*name);
        }
    }
    assert!(bad.is_empty(), "differ from their references: {bad:?}");
}
