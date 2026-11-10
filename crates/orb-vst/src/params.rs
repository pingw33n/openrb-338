//! Host parameters, grouped the way the panel is built: the mix, each device,
//! and each effect. A parameter is the engine's byte. Ableton automates it.
//!
//! The audio thread writes that byte only when the host's value changed, and
//! only through the setter the panel uses (`device_param`, `mixer_global`,
//! `mixer_channel`). A panel drag sets the gesture flag and wins for that
//! byte. MIDI CCs call the same setter. The editor copies the snapshot back
//! into the parameters, so a panel move shows up in Ableton.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use nih_plug::prelude::*;
use orb_engine::frame::Engine;
use orb_engine::song::Device;
use orb_engine::tb303::{self, DEFAULT_PARAMS as TB303};
use orb_engine::tr808::DEFAULT_PARAMS as TR808;
use orb_engine::tr909::{self, DEFAULT_PARAMS as TR909};
use orb_player::Snapshot;

/// One automatable byte.
#[derive(Clone, Copy)]
pub enum Target {
    Master,
    /// Mixer msg 145: 1 the compressor route, 2 the PCF route.
    MixGlobal(u8),
    /// Mixer msg 147: field 0 enabled, 1 level, 2 pan, 3 delay send, 4 distortion.
    MixChannel {
        channel: u8,
        field: u8,
    },
    Ctrl {
        device: Device,
        ctrl: u8,
    },
}

/// Which parameter, which panel control owns the gesture, and where the byte goes.
struct Lane {
    /// The `Params` id. The group test reads it; the host never does.
    #[cfg_attr(not(test), allow(dead_code))]
    vst_id: &'static str,
    panel_id: &'static str,
    param: fn(&PlugParams) -> &IntParam,
    target: Target,
}

fn knob(name: &str, default: i32) -> IntParam {
    IntParam::new(name, default, IntRange::Linear { min: 0, max: 127 })
}

/// Ableton lists the parameter title, not the VST3 unit, so the title carries the module.
fn labeled(module: &str, name: &str, default: i32) -> IntParam {
    knob(&format!("{module} {name}"), default)
}

const fn ctrl(device: Device, ctrl: u8) -> Target {
    Target::Ctrl { device, ctrl }
}

/// Set while the panel is dragging this lane. The audio thread does not overwrite it.
pub struct Gestures {
    flags: Box<[AtomicBool]>,
}

impl Gestures {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            flags: (0..LANES.len()).map(|_| AtomicBool::new(false)).collect(),
        })
    }

    /// `held` is the panel control under the pointer, or none.
    pub fn set_held(&self, held: Option<&str>) {
        for (i, lane) in LANES.iter().enumerate() {
            self.flags[i].store(held == Some(lane.panel_id), Ordering::Relaxed);
        }
    }

    fn get(&self, index: usize) -> bool {
        self.flags[index].load(Ordering::Relaxed)
    }
}

/// The bytes the parameters were last allowed to write. A host value that
/// matches this is not written again, so a panel move or a CC in between stays.
pub fn initial_sent(params: &PlugParams) -> Vec<u8> {
    LANES
        .iter()
        .map(|lane| byte((lane.param)(params)))
        .collect()
}

fn byte(param: &IntParam) -> u8 {
    param.value().clamp(0, 127) as u8
}

/// Write one byte the way the panel's command does.
pub fn write(engine: &mut Engine, target: Target, value: u8) {
    match target {
        Target::Master => engine.mixer_global(0, value),
        Target::MixGlobal(id) => engine.mixer_global(id, value),
        Target::MixChannel { channel, field } => engine.mixer_channel(channel, field, value),
        Target::Ctrl { device, ctrl } => engine.device_param(device, ctrl, value),
    }
}

#[cfg(test)]
fn read_engine(engine: &Engine, target: Target) -> u8 {
    let devices = &engine.frame.devices;
    match target {
        Target::Master => devices.mixer.live.master,
        Target::MixGlobal(1) => devices.mixer.live.comp_id,
        Target::MixGlobal(2) => devices.mixer.live.pcf_id,
        Target::MixGlobal(_) => 0,
        Target::MixChannel { channel, field } => {
            let Some(ch) = devices.mixer.live.channels.get(channel as usize) else {
                return 0;
            };
            match field {
                0 => ch.enabled,
                1 => ch.level,
                2 => ch.pan,
                3 => ch.send,
                4 => ch.dist,
                _ => 0,
            }
        }
        Target::Ctrl { device, ctrl } => live_at(device_live(devices, device), device, ctrl),
    }
}

#[cfg(test)]
fn device_live(devices: &orb_engine::frame::Devices, device: Device) -> &[u8] {
    match device {
        Device::Mixer => &[],
        Device::Tb303_1 => devices.channels[0].live(),
        Device::Tb303_2 => devices.channels[1].live(),
        Device::Tr808 => devices.channels[2].live(),
        Device::Tr909 => devices.channels[3].live(),
        Device::Delay => devices.delay.live(),
        Device::Distortion => devices.dist.live(),
        Device::Pcf => devices.pcf.live(),
        Device::Compressor => devices.comp.live(),
    }
}

fn live_at(block: &[u8], device: Device, ctrl: u8) -> u8 {
    let index = if device == Device::Tr909 {
        tr909::internal_index(ctrl as usize)
    } else {
        ctrl as usize
    };
    block.get(index).copied().unwrap_or(0)
}

fn read_snapshot(snap: &Snapshot, target: Target) -> Option<u8> {
    match target {
        Target::Master => Some(snap.mixer.master),
        Target::MixGlobal(1) => Some(snap.mixer.comp_id),
        Target::MixGlobal(2) => Some(snap.mixer.pcf_id),
        Target::MixGlobal(_) => None,
        Target::MixChannel { channel, field } => {
            let ch = snap.mixer.channels.get(channel as usize)?;
            Some(match field {
                0 => ch.enabled,
                1 => ch.level,
                2 => ch.pan,
                3 => ch.send,
                4 => ch.dist,
                _ => return None,
            })
        }
        Target::Ctrl { device, ctrl } => {
            let block = snap.live[device as usize].as_slice();
            // An empty block has not been published yet. Do not push zeros.
            if block.is_empty() {
                return None;
            }
            Some(live_at(block, device, ctrl))
        }
    }
}

impl PlugParams {
    /// Host bytes that changed, into the engine. A gestured lane is left alone.
    pub fn apply(&self, engine: &mut Engine, sent: &mut [u8], gestures: &Gestures) {
        if sent.len() != LANES.len() {
            return;
        }
        for (i, lane) in LANES.iter().enumerate() {
            let host = byte((lane.param)(self));
            if gestures.get(i) || host == sent[i] {
                continue;
            }
            write(engine, lane.target, host);
            sent[i] = host;
        }
    }

    /// Copy the snapshot into the host parameters so Ableton follows the panel.
    pub fn follow(&self, setter: &ParamSetter, snap: &Snapshot, begun: &mut [bool]) {
        if begun.len() != LANES.len() {
            return;
        }
        for (i, lane) in LANES.iter().enumerate() {
            let Some(current) = read_snapshot(snap, lane.target) else {
                continue;
            };
            let param = (lane.param)(self);
            if current != byte(param) {
                if !begun[i] {
                    setter.begin_set_parameter(param);
                    begun[i] = true;
                }
                setter.set_parameter(param, i32::from(current));
            } else if begun[i] {
                setter.end_set_parameter(param);
                begun[i] = false;
            }
        }
    }
}

#[derive(Params)]
pub struct PlugParams {
    /// The open song, as an `.rbs`. The host stores it with the project.
    #[persist = "rbs"]
    pub song: crate::song::SongBlob,
    #[nested(group = "Mix", id_prefix = "mix")]
    pub mix: MixParams,
    #[nested(group = "303 1", id_prefix = "303_1")]
    pub tb303_1: Tb303Params,
    #[nested(group = "303 2", id_prefix = "303_2")]
    pub tb303_2: Tb303Params,
    #[nested(group = "808", id_prefix = "808")]
    pub tr808: Tr808Params,
    #[nested(group = "909", id_prefix = "909")]
    pub tr909: Tr909Params,
    #[nested(group = "Delay", id_prefix = "delay")]
    pub delay: DelayParams,
    #[nested(group = "Distortion", id_prefix = "dist")]
    pub dist: DistParams,
    #[nested(group = "PCF", id_prefix = "pcf")]
    pub pcf: PcfParams,
    #[nested(group = "Compressor", id_prefix = "comp")]
    pub comp: CompParams,
}

#[derive(Params)]
pub struct MixParams {
    #[id = "master"]
    pub master: IntParam,
    #[id = "comp"]
    pub comp: IntParam,
    #[id = "pcf"]
    pub pcf: IntParam,
    #[nested(group = "303 1", id_prefix = "303_1")]
    pub ch303_1: StripParams,
    #[nested(group = "303 2", id_prefix = "303_2")]
    pub ch303_2: StripParams,
    #[nested(group = "808", id_prefix = "808")]
    pub ch808: StripParams,
    #[nested(group = "909", id_prefix = "909")]
    pub ch909: StripParams,
}

#[derive(Params)]
pub struct StripParams {
    #[id = "on"]
    pub on: IntParam,
    #[id = "level"]
    pub level: IntParam,
    #[id = "pan"]
    pub pan: IntParam,
    #[id = "delay"]
    pub delay: IntParam,
    #[id = "dist"]
    pub dist: IntParam,
}

#[derive(Params)]
pub struct Tb303Params {
    #[id = "tune"]
    pub tune: IntParam,
    #[id = "cutoff"]
    pub cutoff: IntParam,
    #[id = "reso"]
    pub reso: IntParam,
    #[id = "envmod"]
    pub envmod: IntParam,
    #[id = "decay"]
    pub decay: IntParam,
    #[id = "accent"]
    pub accent: IntParam,
    #[id = "wave"]
    pub wave: IntParam,
}

#[derive(Params)]
pub struct Tr808Params {
    #[id = "accent"]
    pub accent: IntParam,
    #[nested(group = "Bass Drum", id_prefix = "bd")]
    pub bd: Bd808,
    #[nested(group = "Snare", id_prefix = "sd")]
    pub sd: Sd808,
    #[nested(group = "Low Tom", id_prefix = "lt")]
    pub lt: Tom808,
    #[nested(group = "Mid Tom", id_prefix = "mt")]
    pub mt: Tom808,
    #[nested(group = "High Tom", id_prefix = "ht")]
    pub ht: Tom808,
    #[nested(group = "Rim Shot", id_prefix = "rs")]
    pub rs: Select808,
    #[nested(group = "Hand Clap", id_prefix = "cp")]
    pub cp: Select808,
    #[nested(group = "Cowbell", id_prefix = "cb")]
    pub cb: Level808,
    #[nested(group = "Cymbal", id_prefix = "cy")]
    pub cy: Cymbal808,
    #[nested(group = "Open Hat", id_prefix = "oh")]
    pub oh: Hat808,
    #[nested(group = "Closed Hat", id_prefix = "ch")]
    pub ch: Level808,
}

#[derive(Params)]
pub struct Bd808 {
    #[id = "level"]
    pub level: IntParam,
    #[id = "tone"]
    pub tone: IntParam,
    #[id = "decay"]
    pub decay: IntParam,
}

#[derive(Params)]
pub struct Sd808 {
    #[id = "level"]
    pub level: IntParam,
    #[id = "tone"]
    pub tone: IntParam,
    #[id = "snappy"]
    pub snappy: IntParam,
}

#[derive(Params)]
pub struct Tom808 {
    #[id = "level"]
    pub level: IntParam,
    #[id = "tuning"]
    pub tuning: IntParam,
    #[id = "select"]
    pub select: IntParam,
}

#[derive(Params)]
pub struct Select808 {
    #[id = "level"]
    pub level: IntParam,
    #[id = "select"]
    pub select: IntParam,
}

#[derive(Params)]
pub struct Level808 {
    #[id = "level"]
    pub level: IntParam,
}

#[derive(Params)]
pub struct Cymbal808 {
    #[id = "level"]
    pub level: IntParam,
    #[id = "tone"]
    pub tone: IntParam,
    #[id = "decay"]
    pub decay: IntParam,
}

#[derive(Params)]
pub struct Hat808 {
    #[id = "level"]
    pub level: IntParam,
    #[id = "decay"]
    pub decay: IntParam,
}

#[derive(Params)]
pub struct Tr909Params {
    #[id = "accent"]
    pub accent: IntParam,
    #[id = "flam"]
    pub flam: IntParam,
    #[nested(group = "Bass Drum", id_prefix = "bd")]
    pub bd: Bd909,
    #[nested(group = "Snare", id_prefix = "sd")]
    pub sd: Sd909,
    #[nested(group = "Low Tom", id_prefix = "lt")]
    pub lt: Tom909,
    #[nested(group = "Mid Tom", id_prefix = "mt")]
    pub mt: Tom909,
    #[nested(group = "High Tom", id_prefix = "ht")]
    pub ht: Tom909,
    #[nested(group = "Rim Shot", id_prefix = "rs")]
    pub rs: Level808,
    #[nested(group = "Hand Clap", id_prefix = "cp")]
    pub cp: Level808,
    #[nested(group = "Hi-Hat", id_prefix = "hh")]
    pub hh: Hat909,
    #[nested(group = "Crash", id_prefix = "cc")]
    pub cc: Tuned909,
    #[nested(group = "Ride", id_prefix = "rc")]
    pub rc: Tuned909,
}

#[derive(Params)]
pub struct Bd909 {
    #[id = "level"]
    pub level: IntParam,
    #[id = "tune"]
    pub tune: IntParam,
    #[id = "attack"]
    pub attack: IntParam,
    #[id = "decay"]
    pub decay: IntParam,
}

#[derive(Params)]
pub struct Sd909 {
    #[id = "level"]
    pub level: IntParam,
    #[id = "tune"]
    pub tune: IntParam,
    #[id = "tone"]
    pub tone: IntParam,
    #[id = "snappy"]
    pub snappy: IntParam,
}

#[derive(Params)]
pub struct Tom909 {
    #[id = "level"]
    pub level: IntParam,
    #[id = "tune"]
    pub tune: IntParam,
    #[id = "decay"]
    pub decay: IntParam,
}

#[derive(Params)]
pub struct Hat909 {
    #[id = "level"]
    pub level: IntParam,
    #[id = "ch_decay"]
    pub ch_decay: IntParam,
    #[id = "oh_decay"]
    pub oh_decay: IntParam,
}

#[derive(Params)]
pub struct Tuned909 {
    #[id = "level"]
    pub level: IntParam,
    #[id = "tune"]
    pub tune: IntParam,
}

#[derive(Params)]
pub struct DelayParams {
    #[id = "on"]
    pub on: IntParam,
    #[id = "steps"]
    pub steps: IntParam,
    #[id = "mode"]
    pub mode: IntParam,
    #[id = "feedback"]
    pub feedback: IntParam,
    #[id = "pan"]
    pub pan: IntParam,
}

#[derive(Params)]
pub struct DistParams {
    #[id = "on"]
    pub on: IntParam,
    #[id = "amount"]
    pub amount: IntParam,
    #[id = "shape"]
    pub shape: IntParam,
}

#[derive(Params)]
pub struct PcfParams {
    #[id = "on"]
    pub on: IntParam,
    #[id = "freq"]
    pub freq: IntParam,
    #[id = "q"]
    pub q: IntParam,
    #[id = "amt"]
    pub amt: IntParam,
    #[id = "wave"]
    pub wave: IntParam,
    #[id = "decay"]
    pub decay: IntParam,
    #[id = "mode"]
    pub mode: IntParam,
}

#[derive(Params)]
pub struct CompParams {
    #[id = "on"]
    pub on: IntParam,
    #[id = "ratio"]
    pub ratio: IntParam,
    #[id = "threshold"]
    pub threshold: IntParam,
}

impl Default for PlugParams {
    fn default() -> Self {
        Self {
            song: crate::song::SongBlob::new(),
            mix: MixParams::default(),
            tb303_1: Tb303Params::new("303 1"),
            tb303_2: Tb303Params::new("303 2"),
            tr808: Tr808Params::default(),
            tr909: Tr909Params::default(),
            delay: DelayParams::default(),
            dist: DistParams::default(),
            pcf: PcfParams::default(),
            comp: CompParams::default(),
        }
    }
}

impl Default for MixParams {
    fn default() -> Self {
        Self {
            master: labeled("Mix", "Master", 100),
            comp: labeled("Mix", "Comp Route", 0),
            pcf: labeled("Mix", "PCF Route", 0),
            ch303_1: StripParams::new("Mix 303 1"),
            ch303_2: StripParams::new("Mix 303 2"),
            ch808: StripParams::new("Mix 808"),
            ch909: StripParams::new("Mix 909"),
        }
    }
}

impl StripParams {
    fn new(module: &str) -> Self {
        Self {
            on: labeled(module, "On", 1),
            level: labeled(module, "Level", 100),
            pan: labeled(module, "Pan", 64),
            delay: labeled(module, "Delay", 0),
            dist: labeled(module, "Distortion", 0),
        }
    }
}

impl Tb303Params {
    fn new(module: &str) -> Self {
        Self {
            tune: labeled(module, "Tune", i32::from(TB303[tb303::TUNE])),
            cutoff: labeled(module, "Cutoff", i32::from(TB303[tb303::CUTOFF])),
            reso: labeled(module, "Resonance", i32::from(TB303[tb303::RESONANCE])),
            envmod: labeled(module, "Env Mod", i32::from(TB303[tb303::ENVMOD])),
            decay: labeled(module, "Decay", i32::from(TB303[tb303::DECAY])),
            accent: labeled(module, "Accent", i32::from(TB303[tb303::ACCENT])),
            wave: labeled(module, "Waveform", i32::from(TB303[tb303::WAVEFORM])),
        }
    }
}

impl Default for Tr808Params {
    fn default() -> Self {
        let b = |i: usize| i32::from(TR808[i]);
        Self {
            accent: labeled("808", "Accent", b(2)),
            bd: Bd808 {
                level: labeled("808 Bass Drum", "Level", b(3)),
                tone: labeled("808 Bass Drum", "Tone", b(4)),
                decay: labeled("808 Bass Drum", "Decay", b(5)),
            },
            sd: Sd808 {
                level: labeled("808 Snare", "Level", b(6)),
                tone: labeled("808 Snare", "Tone", b(7)),
                snappy: labeled("808 Snare", "Snappy", b(8)),
            },
            lt: Tom808 {
                level: labeled("808 Low Tom", "Level", b(9)),
                tuning: labeled("808 Low Tom", "Tuning", b(10)),
                select: labeled("808 Low Tom", "Selector", b(11)),
            },
            mt: Tom808 {
                level: labeled("808 Mid Tom", "Level", b(12)),
                tuning: labeled("808 Mid Tom", "Tuning", b(13)),
                select: labeled("808 Mid Tom", "Selector", b(14)),
            },
            ht: Tom808 {
                level: labeled("808 High Tom", "Level", b(15)),
                tuning: labeled("808 High Tom", "Tuning", b(16)),
                select: labeled("808 High Tom", "Selector", b(17)),
            },
            rs: Select808 {
                level: labeled("808 Rim Shot", "Level", b(18)),
                select: labeled("808 Rim Shot", "Selector", b(19)),
            },
            cp: Select808 {
                level: labeled("808 Hand Clap", "Level", b(20)),
                select: labeled("808 Hand Clap", "Selector", b(21)),
            },
            cb: Level808 {
                level: labeled("808 Cowbell", "Level", b(22)),
            },
            cy: Cymbal808 {
                level: labeled("808 Cymbal", "Level", b(23)),
                tone: labeled("808 Cymbal", "Tone", b(24)),
                decay: labeled("808 Cymbal", "Decay", b(25)),
            },
            oh: Hat808 {
                level: labeled("808 Open Hat", "Level", b(26)),
                decay: labeled("808 Open Hat", "Decay", b(27)),
            },
            ch: Level808 {
                level: labeled("808 Closed Hat", "Level", b(28)),
            },
        }
    }
}

fn b909(ctrl: usize) -> i32 {
    i32::from(TR909[tr909::internal_index(ctrl)])
}

impl Default for Tr909Params {
    fn default() -> Self {
        Self {
            accent: labeled("909", "Accent", b909(2)),
            flam: labeled("909", "Flam", b909(29)),
            bd: Bd909 {
                level: labeled("909 Bass Drum", "Level", b909(3)),
                tune: labeled("909 Bass Drum", "Tune", b909(4)),
                attack: labeled("909 Bass Drum", "Attack", b909(5)),
                decay: labeled("909 Bass Drum", "Decay", b909(6)),
            },
            sd: Sd909 {
                level: labeled("909 Snare", "Level", b909(7)),
                tune: labeled("909 Snare", "Tune", b909(8)),
                tone: labeled("909 Snare", "Tone", b909(9)),
                snappy: labeled("909 Snare", "Snappy", b909(10)),
            },
            lt: Tom909 {
                level: labeled("909 Low Tom", "Level", b909(11)),
                tune: labeled("909 Low Tom", "Tune", b909(12)),
                decay: labeled("909 Low Tom", "Decay", b909(13)),
            },
            mt: Tom909 {
                level: labeled("909 Mid Tom", "Level", b909(14)),
                tune: labeled("909 Mid Tom", "Tune", b909(15)),
                decay: labeled("909 Mid Tom", "Decay", b909(16)),
            },
            ht: Tom909 {
                level: labeled("909 High Tom", "Level", b909(17)),
                tune: labeled("909 High Tom", "Tune", b909(18)),
                decay: labeled("909 High Tom", "Decay", b909(19)),
            },
            rs: Level808 {
                level: labeled("909 Rim Shot", "Level", b909(20)),
            },
            cp: Level808 {
                level: labeled("909 Hand Clap", "Level", b909(21)),
            },
            hh: Hat909 {
                level: labeled("909 Hi-Hat", "Level", b909(22)),
                ch_decay: labeled("909 Hi-Hat", "Closed Decay", b909(23)),
                oh_decay: labeled("909 Hi-Hat", "Open Decay", b909(24)),
            },
            cc: Tuned909 {
                level: labeled("909 Crash", "Level", b909(25)),
                tune: labeled("909 Crash", "Tune", b909(26)),
            },
            rc: Tuned909 {
                level: labeled("909 Ride", "Level", b909(27)),
                tune: labeled("909 Ride", "Tune", b909(28)),
            },
        }
    }
}

impl Default for DelayParams {
    fn default() -> Self {
        let d = orb_engine::delay::DEFAULTS;
        Self {
            on: labeled("Delay", "On", i32::from(d[0])),
            steps: labeled("Delay", "Steps", i32::from(d[1])),
            mode: labeled("Delay", "Mode", i32::from(d[2])),
            feedback: labeled("Delay", "Feedback", i32::from(d[3])),
            pan: labeled("Delay", "Pan", i32::from(d[4])),
        }
    }
}

impl Default for DistParams {
    fn default() -> Self {
        let d = orb_engine::dist::DEFAULTS;
        Self {
            on: labeled("Dist", "On", i32::from(d[0])),
            amount: labeled("Dist", "Amount", i32::from(d[1])),
            shape: labeled("Dist", "Shape", i32::from(d[2])),
        }
    }
}

impl Default for PcfParams {
    fn default() -> Self {
        let d = orb_engine::pcf::DEFAULTS;
        Self {
            on: labeled("PCF", "On", i32::from(d[0])),
            freq: labeled("PCF", "Frequency", i32::from(d[1])),
            q: labeled("PCF", "Resonance", i32::from(d[2])),
            amt: labeled("PCF", "Amount", i32::from(d[3])),
            wave: labeled("PCF", "Wave", i32::from(d[4])),
            decay: labeled("PCF", "Decay", i32::from(d[5])),
            mode: labeled("PCF", "Mode", i32::from(d[6])),
        }
    }
}

impl Default for CompParams {
    fn default() -> Self {
        let d = orb_engine::comp::DEFAULTS;
        Self {
            on: labeled("Comp", "On", i32::from(d[0])),
            ratio: labeled("Comp", "Ratio", i32::from(d[1])),
            threshold: labeled("Comp", "Threshold", i32::from(d[2])),
        }
    }
}

const LANES: &[Lane] = &[
    Lane {
        vst_id: "mix_master",
        panel_id: "master.fader",
        param: |p| &p.mix.master,
        target: Target::Master,
    },
    Lane {
        vst_id: "mix_comp",
        panel_id: "mixer.comp_id",
        param: |p| &p.mix.comp,
        target: Target::MixGlobal(1),
    },
    Lane {
        vst_id: "mix_pcf",
        panel_id: "mixer.pcf_id",
        param: |p| &p.mix.pcf,
        target: Target::MixGlobal(2),
    },
    // The four mixer strips. Field 0 is the channel enable.
    Lane {
        vst_id: "mix_303_1_on",
        panel_id: "mixer1.mute",
        param: |p| &p.mix.ch303_1.on,
        target: Target::MixChannel {
            channel: 0,
            field: 0,
        },
    },
    Lane {
        vst_id: "mix_303_1_level",
        panel_id: "mixer1.level",
        param: |p| &p.mix.ch303_1.level,
        target: Target::MixChannel {
            channel: 0,
            field: 1,
        },
    },
    Lane {
        vst_id: "mix_303_1_pan",
        panel_id: "mixer1.pan",
        param: |p| &p.mix.ch303_1.pan,
        target: Target::MixChannel {
            channel: 0,
            field: 2,
        },
    },
    Lane {
        vst_id: "mix_303_1_delay",
        panel_id: "mixer1.delay",
        param: |p| &p.mix.ch303_1.delay,
        target: Target::MixChannel {
            channel: 0,
            field: 3,
        },
    },
    Lane {
        vst_id: "mix_303_1_dist",
        panel_id: "mixer1.dist",
        param: |p| &p.mix.ch303_1.dist,
        target: Target::MixChannel {
            channel: 0,
            field: 4,
        },
    },
    Lane {
        vst_id: "mix_303_2_on",
        panel_id: "mixer2.mute",
        param: |p| &p.mix.ch303_2.on,
        target: Target::MixChannel {
            channel: 1,
            field: 0,
        },
    },
    Lane {
        vst_id: "mix_303_2_level",
        panel_id: "mixer2.level",
        param: |p| &p.mix.ch303_2.level,
        target: Target::MixChannel {
            channel: 1,
            field: 1,
        },
    },
    Lane {
        vst_id: "mix_303_2_pan",
        panel_id: "mixer2.pan",
        param: |p| &p.mix.ch303_2.pan,
        target: Target::MixChannel {
            channel: 1,
            field: 2,
        },
    },
    Lane {
        vst_id: "mix_303_2_delay",
        panel_id: "mixer2.delay",
        param: |p| &p.mix.ch303_2.delay,
        target: Target::MixChannel {
            channel: 1,
            field: 3,
        },
    },
    Lane {
        vst_id: "mix_303_2_dist",
        panel_id: "mixer2.dist",
        param: |p| &p.mix.ch303_2.dist,
        target: Target::MixChannel {
            channel: 1,
            field: 4,
        },
    },
    Lane {
        vst_id: "mix_808_on",
        panel_id: "mixer3.mute",
        param: |p| &p.mix.ch808.on,
        target: Target::MixChannel {
            channel: 2,
            field: 0,
        },
    },
    Lane {
        vst_id: "mix_808_level",
        panel_id: "mixer3.level",
        param: |p| &p.mix.ch808.level,
        target: Target::MixChannel {
            channel: 2,
            field: 1,
        },
    },
    Lane {
        vst_id: "mix_808_pan",
        panel_id: "mixer3.pan",
        param: |p| &p.mix.ch808.pan,
        target: Target::MixChannel {
            channel: 2,
            field: 2,
        },
    },
    Lane {
        vst_id: "mix_808_delay",
        panel_id: "mixer3.delay",
        param: |p| &p.mix.ch808.delay,
        target: Target::MixChannel {
            channel: 2,
            field: 3,
        },
    },
    Lane {
        vst_id: "mix_808_dist",
        panel_id: "mixer3.dist",
        param: |p| &p.mix.ch808.dist,
        target: Target::MixChannel {
            channel: 2,
            field: 4,
        },
    },
    Lane {
        vst_id: "mix_909_on",
        panel_id: "mixer4.mute",
        param: |p| &p.mix.ch909.on,
        target: Target::MixChannel {
            channel: 3,
            field: 0,
        },
    },
    Lane {
        vst_id: "mix_909_level",
        panel_id: "mixer4.level",
        param: |p| &p.mix.ch909.level,
        target: Target::MixChannel {
            channel: 3,
            field: 1,
        },
    },
    Lane {
        vst_id: "mix_909_pan",
        panel_id: "mixer4.pan",
        param: |p| &p.mix.ch909.pan,
        target: Target::MixChannel {
            channel: 3,
            field: 2,
        },
    },
    Lane {
        vst_id: "mix_909_delay",
        panel_id: "mixer4.delay",
        param: |p| &p.mix.ch909.delay,
        target: Target::MixChannel {
            channel: 3,
            field: 3,
        },
    },
    Lane {
        vst_id: "mix_909_dist",
        panel_id: "mixer4.dist",
        param: |p| &p.mix.ch909.dist,
        target: Target::MixChannel {
            channel: 3,
            field: 4,
        },
    },
    Lane {
        vst_id: "303_1_tune",
        panel_id: "tb303_1.tune",
        param: |p| &p.tb303_1.tune,
        target: ctrl(Device::Tb303_1, 2),
    },
    Lane {
        vst_id: "303_1_cutoff",
        panel_id: "tb303_1.cutoff",
        param: |p| &p.tb303_1.cutoff,
        target: ctrl(Device::Tb303_1, 3),
    },
    Lane {
        vst_id: "303_1_reso",
        panel_id: "tb303_1.reso",
        param: |p| &p.tb303_1.reso,
        target: ctrl(Device::Tb303_1, 4),
    },
    Lane {
        vst_id: "303_1_envmod",
        panel_id: "tb303_1.envmod",
        param: |p| &p.tb303_1.envmod,
        target: ctrl(Device::Tb303_1, 5),
    },
    Lane {
        vst_id: "303_1_decay",
        panel_id: "tb303_1.decay",
        param: |p| &p.tb303_1.decay,
        target: ctrl(Device::Tb303_1, 6),
    },
    Lane {
        vst_id: "303_1_accent",
        panel_id: "tb303_1.accent",
        param: |p| &p.tb303_1.accent,
        target: ctrl(Device::Tb303_1, 7),
    },
    Lane {
        vst_id: "303_1_wave",
        panel_id: "tb303_1.wave",
        param: |p| &p.tb303_1.wave,
        target: ctrl(Device::Tb303_1, 8),
    },
    Lane {
        vst_id: "303_2_tune",
        panel_id: "tb303_2.tune",
        param: |p| &p.tb303_2.tune,
        target: ctrl(Device::Tb303_2, 2),
    },
    Lane {
        vst_id: "303_2_cutoff",
        panel_id: "tb303_2.cutoff",
        param: |p| &p.tb303_2.cutoff,
        target: ctrl(Device::Tb303_2, 3),
    },
    Lane {
        vst_id: "303_2_reso",
        panel_id: "tb303_2.reso",
        param: |p| &p.tb303_2.reso,
        target: ctrl(Device::Tb303_2, 4),
    },
    Lane {
        vst_id: "303_2_envmod",
        panel_id: "tb303_2.envmod",
        param: |p| &p.tb303_2.envmod,
        target: ctrl(Device::Tb303_2, 5),
    },
    Lane {
        vst_id: "303_2_decay",
        panel_id: "tb303_2.decay",
        param: |p| &p.tb303_2.decay,
        target: ctrl(Device::Tb303_2, 6),
    },
    Lane {
        vst_id: "303_2_accent",
        panel_id: "tb303_2.accent",
        param: |p| &p.tb303_2.accent,
        target: ctrl(Device::Tb303_2, 7),
    },
    Lane {
        vst_id: "303_2_wave",
        panel_id: "tb303_2.wave",
        param: |p| &p.tb303_2.wave,
        target: ctrl(Device::Tb303_2, 8),
    },
    Lane {
        vst_id: "808_accent",
        panel_id: "tr808.ac_level",
        param: |p| &p.tr808.accent,
        target: ctrl(Device::Tr808, 2),
    },
    Lane {
        vst_id: "808_bd_level",
        panel_id: "tr808.bd_level",
        param: |p| &p.tr808.bd.level,
        target: ctrl(Device::Tr808, 3),
    },
    Lane {
        vst_id: "808_bd_tone",
        panel_id: "tr808.bd_tone",
        param: |p| &p.tr808.bd.tone,
        target: ctrl(Device::Tr808, 4),
    },
    Lane {
        vst_id: "808_bd_decay",
        panel_id: "tr808.bd_decay",
        param: |p| &p.tr808.bd.decay,
        target: ctrl(Device::Tr808, 5),
    },
    Lane {
        vst_id: "808_sd_level",
        panel_id: "tr808.sd_level",
        param: |p| &p.tr808.sd.level,
        target: ctrl(Device::Tr808, 6),
    },
    Lane {
        vst_id: "808_sd_tone",
        panel_id: "tr808.sd_tone",
        param: |p| &p.tr808.sd.tone,
        target: ctrl(Device::Tr808, 7),
    },
    Lane {
        vst_id: "808_sd_snappy",
        panel_id: "tr808.sd_snappy",
        param: |p| &p.tr808.sd.snappy,
        target: ctrl(Device::Tr808, 8),
    },
    Lane {
        vst_id: "808_lt_level",
        panel_id: "tr808.lt_level",
        param: |p| &p.tr808.lt.level,
        target: ctrl(Device::Tr808, 9),
    },
    Lane {
        vst_id: "808_lt_tuning",
        panel_id: "tr808.lt_tuning",
        param: |p| &p.tr808.lt.tuning,
        target: ctrl(Device::Tr808, 10),
    },
    Lane {
        vst_id: "808_lt_select",
        panel_id: "tr808.slot3_switch",
        param: |p| &p.tr808.lt.select,
        target: ctrl(Device::Tr808, 11),
    },
    Lane {
        vst_id: "808_mt_level",
        panel_id: "tr808.mt_level",
        param: |p| &p.tr808.mt.level,
        target: ctrl(Device::Tr808, 12),
    },
    Lane {
        vst_id: "808_mt_tuning",
        panel_id: "tr808.mt_tuning",
        param: |p| &p.tr808.mt.tuning,
        target: ctrl(Device::Tr808, 13),
    },
    Lane {
        vst_id: "808_mt_select",
        panel_id: "tr808.slot4_switch",
        param: |p| &p.tr808.mt.select,
        target: ctrl(Device::Tr808, 14),
    },
    Lane {
        vst_id: "808_ht_level",
        panel_id: "tr808.ht_level",
        param: |p| &p.tr808.ht.level,
        target: ctrl(Device::Tr808, 15),
    },
    Lane {
        vst_id: "808_ht_tuning",
        panel_id: "tr808.ht_tuning",
        param: |p| &p.tr808.ht.tuning,
        target: ctrl(Device::Tr808, 16),
    },
    Lane {
        vst_id: "808_ht_select",
        panel_id: "tr808.slot5_switch",
        param: |p| &p.tr808.ht.select,
        target: ctrl(Device::Tr808, 17),
    },
    Lane {
        vst_id: "808_rs_level",
        panel_id: "tr808.rs_level",
        param: |p| &p.tr808.rs.level,
        target: ctrl(Device::Tr808, 18),
    },
    Lane {
        vst_id: "808_rs_select",
        panel_id: "tr808.slot6_switch",
        param: |p| &p.tr808.rs.select,
        target: ctrl(Device::Tr808, 19),
    },
    Lane {
        vst_id: "808_cp_level",
        panel_id: "tr808.cp_level",
        param: |p| &p.tr808.cp.level,
        target: ctrl(Device::Tr808, 20),
    },
    Lane {
        vst_id: "808_cp_select",
        panel_id: "tr808.slot7_switch",
        param: |p| &p.tr808.cp.select,
        target: ctrl(Device::Tr808, 21),
    },
    Lane {
        vst_id: "808_cb_level",
        panel_id: "tr808.cb_level",
        param: |p| &p.tr808.cb.level,
        target: ctrl(Device::Tr808, 22),
    },
    Lane {
        vst_id: "808_cy_level",
        panel_id: "tr808.cy_level",
        param: |p| &p.tr808.cy.level,
        target: ctrl(Device::Tr808, 23),
    },
    Lane {
        vst_id: "808_cy_tone",
        panel_id: "tr808.cy_tone",
        param: |p| &p.tr808.cy.tone,
        target: ctrl(Device::Tr808, 24),
    },
    Lane {
        vst_id: "808_cy_decay",
        panel_id: "tr808.cy_decay",
        param: |p| &p.tr808.cy.decay,
        target: ctrl(Device::Tr808, 25),
    },
    Lane {
        vst_id: "808_oh_level",
        panel_id: "tr808.oh_level",
        param: |p| &p.tr808.oh.level,
        target: ctrl(Device::Tr808, 26),
    },
    Lane {
        vst_id: "808_oh_decay",
        panel_id: "tr808.oh_decay",
        param: |p| &p.tr808.oh.decay,
        target: ctrl(Device::Tr808, 27),
    },
    Lane {
        vst_id: "808_ch_level",
        panel_id: "tr808.ch_level",
        param: |p| &p.tr808.ch.level,
        target: ctrl(Device::Tr808, 28),
    },
    Lane {
        vst_id: "909_accent",
        panel_id: "tr909.ac_level",
        param: |p| &p.tr909.accent,
        target: ctrl(Device::Tr909, 2),
    },
    Lane {
        vst_id: "909_flam",
        panel_id: "tr909.flam",
        param: |p| &p.tr909.flam,
        target: ctrl(Device::Tr909, 29),
    },
    Lane {
        vst_id: "909_bd_level",
        panel_id: "tr909.bd_level",
        param: |p| &p.tr909.bd.level,
        target: ctrl(Device::Tr909, 3),
    },
    Lane {
        vst_id: "909_bd_tune",
        panel_id: "tr909.bd_tune",
        param: |p| &p.tr909.bd.tune,
        target: ctrl(Device::Tr909, 4),
    },
    Lane {
        vst_id: "909_bd_attack",
        panel_id: "tr909.bd_attack",
        param: |p| &p.tr909.bd.attack,
        target: ctrl(Device::Tr909, 5),
    },
    Lane {
        vst_id: "909_bd_decay",
        panel_id: "tr909.bd_decay",
        param: |p| &p.tr909.bd.decay,
        target: ctrl(Device::Tr909, 6),
    },
    Lane {
        vst_id: "909_sd_level",
        panel_id: "tr909.sd_level",
        param: |p| &p.tr909.sd.level,
        target: ctrl(Device::Tr909, 7),
    },
    Lane {
        vst_id: "909_sd_tune",
        panel_id: "tr909.sd_tune",
        param: |p| &p.tr909.sd.tune,
        target: ctrl(Device::Tr909, 8),
    },
    Lane {
        vst_id: "909_sd_tone",
        panel_id: "tr909.sd_tone",
        param: |p| &p.tr909.sd.tone,
        target: ctrl(Device::Tr909, 9),
    },
    Lane {
        vst_id: "909_sd_snappy",
        panel_id: "tr909.sd_snappy",
        param: |p| &p.tr909.sd.snappy,
        target: ctrl(Device::Tr909, 10),
    },
    Lane {
        vst_id: "909_lt_level",
        panel_id: "tr909.lt_level",
        param: |p| &p.tr909.lt.level,
        target: ctrl(Device::Tr909, 11),
    },
    Lane {
        vst_id: "909_lt_tune",
        panel_id: "tr909.lt_tune",
        param: |p| &p.tr909.lt.tune,
        target: ctrl(Device::Tr909, 12),
    },
    Lane {
        vst_id: "909_lt_decay",
        panel_id: "tr909.lt_decay",
        param: |p| &p.tr909.lt.decay,
        target: ctrl(Device::Tr909, 13),
    },
    Lane {
        vst_id: "909_mt_level",
        panel_id: "tr909.mt_level",
        param: |p| &p.tr909.mt.level,
        target: ctrl(Device::Tr909, 14),
    },
    Lane {
        vst_id: "909_mt_tune",
        panel_id: "tr909.mt_tune",
        param: |p| &p.tr909.mt.tune,
        target: ctrl(Device::Tr909, 15),
    },
    Lane {
        vst_id: "909_mt_decay",
        panel_id: "tr909.mt_decay",
        param: |p| &p.tr909.mt.decay,
        target: ctrl(Device::Tr909, 16),
    },
    Lane {
        vst_id: "909_ht_level",
        panel_id: "tr909.ht_level",
        param: |p| &p.tr909.ht.level,
        target: ctrl(Device::Tr909, 17),
    },
    Lane {
        vst_id: "909_ht_tune",
        panel_id: "tr909.ht_tune",
        param: |p| &p.tr909.ht.tune,
        target: ctrl(Device::Tr909, 18),
    },
    Lane {
        vst_id: "909_ht_decay",
        panel_id: "tr909.ht_decay",
        param: |p| &p.tr909.ht.decay,
        target: ctrl(Device::Tr909, 19),
    },
    Lane {
        vst_id: "909_rs_level",
        panel_id: "tr909.rs_level",
        param: |p| &p.tr909.rs.level,
        target: ctrl(Device::Tr909, 20),
    },
    Lane {
        vst_id: "909_cp_level",
        panel_id: "tr909.cp_level",
        param: |p| &p.tr909.cp.level,
        target: ctrl(Device::Tr909, 21),
    },
    Lane {
        vst_id: "909_hh_level",
        panel_id: "tr909.hh_level",
        param: |p| &p.tr909.hh.level,
        target: ctrl(Device::Tr909, 22),
    },
    Lane {
        vst_id: "909_hh_ch_decay",
        panel_id: "tr909.ch_decay",
        param: |p| &p.tr909.hh.ch_decay,
        target: ctrl(Device::Tr909, 23),
    },
    Lane {
        vst_id: "909_hh_oh_decay",
        panel_id: "tr909.oh_decay",
        param: |p| &p.tr909.hh.oh_decay,
        target: ctrl(Device::Tr909, 24),
    },
    Lane {
        vst_id: "909_cc_level",
        panel_id: "tr909.cc_level",
        param: |p| &p.tr909.cc.level,
        target: ctrl(Device::Tr909, 25),
    },
    Lane {
        vst_id: "909_cc_tune",
        panel_id: "tr909.cc_tune",
        param: |p| &p.tr909.cc.tune,
        target: ctrl(Device::Tr909, 26),
    },
    Lane {
        vst_id: "909_rc_level",
        panel_id: "tr909.rc_level",
        param: |p| &p.tr909.rc.level,
        target: ctrl(Device::Tr909, 27),
    },
    Lane {
        vst_id: "909_rc_tune",
        panel_id: "tr909.rc_tune",
        param: |p| &p.tr909.rc.tune,
        target: ctrl(Device::Tr909, 28),
    },
    Lane {
        vst_id: "delay_on",
        panel_id: "delay.on",
        param: |p| &p.delay.on,
        target: ctrl(Device::Delay, 0),
    },
    Lane {
        vst_id: "delay_steps",
        panel_id: "delay.steps",
        param: |p| &p.delay.steps,
        target: ctrl(Device::Delay, 1),
    },
    Lane {
        vst_id: "delay_mode",
        panel_id: "delay.triplet",
        param: |p| &p.delay.mode,
        target: ctrl(Device::Delay, 2),
    },
    Lane {
        vst_id: "delay_feedback",
        panel_id: "delay.feedback",
        param: |p| &p.delay.feedback,
        target: ctrl(Device::Delay, 3),
    },
    Lane {
        vst_id: "delay_pan",
        panel_id: "delay.pan",
        param: |p| &p.delay.pan,
        target: ctrl(Device::Delay, 4),
    },
    Lane {
        vst_id: "dist_on",
        panel_id: "dist.on",
        param: |p| &p.dist.on,
        target: ctrl(Device::Distortion, 0),
    },
    Lane {
        vst_id: "dist_amount",
        panel_id: "dist.amount",
        param: |p| &p.dist.amount,
        target: ctrl(Device::Distortion, 1),
    },
    Lane {
        vst_id: "dist_shape",
        panel_id: "dist.shape",
        param: |p| &p.dist.shape,
        target: ctrl(Device::Distortion, 2),
    },
    Lane {
        vst_id: "pcf_on",
        panel_id: "pcf.on",
        param: |p| &p.pcf.on,
        target: ctrl(Device::Pcf, 0),
    },
    Lane {
        vst_id: "pcf_freq",
        panel_id: "pcf.freq",
        param: |p| &p.pcf.freq,
        target: ctrl(Device::Pcf, 1),
    },
    Lane {
        vst_id: "pcf_q",
        panel_id: "pcf.q",
        param: |p| &p.pcf.q,
        target: ctrl(Device::Pcf, 2),
    },
    Lane {
        vst_id: "pcf_amt",
        panel_id: "pcf.amt",
        param: |p| &p.pcf.amt,
        target: ctrl(Device::Pcf, 3),
    },
    Lane {
        vst_id: "pcf_wave",
        panel_id: "pcf.pattern",
        param: |p| &p.pcf.wave,
        target: ctrl(Device::Pcf, 4),
    },
    Lane {
        vst_id: "pcf_decay",
        panel_id: "pcf.dec",
        param: |p| &p.pcf.decay,
        target: ctrl(Device::Pcf, 5),
    },
    Lane {
        vst_id: "pcf_mode",
        panel_id: "pcf.mode",
        param: |p| &p.pcf.mode,
        target: ctrl(Device::Pcf, 6),
    },
    Lane {
        vst_id: "comp_on",
        panel_id: "comp.on",
        param: |p| &p.comp.on,
        target: ctrl(Device::Compressor, 0),
    },
    Lane {
        vst_id: "comp_ratio",
        panel_id: "comp.ratio",
        param: |p| &p.comp.ratio,
        target: ctrl(Device::Compressor, 1),
    },
    Lane {
        vst_id: "comp_threshold",
        panel_id: "comp.threshold",
        param: |p| &p.comp.threshold,
        target: ctrl(Device::Compressor, 2),
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use orb_engine::frame::Devices;
    use orb_engine::tables::Tables;
    use std::collections::HashSet;

    fn engine() -> Engine {
        Engine::new(
            Arc::new(Tables::new()),
            Devices::implemented(Some(rbm::standard::builtin_samples())),
        )
    }

    #[test]
    fn every_parameter_is_a_lane_in_its_group() {
        let params = PlugParams::default();
        let map = params.param_map();
        let lane_ids: HashSet<&str> = LANES.iter().map(|lane| lane.vst_id).collect();
        let map_ids: HashSet<&str> = map.iter().map(|(id, _, _)| id.as_str()).collect();
        assert_eq!(lane_ids, map_ids);
        let group = |id: &str| {
            map.iter()
                .find(|(got, _, _)| got == id)
                .map(|(_, _, group)| group.as_str())
        };
        assert_eq!(group("303_1_cutoff"), Some("303 1"));
        assert_eq!(group("mix_303_1_level"), Some("Mix/303 1"));
        assert_eq!(group("808_bd_decay"), Some("808/Bass Drum"));
        assert_eq!(group("909_hh_ch_decay"), Some("909/Hi-Hat"));
        assert_eq!(group("delay_feedback"), Some("Delay"));
        assert_eq!(group("pcf_freq"), Some("PCF"));
        assert_eq!(group("comp_ratio"), Some("Compressor"));
        let mut names = HashSet::new();
        for lane in LANES {
            let name = (lane.param)(&params).name();
            assert!(
                names.insert(name.to_owned()),
                "duplicate title {name} ({})",
                lane.vst_id
            );
            assert!(
                name.contains(' '),
                "{} title {name} has no module",
                lane.vst_id
            );
        }
        let title = |id: &str| {
            LANES
                .iter()
                .find(|lane| lane.vst_id == id)
                .map(|lane| (lane.param)(&params).name().to_owned())
                .unwrap()
        };
        assert_eq!(title("comp_ratio"), "Comp Ratio");
        assert_eq!(title("comp_threshold"), "Comp Threshold");
        assert_eq!(title("delay_mode"), "Delay Mode");
        assert_eq!(title("303_1_cutoff"), "303 1 Cutoff");
        assert_eq!(title("808_bd_level"), "808 Bass Drum Level");
        assert_eq!(title("mix_comp"), "Mix Comp Route");
        assert_eq!(title("mix_303_1_level"), "Mix 303 1 Level");
    }

    #[test]
    fn defaults_match_the_engine_and_a_host_change_does_not_clobber_the_panel() {
        let params = PlugParams::default();
        let mut engine = engine();
        for lane in LANES {
            assert_eq!(
                byte((lane.param)(&params)),
                read_engine(&engine, lane.target),
                "{}",
                lane.panel_id
            );
        }
        let mut sent = initial_sent(&params);
        let gestures = Gestures::new();
        let cutoff = LANES
            .iter()
            .position(|lane| lane.panel_id == "tb303_1.cutoff")
            .unwrap();
        let target = LANES[cutoff].target;
        // The host parameter is still the default, so a panel write stays.
        write(&mut engine, target, 70);
        params.apply(&mut engine, &mut sent, &gestures);
        assert_eq!(read_engine(&engine, target), 70);
        // A gesture keeps that byte even if the remembered host value differs.
        write(&mut engine, target, 55);
        sent[cutoff] = 10;
        gestures.set_held(Some("tb303_1.cutoff"));
        params.apply(&mut engine, &mut sent, &gestures);
        assert_eq!(read_engine(&engine, target), 55);
        // The gesture ended. The host byte (still the default) is written once.
        gestures.set_held(None);
        params.apply(&mut engine, &mut sent, &gestures);
        assert_eq!(
            read_engine(&engine, target),
            byte((LANES[cutoff].param)(&params))
        );
        params.apply(&mut engine, &mut sent, &gestures);
        assert_eq!(
            read_engine(&engine, target),
            byte((LANES[cutoff].param)(&params))
        );
    }
}
