//! Task-055: the panel's device messages through the player against the original DLL.
//!
//! A scripted session over the corpus song `full_mix`: pattern mode, play, then every kind of
//! device message the panel sends (the knobs and switches of the 303s, the 808, the 909 and the
//! effects, the mixer's 145 and 147, the pattern header, step, whole-pattern and whole-step
//! edits of all three pattern devices), a switch to song mode while playing with more knob and
//! mixer messages there, stop and play again. The player renders it in 1024-frame calls with
//! each call's commands sent before the call; rbhost (`tools/rbhost`, `--live`) ran the same
//! messages through `ReBirth.dll` on the test machine, written to the port before the same
//! calls, and its WAV's sha256 is [`RBHOST`]. The player's WAV must have that hash: the messages
//! do in rb what they do in the original, byte for byte.
//!
//! To redo the comparison: `RB_LIVE_OUT=DIR cargo test --release -p rb-player --test
//! live_messages -- --include-ignored` writes `DIR/NAME.rblv` (a session as raw messages) and
//! `DIR/NAME.wav` per test; run `rbhost.exe --live NAME.rblv full_mix.rbhm out.wav` (the
//! `.rbhm` from `tools/rbhost/rbsmsg.py tests/songs/diag/songs/full_mix.rbs`) and compare the
//! hashes.
//!
//! The recording sessions (task-057) record in song mode and also compare the tracks: song msg
//! 22 queries ([`Ev::Tracks`]) of all nine tracks at two calls, whose msg 37 replies rbhost
//! writes with `--live-replies FILE`; the payloads must equal rb's tracks byte for byte.
//!
//! The mod-change sessions (task-076, `docs/rbm.md` section 14.4) play a drum song and switch
//! its mod with song msg 19 mid-song, `Player::set_mod` here. They need `RB_MOD_DIR`, the
//! recrypted mods (skipped without it). rbhost loads the original mods (the msg 55 of
//! `tools/rbm.py msg` with the mod id the session names) with `--mod-msg` (the song's mod) and
//! `--load-msg` (the others; hex `38 01 9B`, 56 {411}, for the built-in kit), and its `.rbhm`
//! carries the song's mod id (`rbsmsg.message_file(song, 412)`).

mod common;

use std::sync::Arc;

use common::{manifest, samples, send, song, tables};
use rb_engine::frame::export;
use rb_engine::samples::SampleSet;
use rb_engine::song::{Device, Song};
use rb_player::{CALL_FRAMES, Command, Player};
use sha2::{Digest, Sha256};

/// The sha256 of rbhost's WAV of [`session`] (`ReBirth.dll` 2.01, 2026-10-27).
const RBHOST: &str = "960e1218589577b8841ac65e77764d4950ad6614b4ce692c3cf04e392559071d";

/// The sha256 of rbhost's WAV of [`pcf_session`] (2026-10-27).
const RBHOST_PCF: &str = "4a50f82aacd9b7eaf98965197b93ad0a86437362e919484ae1c7f0a2a256172c";

/// The sha256 of rbhost's WAV of [`pcf_303_session`] (task-072, 2026-10-27).
const RBHOST_PCF_303: &str = "7e57e65c1cf82e28d1a47ac00d57607c5b440a2ee8423ed19382e1018dca4913";

/// The engine calls of [`pcf_303_session`].
const PCF_303_CALLS: u32 = 460;

/// The engine calls of the session.
const CALLS: u32 = 600;

/// The `{ctrl, v}` sub of a device.
fn param_sub(d: Device) -> u8 {
    match d {
        Device::Tb303_1 | Device::Tb303_2 => 40,
        Device::Tr808 => 57,
        Device::Tr909 => 74,
        Device::Delay => 84,
        Device::Distortion => 92,
        Device::Compressor => 102,
        Device::Pcf => 114,
        Device::Mixer => unreachable!(),
    }
}

/// The first pattern sub of a pattern device (the header field message); the others follow it.
fn pattern_sub(d: Device) -> u8 {
    match d {
        Device::Tb303_1 | Device::Tb303_2 => 42,
        Device::Tr808 => 59,
        Device::Tr909 => 76,
        _ => unreachable!(),
    }
}

/// The song message the exe would send for `c` (the slot byte is rbhost's to fill).
fn raw(c: &Command) -> Vec<u8> {
    let dev = |d: Device, sub: u8, payload: &[u8]| {
        let mut m = vec![78, 0, 41, d.id(), sub];
        m.extend_from_slice(payload);
        m
    };
    match c {
        Command::SongMode(on) => vec![78, 0, 35, 0, *on as u8],
        Command::Play => vec![78, 0, 35, 1, 1],
        Command::Stop => vec![78, 0, 35, 1, 0],
        Command::Record(on) => vec![78, 0, 35, 2, *on as u8],
        Command::LoopOn(on) => vec![78, 0, 35, 3, *on as u8],
        Command::Locate(t) => global32(8, *t),
        Command::LoopStart(t) => global32(12, *t),
        Command::LoopEnd(t) => global32(16, *t),
        Command::DeviceParam {
            device,
            ctrl,
            value,
        } => dev(*device, param_sub(*device), &[*ctrl, *value]),
        Command::MixerGlobal { id, value } => dev(Device::Mixer, 145, &[*id, *value]),
        Command::MixerChannel {
            channel,
            field,
            value,
        } => dev(Device::Mixer, 147, &[*channel, *field, *value]),
        Command::PatternField {
            device,
            pattern,
            field,
            value,
        } => dev(*device, pattern_sub(*device), &[*pattern, *field, *value]),
        Command::PatternHeader {
            device,
            pattern,
            header,
        } => dev(
            *device,
            pattern_sub(*device) + 1,
            &[*pattern, header[0], header[1]],
        ),
        Command::Pattern {
            device,
            pattern,
            bytes,
        } => {
            let mut p = vec![*pattern];
            p.extend_from_slice(bytes);
            dev(*device, pattern_sub(*device) + 2, &p)
        }
        Command::StepField {
            device,
            pattern,
            step,
            field,
            value,
        } => dev(
            *device,
            pattern_sub(*device) + 3,
            &[*pattern, *step, *field, *value],
        ),
        Command::Step {
            device,
            pattern,
            step,
            bytes,
        } => {
            let n = if matches!(device, Device::Tb303_1 | Device::Tb303_2) {
                6
            } else {
                12
            };
            let mut p = vec![*pattern, *step];
            p.extend_from_slice(&bytes[..n]);
            dev(*device, pattern_sub(*device) + 4, &p)
        }
        _ => unreachable!("not in the session"),
    }
}

/// Song msg 35 with a u32 value (ids 4, 8, 12, 16), big-endian.
fn global32(id: u8, v: u32) -> Vec<u8> {
    let mut m = vec![78, 0, 35, id];
    m.extend_from_slice(&v.to_be_bytes());
    m
}

fn param(device: Device, ctrl: u8, value: u8) -> Command {
    Command::DeviceParam {
        device,
        ctrl,
        value,
    }
}

fn step_field(device: Device, pattern: u8, step: u8, field: u8, value: u8) -> Command {
    Command::StepField {
        device,
        pattern,
        step,
        field,
        value,
    }
}

/// The session: (call, command), in call order.
fn session(pat: [u8; 4]) -> Vec<(u32, Command)> {
    use Device as D;
    let [p1, p2, p8, p9] = pat;
    let mut s: Vec<(u32, Command)> = vec![(0, Command::SongMode(false)), (0, Command::Play)];
    let mut at = |call: u32, c: Command| s.push((call, c));
    // the 303s' knobs and switches, the extra byte 10 and the ignored 9
    at(20, param(D::Tb303_1, 3, 100));
    at(20, param(D::Tb303_1, 4, 120));
    at(21, param(D::Tb303_1, 6, 30));
    at(22, param(D::Tb303_1, 8, 1));
    at(23, param(D::Tb303_1, 10, 5));
    at(23, param(D::Tb303_1, 9, 3));
    at(24, param(D::Tb303_2, 2, 80));
    at(24, param(D::Tb303_2, 7, 127));
    // the 303 step editor on the playing pattern: notes, rest off, accent, slide, up, down
    for st in 0..8u8 {
        let c = 30 + st as u32;
        at(c, step_field(D::Tb303_1, p1, st, 0, (st * 5) % 13));
        at(c, step_field(D::Tb303_1, p1, st, 5, st % 3 / 2));
        at(c, step_field(D::Tb303_1, p1, st, 2, st % 2));
        at(c, step_field(D::Tb303_1, p1, st, 1, (st % 4 == 1) as u8));
        at(c, step_field(D::Tb303_1, p1, st, 3, (st == 6) as u8));
        at(c, step_field(D::Tb303_1, p1, st, 4, (st == 7) as u8));
    }
    // the pattern section: length and shuffle of 303 #2's pattern
    at(
        50,
        Command::PatternField {
            device: D::Tb303_2,
            pattern: p2,
            field: 1,
            value: 8,
        },
    );
    at(
        51,
        Command::PatternField {
            device: D::Tb303_2,
            pattern: p2,
            field: 0,
            value: 1,
        },
    );
    // the 808: knobs, the selector switches, steps, a whole pattern
    at(60, param(D::Tr808, 3, 127));
    at(60, param(D::Tr808, 4, 20));
    at(61, param(D::Tr808, 19, 1));
    at(61, param(D::Tr808, 30, 9));
    for st in [0u8, 4, 8, 12] {
        at(62, step_field(D::Tr808, p8, st, 1, 1));
        at(63, step_field(D::Tr808, p8, st + 2, 11, 1));
    }
    at(64, step_field(D::Tr808, p8, 6, 0, 1));
    let mut pat808 = vec![0u8; 194];
    pat808[1] = 16;
    for st in 0..16 {
        pat808[2 + 12 * st + 1] = (st % 4 == 0) as u8;
        pat808[2 + 12 * st + 2] = (st % 8 == 4) as u8;
        pat808[2 + 12 * st + 11] = (st % 2 == 1) as u8;
    }
    at(
        150,
        Command::Pattern {
            device: D::Tr808,
            pattern: p8,
            bytes: pat808.into_boxed_slice(),
        },
    );
    // the 909: the permuted levels 20..22, flam, the extra byte 31; steps with flam; a step; a
    // header
    at(80, param(D::Tr909, 20, 120));
    at(80, param(D::Tr909, 21, 30));
    at(80, param(D::Tr909, 22, 60));
    at(81, param(D::Tr909, 29, 10));
    at(81, param(D::Tr909, 31, 7));
    for st in [0u8, 3, 6, 10] {
        at(82, step_field(D::Tr909, p9, st, 1, 2));
        at(83, step_field(D::Tr909, p9, st + 1, 2, 3));
        at(84, step_field(D::Tr909, p9, st + 2, 8, 1));
    }
    let mut st909 = [0u8; 12];
    st909[0] = 2;
    st909[7] = 1;
    st909[9] = 3;
    at(
        85,
        Command::Step {
            device: D::Tr909,
            pattern: p9,
            step: 14,
            bytes: st909,
        },
    );
    at(
        86,
        Command::PatternHeader {
            device: D::Tr909,
            pattern: p9,
            header: [1, 15],
        },
    );
    // the effects
    at(100, param(D::Delay, 0, 1));
    at(100, param(D::Delay, 1, 5));
    at(101, param(D::Delay, 2, 0));
    at(101, param(D::Delay, 3, 100));
    at(102, param(D::Delay, 4, 20));
    at(102, param(D::Delay, 5, 99));
    at(103, param(D::Distortion, 0, 1));
    at(103, param(D::Distortion, 1, 110));
    at(104, param(D::Distortion, 2, 90));
    at(105, param(D::Pcf, 0, 1));
    at(105, param(D::Pcf, 1, 40));
    at(106, param(D::Pcf, 2, 100));
    at(106, param(D::Pcf, 4, 30));
    at(107, param(D::Pcf, 6, 0));
    at(108, param(D::Compressor, 0, 1));
    at(108, param(D::Compressor, 1, 120));
    at(109, param(D::Compressor, 2, 20));
    // the mixer: master, routing, a mute and back, levels, pans, sends, dist
    at(120, Command::MixerGlobal { id: 0, value: 90 });
    at(121, Command::MixerGlobal { id: 2, value: 1 });
    at(122, Command::MixerGlobal { id: 1, value: 3 });
    at(123, Command::MixerGlobal { id: 3, value: 9 });
    let ch = |channel, field, value| Command::MixerChannel {
        channel,
        field,
        value,
    };
    at(124, ch(3, 0, 0));
    at(124, ch(0, 4, 1));
    at(125, ch(1, 1, 60));
    at(125, ch(1, 2, 0));
    at(126, ch(2, 3, 90));
    at(126, ch(0, 5, 9));
    at(140, ch(3, 0, 1));
    at(141, ch(3, 2, 127));
    // 303 #2: a whole pattern (as Clear sends it), a whole step, both header bytes
    let mut clear = vec![0u8; 98];
    clear[1] = 16;
    for st in 0..16 {
        clear[2 + 6 * st + 5] = 1;
    }
    at(
        200,
        Command::Pattern {
            device: D::Tb303_2,
            pattern: p2,
            bytes: clear.into_boxed_slice(),
        },
    );
    let mut st303 = [0u8; 12];
    st303[..6].copy_from_slice(&[7, 0, 1, 0, 0, 0]);
    at(
        201,
        Command::Step {
            device: D::Tb303_2,
            pattern: p2,
            step: 0,
            bytes: st303,
        },
    );
    st303[..6].copy_from_slice(&[3, 1, 0, 1, 0, 0]);
    at(
        201,
        Command::Step {
            device: D::Tb303_2,
            pattern: p2,
            step: 4,
            bytes: st303,
        },
    );
    at(
        202,
        Command::PatternHeader {
            device: D::Tb303_2,
            pattern: p2,
            header: [0, 6],
        },
    );
    // song mode while playing (record off), knobs and the mixer there
    at(300, Command::SongMode(true));
    at(310, param(D::Tb303_1, 3, 10));
    at(311, ch(1, 1, 127));
    at(312, Command::MixerGlobal { id: 0, value: 70 });
    at(313, param(D::Tr909, 3, 50));
    at(410, param(D::Tb303_1, 5, 127));
    // stop and play again, a step edit after it
    at(450, Command::Stop);
    at(470, Command::Play);
    at(500, step_field(D::Tb303_1, p1, 2, 0, 12));
    // in call order, the order within a call kept
    s.sort_by_key(|(call, _)| *call);
    s
}

/// task-069: the PCF's wave set to a stepping pattern in pattern mode, then song mode and back
/// to pattern mode while playing. The song's wave 64 (resolution 0) takes the step from the
/// stale stack slot in song mode, which on the 909's channel holds the 909 audio's return
/// address 0x1002B8B4; whether the first block back on wave 30 (call 400) is a new step depends
/// on it.
fn pcf_session() -> Vec<(u32, Command)> {
    vec![
        (0, Command::SongMode(false)),
        (0, Command::Play),
        (106, param(Device::Pcf, 4, 30)),
        (300, Command::SongMode(true)),
        (400, Command::SongMode(false)),
        (450, Command::Stop),
        (470, Command::Play),
    ]
}

/// task-072: the PCF on the 303 channels with wave 108 (resolution 0, length 32, every step a
/// velocity), so every block's step is the stale stack slot `var_1C` modulo 32 and each change
/// of the word there retriggers. The word is what the code before the PCF left at that stack
/// address in the block (the render call's lock, the mixer update's compressor id, the clock's
/// tempo, a compressor on channel 0) or in the block before (the 909 audio's return address, the
/// delay's output pointer, a compressor on channel 3); the session goes through each of them.
fn pcf_303_session() -> Vec<(u32, Command)> {
    use Device as D;
    let global = |id, value| Command::MixerGlobal { id, value };
    vec![
        (0, Command::SongMode(false)),
        (0, Command::Play),
        (0, param(D::Pcf, 4, 108)),
        // the PCF on 303 #1, the compressor on the master, the delay on
        (0, global(2, 1)),
        (60, param(D::Delay, 0, 0)),
        // the compressor on channel 0 and the PCF after it, on 303 #2
        (100, global(1, 2)),
        (100, global(2, 2)),
        (140, param(D::Compressor, 0, 0)),
        (180, param(D::Delay, 0, 1)),
        (220, Command::Stop),
        (260, Command::Play),
        // the compressor on the 909's channel, after the 909 audio, with the delay off
        (300, param(D::Compressor, 0, 1)),
        (300, global(1, 5)),
        (300, param(D::Delay, 0, 0)),
        // back on 303 #1, a real wave, then the junk wave again
        (340, global(2, 1)),
        (360, param(D::Pcf, 4, 30)),
        (380, param(D::Pcf, 4, 108)),
        (420, Command::Stop),
    ]
}

/// task-057: recording. Song mode over a two-bar loop with record on while playing: knobs,
/// switches, a 303 pattern change and an 808 on/off (both rounded up to the bar), the mixer's
/// level and PCF routing, the effects; through a loop wrap (which merges) to record off
/// mid-pass, just after a pattern change that the merge loses (OB-18); later, record on while
/// stopped (rounded down), a locate forward with record on (applies instead of chasing), play,
/// more knobs and record off; then the loop off. `record` false leaves out the Record
/// commands: the control, which must render otherwise.
fn record_session(pat303: u8, record: bool) -> Vec<(u32, Command)> {
    use Device as D;
    let ch = |channel, field, value| Command::MixerChannel {
        channel,
        field,
        value,
    };
    let mut s: Vec<(u32, Command)> = vec![
        (0, Command::SongMode(true)),
        (0, Command::LoopStart(0)),
        (0, Command::LoopEnd(2 * 768)),
        (0, Command::LoopOn(true)),
        (0, Command::Locate(0)),
        (0, Command::Record(true)),
        (0, Command::Play),
        (10, param(D::Tb303_1, 3, 20)),
        (12, param(D::Tb303_1, 4, 100)),
        (15, param(D::Tb303_1, 3, 60)),
        (20, param(D::Tr909, 3, 50)),
        (20, ch(1, 1, 30)),
        (21, Command::MixerGlobal { id: 2, value: 1 }),
        (22, param(D::Delay, 3, 90)),
        (30, param(D::Tb303_1, 1, pat303 ^ 1)),
        (40, param(D::Tr808, 0, 0)),
        (200, param(D::Tb303_2, 5, 120)),
        (210, param(D::Distortion, 1, 100)),
        (250, param(D::Tb303_1, 1, pat303)),
        (252, Command::Record(false)),
        (330, Command::Stop),
        (340, Command::Record(true)),
        (341, param(D::Tb303_1, 6, 10)),
        (342, Command::Locate(1152)),
        (343, param(D::Tb303_1, 5, 30)),
        (350, Command::Play),
        (360, ch(0, 2, 10)),
        (370, param(D::Tr909, 3, 120)),
        (400, Command::Record(false)),
        (450, Command::LoopOn(false)),
    ];
    for k in 0..10u32 {
        s.push((60 + 10 * k, param(D::Tb303_1, 3, (10 * k + 5) as u8)));
    }
    if !record {
        s.retain(|(_, c)| !matches!(c, Command::Record(_)));
    }
    s.sort_by_key(|(call, _)| *call);
    s
}

/// task-057: recording against automation, over `mixer_level_sweep_303` (a level sweep on
/// mixer channel 0): the channel's level knob moved while the sweep plays (held: the track's
/// events of that controller are skipped until the merge), a last move and record off in one
/// call (the move, rounded up past the merge tick, is lost: OB-18; the sweep returns at the
/// next unit), record on and a knob, pattern mode with record on and record off there (a
/// merge with no chase), back to song mode, then record on near bar 999 with the loop off and
/// a knob, so that the song end merges, stops and locates to 0; play from there.
fn sweep_session(record: bool) -> Vec<(u32, Command)> {
    let level = |value| Command::MixerChannel {
        channel: 0,
        field: 1,
        value,
    };
    let mut s: Vec<(u32, Command)> = vec![
        (0, Command::SongMode(true)),
        (0, Command::LoopStart(0)),
        (0, Command::LoopEnd(4 * 768)),
        (0, Command::LoopOn(true)),
        (0, Command::Locate(0)),
        (0, Command::Record(true)),
        (0, Command::Play),
        (
            50,
            Command::MixerChannel {
                channel: 0,
                field: 2,
                value: 20,
            },
        ),
        // a change just before record off, rounded up past the merge tick: lost (OB-18)
        (141, level(99)),
        (141, Command::Record(false)),
        (200, Command::Record(true)),
        (210, level(127)),
        (220, Command::SongMode(false)),
        (230, Command::Record(false)),
        (240, param(Device::Tb303_1, 3, 30)),
        (250, Command::SongMode(true)),
        (300, Command::LoopOn(false)),
        (300, Command::Locate(766_464)),
        (301, Command::Record(true)),
        (310, level(40)),
        (450, Command::Play),
    ];
    for k in 0..10u8 {
        s.push((30 + 10 * k as u32, level(10 * k)));
    }
    if !record {
        s.retain(|(_, c)| !matches!(c, Command::Record(_)));
    }
    s.sort_by_key(|(call, _)| *call);
    s
}

/// The sha256 of rbhost's WAV of [`record_session`] over `full_mix` (`ReBirth.dll` 2.01,
/// 2026-10-27).
const RBHOST_RECORD: &str = "bd067446fcb9f5bb193f0bfc7c2cb620f2b28fe004941ec8e88ba7c10ee1bfb8";

/// The sha256 of rbhost's WAV of [`sweep_session`] (2026-10-27).
const RBHOST_SWEEP: &str = "e821590917a3cbe3e77a34d7e63fcb4275ea6607495694631e4cf39fe52e5a94";

/// The sha256 of the msg 37 payloads (each reply's bytes after its head `{78, slot, 37}`, as
/// [`track_reply`] builds them) the DLL gave the track queries of [`recording_equals_rbhost`]:
/// all nine tracks after the first record off (call 253) and at the end (call 599), rbhost
/// `--live-replies` (2026-10-27).
const RBHOST_RECORD_TRACKS: &str =
    "e44e1bff6418fcdaa7036b15c44901358455eb1022743d5199e63ad69ffea36e";

/// The same for [`recording_against_automation_equals_rbhost`] (calls 142 and 599).
const RBHOST_SWEEP_TRACKS: &str =
    "703927890fe42d7cd08c87ff4f1001146bcea6d480c17cc567edd487ea5379d2";

/// `events` with the queries of all nine tracks at each of `calls`.
fn with_track_queries(mut events: Vec<(u32, Ev)>, calls: &[u32]) -> Vec<(u32, Ev)> {
    for &c in calls {
        events.extend((0..9).map(|t| (c, Ev::Tracks(t))));
    }
    events.sort_by_key(|(call, _)| *call);
    events
}

fn sha256(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[test]
fn recording_against_automation_equals_rbhost() {
    let s = song(&manifest()["diag"]["mixer_level_sweep_303"]);
    let ev = with_track_queries(cmds(sweep_session(true)), &[142, CALLS - 1]);
    let (sha, tracks) = render_with_tracks("sweep", &s, samples(), CALLS, ev);
    let plain = render(
        "sweep_off",
        &s,
        samples(),
        CALLS,
        cmds(sweep_session(false)),
    );
    assert_ne!(sha, plain, "the recording changes nothing");
    assert_eq!(
        sha, RBHOST_SWEEP,
        "the player's session differs from rbhost's"
    );
    assert_eq!(
        sha256(&tracks.concat()),
        RBHOST_SWEEP_TRACKS,
        "the player's tracks differ from the DLL's"
    );
}

#[test]
fn recording_equals_rbhost() {
    let s = song(&manifest()["diag"]["full_mix"]);
    let p = s.device_payload(Device::Tb303_1)[1];
    let ev = with_track_queries(cmds(record_session(p, true)), &[253, CALLS - 1]);
    let (sha, tracks) = render_with_tracks("record", &s, samples(), CALLS, ev);
    let plain = render(
        "record_off",
        &s,
        samples(),
        CALLS,
        cmds(record_session(p, false)),
    );
    assert_ne!(sha, plain, "the recording changes nothing");
    assert_eq!(
        sha, RBHOST_RECORD,
        "the player's session differs from rbhost's"
    );
    assert_eq!(
        sha256(&tracks.concat()),
        RBHOST_RECORD_TRACKS,
        "the player's tracks differ from the DLL's"
    );
}

/// The pattern each pattern device plays in pattern mode: its data block's controller 1.
fn patterns(s: &Song) -> [u8; 4] {
    [
        Device::Tb303_1,
        Device::Tb303_2,
        Device::Tr808,
        Device::Tr909,
    ]
    .map(|d| s.device_payload(d)[1])
}

/// What a session does before a call: a panel command, or the Mods menu's change to the mod
/// with this id (song msg 19, `docs/rbm.md` section 14) and its samples.
enum Ev {
    Cmd(Command),
    Mod(u16, Arc<SampleSet>),
    /// Song msg 22 `{track, u32 from, u32 to}` (0x10012346 → `sub_10020EC0`), the exe's query
    /// of a track's events (nodes `from..to` of the first list, clamped to its length), which
    /// the DLL answers with a msg 37 of `{track, u32 from, u32 to}` and 7-byte records `[99,
    /// u32 tick, ctrl, value]` (big-endian). In rb nothing is sent: the track is read from the
    /// current song before the call renders, so the call must hold no command.
    Tracks(u8),
}

fn cmds(events: Vec<(u32, Command)>) -> Vec<(u32, Ev)> {
    events.into_iter().map(|(c, x)| (c, Ev::Cmd(x))).collect()
}

/// The msg 37 payload of [`Ev::Tracks`]: track `t`'s events, all of them.
fn track_reply(t: u8, events: &[rb_engine::seq::Event]) -> Vec<u8> {
    let mut b = vec![t];
    b.extend_from_slice(&0u32.to_be_bytes());
    b.extend_from_slice(&(events.len() as u32).to_be_bytes());
    for e in events {
        b.push(99);
        b.extend_from_slice(&(e.tick as u32).to_be_bytes());
        b.extend_from_slice(&[e.ctrl, e.value]);
    }
    b
}

/// Renders `events` over `s` (built with `kit`) through the player for `calls` calls and returns
/// its WAV's sha256; with `RB_LIVE_OUT` set, writes the session for rbhost and the WAV as `name`.
fn render(name: &str, s: &Song, kit: Arc<SampleSet>, calls: u32, events: Vec<(u32, Ev)>) -> String {
    render_with_tracks(name, s, kit, calls, events).0
}

/// [`render`], and the msg 37 payloads of the session's [`Ev::Tracks`] queries, in order.
fn render_with_tracks(
    name: &str,
    s: &Song,
    kit: Arc<SampleSet>,
    calls: u32,
    events: Vec<(u32, Ev)>,
) -> (String, Vec<Vec<u8>>) {
    let mut replies = Vec::new();
    let mut rblv = b"RBLV".to_vec();
    rblv.extend_from_slice(&1u32.to_le_bytes());
    rblv.extend_from_slice(&calls.to_le_bytes());
    rblv.extend_from_slice(&(events.len() as u32).to_le_bytes());
    for (call, e) in &events {
        let r = match e {
            Ev::Cmd(c) => raw(c),
            // sub_4259FF: song msg 19 {u16 BE id}
            Ev::Mod(id, _) => vec![78, 0, 19, (id >> 8) as u8, *id as u8],
            Ev::Tracks(t) => {
                let mut m = vec![78, 0, 22, *t];
                m.extend_from_slice(&0u32.to_be_bytes());
                m.extend_from_slice(&(rb_engine::seq::POOL_NODES as u32).to_be_bytes());
                m
            }
        };
        rblv.extend_from_slice(&call.to_le_bytes());
        rblv.extend_from_slice(&(r.len() as u32).to_le_bytes());
        rblv.extend_from_slice(&r);
    }
    let (mut p, mut a) = Player::new(tables(), Some(samples()));
    assert!(p.load(s, Some(kit)).is_ok());
    let mut out = vec![0i16; 2 * CALL_FRAMES * calls as usize];
    let mut it = events.into_iter().peekable();
    for (call, buf) in out.chunks_mut(2 * CALL_FRAMES).enumerate() {
        let mut sent = false;
        while let Some((_, e)) = it.next_if(|(at, _)| *at == call as u32) {
            match e {
                Ev::Cmd(c) => send(&mut p, c),
                Ev::Mod(_, m) => assert!(p.set_mod(m).is_ok(), "the command queue is full"),
                Ev::Tracks(t) => {
                    assert!(!sent, "a track query in a call with commands");
                    let seq = &a.current().frame.seq;
                    replies.push(track_reply(t, seq.events(t as usize)));
                    continue;
                }
            }
            sent = true;
        }
        a.render_i16(buf);
    }
    assert!(it.next().is_none(), "the events are not in call order");
    assert!(out.iter().any(|&x| x != 0));
    let wav = export::wav_bytes(&out);
    let sha: String = Sha256::digest(&wav)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if let Some(dir) = common::env_path("RB_LIVE_OUT") {
        std::fs::write(dir.join(format!("{name}.rblv")), &rblv).unwrap();
        std::fs::write(dir.join(format!("{name}.wav")), &wav).unwrap();
        eprintln!("{name}: wav sha256 {sha}");
        if !replies.is_empty() {
            std::fs::write(dir.join(format!("{name}.tracks")), replies.concat()).unwrap();
        }
    }
    (sha, replies)
}

#[test]
fn the_session_equals_rbhost() {
    let s = song(&manifest()["diag"]["full_mix"]);
    let sha = render("live", &s, samples(), CALLS, cmds(session(patterns(&s))));
    assert_eq!(sha, RBHOST, "the player's session differs from rbhost's");
}

#[test]
fn pcf_wave_after_returning_to_pattern_mode() {
    let s = song(&manifest()["diag"]["full_mix"]);
    let sha = render("pcf", &s, samples(), CALLS, cmds(pcf_session()));
    assert_eq!(
        sha, RBHOST_PCF,
        "the player's session differs from rbhost's"
    );
}

#[test]
fn pcf_junk_wave_on_the_303_channels() {
    let s = song(&manifest()["diag"]["full_mix"]);
    let sha = render(
        "pcf303",
        &s,
        samples(),
        PCF_303_CALLS,
        cmds(pcf_303_session()),
    );
    assert_eq!(
        sha, RBHOST_PCF_303,
        "the player's session differs from rbhost's"
    );
}

// ------------------------------------------------------------------ the mod change (task-076)

/// The id of the built-in kit (exe sub_41693C, message 56 {411}).
const BUILTIN: u16 = 411;

/// The calls of a mod-change session.
const MOD_CALLS: u32 = 400;

/// A recrypted mod of `RB_MOD_DIR` loaded over the built-in kit, as `rb render` loads it; `None`
/// (the test is skipped) when `RB_MOD_DIR` is not set.
fn file_mod(file: &str) -> Option<Arc<SampleSet>> {
    let p = common::env_path("RB_MOD_DIR")?.join(file);
    let data = std::fs::read(&p).unwrap_or_else(|err| panic!("{}: {err}", p.display()));
    let rbm = rbm::Rbm::parse(data).unwrap();
    let table = rbm::KeyTable::embedded(&rbm)
        .filter(|t| rbm.slots_covered(t))
        .unwrap_or_else(|| panic!("{}: not a recrypted mod", p.display()));
    Some(Arc::new(rbm.load_samples(&table, &samples()).unwrap().set))
}

/// Pattern mode and play at call 0, the transport commands `transport`, and the mod changes
/// `changes` (call, mod id, kit) over `s` built with `kit`: the player's sha256 must be `expect`,
/// rbhost's, and the same session without the changes must render otherwise (the changes matter).
fn mod_session(
    name: &str,
    s: &Song,
    kit: Arc<SampleSet>,
    transport: &[(u32, bool)],
    changes: Vec<(u32, u16, Arc<SampleSet>)>,
    expect: &str,
) {
    let base = || {
        let mut v = vec![(0, Command::SongMode(false)), (0, Command::Play)];
        v.extend(
            transport
                .iter()
                .map(|&(c, play)| (c, if play { Command::Play } else { Command::Stop })),
        );
        cmds(v)
    };
    let mut ev = base();
    ev.extend(changes.into_iter().map(|(c, id, m)| (c, Ev::Mod(id, m))));
    ev.sort_by_key(|(call, _)| *call);
    let sha = render(name, s, kit.clone(), MOD_CALLS, ev);
    assert_eq!(sha, expect, "the player's session differs from rbhost's");
    let plain = render(&format!("{name}_unchanged"), s, kit, MOD_CALLS, base());
    assert_ne!(plain, expect, "the mod changes change nothing");
}

/// rbhost's WAV of [`mods_on_808_and_909`] (2026-10-27): `drums_kit` (808 and 909) built with
/// the built-in kit, PBE2.rbm (id 412) and Orbit2.rbm (413) loaded with `--load-msg`; 412 at call
/// 100, 413 at 200, stop at 270, the built-in kit at 290 while stopped, play at 300.
const RBHOST_MOD_KIT: &str = "0ded17da6c345854237f3176b1c605ddc117defd3a331ba09a36f5cba3cbe841";

/// rbhost's WAV of [`mod_change_808_long_cymbal`]: `808_cymbal` built with Metallicon.rbm
/// (`--mod-msg`, 412), MSM2.rbm (413) and the built-in kit loaded; 413 at call 150, 411 at 250.
const RBHOST_MOD_808: &str = "3954a36be3017e835f22bd15bfed4dfde4b90135862ac8b5ad317df28f4ea4ba";

/// rbhost's WAV of [`mod_change_909_long_crash`]: `909_crash` built with Orbit2.rbm (412),
/// the built-in kit and PBE2.rbm (413) loaded; 411 at call 150, 413 at 250.
const RBHOST_MOD_909: &str = "cf414eb6493779e935fa8e12fea27c9fadeaf045beaaa5febb9722efbc98d0ba";
#[test]
fn mods_on_808_and_909() {
    let (Some(pbe), Some(orbit)) = (file_mod("PBE2.rbm"), file_mod("Orbit2.rbm")) else {
        return eprintln!("skipped: RB_MOD_DIR (the recrypted mods) is not set");
    };
    let s = song(&manifest()["diag"]["drums_kit"]);
    let changes = vec![
        (100, 412, pbe),
        (200, 413, orbit),
        (290, BUILTIN, samples()),
    ];
    let transport = [(270, false), (300, true)];
    mod_session(
        "mod_kit",
        &s,
        samples(),
        &transport,
        changes,
        RBHOST_MOD_KIT,
    );
}

#[test]
fn mod_change_808_long_cymbal() {
    let (Some(metal), Some(msm)) = (file_mod("Metallicon.rbm"), file_mod("MSM2.rbm")) else {
        return eprintln!("skipped: RB_MOD_DIR (the recrypted mods) is not set");
    };
    let s = song(&manifest()["diag"]["808_cymbal"]);
    let changes = vec![(150, 413, msm), (250, BUILTIN, samples())];
    mod_session("mod_808", &s, metal, &[], changes, RBHOST_MOD_808);
}

#[test]
fn mod_change_909_long_crash() {
    let (Some(orbit), Some(pbe)) = (file_mod("Orbit2.rbm"), file_mod("PBE2.rbm")) else {
        return eprintln!("skipped: RB_MOD_DIR (the recrypted mods) is not set");
    };
    let s = song(&manifest()["diag"]["909_crash"]);
    let changes = vec![(150, BUILTIN, samples()), (250, 413, pbe)];
    mod_session("mod_909", &s, orbit, &[], changes, RBHOST_MOD_909);
}
