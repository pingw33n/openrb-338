#![allow(dead_code)]
//! Engine blocks to the host's buffer.
//!
//! The engine renders at 44100 Hz in calls of 64 frames ([`BLOCK`]). At
//! 44100 Hz the host gets those frames in order, and the stereo mix is the
//! engine's int16 conversion divided by 32768, the same scale `orb-player`
//! uses. Any other rate is a linear interpolation between those frames, which
//! is not the engine's sample values.

use std::collections::VecDeque;

use orb_engine::frame::{BLOCK, Engine};

/// Mix L/R, four modules as stereo, the delay send as stereo, pre-master.
pub const CHANS: usize = 14;
const RATE: f32 = 44_100.0;

/// A queue of engine frames and, off 44100 Hz, a read position into it.
pub struct Pull {
    q: VecDeque<[f32; CHANS]>,
    direct: bool,
    /// Engine frames advanced per host frame.
    ratio: f64,
    /// Fractional position between `held` and the front of `q`.
    pos: f64,
    held: [f32; CHANS],
}

impl Pull {
    pub fn new(sample_rate: f32, max_host_frames: usize) -> Self {
        let direct = (sample_rate - RATE).abs() < 0.5;
        let ratio = if sample_rate > 0.0 {
            (RATE as f64) / sample_rate as f64
        } else {
            1.0
        };
        // Enough engine frames for one host buffer down to about a quarter of 44100.
        let cap = max_host_frames.saturating_mul(4).saturating_add(BLOCK * 2);
        Pull {
            q: VecDeque::with_capacity(cap),
            direct,
            ratio,
            pos: 0.0,
            held: [0.0; CHANS],
        }
    }

    /// Write `out.len()` host frames. `out` is not cleared first.
    pub fn fill(&mut self, engine: &mut Engine, out: &mut [[f32; CHANS]]) {
        if self.direct {
            while self.q.len() < out.len() {
                push_block(engine, &mut self.q);
            }
            for frame in out.iter_mut() {
                *frame = self.q.pop_front().unwrap_or([0.0; CHANS]);
            }
            return;
        }
        let need = (self.pos + self.ratio * out.len() as f64).floor() as usize + 2;
        while self.q.len() < need {
            push_block(engine, &mut self.q);
        }
        for frame in out.iter_mut() {
            *frame = self.next_resampled();
        }
    }

    fn next_resampled(&mut self) -> [f32; CHANS] {
        let t = self.pos.clamp(0.0, 1.0);
        let nxt = self.q.front().copied().unwrap_or(self.held);
        let mut o = [0.0; CHANS];
        for c in 0..CHANS {
            let a = self.held[c] as f64;
            let b = nxt[c] as f64;
            o[c] = (a * (1.0 - t) + b * t) as f32;
        }
        self.pos += self.ratio;
        while self.pos >= 1.0 {
            match self.q.pop_front() {
                Some(s) => self.held = s,
                None => {
                    self.pos = 0.0;
                    break;
                }
            }
            self.pos -= 1.0;
        }
        o
    }
}

fn push_block(engine: &mut Engine, q: &mut VecDeque<[f32; CHANS]>) {
    let mut buf = [0i16; BLOCK * 2];
    let before = engine.fault().map(|f| f.calls).unwrap_or(0);
    engine.render(&mut buf, BLOCK);
    let faulted = engine.fault().map(|f| f.calls).unwrap_or(0) != before;
    let cap = *engine.frame.capture();
    for i in 0..BLOCK {
        if faulted {
            q.push_back([0.0; CHANS]);
            continue;
        }
        let mut f = [0.0; CHANS];
        f[0] = buf[2 * i] as f32 / 32768.0;
        f[1] = buf[2 * i + 1] as f32 / 32768.0;
        for c in 0..4 {
            f[2 + 2 * c] = cap.channel[c][0][i];
            f[3 + 2 * c] = cap.channel[c][1][i];
        }
        f[10] = cap.send[i];
        f[11] = cap.send[i];
        f[12] = cap.pre_master[0][i];
        f[13] = cap.pre_master[1][i];
        q.push_back(f);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use orb_engine::frame::Devices;
    use orb_engine::tables::Tables;
    use std::sync::Arc;

    fn engine() -> Engine {
        Engine::new(Arc::new(Tables::new()), Devices::placeholders())
    }

    #[test]
    fn at_44100_the_mix_is_the_engine_blocks() {
        let mut a = engine();
        let mut b = engine();
        let mut pull = Pull::new(RATE, 256);
        let mut out = vec![[0.0; CHANS]; 128];
        pull.fill(&mut a, &mut out);
        let mut buf = [0i16; 256];
        b.render(&mut buf, 128);
        for i in 0..128 {
            let l = buf[2 * i] as f32 / 32768.0;
            let r = buf[2 * i + 1] as f32 / 32768.0;
            assert_eq!(out[i][0], l);
            assert_eq!(out[i][1], r);
        }
    }
}
