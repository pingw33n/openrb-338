//! The audio output: an [`Audio`] in a cpal output stream (task-040 chose cpal).
//!
//! The stream runs at 44.1 kHz whenever the device offers it, and then the engine's samples go
//! out unchanged (`x / 32768` as f32, converted to the device's sample format by cpal's
//! `FromSample`). A device without 44.1 kHz gets the engine's output resampled to its default
//! rate by rubato's asynchronous sinc resampler ([`rubato::Async::new_sinc`], 128 taps, a
//! Blackman-Harris window, 256x oversampling, cubic interpolation), fed one engine call (1024
//! frames) at a time (`FixedAsync::Input`); its buffers are allocated when the stream is built
//! and `process_into_buffer` allocates nothing. Stereo goes to the first two channels of a
//! device with more (the others are zero), and the mean of both to a mono device.
//!
//! The offline render does not use any of this: it is [`rb_engine::frame::export`].

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{ErrorKind, FromSample, SampleFormat, SizedSample, StreamConfig};
use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{
    Async, FixedAsync, Resampler as _, SincInterpolationParameters, SincInterpolationType,
    WindowFunction,
};

use crate::{Audio, CALL_FRAMES, RATE};

/// The frames converted per step of a device callback.
const SCRATCH_FRAMES: usize = 1024;

/// An error of [`start`].
#[derive(Debug)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

fn err(what: &str, e: impl std::fmt::Display) -> Error {
    Error(format!("{what}: {e}"))
}

/// A running output stream; dropping it stops the sound.
pub struct Stream {
    _stream: cpal::Stream,
    /// The device's name.
    pub device: String,
    /// The stream's configuration.
    pub config: StreamConfig,
    /// The stream's sample format.
    pub format: SampleFormat,
    /// Whether the engine's output is resampled (the device does not offer 44.1 kHz).
    pub resampled: bool,
    failed: Arc<AtomicBool>,
}

impl Stream {
    /// Whether the stream reported an error other than an underrun or a denied real-time
    /// priority (e.g. the device went away); the sound has then stopped.
    pub fn failed(&self) -> bool {
        self.failed.load(Ordering::Relaxed)
    }
}

/// The engine's output at the device's rate, channel count and sample format: the body of the
/// stream's callback, usable without a device.
pub struct Output {
    audio: Audio,
    channels: usize,
    resampler: Option<Resample>,
    scratch: Box<[f32; 2 * SCRATCH_FRAMES]>,
}

/// The resampler and its buffers.
struct Resample {
    rs: Async<f32>,
    input: Vec<f32>,
    output: Vec<f32>,
    /// The frames of `output` the last chunk produced, and those already handed out.
    len: usize,
    pos: usize,
    /// [`Output::inject_resampler_error`].
    fail_next: bool,
}

impl Resample {
    fn new(rate: u32) -> Result<Self, Error> {
        let params = SincInterpolationParameters::new(128, WindowFunction::BlackmanHarris2)
            .oversampling_factor(256)
            .interpolation(SincInterpolationType::Cubic);
        let rs = Async::<f32>::new_sinc(
            rate as f64 / RATE as f64,
            1.1,
            &params,
            CALL_FRAMES,
            2,
            FixedAsync::Input,
        )
        .map_err(|e| err("the resampler", e))?;
        let max = rs.output_frames_max();
        Ok(Resample {
            rs,
            input: vec![0.0; 2 * CALL_FRAMES],
            output: vec![0.0; 2 * max],
            len: 0,
            pos: 0,
            fail_next: false,
        })
    }

    /// One engine call through the resampler into `output`; the frames written, or `None` when
    /// the buffers or the resampler refuse (the sizes are the resampler's own, so only a defect
    /// or [`Output::inject_resampler_error`] gets there).
    fn chunk(&mut self, audio: &mut Audio) -> Option<usize> {
        audio.render_f32(&mut self.input);
        // a forced error: an output buffer too small for the resampler
        let max = if self.fail_next {
            self.fail_next = false;
            0
        } else {
            self.output.len() / 2
        };
        let inp = InterleavedSlice::new(&self.input[..], 2, CALL_FRAMES).ok()?;
        let mut outp = InterleavedSlice::new_mut(&mut self.output[..], 2, max).ok()?;
        let (_, written) = self.rs.process_into_buffer(&inp, &mut outp, None).ok()?;
        Some(written)
    }

    /// Fills `out` (interleaved stereo) at the device rate. On a resampler error it returns
    /// `false` and the caller silences the rest of the device callback; the engine call of the
    /// failed chunk is lost and the next callback starts a new chunk.
    fn pull(&mut self, audio: &mut Audio, out: &mut [f32]) -> bool {
        let frames = out.len() / 2;
        let mut i = 0;
        while i < frames {
            if self.pos == self.len {
                self.pos = 0;
                match self.chunk(audio) {
                    Some(n) => self.len = n,
                    None => {
                        self.len = 0;
                        return false;
                    }
                }
                continue;
            }
            let n = (self.len - self.pos).min(frames - i);
            out[2 * i..2 * (i + n)].copy_from_slice(&self.output[2 * self.pos..2 * (self.pos + n)]);
            self.pos += n;
            i += n;
        }
        true
    }
}

impl Output {
    /// `audio` for a device of `channels` channels at `rate` Hz (resampled unless 44100).
    pub fn new(audio: Audio, channels: usize, rate: u32) -> Result<Self, Error> {
        if channels == 0 {
            return Err(Error("a device with no channels".into()));
        }
        let resampler = if rate == RATE {
            None
        } else {
            Some(Resample::new(rate)?)
        };
        Ok(Output {
            audio,
            channels,
            resampler,
            scratch: Box::new([0.0; 2 * SCRATCH_FRAMES]),
        })
    }

    /// The callback: fills `data` (interleaved, the device's channels). A resampler error makes
    /// the rest of this callback silent and is counted ([`crate::Snapshot::output_errors`]).
    pub fn fill<T: SizedSample + FromSample<f32>>(&mut self, data: &mut [T]) {
        let ch = self.channels;
        let silent = T::from_sample(0.0f32);
        let mut failed = false;
        for chunk in data.chunks_mut(ch * SCRATCH_FRAMES) {
            if failed {
                chunk.fill(silent);
                continue;
            }
            let frames = chunk.len() / ch;
            let s = &mut self.scratch[..2 * frames];
            match &mut self.resampler {
                None => self.audio.render_f32(s),
                Some(r) => {
                    if !r.pull(&mut self.audio, s) {
                        failed = true;
                        self.audio.output_error();
                        chunk.fill(silent);
                        continue;
                    }
                }
            }
            for (f, lr) in chunk.chunks_exact_mut(ch).zip(s.as_chunks::<2>().0) {
                if ch == 1 {
                    f[0] = T::from_sample((lr[0] + lr[1]) * 0.5);
                } else {
                    f[0] = T::from_sample(lr[0]);
                    f[1] = T::from_sample(lr[1]);
                    for x in &mut f[2..] {
                        *x = silent;
                    }
                }
            }
        }
    }

    pub fn audio(&self) -> &Audio {
        &self.audio
    }

    /// For tests: the resampled frames not yet handed out are dropped, and the next resampler
    /// chunk (made by the next [`Output::fill`]) gets an output buffer too small, so the
    /// resampler returns an error. Returns `false` when this output does not resample.
    #[doc(hidden)]
    pub fn inject_resampler_error(&mut self) -> bool {
        match &mut self.resampler {
            Some(r) => {
                r.pos = r.len;
                r.fail_next = true;
                true
            }
            None => false,
        }
    }
}

/// The configuration to open: 44.1 kHz with two channels when offered (f32 preferred), else
/// 44.1 kHz with any channel count, else the device's default (resampled).
fn choose(device: &cpal::Device) -> Result<(StreamConfig, SampleFormat), Error> {
    let ranges: Vec<_> = device
        .supported_output_configs()
        .map_err(|e| err("the output configurations", e))?
        .filter(|r| r.min_sample_rate() <= RATE && RATE <= r.max_sample_rate())
        .collect();
    let score = |r: &cpal::SupportedStreamConfigRange| {
        (
            r.channels() == 2,
            r.sample_format() == SampleFormat::F32,
            r.channels() >= 2,
        )
    };
    if let Some(r) = ranges.iter().max_by_key(|r| score(r)) {
        let c = r.with_sample_rate(RATE);
        return Ok((c.config(), c.sample_format()));
    }
    let c = device
        .default_output_config()
        .map_err(|e| err("the default output configuration", e))?;
    Ok((c.config(), c.sample_format()))
}

fn build<T: SizedSample + FromSample<f32>>(
    device: &cpal::Device,
    config: StreamConfig,
    mut out: Output,
    failed: Arc<AtomicBool>,
) -> Result<cpal::Stream, Error> {
    device
        .build_output_stream::<T, _, _>(
            config,
            move |data: &mut [T], _| out.fill(data),
            move |e: cpal::Error| {
                eprintln!("rb-player: audio output: {e}");
                // an underrun or a denied real-time priority leaves the stream running
                if !matches!(e.kind(), ErrorKind::Xrun | ErrorKind::RealtimeDenied) {
                    failed.store(true, Ordering::Relaxed);
                }
            },
            None,
        )
        .map_err(|e| err("the output stream", e))
}

/// Plays `audio` through the default output device.
pub fn start(audio: Audio) -> Result<Stream, Error> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| Error("no default output device".into()))?;
    let name = device
        .description()
        .map(|d| d.to_string())
        .unwrap_or_else(|_| "?".into());
    let (config, format) = choose(&device)?;
    let out = Output::new(audio, config.channels as usize, config.sample_rate)?;
    let resampled = config.sample_rate != RATE;
    let failed = Arc::new(AtomicBool::new(false));
    let f = failed.clone();
    let stream = match format {
        SampleFormat::F32 => build::<f32>(&device, config, out, f),
        SampleFormat::F64 => build::<f64>(&device, config, out, f),
        SampleFormat::I16 => build::<i16>(&device, config, out, f),
        SampleFormat::I32 => build::<i32>(&device, config, out, f),
        SampleFormat::U16 => build::<u16>(&device, config, out, f),
        SampleFormat::U8 => build::<u8>(&device, config, out, f),
        SampleFormat::I8 => build::<i8>(&device, config, out, f),
        f => Err(Error(format!("unsupported sample format {f}"))),
    }?;
    stream.play().map_err(|e| err("starting the stream", e))?;
    Ok(Stream {
        _stream: stream,
        device: name,
        config,
        format,
        resampled,
        failed,
    })
}
