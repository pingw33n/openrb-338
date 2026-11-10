//! Host frames from the player's 44100 Hz blocks.
//!
//! At 44100 Hz the host gets the engine frames in order. Any other rate is a
//! linear interpolation, which is not the engine's sample values.

use std::collections::VecDeque;

use orb_player::Audio;

use crate::pull::CHANS;

const RATE: f32 = 44_100.0;

pub struct Out {
    q: VecDeque<[f32; CHANS]>,
    direct: bool,
    ratio: f64,
    pos: f64,
    held: [f32; CHANS],
    mix: Vec<f32>,
    taps: Vec<[f32; 12]>,
}

impl Out {
    pub fn new(sample_rate: f32, max_host_frames: usize) -> Self {
        let direct = (sample_rate - RATE).abs() < 0.5;
        let ratio = if sample_rate > 0.0 {
            (RATE as f64) / sample_rate as f64
        } else {
            1.0
        };
        let cap = max_host_frames.saturating_mul(4).saturating_add(128);
        Out {
            q: VecDeque::with_capacity(cap),
            direct,
            ratio,
            pos: 0.0,
            held: [0.0; CHANS],
            mix: Vec::with_capacity(cap * 2),
            taps: Vec::with_capacity(cap),
        }
    }

    pub fn fill(&mut self, audio: &mut Audio, dest: &mut [[f32; CHANS]]) {
        if self.direct {
            self.render(audio, dest.len());
            for frame in dest.iter_mut() {
                *frame = self.q.pop_front().unwrap_or([0.0; CHANS]);
            }
            return;
        }
        let need = (self.pos + self.ratio * dest.len() as f64).floor() as usize + 2;
        while self.q.len() < need {
            let n = (need - self.q.len()).max(64);
            self.render(audio, n);
        }
        for frame in dest.iter_mut() {
            *frame = self.next_resampled();
        }
    }

    fn render(&mut self, audio: &mut Audio, n: usize) {
        self.mix.resize(n * 2, 0.0);
        self.taps.resize(n, [0.0; 12]);
        audio.render_f32_taps(&mut self.mix[..n * 2], &mut self.taps[..n]);
        for i in 0..n {
            let mut f = [0.0; CHANS];
            f[0] = self.mix[2 * i];
            f[1] = self.mix[2 * i + 1];
            f[2..14].copy_from_slice(&self.taps[i]);
            self.q.push_back(f);
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
