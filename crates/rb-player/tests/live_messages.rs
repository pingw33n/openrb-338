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

mod common;

use common::{manifest, samples, send, song, tables};
use rb_engine::frame::export;
use rb_engine::song::{Device, Song};
use rb_player::{CALL_FRAMES, Command, Player};
use sha2::{Digest, Sha256};

/// The sha256 of rbhost's WAV of [`session`] (`ReBirth.dll` 2.01, 2026-10-27).
const RBHOST: &str = "960e1218589577b8841ac65e77764d4950ad6614b4ce692c3cf04e392559071d";

/// The sha256 of rbhost's WAV of [`pcf_session`] (2026-10-27).
const RBHOST_PCF: &str = "4a50f82aacd9b7eaf98965197b93ad0a86437362e919484ae1c7f0a2a256172c";

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
    // the 909: the permuted levels 20..22, flam, the extra byte 31; steps with flam; a step; a header
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

/// Renders `events` over `full_mix` through the player and returns its WAV's sha256; with
/// `RB_LIVE_OUT` set, writes the session for rbhost and the WAV as `name`.
fn render(name: &str, s: &Song, events: Vec<(u32, Command)>) -> String {
    let mut rblv = b"RBLV".to_vec();
    rblv.extend_from_slice(&1u32.to_le_bytes());
    rblv.extend_from_slice(&CALLS.to_le_bytes());
    rblv.extend_from_slice(&(events.len() as u32).to_le_bytes());
    for (call, c) in &events {
        let r = raw(c);
        rblv.extend_from_slice(&call.to_le_bytes());
        rblv.extend_from_slice(&(r.len() as u32).to_le_bytes());
        rblv.extend_from_slice(&r);
    }
    let (mut p, mut a) = Player::new(tables(), Some(samples()));
    assert!(p.load(s, Some(samples())).is_ok());
    let mut out = vec![0i16; 2 * CALL_FRAMES * CALLS as usize];
    let mut it = events.into_iter().peekable();
    for (call, buf) in out.chunks_mut(2 * CALL_FRAMES).enumerate() {
        while let Some((_, c)) = it.next_if(|(at, _)| *at == call as u32) {
            send(&mut p, c);
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
    if let Ok(dir) = std::env::var("RB_LIVE_OUT") {
        let dir = std::path::Path::new(&dir);
        std::fs::write(dir.join(format!("{name}.rblv")), &rblv).unwrap();
        std::fs::write(dir.join(format!("{name}.wav")), &wav).unwrap();
        eprintln!("{name}: wav sha256 {sha}");
    }
    sha
}

#[test]
fn the_session_equals_rbhost() {
    let s = song(&manifest()["diag"]["full_mix"]);
    let sha = render("live", &s, session(patterns(&s)));
    assert_eq!(sha, RBHOST, "the player's session differs from rbhost's");
}

#[test]
fn pcf_wave_after_returning_to_pattern_mode() {
    let s = song(&manifest()["diag"]["full_mix"]);
    let sha = render("pcf", &s, pcf_session());
    assert_eq!(
        sha, RBHOST_PCF,
        "the player's session differs from rbhost's"
    );
}
