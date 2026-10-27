//! The engine frame: the 64-frame block loop, the mixer, the int16 output conversion
//! (`docs/initial_investigation.md` sections 3 and 4) and, in [`export`], the export driver.
//!
//! The DLL's song object (ctor `sub_10011350`) renders through `sub_100139E0` (one
//! `SafeSoundCallback` worth of frames, the leftover buffer and the int16 conversion), which calls
//! the mixer block `sub_10013444` once per 64 frames; every 16 blocks that block runs the mixer
//! update `sub_10012C14`. [`Engine`] is that song object: [`Engine::render`] is `sub_100139E0`,
//! [`Frame::block`] is `sub_10013444`, [`Frame::mixer_update`] is `sub_10012C14`.
//!
//! The channel devices, the insert effects and the send effect plug in through the traits
//! [`ChannelDevice`], [`InsertEffect`] and [`SendEffect`], which share the slots the sequencer
//! and the song messages call, [`SongDevice`]; [`Silent`], [`Bypass`] and [`IdleSend`] are the
//! placeholders until their tasks implement them. The mixer device is [`MixerDevice`], and the
//! sequencer ([`crate::seq::Sequencer`]) runs at the start of every block.
//!
//! Every computation follows the crate's numerics rules: f64 arithmetic in the original's operand
//! order, an f32 rounding at every `fstp dword`/`fst dword`, no FMA.

pub mod export;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use crate::process::ProcessState;
use crate::samples::SampleSet;
use crate::seq::Sequencer;
use crate::song;
use crate::tables::Tables;

/// Frames per block.
pub const BLOCK: usize = 64;
/// One block of one mono bus.
pub type Block = [f32; BLOCK];
/// The denormal guard every bus is filled with before it accumulates (`0x358637BD`, the
/// immediate at 0x10013471 and 0x10013AD3).
pub const GUARD: f32 = f32::from_bits(0x3586_37BD);
/// The most frames one [`Engine::render`] call renders (`cmp eax, 4000h` at 0x100139ED).
pub const MAX_FRAMES: usize = 0x4000;
/// The mixer runs its update when `counter & 15 == 0` (0x10013453).
pub const UPDATE_PERIOD: u32 = 16;
/// The block counter's constructor value (0x1001145A).
pub const COUNTER_INIT: u32 = 10;

// The constants of the mixer block, update and conversion, decoded from their bytes.
/// `dbl_10151F48`: the smoothing's "keep" factor.
const KEEP: f64 = f64::from_bits(0x3FED_C28F_5C28_F5C3); // 0.93
/// `dbl_10151F50`: the smoothing's "take" factor.
const TAKE: f64 = f64::from_bits(0x3FB1_EB85_1EB8_51EC); // 0.07
/// `dbl_10151F58`: 1/32, the per-pair ramp step factor.
const RAMP: f64 = f64::from_bits(0x3FA0_0000_0000_0000);
/// `dbl_10151F70`: the int16 scale.
const SCALE: f64 = 32768.0;
/// `flt_10151F78`, `flt_10151F7C`: the int16 clamp.
const CLAMP_HI: f32 = f32::from_bits(0x46FF_FE00); // 32767.0
const CLAMP_LO: f32 = f32::from_bits(0xC700_0000); // -32768.0
/// `flt_10151F30`, `flt_10151F44`: the 1e-6 the update adds to every target.
const OFFSET: f32 = GUARD;
/// `dbl_10151F20`: the Vintage send's factor.
const VINTAGE_SEND: f64 = 0.5;
/// `dbl_10151F28`: 1e-6 as an f32 widened to f64 (`0x3EB0C6F7A0000000`).
const VINTAGE_OFFSET: f64 = f64::from_bits(0x3EB0_C6F7_A000_0000);
/// `flt_10151F34`: the meters' full scale.
const METER_FULL: f32 = 1.0;
/// `flt_10151F38`: the meters' decay per update.
const METER_DECAY: f32 = f32::from_bits(0x3F6B_851F); // 0.92
/// `flt_10151F3C`: the meter byte scale.
const METER_SCALE: f32 = 127.0;
/// `flt_10151F40`: the master gain factor.
const MASTER_SCALE: f32 = 2.0;
/// The meter index step per block (`add eax, 17h` at 0x10013481).
const METER_STEP: u16 = 23;

/// What every device and effect may read while it renders a block.
pub struct BlockContext<'a> {
    pub tables: &'a Tables,
    /// The song's meter sample index (`song+0xA0`), already advanced for this block; the
    /// devices' and effects' peak meters read their bus at this index (e.g. the delay,
    /// `sub_10003AD0(this)+160` in 0x1000F940).
    pub meter_index: usize,
    /// The song's tempo, BPM*1000 (`sub_1001FCF0`), which the delay's update reads.
    pub tempo: u32,
    /// The tick of the song's position (`sub_1001F870`), already advanced by this block's
    /// sequencer step; the PCF's pattern step follows it (0x10016756).
    pub tick: i32,
}

/// A block aborted by a CPU exception in the original: the PCF's division by a zero pattern
/// length (0x10016811). `SafeSoundCallback` (0x1001AE40) runs the render inside a C++
/// `try` with a `catch (...)` (FuncInfo 0x10152AD8, handler 0x1001B055), which catches the
/// divide error (the all-zero references of `pcf_*_wave127` show it); it resumes at 0x1001B03D,
/// past both the render and the zero fill of the buffer, so the call returns with the int16
/// buffer as the caller left it and with the song's state as far as the block got.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    /// The PCF's step `q % len` (`idiv` at 0x10016811) with a pattern length of 0: the 17
    /// waves 55, 111 and 113..127 read one past the 55 patterns ([`crate::pcf::pattern`]).
    PcfZeroLength { wave: u8 },
}

impl std::fmt::Display for Fault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Fault::PcfZeroLength { wave } => write!(
                f,
                "the PCF's wave {wave} has pattern length 0 and divides by zero (0x10016811)"
            ),
        }
    }
}

/// The faults of an [`Engine`]'s render calls: the first one and how many calls faulted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FaultRecord {
    /// The first fault.
    pub first: Fault,
    /// The block it ended, counted from 0 over every block the engine started
    /// ([`Frame::blocks`]).
    pub block: u64,
    /// The render calls that faulted (each left its int16 buffer unwritten).
    pub calls: u64,
}

/// The device vtable slots that the sequencer ([`crate::seq`]) and the song messages call
/// (`CDevice`, vtable 0x10032DB4; the 303 0x10033FD4, the 808 0x10034154, the 909 0x10034294).
///
/// **Timing.** Every call arrives between two blocks: the sequencer runs at the start of a block,
/// after the mixer update and before any device renders (0x1001346A), and song messages are
/// applied at the start of a `SafeSoundCallback` call. A step, a note and a parameter change
/// therefore take effect at sample 0 of the next block the device renders; the original has no
/// sub-block positions. Within one block the sequencer makes every set-param call (track order,
/// then list order) before any step call (song order).
///
/// Slots 1..3 are no-ops (`retn` at 0x10010380, 0x1001BF40, 0x10012C00) for the mixer, the delay
/// and the inserts; only the 303s, the 808 and the 909 override them.
///
/// `Send`, so that a whole [`Engine`] can be built on one thread and rendered on another (the
/// real-time player builds a song on the UI thread and renders it in the audio callback).
pub trait SongDevice: Send {
    /// Slot 1: the clock reached a new tick (0x1001F464): `tick` is `trunc(pos)`, `shuffled` its
    /// shuffled tick ([`crate::seq::shuffled`]), `tempo` the song's BPM*1000. Called in every
    /// block whose tick passed the last one, playing, in either mode; the 303's step function
    /// uses `shuffled` when the pattern's shuffle flag is set (0x100253D0).
    fn step(&mut self, _tick: i32, _shuffled: i32, _tempo: i32) {}
    /// Slot 2: the position moved to `tick` (locate, the loop wrap, pattern mode). `now` is
    /// `trunc(pos)` of the song's position at the call (`sub_1001F870`), which the 303's slot 2
    /// reads instead of its argument (0x100255E9); it differs from `tick` at the loop wrap and
    /// when pattern mode is entered.
    fn relocate(&mut self, _tick: i32, _now: i32) {}
    /// Slot 3: play started (0x1001F7E4); `now` is `trunc(pos)` (the 303 reads it, 0x100255B9).
    fn start(&mut self, _now: i32) {}
    /// Slot 4: stop (`sub_10021840`, only when the song was playing).
    fn stop(&mut self) {}
    /// Slot 5: msg 33's reset (0x100141D2).
    fn reset(&mut self) {}
    /// Slot 6: note-off and silence (song switch, `sub_10013D40`). The delay's slot 6
    /// (0x100100E0) clears the process-global delay buffer only.
    fn silence(&mut self) {}
    /// Slot 9: the song mode changed (`sub_10013FB0`).
    fn set_song_mode(&mut self, _on: bool) {}
    /// Slot 10 with the device's data `sub` (song msg 41): the parameter and pattern block of
    /// `song::Song::device_payload`. `song_mode` is the global byte 0 (`sub_1001FCB0`), which
    /// decides whether the block also goes live (`docs/song_load.md` section 4).
    fn data(&mut self, _payload: &[u8], _song_mode: bool) {}
    /// Slot 11: set controller `ctrl` (a track event, the chase).
    fn set_param(&mut self, _ctrl: u8, _value: u8) {}
    /// Not a slot: the song's sound-model byte (`song+0x16D`, msg 36 byte 21) changed. The 303
    /// reads the byte at note-on, note-off and in its audio (`sub_10013DA0()+0x15`).
    fn set_sound_model(&mut self, _vintage: bool) {}
    /// Slots 7 and 8, called once on every device by the song ctor (0x10011941, 0x10011985,
    /// slot 8 with tick 0) with the device's own track: the 303's set the track's quantisation
    /// and insert its initial events.
    fn init_track(&mut self, _track: &mut crate::seq::Track) {}
    /// Not a slot: the live parameter block, the bytes the audio reads and the panel polls
    /// (controllers by number, plus the bytes the device writes itself: the 303's, 808's and
    /// 909's last step at 9, 29 and 30, the delay's meter at 5). Empty for a placeholder; the
    /// mixer's live set is [`MixerDevice::live`].
    fn live(&self) -> &[u8] {
        &[]
    }
}

/// A mixer channel's sound source: TB-303 #1, #2, TR-808 or TR-909.
pub trait ChannelDevice: SongDevice {
    /// Vtable slot 14 (0x10013580): render one block into `bus`, overwriting all of it.
    fn process(&mut self, bus: &mut Block, ctx: &BlockContext);
    /// Not a slot: the pattern the device plays (the 303's `+0xC94`, the 808's `+0x18D0`, the
    /// 909's `+0x18D3`), latched from controller 1 at a bar line. `None` for a placeholder.
    fn playing_pattern(&self) -> Option<u8> {
        None
    }
}

/// An insert effect: the distortion, the PCF or the compressor.
pub trait InsertEffect: SongDevice {
    /// Slot 15 (0x100135C3): process a channel bus in place.
    fn process(&mut self, bus: &mut Block, ctx: &BlockContext);
    /// Slot 14 (0x100138AC): process the master in place (only the compressor sits there).
    fn process_stereo(&mut self, _l: &mut Block, _r: &mut Block, _ctx: &BlockContext) {}
    /// Slot 16 (0x100130B2): the parameter update, run by every mixer update.
    fn update(&mut self, _ctx: &BlockContext) {}
    /// Not a slot: the fault of the last [`InsertEffect::process`], if it faulted; reading it
    /// clears it. Only the PCF faults.
    fn faulted(&mut self) -> Option<Fault> {
        None
    }
}

/// The send effect (the delay).
pub trait SendEffect: SongDevice {
    /// Slot 14 (0x10013865): read the send bus, add into the master and refill the send bus with
    /// [`GUARD`] (the delay does that in both its enabled and disabled paths, 0x1000F940).
    fn process(&mut self, send: &mut Block, l: &mut Block, r: &mut Block, ctx: &BlockContext);
}

/// The song's devices, in the roles the mixer gives them.
pub struct Devices {
    /// Device 0, the mixer device (`CMixerDevice`, vtable 0x10033214), whose parameters the mixer
    /// update reads.
    pub mixer: MixerDevice,
    /// Channel c's source: 0 TB-303 #1, 1 TB-303 #2, 2 TR-808, 3 TR-909 (`song+0x1C8` list,
    /// filled by `sub_10014064` in the ctor's order).
    pub channels: [Box<dyn ChannelDevice>; 4],
    pub dist: Box<dyn InsertEffect>,
    pub pcf: Box<dyn InsertEffect>,
    pub comp: Box<dyn InsertEffect>,
    pub delay: Box<dyn SendEffect>,
    /// The process globals the devices share (the drum counters); a caller may preset them
    /// before an export ([`ProcessState::set_drum_counters`]).
    pub process: ProcessState,
}

impl Devices {
    /// Placeholders everywhere: silent sources, bypassing inserts, an idle send.
    pub fn placeholders() -> Self {
        Devices {
            mixer: MixerDevice::default(),
            channels: std::array::from_fn(|_| Box::new(Silent) as Box<dyn ChannelDevice>),
            dist: Box::new(Bypass),
            pcf: Box::new(Bypass),
            comp: Box::new(Bypass),
            delay: Box::new(IdleSend),
            process: ProcessState::new(),
        }
    }

    /// The engine's devices: the two TB-303s ([`crate::tb303::Tb303`]), the distortion
    /// ([`crate::dist::Distortion`]), the PCF ([`crate::pcf::Pcf`]), the compressor
    /// ([`crate::comp::Compressor`]), the delay ([`crate::delay::Delay`]) and, given the drum
    /// samples, the TR-808 ([`crate::tr808::Tr808`]) and the TR-909 ([`crate::tr909::Tr909`]);
    /// without samples silent drum machines (exact for a song whose drum machines play no
    /// step).
    pub fn implemented(samples: Option<Arc<SampleSet>>) -> Self {
        Self::implemented_with(samples, ProcessState::new())
    }

    /// [`Devices::implemented`] sharing the process globals `process` (the drum counters and
    /// the delay line) with the songs already built on it: in the original every song of the
    /// process shares the DLL's globals ([`crate::process`]).
    pub fn implemented_with(samples: Option<Arc<SampleSet>>, process: ProcessState) -> Self {
        let mut d = Self::placeholders();
        d.process = process;
        d.channels[0] = Box::new(crate::tb303::Tb303::new());
        d.channels[1] = Box::new(crate::tb303::Tb303::new());
        d.dist = Box::new(crate::dist::Distortion::new());
        d.pcf = Box::new(crate::pcf::Pcf::new());
        d.comp = Box::new(crate::comp::Compressor::new());
        d.delay = Box::new(crate::delay::Delay::new(d.process.clone()));
        if let Some(s) = samples {
            d.channels[2] = Box::new(crate::tr808::Tr808::new(s.clone(), d.process.clone()));
            d.channels[3] = Box::new(crate::tr909::Tr909::new(s, d.process.clone()));
        }
        d
    }

    /// The device with id `d` (`sub_10011E60`, `sub_10012B40`: the device list searched by id).
    pub fn device_mut(&mut self, d: song::Device) -> &mut dyn SongDevice {
        use song::Device as D;
        match d {
            D::Mixer => &mut self.mixer,
            D::Tb303_1 => &mut *self.channels[0],
            D::Tb303_2 => &mut *self.channels[1],
            D::Tr808 => &mut *self.channels[2],
            D::Tr909 => &mut *self.channels[3],
            D::Delay => &mut *self.delay,
            D::Distortion => &mut *self.dist,
            D::Pcf => &mut *self.pcf,
            D::Compressor => &mut *self.comp,
        }
    }

    /// Calls `f` on every device in the song's device order (`song+0x18C`, `song+0x1C4`
    /// entries, filled in the ctor's order: mixer, 303, 303, 808, 909, delay, dist, PCF, comp),
    /// which every loop over the devices uses.
    pub fn for_each(&mut self, mut f: impl FnMut(&mut dyn SongDevice)) {
        f(&mut self.mixer);
        for d in &mut self.channels {
            f(&mut **d);
        }
        f(&mut *self.delay);
        f(&mut *self.dist);
        f(&mut *self.pcf);
        f(&mut *self.comp);
    }

    /// Slot 4 on every device.
    pub fn stop_all(&mut self) {
        self.for_each(|d| d.stop());
    }

    /// Slot 5 on every device (msg 33, 0x100141BA..0x100141E5).
    pub fn reset_all(&mut self) {
        self.for_each(|d| d.reset());
    }

    /// Slot 6 on every device (`sub_10013D40`).
    pub fn silence_all(&mut self) {
        self.for_each(|d| d.silence());
    }
}

/// A placeholder source that outputs zeros: what an idle TR-808 or TR-909 outputs (their audio
/// slots 0x100280B0 and 0x1002B6F0 zero the bus, and a voice that is not active adds nothing:
/// the tests of its byte `+8` at 0x1001E4D7, 0x1001E707, 0x1001E9A7; task-023).
pub struct Silent;
impl SongDevice for Silent {}
impl ChannelDevice for Silent {
    fn process(&mut self, bus: &mut Block, _ctx: &BlockContext) {
        bus.fill(0.0);
    }
}

/// A placeholder insert that leaves the signal alone.
pub struct Bypass;
impl SongDevice for Bypass {}
impl InsertEffect for Bypass {
    fn process(&mut self, _bus: &mut Block, _ctx: &BlockContext) {}
}

/// A placeholder send effect that only refills the send bus, as the delay does
/// (0x1000F940; the disabled delay adds nothing to the master).
pub struct IdleSend;
impl SongDevice for IdleSend {}
impl SendEffect for IdleSend {
    fn process(&mut self, send: &mut Block, _l: &mut Block, _r: &mut Block, _ctx: &BlockContext) {
        send.fill(GUARD);
    }
}

/// One mixer channel's parameters (the mixer device's channel block, `sub_10015D10`: mixer
/// `+0x40 + 6c`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelParams {
    pub enabled: u8,
    pub level: u8,
    pub pan: u8,
    pub send: u8,
    pub dist: u8,
}

impl Default for ChannelParams {
    /// The mixer device ctor's values (`sub_10019D50`): enabled, level 100, centered, no send,
    /// no distortion.
    fn default() -> Self {
        ChannelParams {
            enabled: 1,
            level: 100,
            pan: 64,
            send: 0,
            dist: 0,
        }
    }
}

/// The mixer device's parameters, which the mixer update reads (`sub_10015D30`: mixer `+0x30`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MixerParams {
    pub master: u8,
    /// 0 off, 1 the master, 2..5 channel 0..3.
    pub comp_id: u8,
    /// 0 off, 1..4 channel 0..3.
    pub pcf_id: u8,
    pub channels: [ChannelParams; 4],
}

impl Default for MixerParams {
    /// The mixer device ctor's values (`sub_10015400` → `sub_10019CC0`: master 100, comp and PCF
    /// off).
    fn default() -> Self {
        MixerParams {
            master: 100,
            comp_id: 0,
            pcf_id: 0,
            channels: [ChannelParams::default(); 4],
        }
    }
}

impl From<&song::Mixer> for MixerParams {
    fn from(m: &song::Mixer) -> Self {
        MixerParams {
            master: m.master,
            comp_id: m.comp_id,
            pcf_id: m.pcf_id,
            channels: m.channels.map(|c| ChannelParams {
                enabled: c.enabled,
                level: c.level,
                pan: c.pan,
                send: c.delay_send,
                dist: c.dist,
            }),
        }
    }
}

impl MixerParams {
    /// The msg 41 `0/143` block (`sub_10019DE0`): master, comp id, PCF id, two unused bytes,
    /// then per channel enabled, level, pan, send, dist and an unused byte.
    fn from_payload(b: &[u8]) -> Self {
        let mut p = MixerParams {
            master: b[0],
            comp_id: b[1],
            pcf_id: b[2],
            channels: [ChannelParams::default(); 4],
        };
        for (c, ch) in p.channels.iter_mut().enumerate() {
            let o = 5 + 6 * c;
            *ch = ChannelParams {
                enabled: b[o],
                level: b[o + 1],
                pan: b[o + 2],
                send: b[o + 3],
                dist: b[o + 4],
            };
        }
        p
    }
}

/// The mixer device (`CMixerDevice`, vtable 0x10033214, ctor `sub_10015400`): the live
/// parameters at `+0x30`, which the mixer update reads, and the pattern-mode copy at `+0x58`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MixerDevice {
    pub live: MixerParams,
    pub pattern: MixerParams,
}

impl SongDevice for MixerDevice {
    /// Slot 9, 0x10016280: entering song mode saves the live set into the pattern copy; leaving
    /// it carries the live master and channel enables into the pattern copy, then restores the
    /// live set from it.
    fn set_song_mode(&mut self, on: bool) {
        if on {
            self.pattern = self.live;
        } else {
            self.pattern.master = self.live.master;
            for c in 0..4 {
                self.pattern.channels[c].enabled = self.live.channels[c].enabled;
            }
            self.live = self.pattern;
        }
    }

    /// Msg 143, 0x1001588F: the block goes into the pattern copy; then song mode copies only the
    /// master and the channel enables live (no controller reaches them), pattern mode all of it.
    fn data(&mut self, payload: &[u8], song_mode: bool) {
        self.pattern = MixerParams::from_payload(payload);
        if song_mode {
            self.live.master = self.pattern.master;
            for c in 0..4 {
                self.live.channels[c].enabled = self.pattern.channels[c].enabled;
            }
        } else {
            self.live = self.pattern;
        }
    }

    /// Slot 8, 0x100160F0, with tick 0 from the song ctor (slot 7 is the no-op 0x1000F4E0):
    /// the pattern copy's compressor id (controller 1, `+0x5C`) and PCF id (2, `+0x60`), then
    /// for each channel c its level, pan, send and dist (`6..9 + 6c`, `+0x69..+0x6C + 6c`)
    /// inserted into the mixer's track (`sub_1001F900`). The master and the enables have no
    /// controller.
    fn init_track(&mut self, track: &mut crate::seq::Track) {
        let p = self.pattern;
        track.insert(0, 1, p.comp_id);
        track.insert(0, 2, p.pcf_id);
        for (c, ch) in p.channels.iter().enumerate() {
            let base = 5 + 6 * c as u8;
            track.insert(0, base + 1, ch.level);
            track.insert(0, base + 2, ch.pan);
            track.insert(0, base + 3, ch.send);
            track.insert(0, base + 4, ch.dist);
        }
    }

    /// Slot 11, 0x10015C00: controller 1 is the compressor id, 2 the PCF id, and `5 + 6c + k`
    /// sets channel c's field k (1 level, 2 pan, 3 send, 4 dist); field 0 (enabled) and the
    /// controllers below 5 are not set. The exe accepts only 1, 2 and `6..9 + 6c` (c = 0..3) on
    /// track 0 at load (`sub_42F8F0`, `docs/song_load.md` section 5; `load::control_range`), so
    /// c is at most 3 (a larger c would write past the channel blocks in the original).
    fn set_param(&mut self, ctrl: u8, value: u8) {
        match ctrl {
            1 => self.live.comp_id = value,
            2 => self.live.pcf_id = value,
            _ => {
                let x = ctrl as i32 - 5;
                let (c, k) = ((x / 6) as u8 as usize, (x % 6) as u8);
                if let Some(ch) = self.live.channels.get_mut(c) {
                    match k {
                        1 => ch.level = value,
                        2 => ch.pan = value,
                        3 => ch.send = value,
                        4 => ch.dist = value,
                        _ => {}
                    }
                }
            }
        }
    }
}

impl MixerDevice {
    /// Msg 145 `{0, v}` (0x10015928): the live master level. It has no controller, so the
    /// record hook is not called.
    pub fn set_master(&mut self, v: u8) {
        self.live.master = v;
    }

    /// Msg 147 `{c, 0, v}` (0x10015A12): channel c's live enabled byte (the panel's mute). It
    /// has no controller, so the record hook is not called. A channel above 3 is ignored (the
    /// original would write past the channel blocks; the panel sends only 0..3).
    pub fn set_enabled(&mut self, c: usize, v: u8) {
        if let Some(ch) = self.live.channels.get_mut(c) {
            ch.enabled = v;
        }
    }
}

/// The insert slots a mixer channel routes its bus through, in processing order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Routing {
    /// `+0xC4` of the channel block.
    pub dist: bool,
    /// `+0xC8`.
    pub pcf: bool,
    /// `+0xCC`.
    pub comp: bool,
}

/// One mixer channel's state: the 44-byte block at `song+0xA4 + 44c`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChannelState {
    /// `+0xA8`: the left gain's target.
    pub target_l: f32,
    /// `+0xA4`: the right gain's target.
    pub target_r: f32,
    /// `+0xB0`: the left gain at the start of the next block.
    pub gain_l: f32,
    /// `+0xAC`.
    pub gain_r: f32,
    /// `+0xB4`: the send gain (no smoothing).
    pub send: f32,
    /// `+0xB8`: the peak meter (display only).
    pub peak: f32,
    /// The meter byte the update writes into the mixer device's channel block (`+5`).
    pub meter: u8,
    pub routing: Routing,
}

impl Default for ChannelState {
    /// The song ctor's values (0x100113AF..0x100113D1): every gain 1e-6, no peak, no insert.
    fn default() -> Self {
        ChannelState {
            target_l: GUARD,
            target_r: GUARD,
            gain_l: GUARD,
            gain_r: GUARD,
            send: GUARD,
            peak: 0.0,
            meter: 0,
            routing: Routing::default(),
        }
    }
}

/// The mixer's state in the song object.
#[derive(Debug, Clone, PartialEq)]
pub struct MixerState {
    /// `song+0x188`: the block counter; the update runs when `counter & 15 == 0`.
    pub counter: u32,
    /// `song+0xA0`: the meter sample index, `(i + 23) & 63` per block.
    pub meter_index: u16,
    pub channels: [ChannelState; 4],
    /// `song+0x17C`.
    pub master_target: f32,
    /// `song+0x180`.
    pub master: f32,
    /// `song+0x184` non-null: the compressor on the master.
    pub master_comp: bool,
    /// `song+0`, `song+4`: the master peak meters (display only).
    pub master_peak: [f32; 2],
    /// The master meter bytes (mixer device `+0x3C`, `+0x3D`).
    pub master_meter: [u8; 2],
    /// `song+0x160C`: the send bus (the delay's input), filled with 1e-6 at load and by msg 33.
    pub send: Block,
}

impl Default for MixerState {
    /// The song ctor's values, before its one mixer update (0x100119D4).
    fn default() -> Self {
        MixerState {
            counter: COUNTER_INIT,
            meter_index: 0,
            channels: [ChannelState::default(); 4],
            master_target: 1.0,
            master: 1.0,
            master_comp: false,
            master_peak: [0.0; 2],
            master_meter: [0; 2],
            send: [GUARD; BLOCK],
        }
    }
}

/// The output side of `sub_100139E0`: the leftover block and its position.
#[derive(Debug, Clone, PartialEq)]
pub struct OutputState {
    /// `song+0x208`: `L > 0` means samples `L..64` of the leftover block are still to be output.
    pub leftover_pos: i32,
    /// `song+0x20C`, channels 0 (L) and 1 (R) of the 18 (the others are never enabled).
    pub leftover: [Block; 2],
}

impl Default for OutputState {
    /// The song ctor's values (0x10011406, 0x10011519): position 0, zeros.
    fn default() -> Self {
        OutputState {
            leftover_pos: 0,
            leftover: [[0.0; BLOCK]; 2],
        }
    }
}

/// The frame's whole state, which a caller may set to reproduce a capture protocol
/// (`docs/export_state.md`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EngineState {
    pub mixer: MixerState,
    pub output: OutputState,
}

/// The mixer and its devices: everything one block touches.
pub struct Frame {
    pub state: MixerState,
    /// `song+0x16D`, the sound-model byte non-zero: the pan-dependent send.
    pub vintage: bool,
    pub devices: Devices,
    /// `song+8`, run by every block.
    pub seq: Sequencer,
    tables: Arc<Tables>,
    /// `unk_1010012C`: the channel bus every device renders into in turn (a process global).
    bus: Block,
    /// Not in the original: the blocks started so far, for [`FaultRecord::block`].
    pub blocks: u64,
}

fn st(x: f64) -> f32 {
    x as f32
}

fn abs_bits(x: f32) -> f32 {
    // `cmp dword, 80000000h; jbe` then `fchs`: the sign cleared for every negative value
    f32::from_bits(x.to_bits() & 0x7FFF_FFFF)
}

/// The meter byte: `0x7F` above full scale, else `trunc(x * 127)` (`fistp` with truncation; the
/// low byte is stored).
fn meter_byte(x: f64, full: bool) -> u8 {
    if full {
        0x7F
    } else {
        (x * METER_SCALE as f64) as i64 as u8
    }
}

impl Frame {
    pub fn new(tables: Arc<Tables>, devices: Devices) -> Self {
        Frame {
            state: MixerState::default(),
            vintage: false,
            devices,
            seq: Sequencer::default(),
            tables,
            bus: [0.0; BLOCK],
            blocks: 0,
        }
    }

    pub fn tables(&self) -> &Tables {
        &self.tables
    }

    /// The mixer update `sub_10012C14`, every 16 blocks and once at the end of the song ctor.
    pub fn mixer_update(&mut self) {
        let t = &self.tables.level;
        let pan = &self.tables.pan;
        let p = self.devices.mixer.live;
        let ctx = BlockContext {
            tables: &self.tables,
            meter_index: self.state.meter_index as usize,
            tempo: self.seq.globals.tempo,
            tick: self.seq.tick_pos(),
        };
        for (c, ch) in self.state.channels.iter_mut().enumerate() {
            let cp = p.channels[c];
            let lev = if cp.enabled != 0 { cp.level } else { 0 } as usize;
            let pg = pan[lev][cp.pan as usize];
            // 0x10012C8C..0x10012CBC: fld 1e-6; fadd PAN[0] → +0xA8; fadd PAN[1] → +0xA4
            ch.target_l = st(OFFSET as f64 + pg[0] as f64);
            ch.target_r = st(OFFSET as f64 + pg[1] as f64);
            let ts = t[cp.send as usize] as f64;
            ch.send = if !self.vintage {
                // 0x10012CCD: T[lev] * T[send] + 1e-6
                st(t[lev] as f64 * ts + OFFSET as f64)
            } else {
                // 0x10012CF2: (PAN[0] + PAN[1]) * T[send] * 0.5 + 1e-6
                st((pg[0] as f64 + pg[1] as f64) * ts * VINTAGE_SEND + VINTAGE_OFFSET)
            };
            // 0x10012D1A..0x10012D96: the meter byte, then the peak decays
            let m = t[lev] as f64 * ch.peak as f64;
            ch.meter = meter_byte(m, m > METER_FULL as f64);
            ch.peak = st(ch.peak as f64 * METER_DECAY as f64);
            ch.routing = Routing {
                dist: cp.dist != 0,
                pcf: p.pcf_id as usize == c + 1,
                comp: p.comp_id as usize == c + 2,
            };
        }
        // 0x100130A4: slot 16 of every insert, in list order (`song+0x1EC`: dist, PCF, comp)
        self.devices.dist.update(&ctx);
        self.devices.pcf.update(&ctx);
        self.devices.comp.update(&ctx);
        // 0x100130D7: 2 * T[master] + 1e-6
        self.state.master_target =
            st(MASTER_SCALE as f64 * t[p.master as usize] as f64 + OFFSET as f64);
        self.state.master_comp = p.comp_id == 1;
        // 0x10013189..0x100131FD: the master meters (an integer compare of the float bits)
        for k in 0..2 {
            let pk = self.state.master_peak[k];
            if pk.to_bits() as i32 > 1.0f32.to_bits() as i32 {
                self.state.master_peak[k] = 1.0;
                self.state.master_meter[k] = 0x7F;
            } else {
                self.state.master_meter[k] = meter_byte(pk as f64, false);
            }
        }
        for k in 0..2 {
            self.state.master_peak[k] = st(self.state.master_peak[k] as f64 * METER_DECAY as f64);
        }
    }

    /// The mixer block `sub_10013444`: one block into the master buses `l` and `r`. A
    /// [`Fault`] ends it where the original's exception does, right after the faulting insert:
    /// the rest of the block (later channels, the send, the master) does not run.
    pub fn block(&mut self, l: &mut Block, r: &mut Block) -> Result<(), Fault> {
        self.blocks += 1;
        if self.state.counter & (UPDATE_PERIOD - 1) == 0 {
            self.mixer_update();
        }
        self.state.counter = self.state.counter.wrapping_add(1);
        self.seq.tick(&mut self.devices);
        let mi = (self.state.meter_index + METER_STEP) & 63;
        self.state.meter_index = mi;
        let mi = mi as usize;
        l.fill(GUARD);
        r.fill(GUARD);
        let ctx = BlockContext {
            tables: &self.tables,
            meter_index: mi,
            tempo: self.seq.globals.tempo,
            tick: self.seq.tick_pos(),
        };
        let bus = &mut self.bus;
        for (c, ch) in self.state.channels.iter_mut().enumerate() {
            self.devices.channels[c].process(bus, &ctx);
            let inserts = [
                (ch.routing.dist, &mut self.devices.dist),
                (ch.routing.pcf, &mut self.devices.pcf),
                (ch.routing.comp, &mut self.devices.comp),
            ];
            for (routed, ins) in inserts {
                if routed {
                    ins.process(bus, &ctx);
                    if let Some(f) = ins.faulted() {
                        return Err(f);
                    }
                }
            }
            // 0x100135FA: the channel's peak meter
            let v = abs_bits(bus[mi]);
            if v > ch.peak {
                ch.peak = v;
            }
            // 0x10013650: send += bus * sendGain
            let sg = ch.send as f64;
            for (s, &b) in self.state.send.iter_mut().zip(bus.iter()) {
                *s = st(b as f64 * sg + *s as f64);
            }
            // 0x100136B7: smoothing, n = old*0.93 + target*0.07 stored as f32; the ramp steps
            // from the stored old value by (n_unrounded - old)/32 every two samples
            let old_l = ch.gain_l as f64;
            let old_r = ch.gain_r as f64;
            let new_l = old_l * KEEP + ch.target_l as f64 * TAKE;
            let new_r = old_r * KEEP + ch.target_r as f64 * TAKE;
            ch.gain_l = st(new_l);
            ch.gain_r = st(new_r);
            let dl = (new_l - old_l) * RAMP;
            let dr = (new_r - old_r) * RAMP;
            // 0x10013742: the pan and mix loop
            let (mut gl, mut gr) = (old_l, old_r);
            for i in (0..BLOCK).step_by(2) {
                for j in i..i + 2 {
                    let b = bus[j] as f64;
                    l[j] = st(b * gl + l[j] as f64);
                    r[j] = st(b * gr + r[j] as f64);
                }
                gl += dl;
                gr += dr;
            }
        }
        // 0x10013844: the send effect
        self.devices.delay.process(&mut self.state.send, l, r, &ctx);
        // 0x10013883: the compressor on the master
        if self.state.master_comp {
            self.devices.comp.process_stereo(l, r, &ctx);
        }
        // 0x100138AE: the master gain, constant over the block, unrounded
        let m = self.state.master as f64 * KEEP + self.state.master_target as f64 * TAKE;
        self.state.master = st(m);
        for x in l.iter_mut().chain(r.iter_mut()) {
            *x = st(*x as f64 * m);
        }
        // 0x10013960: the master peak meters
        for (k, buf) in [&*l, &*r].into_iter().enumerate() {
            let v = abs_bits(buf[mi]);
            if v > self.state.master_peak[k] {
                self.state.master_peak[k] = v;
            }
        }
        Ok(())
    }
}

/// The float-to-int16 conversion of `sub_100139E0` (0x10013C4E): `x = f * 32768` in f64,
/// clamped to the f32 constants 32767 and -32768 (a NaN takes the lower bound), truncated
/// toward zero (`fistp` under a truncating control word).
pub fn to_i16(f: f32) -> i16 {
    let x = f as f64 * SCALE;
    let x = if x > CLAMP_HI as f64 {
        CLAMP_HI as f64
    } else if x.is_nan() || x < CLAMP_LO as f64 {
        CLAMP_LO as f64
    } else {
        x
    };
    x as i64 as i16
}

/// What top-level msg 53 finds when it makes a song current (`sub_1001BDA4`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activation {
    /// Another song was current: in the app always ("Untitled song" exists from the start, and
    /// an opened song replaces it), and in rbhost by default.
    ReplacesCurrent {
        /// The current song's play flag (`sub_10013DA0()+1`, its globals byte 1 at
        /// `song+0x159`, set by play 0x1001F80A and cleared by stop 0x100218CA), tested at
        /// 0x1001BDC7. `false` in every corpus capture: the untitled song is never played.
        current_playing: bool,
    },
    /// No song was current (rbhost `--no-untitled`).
    NoneCurrent,
}

/// The song object as `SafeSoundCallback` drives it.
pub struct Engine {
    pub frame: Frame,
    pub output: OutputState,
    /// `unk_100E012C`, `unk_100F012C`: the float output of the current call.
    out: [Box<[f32; MAX_FRAMES]>; 2],
    /// Not in the original: the render calls that ended in a [`Fault`].
    fault: Option<FaultRecord>,
}

fn zero_buf() -> Box<[f32; MAX_FRAMES]> {
    vec![0.0; MAX_FRAMES]
        .into_boxed_slice()
        .try_into()
        .ok()
        .unwrap()
}

fn block_mut(buf: &mut [f32], at: usize) -> &mut Block {
    (&mut buf[at..at + BLOCK]).try_into().unwrap()
}

impl Engine {
    /// A new song as the ctor `sub_10011350` leaves it: the constructor state, every device's
    /// slot 7 then slot 8 with its own track ([`SongDevice::init_track`], 0x10011941 and
    /// 0x10011985), the tempo 140000 (`sub_1001FCD0`, 0x100119BF), a locate to 0 (0x100119CC,
    /// which relocates every device), then one mixer update with the mixer device's default
    /// parameters (0x100119D4).
    pub fn new(tables: Arc<Tables>, devices: Devices) -> Self {
        let mut e = Engine::with_state(tables, devices, EngineState::default());
        let f = &mut e.frame;
        for d in song::Device::ALL {
            f.devices
                .device_mut(d)
                .init_track(&mut f.seq.tracks[d.id() as usize]);
        }
        f.seq.set_tempo(140_000);
        f.seq.locate(0, &mut f.devices);
        f.mixer_update();
        e
    }

    /// A song with an explicit frame state, the sequencer's ctor state and no constructor
    /// update.
    pub fn with_state(tables: Arc<Tables>, devices: Devices, state: EngineState) -> Self {
        let mut frame = Frame::new(tables, devices);
        frame.state = state.mixer;
        Engine {
            frame,
            output: state.output,
            out: [zero_buf(), zero_buf()],
            fault: None,
        }
    }

    /// The faults of the render calls so far: the first, and how many calls faulted. A faulting
    /// call leaves its int16 buffer as the caller had it, as the original does.
    pub fn fault(&self) -> Option<&FaultRecord> {
        self.fault.as_ref()
    }

    /// The song load's messages (`docs/song_load.md` section 2, steps 5 and 6), in order: msg
    /// 36, the nine msg-41 device blocks in the exe's order, the msg-37 events of tracks 0..8,
    /// then msg 32. None of them runs a mixer update.
    pub fn load(&mut self, song: &song::Song) {
        use song::Device as D;
        self.set_globals(&song.globals);
        for d in [
            D::Mixer,
            D::Delay,
            D::Pcf,
            D::Distortion,
            D::Compressor,
            D::Tb303_1,
            D::Tb303_2,
            D::Tr808,
            D::Tr909,
        ] {
            self.device_data(d, &song.device_payload(d));
        }
        for (i, events) in song.tracks.iter().enumerate() {
            self.frame.seq.add_events(i, events);
        }
        self.relocate();
    }

    /// Song msg 36, 0x100126B2: the global struct, applied field by field in this order: song
    /// mode, play or stop, record, loop on, tempo, locate, loop start, loop end, shuffle, then
    /// the stores of byte 21 (`+0x16D`, the sound model) and byte 22 (`+0x16E`).
    pub fn set_globals(&mut self, g: &song::Globals) {
        let f = &mut self.frame;
        let (seq, d) = (&mut f.seq, &mut f.devices);
        seq.set_song_mode(g.song_mode, d);
        if g.play == 0 {
            seq.stop(d);
        } else {
            seq.play(d);
        }
        seq.set_record(g.byte2);
        seq.set_loop_on(g.loop_on);
        seq.set_tempo(g.tempo);
        seq.locate(g.locate as i32, d);
        seq.set_loop_start(g.loop_start);
        seq.set_loop_end(g.loop_end);
        seq.set_shuffle(g.shuffle);
        seq.globals.vintage = g.vintage;
        seq.globals.byte22 = g.byte22;
        f.vintage = g.is_vintage();
        let v = f.vintage;
        f.devices.for_each(|d| d.set_sound_model(v));
    }

    /// Song msg 41 (0x1001285E): device `d`'s data block, with the current song mode.
    pub fn device_data(&mut self, d: song::Device, payload: &[u8]) {
        let f = &mut self.frame;
        let song_mode = f.seq.globals.song_mode != 0;
        f.devices.device_mut(d).data(payload, song_mode);
    }

    /// Song msg 32 (0x10012526): locate to the current tick, which chases the tracks in song
    /// mode and relocates the devices.
    pub fn relocate(&mut self) {
        let f = &mut self.frame;
        let t = f.seq.tick_pos();
        f.seq.locate(t, &mut f.devices);
    }

    /// Top-level msg 53, `sub_1001BDA4`, making this song the pending one (0x1001BDB6) and, unless
    /// the current song is playing, the current one.
    pub fn activate(&mut self, a: Activation) {
        match a {
            // 0x1001BDC7..0x1001BDCC: the current song is playing, so the routine returns at
            // 0x1001BE03 with no slot 6 and no switch; the switch is made later by
            // `sub_1001BE74`, when the pending song is played (msg 35 `{1,1}`, `sub_1001BF50`) or
            // on a MIDI Start (`sub_1001C524`). The engine holds one song, so this arm leaves the
            // new song not current; the real-time player (`rb-player`) models the switch
            // (docs/export_state.md §3)
            Activation::ReplacesCurrent {
                current_playing: true,
            } => {}
            // 0x1001BDD0: the old song's slot 6 (`sub_10013D40`); of it only the delay's
            // 0x100100E0 reaches this song: it refills the process-global delay line and sets
            // its gate. Every delay's slot 6 does only that, so this song's delay stands in for
            // the old one's ([`crate::delay::Delay`] refills [`ProcessState`]'s line).
            Activation::ReplacesCurrent {
                current_playing: false,
            } => self.frame.devices.delay.silence(),
            // 0x1001BDF3: slot 6 on this song's own devices
            Activation::NoneCurrent => self.frame.devices.silence_all(),
        }
    }

    /// The process globals of this song's devices.
    pub fn process_state(&self) -> &ProcessState {
        &self.frame.devices.process
    }

    pub fn state(&self) -> EngineState {
        EngineState {
            mixer: self.frame.state.clone(),
            output: self.output.clone(),
        }
    }

    /// `sub_100139E0(song, buf, n)`: renders `n` frames (clamped to 16384) of interleaved int16
    /// stereo into `buf[..2n]`, through the leftover block. A call with `n = 0` renders nothing
    /// but drops a pending leftover (`L` becomes `L - 64`).
    pub fn render(&mut self, buf: &mut [i16], n: usize) {
        let n = n.min(MAX_FRAMES);
        let [ol, or] = &mut self.out;
        let lp = self.output.leftover_pos;
        let mut pos = 0usize;
        let mut rem = n as i32;
        if lp > 0 {
            // 0x10013AE6: samples L..63 of the leftover block first, refilled with the guard
            let lp = lp as usize;
            for (o, lo) in [&mut **ol, &mut **or]
                .into_iter()
                .zip(self.output.leftover.iter_mut())
            {
                o[..BLOCK - lp].copy_from_slice(&lo[lp..]);
                lo[lp..].fill(GUARD);
            }
            pos = BLOCK - lp;
            rem -= pos as i32;
        }
        while rem > 0 {
            let whole = rem > 63;
            let done = if whole {
                self.frame
                    .block(block_mut(&mut ol[..], pos), block_mut(&mut or[..], pos))
            } else {
                // 0x10013BAA: the partial block is rendered whole into the leftover block
                let [ll, lr] = &mut self.output.leftover;
                self.frame.block(ll, lr)
            };
            if let Err(f) = done {
                // a fault ([`Fault`]): the call ends here, before the leftover position is
                // stored (0x10013BBA) and before the int16 conversion, so `buf` keeps what the
                // caller had in it
                let block = self.frame.blocks - 1;
                let r = self.fault.get_or_insert(FaultRecord {
                    first: f,
                    block,
                    calls: 0,
                });
                r.calls += 1;
                return;
            }
            if !whole {
                break;
            }
            pos += BLOCK;
            rem -= BLOCK as i32;
        }
        self.output.leftover_pos = rem;
        if rem > 0 {
            let k = rem as usize;
            for (o, lo) in [&mut **ol, &mut **or]
                .into_iter()
                .zip(self.output.leftover.iter_mut())
            {
                o[pos..pos + k].copy_from_slice(&lo[..k]);
                lo[..k].fill(GUARD);
            }
        }
        for i in 0..n {
            buf[2 * i] = to_i16(ol[i]);
            buf[2 * i + 1] = to_i16(or[i]);
        }
    }

    /// Song msg 35 `{1,0}` (`sub_1001E100`).
    pub fn stop(&mut self) {
        let f = &mut self.frame;
        f.seq.stop(&mut f.devices);
    }

    /// Song msg 35 `{8,tick}` (`sub_1001BF70`; the u32 is used as a signed tick).
    pub fn locate(&mut self, tick: u32) {
        let f = &mut self.frame;
        f.seq.locate(tick as i32, &mut f.devices);
    }

    /// Song msg 35 `{1,1}` (`sub_1001BF50`).
    pub fn play(&mut self) {
        let f = &mut self.frame;
        f.seq.play(&mut f.devices);
    }

    /// The song's play flag (`song+0x159`, globals byte 1): what msg 53 and `sub_1001BE74` test
    /// on the current song.
    pub fn playing(&self) -> bool {
        self.frame.seq.globals.play != 0
    }

    /// `sub_100218A0`: the stop of `sub_1001BE74`'s song switch, without the bar locate of msg
    /// 35 `{1,0}`: when playing, clear the play flag and run slot 4; prune every track; in song
    /// mode chase every track to the current tick.
    pub fn halt(&mut self) {
        let f = &mut self.frame;
        f.seq.halt(&mut f.devices);
    }

    /// Song msg 35 `{0,m}` (`sub_1001FBE0`): song mode (1) or pattern mode (0).
    pub fn set_song_mode(&mut self, mode: u8) {
        let f = &mut self.frame;
        f.seq.set_song_mode(mode, &mut f.devices);
    }

    /// Song msg 35 `{3,v}`: the loop switch.
    pub fn set_loop_on(&mut self, v: u8) {
        self.frame.seq.set_loop_on(v);
    }

    /// Song msg 35 `{4,v}`: the tempo, BPM*1000.
    pub fn set_tempo(&mut self, v: u32) {
        self.frame.seq.set_tempo(v);
    }

    /// Song msg 35 `{12,v}`: the loop start in ticks.
    pub fn set_loop_start(&mut self, v: u32) {
        self.frame.seq.set_loop_start(v);
    }

    /// Song msg 35 `{16,v}`: the loop end in ticks.
    pub fn set_loop_end(&mut self, v: u32) {
        self.frame.seq.set_loop_end(v);
    }

    /// Song msg 35 `{20,v}`: the shuffle amount.
    pub fn set_shuffle(&mut self, v: u8) {
        self.frame.seq.set_shuffle(v);
    }

    /// Song msg 35 `{21,v}`: the sound-model byte (`song+0x16D`; the menu's "Vintage ReBirth
    /// Synth Sound", `docs/song_load.md` section 3), stored as msg 36 stores it; the 303s read
    /// it at note-on, note-off and in their audio, the mixer update in its send.
    pub fn set_vintage(&mut self, v: u8) {
        let f = &mut self.frame;
        f.seq.globals.vintage = v;
        f.vintage = v != 0;
        let on = f.vintage;
        f.devices.for_each(|d| d.set_sound_model(on));
    }

    /// Slot 11 of device `d` with controller `ctrl` (`sub_10012B40`): what a track event does,
    /// and what the panel's knob and switch messages do with record off (the 303's msg 40
    /// `{ctrl, v}` at 0x10025CFC stores the live byte as slot 11 does, then calls the record
    /// hook `sub_10021340`, which acts only in song mode with record on; the mixer's msg 145
    /// ids 1, 2 and msg 147 fields 1..4 likewise).
    pub fn set_param(&mut self, d: song::Device, ctrl: u8, value: u8) {
        self.frame.devices.device_mut(d).set_param(ctrl, value);
    }

    /// Song msg 33, `sub_10014140`: the master's current gain and peaks to 0, the send buses to
    /// the guard, the leftover blocks to 0 (not its position), then slot 5 on every device.
    /// The three send buses are `song+0x160C + 0x100k`, one per send-effect slot registered by
    /// `sub_100140B4` (bus pointers `song+0x1FC + 4k`, count `song+0x1E8`); the ctor registers
    /// only the delay, so buses 1 and 2 (`+0x170C`, `+0x180C`) are never read and are not
    /// modelled.
    pub fn reset(&mut self) {
        let s = &mut self.frame.state;
        s.master = 0.0;
        s.master_peak = [0.0; 2];
        s.send = [GUARD; BLOCK];
        self.output.leftover = [[0.0; BLOCK]; 2];
        self.frame.devices.reset_all();
    }
}
