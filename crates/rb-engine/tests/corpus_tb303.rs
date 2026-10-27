//! The TB-303s against the original (task-023).
//!
//! Every song of the diagnostic corpus has the delay enabled, and an enabled delay adds its
//! output (about 1e-6 at idle, plus 1e-6 × the channels D samples later) into the master, so the
//! corpus references of the TB-303 songs depend on the delay. The 303s are checked two ways: on
//! the songs as they are against their corpus references, with the engine's delay (task-026,
//! task-049; the golden test `corpus_golden.rs`, task-029, with the control here), and on the
//! songs with the delay disabled against rbhost renders of those variants
//! (`tests/fixtures/tb303_nodelay.json`, made with `tools/nodelay.py`, task-023). No corpus song
//! disables the delay, so the second is the only reference check of the delay's disabled path
//! (0x1000FAD1).

mod common;

use std::sync::Arc;

use common::without_delay;
use common::{differing, json, manifest, render_with, root, sha256, song};
use rb_engine::frame::{Devices, Silent};
use rb_engine::song::Song;
use rb_engine::tables::Tables;

/// The WAV file of the export of `s` after `idle` idle blocks, with the devices implemented so
/// far, the engine's delay included (no drum samples: these songs play no drum step).
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

/// Every TB-303 song, with the delay disabled, renders byte-identical to rbhost's render of the
/// same variant (the WAV's sha256, header included), in both sound models: the 303s without the
/// delay's output, and the only reference check of the disabled delay (0x1000FAD1).
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

/// The control: with both TB-303s silent, a 303 song differs from its reference, so the golden
/// test (`corpus_golden.rs`, which compares every song as it is with its corpus reference) hears
/// the 303s. (`silence` and `mixer_master_low`, whose master is low, may not.)
#[test]
fn tb303_songs_differ_without_the_303s() {
    let silent = || {
        let mut d = Devices::implemented(None);
        d.channels[0] = Box::new(Silent);
        d.channels[1] = Box::new(Silent);
        d
    };
    let names = ["303_saw_lowcut", "303_vintage_saw", "303_two_voices"];
    assert_eq!(differing(&names, silent), names);
}
