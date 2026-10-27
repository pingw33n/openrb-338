//! The real-time player (task-043): the engine of `rb-engine` rendered in an audio callback,
//! driven by commands from the UI thread.
//!
//! # Threads
//!
//! [`Player::new`] returns two halves:
//!
//! - [`Player`], the UI side: it builds songs ([`Player::load`]), sends [`Command`]s, drops what
//!   the callback hands back, and reads the [`Snapshot`] the callback publishes.
//! - [`Audio`], the callback side: [`Audio::render_i16`] and [`Audio::render_f32`] are the body
//!   of the audio callback at 44.1 kHz; [`output::start`] runs them in a cpal output stream
//!   (resampled when the device does not offer 44.1 kHz).
//!
//! The halves share nothing but three lock-free single-producer single-consumer channels, all
//! allocated up front: the command queue and the garbage queue (`rtrb`), and the snapshot
//! (`triple_buffer`). The callback takes no lock and does not allocate or free memory: every box
//! it lets go (a replaced song, a data block) goes back to the UI thread through the garbage
//! queue, and the engine's own state is lock-free ([`rb_engine::process`]). The
//! `no_alloc` test checks it with a counting allocator.
//!
//! # Timing, as the original
//!
//! The engine renders in calls of [`CALL_FRAMES`] = 1024 frames, the frames of one MME buffer of
//! the original (`docs/export_state.md` section 2.1), into a buffer the callback drains; the
//! commands queued so far are applied at the start of each call, before it renders, as
//! `SafeSoundCallback` drains the panel's messages before it renders (section 2.3). So a command
//! takes effect at the next 64-frame block after the 1024-frame call that is playing, and the
//! device's own buffer size does not change the engine's output. The original drains with a
//! time budget (at least one message per call); the player drains every queued command.
//!
//! # Songs
//!
//! A song is built on the UI thread ([`Player::build`]: the song constructor, then the load's
//! messages, [`rb_engine::frame::Engine::load`]), sharing the process globals of the songs
//! before it (the drum counters and the delay line, which the original keeps in the DLL), and
//! sent with [`Command::Load`]. The callback makes it current as top-level msg 53 does
//! (`sub_1001BDA4`, `docs/export_state.md` section 3): when the current song is stopped, slot 6
//! runs on the current song (which refills the shared delay line) and the new song becomes
//! current at once; when it plays, the new song is only pending (not rendered), and it becomes
//! current when it is played: msg 35 `{1,1}` on the pending song (`sub_1001BF50` →
//! `sub_1001BE74`) halts the old song, runs its slot 6, starts the new one and makes it
//! current. (The other path into `sub_1001BE74`, `sub_1001C524`, runs only on a MIDI Start
//! message, out of scope.) Every command addresses the newest song, current or pending, as the
//! original's panel addresses its own song. A mod is the sample set a song is built with (how
//! the original changes the mod of an open song is task-056).
//!
//! Not modelled yet: the panel's own device edit messages (task-055; a pattern edit goes as a
//! whole data block meanwhile) and recording (task-057; there is no Record command).

pub mod output;

use std::sync::Arc;

use rb_engine::frame::export::REALTIME_FRAMES;
use rb_engine::frame::{Devices, Engine, MixerParams};
use rb_engine::process::ProcessState;
use rb_engine::samples::SampleSet;
use rb_engine::song::{Device, Song};
use rb_engine::tables::Tables;

/// The engine's output rate.
pub const RATE: u32 = 44_100;
/// The frames of one engine call: one MME buffer of the original.
pub const CALL_FRAMES: usize = REALTIME_FRAMES;
/// The command queue's capacity.
pub const COMMANDS: usize = 1024;
/// The garbage queue's capacity. Each command lets go of at most [`GARBAGE_PER_COMMAND`]
/// boxes, and the callback takes a command only when that many slots are free, so nothing is
/// ever dropped in the callback; a UI that does not collect stalls its commands, not the audio.
pub const GARBAGE: usize = 64;
/// The boxes one command can hand back: a load replaces the pending song and the current one.
pub const GARBAGE_PER_COMMAND: usize = 2;

/// What the UI sends to the newest song (current, or pending: see the crate doc). Each variant
/// names the engine message it stands for.
pub enum Command {
    /// Song msg 35 `{1,1}`: play (on a pending song, the switch of `sub_1001BE74`).
    Play,
    /// Song msg 35 `{1,0}`: stop, then locate to the start of the bar.
    Stop,
    /// Song msg 35 `{8,tick}`: locate.
    Locate(u32),
    /// Song msg 35 `{0,m}`: song mode (`true`) or pattern mode.
    SongMode(bool),
    /// Song msg 35 `{3,v}`.
    LoopOn(bool),
    /// Song msg 35 `{12,v}`, ticks.
    LoopStart(u32),
    /// Song msg 35 `{16,v}`, ticks.
    LoopEnd(u32),
    /// Song msg 35 `{4,v}`, BPM*1000.
    Tempo(u32),
    /// Song msg 35 `{20,v}`.
    Shuffle(u8),
    /// Song msg 35 `{21,v}`: the Vintage sound model.
    Vintage(bool),
    /// A device controller, as the panel's knobs, switches and pattern buttons set it (slot 11;
    /// [`Engine::set_param`]). The pattern (controller 1) and the on/off switch (0) of a 303,
    /// 808 or 909 are latched by the device at the next bar line.
    Param { device: Device, ctrl: u8, value: u8 },
    /// The mixer's master level (msg 145 `{0,v}`), not a controller.
    Master(u8),
    /// A mixer channel's on/off (mute) switch (msg 147 `{c,0,v}`), not a controller.
    ChannelEnabled { channel: u8, on: bool },
    /// A device's whole data block (song msg 41, [`Song::device_payload`]): the parameters and
    /// the patterns. It goes into the pattern-mode copy, and live only in pattern mode. Pattern
    /// edits can be sent this way until the panel's own edit messages are modelled (task-055).
    DeviceData { device: Device, payload: Box<[u8]> },
    /// Make a song built by [`Player::build`] current (top-level msg 53).
    Load(Box<Engine>),
}

/// What the callback hands back to the UI thread to be dropped there.
pub enum Garbage {
    Engine(Box<Engine>),
    Bytes(Box<[u8]>),
}

/// One device's live parameter block ([`rb_engine::frame::SongDevice::live`]), inline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiveBlock {
    len: u8,
    bytes: [u8; LiveBlock::MAX],
}

impl Default for LiveBlock {
    fn default() -> Self {
        LiveBlock {
            len: 0,
            bytes: [0; LiveBlock::MAX],
        }
    }
}

impl LiveBlock {
    /// The longest block, the TR-909's.
    pub const MAX: usize = 33;

    fn set(&mut self, b: &[u8]) {
        let n = b.len().min(Self::MAX);
        self.bytes[..n].copy_from_slice(&b[..n]);
        self.len = n as u8;
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.bytes[..self.len as usize]
    }
}

/// What the UI reads back: the current song's state after the last engine call.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Snapshot {
    /// The frames the engine has rendered (44.1 kHz), a multiple of [`CALL_FRAMES`].
    pub frames: u64,
    /// The songs made current so far (0: the initial empty song).
    pub generation: u32,
    /// A loaded song waits for Play (the current one is playing).
    pub pending: bool,
    /// The song position in ticks (768 per bar).
    pub tick: i32,
    pub playing: bool,
    pub song_mode: bool,
    pub loop_on: bool,
    pub loop_start: u32,
    pub loop_end: u32,
    /// BPM*1000.
    pub tempo: u32,
    pub shuffle: u8,
    pub vintage: bool,
    /// The mixer's live parameters (what the mixer update reads, automation included).
    pub mixer: MixerParams,
    /// The channel meter bytes (0..127) of the last mixer update.
    pub channel_meters: [u8; 4],
    /// The master meter bytes, left and right.
    pub master_meters: [u8; 2],
    /// The pattern each channel device plays (303 #1, 303 #2, 808, 909).
    pub patterns: [Option<u8>; 4],
    /// Every device's live block, by device id (index 0, the mixer, is empty: see `mixer`).
    /// The playing step of the 303s, the 808 and the 909 is their byte 9, 29 and 30.
    pub live: [LiveBlock; 9],
    /// The engine calls that faulted (the PCF's division by zero); a faulting call outputs
    /// silence here (the original leaves the MME buffer as it was).
    pub faulted_calls: u64,
    /// The device callbacks whose resampler failed and that output silence instead
    /// ([`output::Output`]); counted up to the last engine call.
    pub output_errors: u64,
}

/// The UI side.
pub struct Player {
    commands: rtrb::Producer<Command>,
    garbage: rtrb::Consumer<Garbage>,
    snapshot: triple_buffer::Output<Snapshot>,
    tables: Arc<Tables>,
    process: ProcessState,
}

/// The callback side.
pub struct Audio {
    current: Box<Engine>,
    pending: Option<Box<Engine>>,
    commands: rtrb::Consumer<Command>,
    garbage: rtrb::Producer<Garbage>,
    snapshot: triple_buffer::Input<Snapshot>,
    /// The int16 output of the last engine call.
    buf: Box<[i16; 2 * CALL_FRAMES]>,
    /// The frames of `buf` already handed out.
    pos: usize,
    frames: u64,
    generation: u32,
    output_errors: u64,
}

impl Player {
    /// A player whose current song is a new empty song (the song constructor, as the app's
    /// "Untitled song" before a song is opened), with the drum samples `samples` (`None`: silent
    /// drum machines).
    pub fn new(tables: Arc<Tables>, samples: Option<Arc<SampleSet>>) -> (Player, Audio) {
        let process = ProcessState::new();
        let current = Box::new(Engine::new(
            tables.clone(),
            Devices::implemented_with(samples, process.clone()),
        ));
        let (cp, cc) = rtrb::RingBuffer::new(COMMANDS);
        let (gp, gc) = rtrb::RingBuffer::new(GARBAGE);
        let (si, so) = triple_buffer::triple_buffer(&Snapshot::default());
        let mut audio = Audio {
            current,
            pending: None,
            commands: cc,
            garbage: gp,
            snapshot: si,
            buf: Box::new([0; 2 * CALL_FRAMES]),
            pos: CALL_FRAMES,
            frames: 0,
            generation: 0,
            output_errors: 0,
        };
        audio.publish();
        let player = Player {
            commands: cp,
            garbage: gc,
            snapshot: so,
            tables,
            process,
        };
        (player, audio)
    }

    /// The process globals every song of this player shares.
    pub fn process(&self) -> &ProcessState {
        &self.process
    }

    /// A song as the original makes it when a file is opened: the song constructor (msg 52),
    /// then the load's messages (msgs 36, 41, 37, 32), on this player's process globals.
    pub fn build(&self, song: &Song, samples: Option<Arc<SampleSet>>) -> Box<Engine> {
        let mut e = Box::new(Engine::new(
            self.tables.clone(),
            Devices::implemented_with(samples, self.process.clone()),
        ));
        e.load(song);
        e
    }

    /// [`Player::build`] and [`Command::Load`].
    pub fn load(&mut self, song: &Song, samples: Option<Arc<SampleSet>>) -> Result<(), Command> {
        let e = self.build(song, samples);
        self.send(Command::Load(e))
    }

    /// Queues a command (after dropping what the callback handed back). A full queue returns
    /// the command. A data block of the wrong length is refused here (the device would read
    /// past it).
    pub fn send(&mut self, c: Command) -> Result<(), Command> {
        self.collect_garbage();
        if let Command::DeviceData { device, payload } = &c
            && payload.len() != device.data_len()
        {
            return Err(c);
        }
        if let Command::Load(e) = &c {
            assert!(
                e.process_state().same(&self.process),
                "a song for this player is built by Player::build"
            );
        }
        self.commands.push(c).map_err(|rtrb::PushError::Full(c)| c)
    }

    /// Drops what the callback handed back; returns how many boxes.
    pub fn collect_garbage(&mut self) -> usize {
        let mut n = 0;
        while let Ok(g) = self.garbage.pop() {
            drop(g);
            n += 1;
        }
        n
    }

    /// The latest snapshot.
    pub fn snapshot(&mut self) -> &Snapshot {
        self.snapshot.read()
    }
}

impl Audio {
    /// Fills `out` (interleaved stereo int16 at 44.1 kHz, `out.len() / 2` frames) with the
    /// engine's output, running an engine call whenever the last one is used up.
    pub fn render_i16(&mut self, out: &mut [i16]) {
        self.pull(out.len() / 2, |i, l, r| {
            out[2 * i] = l;
            out[2 * i + 1] = r;
        });
    }

    /// [`Audio::render_i16`] as f32 (`x / 32768`, exact).
    pub fn render_f32(&mut self, out: &mut [f32]) {
        self.pull(out.len() / 2, |i, l, r| {
            out[2 * i] = l as f32 / 32768.0;
            out[2 * i + 1] = r as f32 / 32768.0;
        });
    }

    fn pull(&mut self, frames: usize, mut put: impl FnMut(usize, i16, i16)) {
        let mut i = 0;
        while i < frames {
            if self.pos == CALL_FRAMES {
                self.call();
            }
            let n = (CALL_FRAMES - self.pos).min(frames - i);
            for k in 0..n {
                let s = 2 * (self.pos + k);
                put(i + k, self.buf[s], self.buf[s + 1]);
            }
            self.pos += n;
            i += n;
        }
    }

    /// One engine call: the queued commands, then 1024 frames, then the snapshot.
    fn call(&mut self) {
        while self.garbage.slots() >= GARBAGE_PER_COMMAND {
            let Ok(c) = self.commands.pop() else { break };
            self.apply(c);
        }
        // a faulting call writes nothing (`Engine::fault`): silence rather than the last call
        self.buf.fill(0);
        let faults = self.faults();
        self.current.render(&mut self.buf[..], CALL_FRAMES);
        if self.faults() != faults {
            self.buf.fill(0);
        }
        self.pos = 0;
        self.frames += CALL_FRAMES as u64;
        self.publish();
    }

    fn faults(&self) -> u64 {
        self.current.fault().map_or(0, |f| f.calls)
    }

    fn throw(&mut self, g: Garbage) {
        // `call` keeps GARBAGE_PER_COMMAND slots free for every command it applies
        if let Err(rtrb::PushError::Full(g)) = self.garbage.push(g) {
            // unreachable; leak rather than free memory in the callback
            std::mem::forget(g);
        }
    }

    /// The song the commands address: the pending one, else the current one.
    fn target(&mut self) -> &mut Engine {
        match &mut self.pending {
            Some(p) => p,
            None => &mut self.current,
        }
    }

    fn apply(&mut self, c: Command) {
        match c {
            Command::Play => {
                if let Some(new) = self.pending.take() {
                    self.switch(new);
                } else {
                    self.current.play();
                }
            }
            Command::Stop => self.target().stop(),
            Command::Locate(t) => self.target().locate(t),
            Command::SongMode(on) => self.target().set_song_mode(on as u8),
            Command::LoopOn(on) => self.target().set_loop_on(on as u8),
            Command::LoopStart(t) => self.target().set_loop_start(t),
            Command::LoopEnd(t) => self.target().set_loop_end(t),
            Command::Tempo(t) => self.target().set_tempo(t),
            Command::Shuffle(v) => self.target().set_shuffle(v),
            Command::Vintage(on) => self.target().set_vintage(on as u8),
            Command::Param {
                device,
                ctrl,
                value,
            } => self.target().set_param(device, ctrl, value),
            Command::Master(v) => self.target().frame.devices.mixer.set_master(v),
            Command::ChannelEnabled { channel, on } => self
                .target()
                .frame
                .devices
                .mixer
                .set_enabled(channel as usize, on as u8),
            Command::DeviceData { device, payload } => {
                if payload.len() == device.data_len() {
                    self.target().device_data(device, &payload);
                }
                self.throw(Garbage::Bytes(payload));
            }
            Command::Load(new) => self.activate(new),
        }
    }

    /// Top-level msg 53, `sub_1001BDA4`: the new song becomes the pending one (0x1001BDB6); unless
    /// the current song plays (0x1001BDC7), slot 6 runs on the current song (0x1001BDD0; of it
    /// only the delay's refill of the shared line outlives the song) and the new song becomes
    /// current (`sub_1001BD24`).
    fn activate(&mut self, new: Box<Engine>) {
        if let Some(old) = self.pending.take() {
            self.throw(Garbage::Engine(old));
        }
        if self.current.playing() {
            self.pending = Some(new);
        } else {
            self.current.frame.devices.silence_all();
            self.make_current(new);
        }
    }

    /// Msg 35 `{1,1}` on the pending song, `sub_1001BE74`: when the current song plays it is
    /// halted (`sub_100218A0`, 0x1001BEB1) and gets slot 6 (0x1001BECB); the new song is started
    /// (0x1001BF23: the play body 0x1001F7E4) and made current (0x1001BF07).
    fn switch(&mut self, mut new: Box<Engine>) {
        if self.current.playing() {
            self.current.halt();
            self.current.frame.devices.silence_all();
        }
        new.play();
        self.make_current(new);
    }

    fn make_current(&mut self, new: Box<Engine>) {
        let old = std::mem::replace(&mut self.current, new);
        self.throw(Garbage::Engine(old));
        self.generation += 1;
    }

    fn publish(&mut self) {
        let e = &self.current;
        let f = &e.frame;
        let g = &f.seq.globals;
        let s = self.snapshot.input_buffer_mut();
        s.frames = self.frames;
        s.generation = self.generation;
        s.pending = self.pending.is_some();
        s.tick = f.seq.tick_pos();
        s.playing = g.play != 0;
        s.song_mode = g.song_mode != 0;
        s.loop_on = g.loop_on != 0;
        s.loop_start = g.loop_start;
        s.loop_end = g.loop_end;
        s.tempo = g.tempo;
        s.shuffle = g.shuffle;
        s.vintage = f.vintage;
        s.mixer = f.devices.mixer.live;
        for (m, ch) in s.channel_meters.iter_mut().zip(&f.state.channels) {
            *m = ch.meter;
        }
        s.master_meters = f.state.master_meter;
        for (p, d) in s.patterns.iter_mut().zip(&f.devices.channels) {
            *p = d.playing_pattern();
        }
        let d = &f.devices;
        s.live[0] = LiveBlock::default();
        for (c, dev) in d.channels.iter().enumerate() {
            s.live[1 + c].set(dev.live());
        }
        s.live[Device::Delay as usize].set(d.delay.live());
        s.live[Device::Distortion as usize].set(d.dist.live());
        s.live[Device::Pcf as usize].set(d.pcf.live());
        s.live[Device::Compressor as usize].set(d.comp.live());
        s.faulted_calls = e.fault().map_or(0, |f| f.calls);
        s.output_errors = self.output_errors;
        self.snapshot.publish();
    }

    /// The current song (for tests and diagnostics; the UI reads the [`Snapshot`]).
    pub fn current(&self) -> &Engine {
        &self.current
    }

    /// A device callback output silence because the resampler failed.
    pub(crate) fn output_error(&mut self) {
        self.output_errors += 1;
    }
}
