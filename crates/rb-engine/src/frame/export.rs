//! The export driver: "Export Loop as Audio File" as Rebirth.exe runs it (`sub_427988` →
//! `sub_4280D5`, `docs/export_state.md` section 4) after `N` idle blocks, which is how
//! `tools/rbhost` reproduces the app's exports (`docs/export_state.md` sections 12 and 13).

use super::{Activation, BLOCK, Engine};
use crate::song::{Globals, Song};

/// The frames of one realtime MME call (`docs/export_state.md` section 2.1): the idle blocks are
/// rendered 16 at a time, as the app's stream and rbhost do.
pub const REALTIME_FRAMES: usize = 1024;
/// The frames of a full export chunk (`cmp [ebp+var_14], 800h` at 0x428186).
pub const CHUNK_FRAMES: i32 = 2048;
/// `dbl_4BBE08`: ticks per quarter note.
const PPQ: f64 = f64::from_bits(0x4068_0000_0000_0000); // 192.0
/// `dbl_4BBE10`: 60 s * 44100 frames * 1000 (the tempo is BPM * 1000).
const FRAMES_MILLI_MINUTE: f64 = f64::from_bits(0x41E3_B6D8_3000_0000); // 2646000000.0

/// What the export queries from the engine before it starts (msg 20, ids 4, 12 and 16).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportRange {
    /// BPM * 1000.
    pub tempo: u32,
    /// The loop start and end in ticks.
    pub loop_start: u32,
    pub loop_end: u32,
}

impl From<&Globals> for ExportRange {
    fn from(g: &Globals) -> Self {
        ExportRange {
            tempo: g.tempo,
            loop_start: g.loop_start,
            loop_end: g.loop_end,
        }
    }
}

/// The frame counts of the export's `SafeSoundCallback` calls, `sub_4280D5` (0x428139..0x4281DB):
/// ```text
/// pos = (double)start
/// do  n = ftol((end - pos) / (tempo * (192.0 / 2646000000.0)))
///     if n >= 2048: n = 2048 else last
///     step = (tempo * n) * (192.0 / 2646000000.0)
///     render n; pos = pos + step
/// until last
/// ```
/// The values are 32-bit `fild`s; `192/2646e6` is divided at run time; `__ftol` truncates.
pub fn chunk_sizes(r: ExportRange) -> Vec<i32> {
    let tempo = r.tempo as i32 as f64;
    let end = r.loop_end as i32 as f64;
    let mut pos = r.loop_start as i32 as f64;
    let mut out = Vec::new();
    loop {
        let mut n = ((end - pos) / (tempo * (PPQ / FRAMES_MILLI_MINUTE))) as i64 as i32;
        let last = n < CHUNK_FRAMES;
        if !last {
            n = CHUNK_FRAMES;
        }
        let step = (tempo * n as f64) * (PPQ / FRAMES_MILLI_MINUTE);
        out.push(n);
        pos += step;
        if last {
            return out;
        }
    }
}

/// The exported frame count: the sum of the chunks (the file gets `4*n` bytes per chunk).
pub fn frame_count(r: ExportRange) -> usize {
    chunk_sizes(r).iter().map(|&n| n.max(0) as usize).sum()
}

/// Renders `idle_blocks` 64-frame blocks with the transport stopped: `idle_blocks / 16` calls of
/// 1024 frames, then one call for the remainder (rbhost's `render_idle`).
pub fn render_idle(engine: &mut Engine, idle_blocks: u32) {
    let mut buf = vec![0i16; 2 * REALTIME_FRAMES];
    for _ in 0..idle_blocks / 16 {
        engine.render(&mut buf, REALTIME_FRAMES);
    }
    let rest = (idle_blocks % 16) as usize * BLOCK;
    if rest > 0 {
        engine.render(&mut buf, rest);
    }
}

/// The export as rbhost runs it (`tools/rbhost/rbhost.c`, `docs/export_state.md` section 4) on
/// a song the caller has loaded ([`Engine::load`]) and activated ([`Engine::activate`]):
/// `idle_blocks` idle blocks, stop, locate to the loop start, play, the chunks of
/// [`chunk_sizes`], then stop, msg 33 and a 0-frame call (which drops the last chunk's leftover).
/// Each message is applied by a 0-frame call that renders nothing, so the messages are applied
/// here directly. Returns interleaved int16 stereo.
pub fn export(engine: &mut Engine, range: ExportRange, idle_blocks: u32) -> Vec<i16> {
    render_idle(engine, idle_blocks);
    engine.stop();
    engine.render(&mut [], 0);
    engine.locate(range.loop_start);
    engine.play();
    engine.render(&mut [], 0);
    let mut out = Vec::with_capacity(2 * frame_count(range));
    let mut buf = vec![0i16; 2 * CHUNK_FRAMES as usize];
    for n in chunk_sizes(range) {
        // rbhost writes only positive counts; the app would pass a negative byte count to its
        // writer, which never happens for a loop of positive length
        let n = n.max(0) as usize;
        engine.render(&mut buf, n);
        out.extend_from_slice(&buf[..2 * n]);
    }
    engine.stop();
    engine.reset();
    engine.render(&mut [], 0);
    out
}

/// rbhost's whole run on a new song (by default, `--idle-blocks N`): load `song`, activate it
/// over the "Untitled song" ([`Activation::ReplacesCurrent`]), then [`export`] with the range
/// the export queries (msg 20 ids 4, 12, 16, which read the loaded globals).
pub fn export_song(engine: &mut Engine, song: &Song, idle_blocks: u32) -> Vec<i16> {
    engine.load(song);
    engine.activate(Activation::ReplacesCurrent);
    export(engine, ExportRange::from(&song.globals), idle_blocks)
}

/// The 44-byte header rbhost and the app write before the samples (16-bit stereo 44100 Hz PCM).
pub fn wav_header(frames: usize) -> [u8; 44] {
    let data = (frames * 4) as u32;
    let mut h = [0u8; 44];
    h[0..4].copy_from_slice(b"RIFF");
    h[4..8].copy_from_slice(&(36 + data).to_le_bytes());
    h[8..16].copy_from_slice(b"WAVEfmt ");
    h[16..20].copy_from_slice(&16u32.to_le_bytes());
    h[20..22].copy_from_slice(&1u16.to_le_bytes());
    h[22..24].copy_from_slice(&2u16.to_le_bytes());
    h[24..28].copy_from_slice(&44100u32.to_le_bytes());
    h[28..32].copy_from_slice(&(44100u32 * 4).to_le_bytes());
    h[32..34].copy_from_slice(&4u16.to_le_bytes());
    h[34..36].copy_from_slice(&16u16.to_le_bytes());
    h[36..40].copy_from_slice(b"data");
    h[40..44].copy_from_slice(&data.to_le_bytes());
    h
}

/// The whole WAV file of an export.
pub fn wav_bytes(samples: &[i16]) -> Vec<u8> {
    let mut b = Vec::with_capacity(44 + 2 * samples.len());
    b.extend_from_slice(&wav_header(samples.len() / 2));
    for s in samples {
        b.extend_from_slice(&s.to_le_bytes());
    }
    b
}
