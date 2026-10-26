//! The TB-303s against the original (task-023).
//!
//! Every song of the diagnostic corpus has the delay enabled, and an enabled delay adds its
//! output (about 1e-6 at idle, plus 1e-6 × the channels D samples later) into the master, so the
//! corpus references of the TB-303 songs depend on the delay (task-026). Until it exists the 303s
//! are checked two ways: on the songs with the delay disabled against rbhost renders of those
//! variants (`tests/fixtures/tb303_nodelay.json`, made with `tools/nodelay.py`), and on the songs
//! as they are against their corpus references with a test-only model of the delay
//! ([`TestDelay`]). With the engine's own delay, the references are task-049.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use rb_engine::frame::export;
use rb_engine::frame::{Devices, Engine};
use rb_engine::song::{Delay, Device, Song};
use rb_engine::tables::Tables;
use serde_json::Value;
use sha2::{Digest, Sha256};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn json(p: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap()
}

fn manifest() -> Value {
    json(&root().join("tests/refs/manifest.json"))
}

fn song(entry: &Value) -> Song {
    let p = root().join(entry["song"].as_str().unwrap());
    let data = std::fs::read(&p).unwrap_or_else(|err| panic!("{}: {err}", p.display()));
    rb_engine::load::convert(&rbs::Song::parse(&data).unwrap()).unwrap()
}

/// The WAV file of the export of `s` after `idle` idle blocks, with the devices implemented so
/// far.
fn render(tables: &Arc<Tables>, s: &Song, idle: u32) -> Vec<u8> {
    let mut eng = Engine::new(tables.clone(), Devices::implemented());
    export::wav_bytes(&export::export_song(&mut eng, s, idle))
}

/// A test-only model of the delay (`CDelayEffect`, vtable 0x10032CB4), only to show that the
/// difference between the engine and the corpus references is the delay's (task-023 review).
/// It is not the engine's delay (task-026): it takes the tempo as a constant and omits the
/// meters. From the disassembly: the update 0x1000F754 every 16 blocks, the enabled block
/// 0x1000F940..0x1000FACE and the disabled one 0x1000FAD1.
struct TestDelay {
    tables: Arc<Tables>,
    tempo: i32,
    /// `+0x34..+0x38`: enabled, steps, mode, feedback, pan.
    params: [u8; 5],
    /// `+0x30`: the block counter.
    counter: u16,
    /// `+0x54`, `+0x58` targets, `+0x5C`, `+0x60` gains, `+0x64` feedback.
    target: [f32; 2],
    gain: [f32; 2],
    fb: f32,
    /// `+0x68` the delay in samples, `+0x6C` the write position.
    d: u32,
    w: u32,
    /// `flt_10054794` and its gate `byte_10054790`.
    buf: Vec<f32>,
    gate: bool,
}

const GUARD: f32 = f32::from_bits(0x3586_37BD);

impl TestDelay {
    fn new(tables: Arc<Tables>, tempo: i32) -> Self {
        TestDelay {
            tables,
            tempo,
            params: [0; 5],
            counter: 0,
            target: [0.0; 2],
            gain: [0.0; 2],
            fb: 0.0,
            d: 0,
            w: 0,
            buf: vec![0.0; 1 << 17],
            gate: false,
        }
    }

    /// 0x1000F7E5..0x1000F8E9.
    fn update(&mut self) {
        let [_, steps, mode, fb, pan] = self.params;
        let q = if mode == 0 {
            3 * self.tempo / 100
        } else {
            4 * self.tempo / 100
        };
        self.d = (26_460_000u32.wrapping_mul(steps as u32) / q as u32).min(0x1FFFF);
        let p = self.tables.pan[127][pan as usize];
        self.target = [
            (GUARD as f64 + p[0] as f64) as f32,
            (GUARD as f64 + p[1] as f64) as f32,
        ];
        self.fb = (fb as f64 * (1.0 / 127.0)) as f32;
    }
}

impl rb_engine::frame::SongDevice for TestDelay {
    fn silence(&mut self) {
        self.buf.fill(GUARD);
        self.gate = true;
    }
    fn reset(&mut self) {
        self.silence();
    }
    fn set_param(&mut self, ctrl: u8, value: u8) {
        if let Some(p) = self.params.get_mut(ctrl as usize) {
            *p = value;
        }
    }
}

impl rb_engine::frame::SendEffect for TestDelay {
    fn process(
        &mut self,
        send: &mut rb_engine::frame::Block,
        l: &mut rb_engine::frame::Block,
        r: &mut rb_engine::frame::Block,
        _ctx: &rb_engine::frame::BlockContext,
    ) {
        if self.params[0] == 0 {
            if !self.gate {
                self.buf.fill(GUARD);
                self.gate = true;
            }
            send.fill(GUARD);
            return;
        }
        self.gate = false;
        if self.counter & 15 == 0 {
            self.update();
        }
        self.counter = self.counter.wrapping_add(1);
        // gain = gain*0.9 + target*0.1, stored as f32; the loop uses the unrounded value
        let gl = self.gain[0] as f64 * 0.9 + self.target[0] as f64 * 0.1;
        let gr = 0.9 * self.gain[1] as f64 + 0.1 * self.target[1] as f64;
        self.gain = [gl as f32, gr as f32];
        let fb = self.fb as f64;
        let mut rd = self.w.wrapping_sub(self.d) & 0x1FFFF;
        for i in 0..64 {
            let x = self.buf[rd as usize] as f64;
            l[i] = (x * gl + l[i] as f64) as f32;
            r[i] = (r[i] as f64 + x * gr) as f32;
            self.buf[self.w as usize] = (x * fb + send[i] as f64) as f32;
            send[i] = GUARD;
            rd = (rd + 1) & 0x1FFFF;
            self.w = (self.w + 1) & 0x1FFFF;
        }
    }
}

/// [`render`] with the test-only delay model.
fn render_with_test_delay(tables: &Arc<Tables>, s: &Song, idle: u32) -> Vec<u8> {
    let mut d = Devices::implemented();
    d.delay = Box::new(TestDelay::new(tables.clone(), s.globals.tempo as i32));
    let mut eng = Engine::new(tables.clone(), d);
    export::wav_bytes(&export::export_song(&mut eng, s, idle))
}

fn sha256(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// The song with the delay disabled: the chunk's enabled byte and every `enabled` event of the
/// delay's track set to 0, the edit `tools/nodelay.py` makes on the `.rbs` for rbhost.
fn without_delay(mut s: Song) -> Song {
    s.delay.params[Delay::ENABLED] = 0;
    for e in &mut s.tracks[Device::Delay.id() as usize] {
        if e.ctrl as usize == Delay::ENABLED {
            e.value = 0;
        }
    }
    s
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
        let wav = render_with_test_delay(&tables, &song(e), idle);
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
