//! The engine frame: the 64-frame block loop, the mixer, the int16 output conversion
//! (`docs/initial_investigation.md` sections 3 and 4) and, in [`export`], the export driver.
//!
//! The DLL's song object (ctor `sub_10011350`) renders through `sub_100139E0` (one
//! `SafeSoundCallback` worth of frames, the leftover buffer and the int16 conversion), which calls
//! the mixer block `sub_10013444` once per 64 frames; every 16 blocks that block runs the mixer
//! update `sub_10012C14`. [`Engine`] is that song object: [`Engine::render`] is `sub_100139E0`,
//! [`Frame::block`] is `sub_10013444`, [`Frame::mixer_update`] is `sub_10012C14`.
//!
//! The channel devices, the insert effects, the send effect and the sequencer plug in through the
//! traits [`ChannelDevice`], [`InsertEffect`], [`SendEffect`] and [`Transport`]; [`Silent`],
//! [`Bypass`], [`IdleSend`] and [`Idle`] are the placeholders until their tasks implement them.
//!
//! Every computation follows the crate's numerics rules: f64 arithmetic in the original's operand
//! order, an f32 rounding at every `fstp dword`/`fst dword`, no FMA.

pub mod export;

#[cfg(test)]
mod tests;

use std::sync::Arc;

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
}

/// A mixer channel's sound source: TB-303 #1, #2, TR-808 or TR-909.
pub trait ChannelDevice {
    /// Vtable slot 14 (0x10013580): render one block into `bus`, overwriting all of it.
    fn process(&mut self, bus: &mut Block, ctx: &BlockContext);
    /// Slot 4: stop (song msg 35 `{1,0}`, `sub_10021840`).
    fn stop(&mut self) {}
    /// Slot 5: msg 33's reset (0x100141D2).
    fn reset(&mut self) {}
    /// Slot 6: note-off and silence (song switch, `sub_10013D40`).
    fn silence(&mut self) {}
}

/// An insert effect: the distortion, the PCF or the compressor.
pub trait InsertEffect {
    /// Slot 15 (0x100135C3): process a channel bus in place.
    fn process(&mut self, bus: &mut Block, ctx: &BlockContext);
    /// Slot 14 (0x100138AC): process the master in place (only the compressor sits there).
    fn process_stereo(&mut self, _l: &mut Block, _r: &mut Block, _ctx: &BlockContext) {}
    /// Slot 16 (0x100130B2): the parameter update, run by every mixer update.
    fn update(&mut self, _ctx: &BlockContext) {}
    /// Slot 4.
    fn stop(&mut self) {}
    /// Slot 5.
    fn reset(&mut self) {}
    /// Slot 6.
    fn silence(&mut self) {}
}

/// The send effect (the delay).
pub trait SendEffect {
    /// Slot 14 (0x10013865): read the send bus, add into the master and refill the send bus with
    /// [`GUARD`] (the delay does that in both its enabled and disabled paths, 0x1000F940).
    fn process(&mut self, send: &mut Block, l: &mut Block, r: &mut Block, ctx: &BlockContext);
    /// Slot 4.
    fn stop(&mut self) {}
    /// Slot 5.
    fn reset(&mut self) {}
    /// Slot 6.
    fn silence(&mut self) {}
}

/// The song's devices, in the roles the mixer gives them.
pub struct Devices {
    /// Channel c's source: 0 TB-303 #1, 1 TB-303 #2, 2 TR-808, 3 TR-909 (`song+0x1C8` list,
    /// filled by `sub_10014064` in the ctor's order).
    pub channels: [Box<dyn ChannelDevice>; 4],
    pub dist: Box<dyn InsertEffect>,
    pub pcf: Box<dyn InsertEffect>,
    pub comp: Box<dyn InsertEffect>,
    pub delay: Box<dyn SendEffect>,
}

impl Devices {
    /// Placeholders everywhere: silent sources, bypassing inserts, an idle send.
    pub fn placeholders() -> Self {
        Devices {
            channels: std::array::from_fn(|_| Box::new(Silent) as Box<dyn ChannelDevice>),
            dist: Box::new(Bypass),
            pcf: Box::new(Bypass),
            comp: Box::new(Bypass),
            delay: Box::new(IdleSend),
        }
    }

    /// Slot 4 on every device in the song's device order (`song+0x18C`: mixer, 303, 303, 808,
    /// 909, delay, dist, PCF, comp; the mixer's own slot is a no-op).
    pub fn stop_all(&mut self) {
        for d in &mut self.channels {
            d.stop();
        }
        self.delay.stop();
        self.dist.stop();
        self.pcf.stop();
        self.comp.stop();
    }

    /// Slot 5 on every device, in the same order (msg 33, 0x100141BA..0x100141E5).
    pub fn reset_all(&mut self) {
        for d in &mut self.channels {
            d.reset();
        }
        self.delay.reset();
        self.dist.reset();
        self.pcf.reset();
        self.comp.reset();
    }

    /// Slot 6 on every device, in the same order.
    pub fn silence_all(&mut self) {
        for d in &mut self.channels {
            d.silence();
        }
        self.delay.silence();
        self.dist.silence();
        self.pcf.silence();
        self.comp.silence();
    }
}

/// The sequencer and transport, the seam for task-022.
pub trait Transport {
    /// `sub_1001EF20`, run by every block after the mixer update and before any device renders.
    fn tick(&mut self, _devices: &mut Devices, _mixer: &mut MixerParams) {}
    /// Song msg 35 `{1,0}` (`sub_100218A0`): stop the devices, chase, locate to the bar start.
    fn stop(&mut self, devices: &mut Devices, _mixer: &mut MixerParams) {
        devices.stop_all();
    }
    /// Song msg 35 `{8,tick}`.
    fn locate(&mut self, _tick: u32, _devices: &mut Devices, _mixer: &mut MixerParams) {}
    /// Song msg 35 `{1,1}`.
    fn play(&mut self, _devices: &mut Devices, _mixer: &mut MixerParams) {}
}

/// A placeholder transport that never plays: every block is an idle block.
pub struct Idle;
impl Transport for Idle {}

/// A placeholder source that outputs the guard value only.
pub struct Silent;
impl ChannelDevice for Silent {
    fn process(&mut self, bus: &mut Block, _ctx: &BlockContext) {
        bus.fill(GUARD);
    }
}

/// A placeholder insert that leaves the signal alone.
pub struct Bypass;
impl InsertEffect for Bypass {
    fn process(&mut self, _bus: &mut Block, _ctx: &BlockContext) {}
}

/// A placeholder send effect that only refills the send bus, as the delay does
/// (0x1000F940; the disabled delay adds nothing to the master).
pub struct IdleSend;
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
    pub params: MixerParams,
    /// `song+0x16D`, the sound-model byte non-zero: the pan-dependent send.
    pub vintage: bool,
    pub devices: Devices,
    pub transport: Box<dyn Transport>,
    tables: Arc<Tables>,
    /// `unk_1010012C`: the channel bus every device renders into in turn (a process global).
    bus: Block,
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
    pub fn new(tables: Arc<Tables>, devices: Devices, transport: Box<dyn Transport>) -> Self {
        Frame {
            state: MixerState::default(),
            params: MixerParams::default(),
            vintage: false,
            devices,
            transport,
            tables,
            bus: [0.0; BLOCK],
        }
    }

    pub fn tables(&self) -> &Tables {
        &self.tables
    }

    /// The mixer update `sub_10012C14`, every 16 blocks and once at the end of the song ctor.
    pub fn mixer_update(&mut self) {
        let t = &self.tables.level;
        let pan = &self.tables.pan;
        let p = self.params;
        let ctx = BlockContext {
            tables: &self.tables,
            meter_index: self.state.meter_index as usize,
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

    /// The mixer block `sub_10013444`: one block into the master buses `l` and `r`.
    pub fn block(&mut self, l: &mut Block, r: &mut Block) {
        if self.state.counter & (UPDATE_PERIOD - 1) == 0 {
            self.mixer_update();
        }
        self.state.counter = self.state.counter.wrapping_add(1);
        self.transport.tick(&mut self.devices, &mut self.params);
        let mi = (self.state.meter_index + METER_STEP) & 63;
        self.state.meter_index = mi;
        let mi = mi as usize;
        l.fill(GUARD);
        r.fill(GUARD);
        let ctx = BlockContext {
            tables: &self.tables,
            meter_index: mi,
        };
        let bus = &mut self.bus;
        for (c, ch) in self.state.channels.iter_mut().enumerate() {
            self.devices.channels[c].process(bus, &ctx);
            if ch.routing.dist {
                self.devices.dist.process(bus, &ctx);
            }
            if ch.routing.pcf {
                self.devices.pcf.process(bus, &ctx);
            }
            if ch.routing.comp {
                self.devices.comp.process(bus, &ctx);
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

/// The song object as `SafeSoundCallback` drives it.
pub struct Engine {
    pub frame: Frame,
    pub output: OutputState,
    /// `unk_100E012C`, `unk_100F012C`: the float output of the current call.
    out: [Box<[f32; MAX_FRAMES]>; 2],
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
    /// A new song as the ctor `sub_10011350` leaves it: the constructor state, then one mixer
    /// update with the mixer device's default parameters (0x100119D4).
    pub fn new(tables: Arc<Tables>, devices: Devices, transport: Box<dyn Transport>) -> Self {
        let mut e = Engine::with_state(tables, devices, transport, EngineState::default());
        e.frame.mixer_update();
        e
    }

    /// A song with an explicit state and no constructor update.
    pub fn with_state(
        tables: Arc<Tables>,
        devices: Devices,
        transport: Box<dyn Transport>,
        state: EngineState,
    ) -> Self {
        let mut frame = Frame::new(tables, devices, transport);
        frame.state = state.mixer;
        Engine {
            frame,
            output: state.output,
            out: [zero_buf(), zero_buf()],
        }
    }

    /// Sets what the song load sets for the frame: the mixer device's block (song msg 41
    /// `0/143`) and the sound model (msg 36 byte 21). Neither runs a mixer update.
    pub fn load(&mut self, song: &song::Song) {
        self.frame.params = MixerParams::from(&song.mixer);
        self.frame.vintage = song.globals.is_vintage();
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
            if rem > 63 {
                self.frame
                    .block(block_mut(&mut ol[..], pos), block_mut(&mut or[..], pos));
                pos += BLOCK;
                rem -= BLOCK as i32;
            } else {
                // 0x10013BAA: the partial block is rendered whole into the leftover block
                let [ll, lr] = &mut self.output.leftover;
                self.frame.block(ll, lr);
                break;
            }
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

    /// Song msg 35 `{1,0}`.
    pub fn stop(&mut self) {
        let f = &mut self.frame;
        f.transport.stop(&mut f.devices, &mut f.params);
    }

    /// Song msg 35 `{8,tick}`.
    pub fn locate(&mut self, tick: u32) {
        let f = &mut self.frame;
        f.transport.locate(tick, &mut f.devices, &mut f.params);
    }

    /// Song msg 35 `{1,1}`.
    pub fn play(&mut self) {
        let f = &mut self.frame;
        f.transport.play(&mut f.devices, &mut f.params);
    }

    /// Song msg 33, `sub_10014140`: the master's current gain and peaks to 0, the send buses to
    /// the guard, the leftover blocks to 0 (not its position), then slot 5 on every device.
    pub fn reset(&mut self) {
        let s = &mut self.frame.state;
        s.master = 0.0;
        s.master_peak = [0.0; 2];
        s.send = [GUARD; BLOCK];
        self.output.leftover = [[0.0; BLOCK]; 2];
        self.frame.devices.reset_all();
    }
}
