//! Helpers of the device corpus tests (`corpus_tb303.rs`, `corpus_tr808.rs`): the manifest,
//! the songs, and a test-only model of the delay.
//!
//! Every song of the diagnostic corpus has the delay enabled, and an enabled delay adds its
//! output (about 1e-6 at idle, plus 1e-6 x the channels D samples later) into the master, so the
//! corpus references depend on the delay. [`TestDelay`] stood in for it until the engine got
//! its own ([`rb_engine::delay::Delay`], task-026, which `corpus_effects.rs` checks on every
//! song of these tests); task-049 switches the device tests over.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use rb_engine::frame::export;
use rb_engine::frame::{Devices, Engine};
use rb_engine::samples::SampleSet;
use rb_engine::song::{Delay, Device, Song};
use rb_engine::tables::Tables;
use serde_json::Value;
use sha2::{Digest, Sha256};

pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn json(p: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap()
}

pub fn manifest() -> Value {
    json(&root().join("tests/refs/manifest.json"))
}

/// The song of a manifest entry, converted to the engine's model.
pub fn song(entry: &Value) -> Song {
    let p = root().join(entry["song"].as_str().unwrap());
    let data = std::fs::read(&p).unwrap_or_else(|err| panic!("{}: {err}", p.display()));
    rb_engine::load::convert(&rbs::Song::parse(&data).unwrap()).unwrap()
}

/// The built-in samples of `original/ReBirth.dll`.
pub fn samples() -> Arc<SampleSet> {
    let p = root().join("original/ReBirth.dll");
    let dll = std::fs::read(&p).unwrap_or_else(|err| panic!("{}: {err}", p.display()));
    Arc::new(SampleSet::from_dll(&dll).unwrap())
}

pub fn sha256(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// The WAV file of the export of `s` after `idle` idle blocks, with `devices`.
pub fn render_with(tables: &Arc<Tables>, devices: Devices, s: &Song, idle: u32) -> Vec<u8> {
    let mut eng = Engine::new(tables.clone(), devices);
    export::wav_bytes(&export::export_song(&mut eng, s, idle))
}

/// [`render_with`] the engine's devices and [`TestDelay`] in place of the idle send.
pub fn render_with_test_delay(
    tables: &Arc<Tables>,
    mut devices: Devices,
    s: &Song,
    idle: u32,
) -> Vec<u8> {
    devices.delay = Box::new(TestDelay::new(tables.clone(), s.globals.tempo as i32));
    render_with(tables, devices, s, idle)
}

/// The song with the delay disabled: the chunk's enabled byte and every `enabled` event of the
/// delay's track set to 0, the edit `tools/nodelay.py` makes on the `.rbs` for rbhost.
pub fn without_delay(mut s: Song) -> Song {
    s.delay.params[Delay::ENABLED] = 0;
    for e in &mut s.tracks[Device::Delay.id() as usize] {
        if e.ctrl as usize == Delay::ENABLED {
            e.value = 0;
        }
    }
    s
}

/// A test-only model of the delay (`CDelayEffect`, vtable 0x10032CB4), only to show that the
/// difference between the engine and the corpus references is the delay's (task-023 review).
/// It is not the engine's delay (task-026): it takes the tempo as a constant and omits the
/// meters. From the disassembly: the update 0x1000F754 every 16 blocks, the enabled block
/// 0x1000F940..0x1000FACE and the disabled one 0x1000FAD1.
pub struct TestDelay {
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
    pub fn new(tables: Arc<Tables>, tempo: i32) -> Self {
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
