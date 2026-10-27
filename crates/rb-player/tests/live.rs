//! The player's live behavior (task-043, AC #1): transport, pattern and song mode, live
//! parameters sent from another thread, the pattern latch, the mute, the song switch, the
//! readback, the resampled output, and the mixer's constructor track (task-034).

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use common::{manifest, run, samples, send, song, tables};
use rb_engine::song::{Device, Song, TrackEvent};
use rb_player::output::Output;
use rb_player::{Audio, CALL_FRAMES, Command, Player};

const BAR: i32 = 768;

fn diag(name: &str) -> Song {
    song(&manifest()["diag"][name])
}

/// A player with `s` loaded and the first call made.
fn player(s: &Song) -> (Player, Audio) {
    let (mut p, mut a) = Player::new(tables(), Some(samples()));
    assert!(p.load(s, Some(samples())).is_ok());
    run(&mut a, 1);
    assert_eq!(p.snapshot().generation, 1);
    (p, a)
}

/// Engine calls until the snapshot's tick reaches `tick` (at most `max` calls).
fn run_to(p: &mut Player, a: &mut Audio, tick: i32, max: usize) {
    for _ in 0..max {
        if p.snapshot().tick >= tick {
            return;
        }
        run(a, 1);
    }
    panic!("tick {tick} not reached: {}", p.snapshot().tick);
}

fn peak(x: &[i16]) -> i32 {
    x.iter().map(|&s| (s as i32).abs()).max().unwrap_or(0)
}

#[test]
fn transport_play_stop_locate() {
    let (mut p, mut a) = player(&diag("303_slide_accent"));
    let s = *p.snapshot();
    assert!(!s.playing && s.song_mode && s.tick == 0 && s.frames == CALL_FRAMES as u64);
    send(&mut p, Command::Play);
    run(&mut a, 20);
    let s = *p.snapshot();
    assert!(s.playing && s.tick > 0);
    // 20 calls of 1024 frames: 20 * 16 blocks of tempo * TEMPO_STEP ticks
    let inc = s.tempo as f64 * rb_engine::seq::TEMPO_STEP;
    assert_eq!(s.tick, (20.0 * 16.0 * inc) as i32);
    send(&mut p, Command::Locate(5 * BAR as u32 + 100));
    run(&mut a, 1);
    assert!(p.snapshot().tick >= 5 * BAR + 100);
    // stop locates to the start of the bar
    send(&mut p, Command::Stop);
    run(&mut a, 2);
    let s = *p.snapshot();
    assert!(!s.playing);
    assert_eq!(s.tick, 5 * BAR);
    // the song plays: silence while stopped is not silence while playing
    send(&mut p, Command::Locate(0));
    send(&mut p, Command::Play);
    assert!(peak(&run(&mut a, 40)) > 1000);
}

#[test]
fn pattern_mode_and_song_mode() {
    let (mut p, mut a) = player(&diag("303_slide_accent"));
    send(&mut p, Command::Locate(3 * BAR as u32));
    send(&mut p, Command::SongMode(false));
    run(&mut a, 1);
    let s = *p.snapshot();
    assert!(!s.song_mode);
    send(&mut p, Command::Play);
    let out = run(&mut a, 40);
    assert!(p.snapshot().playing && peak(&out) > 1000);
    // back to song mode: the position is the song-mode tick saved when pattern mode was entered
    send(&mut p, Command::Stop);
    send(&mut p, Command::SongMode(true));
    run(&mut a, 1);
    let s = *p.snapshot();
    assert!(s.song_mode && !s.playing);
    assert_eq!(s.tick, 3 * BAR);
}

#[test]
fn a_pattern_change_waits_for_the_bar_line() {
    let (mut p, mut a) = player(&diag("303_slide_accent"));
    send(&mut p, Command::SongMode(false));
    send(&mut p, Command::Play);
    run_to(&mut p, &mut a, BAR / 4, 200);
    let before = p.snapshot().patterns[0].unwrap();
    let new = (before + 5) % 32;
    send(
        &mut p,
        Command::Param {
            device: Device::Tb303_1,
            ctrl: 1,
            value: new,
        },
    );
    run(&mut a, 1);
    // the controller is live at once, the playing pattern only from the next bar line
    assert_eq!(p.snapshot().live[1].as_slice()[1], new);
    assert_eq!(p.snapshot().patterns[0], Some(before));
    run_to(&mut p, &mut a, BAR - 30, 400);
    assert_eq!(p.snapshot().patterns[0], Some(before));
    run_to(&mut p, &mut a, BAR + 30, 400);
    assert_eq!(p.snapshot().patterns[0], Some(new));
}

#[test]
fn live_parameters_from_another_thread() {
    let (mut p, mut a) = player(&diag("full_mix"));
    send(&mut p, Command::Play);
    let done = Arc::new(AtomicBool::new(false));
    let audio = {
        let done = done.clone();
        std::thread::spawn(move || {
            let mut buf = vec![0i16; 2 * 256];
            while !done.load(Ordering::Relaxed) {
                a.render_i16(&mut buf);
            }
            a
        })
    };
    let wait = |p: &mut Player, f: &dyn Fn(&rb_player::Snapshot) -> bool| {
        for _ in 0..100_000 {
            if f(p.snapshot()) {
                return;
            }
            std::thread::yield_now();
        }
        panic!("the change never showed: {:?}", p.snapshot());
    };
    let cmds = [
        (Device::Tb303_1, 3u8, 17u8),
        (Device::Tb303_2, 4, 99),
        (Device::Tr808, 3, 5),
        (Device::Tr909, 3, 6),
        (Device::Delay, 3, 33),
        (Device::Distortion, 1, 44),
        (Device::Pcf, 2, 55),
        (Device::Compressor, 1, 66),
    ];
    for (device, ctrl, value) in cmds {
        send(
            &mut p,
            Command::Param {
                device,
                ctrl,
                value,
            },
        );
        wait(&mut p, &|s| {
            s.live[device as usize].as_slice()[ctrl as usize] == value
        });
    }
    send(
        &mut p,
        Command::Param {
            device: Device::Mixer,
            ctrl: 7,
            value: 3,
        },
    );
    send(&mut p, Command::Master(77));
    send(
        &mut p,
        Command::ChannelEnabled {
            channel: 1,
            on: false,
        },
    );
    send(&mut p, Command::Tempo(97_000));
    send(&mut p, Command::Shuffle(127));
    send(&mut p, Command::Vintage(true));
    send(&mut p, Command::LoopOn(true));
    send(&mut p, Command::LoopStart(BAR as u32));
    send(&mut p, Command::LoopEnd(4 * BAR as u32));
    wait(&mut p, &|s| {
        s.mixer.channels[0].pan == 3
            && s.mixer.master == 77
            && s.mixer.channels[1].enabled == 0
            && s.tempo == 97_000
            && s.shuffle == 127
            && s.vintage
            && s.loop_on
            && s.loop_start == BAR as u32
            && s.loop_end == 4 * BAR as u32
    });
    done.store(true, Ordering::Relaxed);
    let a = audio.join().unwrap();
    assert!(a.current().frame.vintage);
}

#[test]
fn the_mute_silences_its_channel() {
    // the 303 is the only source; the delay's echo of it fades with it
    let (mut p, mut a) = player(&diag("303_slide_accent"));
    send(&mut p, Command::Play);
    assert!(peak(&run(&mut a, 30)) > 1000);
    send(
        &mut p,
        Command::ChannelEnabled {
            channel: 0,
            on: false,
        },
    );
    run(&mut a, 60);
    assert!(peak(&run(&mut a, 20)) < 50);
    send(
        &mut p,
        Command::ChannelEnabled {
            channel: 0,
            on: true,
        },
    );
    run(&mut a, 10);
    assert!(peak(&run(&mut a, 20)) > 1000);
}

#[test]
fn a_song_loaded_while_playing_waits_for_play() {
    let first = diag("303_slide_accent");
    let mut second = diag("delay_automated");
    second.globals.tempo = 123_000;
    let (mut p, mut a) = player(&first);
    send(&mut p, Command::Play);
    run(&mut a, 10);
    // over a playing song: pending, not rendered; the commands go to it
    assert!(p.load(&second, Some(samples())).is_ok());
    send(&mut p, Command::Shuffle(5));
    run(&mut a, 5);
    let s = *p.snapshot();
    assert!(s.pending && s.playing && s.generation == 1);
    assert_eq!(s.tempo, first.globals.tempo);
    assert_eq!(s.shuffle, first.globals.shuffle);
    // play on it: the old song is halted and silenced, the new one starts
    send(&mut p, Command::Play);
    run(&mut a, 1);
    let s = *p.snapshot();
    assert!(!s.pending && s.playing && s.generation == 2);
    assert_eq!((s.tempo, s.shuffle), (123_000, 5));
    // over a stopped song: at once
    send(&mut p, Command::Stop);
    run(&mut a, 1);
    assert!(p.load(&first, Some(samples())).is_ok());
    run(&mut a, 1);
    let s = *p.snapshot();
    assert!(!s.pending && !s.playing && s.generation == 3);
    assert_eq!(s.tempo, first.globals.tempo);
    // every song shares the process globals: the new song's delay line is the player's
    assert!(a.current().process_state().same(p.process()));
}

#[test]
fn a_data_block_of_the_wrong_length_is_refused() {
    let (mut p, _a) = player(&diag("303_slide_accent"));
    let c = Command::DeviceData {
        device: Device::Tb303_1,
        payload: vec![0; 5].into_boxed_slice(),
    };
    assert!(p.send(c).is_err());
}

/// task-034: the mixer's constructor inserts a tick-0 event of every automatable controller with
/// its constructor value (`MixerDevice::init_track`), so a mixer controller that the song
/// automates only later returns to that value on a chase back to before its first event: a
/// locate while playing, a stop and locate, and the loop wrap. No export can hear it (an export is
/// one pass from one locate); the player can.
#[test]
fn a_mixer_controller_chased_back_returns_to_its_constructor_value() {
    let mut s = diag("303_slide_accent");
    // channel 0's pan (controller 7): the chunk says 20 (it goes to the pattern copy only, in
    // song mode), no tick-0 event, an event at bar 1 to 0
    s.mixer.channels[0].pan = 20;
    s.tracks[0].retain(|e| e.ctrl != 7);
    s.tracks[0].push(TrackEvent {
        tick: BAR as u32,
        ctrl: 7,
        value: 0,
    });
    s.globals.loop_on = 0;
    let pan = |p: &mut Player| p.snapshot().mixer.channels[0].pan;
    let (mut p, mut a) = player(&s);
    assert!(p.snapshot().song_mode);
    assert_eq!(pan(&mut p), 64);
    send(&mut p, Command::Play);
    run_to(&mut p, &mut a, BAR + 24, 400);
    assert_eq!(pan(&mut p), 0);
    // a locate back while playing
    send(&mut p, Command::Locate(0));
    run(&mut a, 1);
    assert_eq!(pan(&mut p), 64);
    run_to(&mut p, &mut a, BAR + 24, 400);
    assert_eq!(pan(&mut p), 0);
    // stop, then locate to the start
    send(&mut p, Command::Stop);
    run(&mut a, 1);
    assert_eq!(pan(&mut p), 0);
    send(&mut p, Command::Locate(0));
    run(&mut a, 1);
    assert_eq!(pan(&mut p), 64);
    // the loop wrap: bars 0..2 looped, the event at bar 1 undone at every wrap
    send(&mut p, Command::LoopStart(0));
    send(&mut p, Command::LoopEnd(2 * BAR as u32));
    send(&mut p, Command::LoopOn(true));
    send(&mut p, Command::Play);
    run_to(&mut p, &mut a, BAR + 24, 400);
    assert_eq!(pan(&mut p), 0);
    let mut wrapped = false;
    for _ in 0..400 {
        let t = p.snapshot().tick;
        run(&mut a, 1);
        if p.snapshot().tick < t {
            wrapped = true;
            break;
        }
    }
    assert!(wrapped && p.snapshot().tick < BAR);
    assert_eq!(pan(&mut p), 64);
}

/// A resampler error outputs silence for the rest of that device callback, is counted in the
/// snapshot, and the next callback plays on.
#[test]
fn a_resampler_error_gives_silence_and_is_counted() {
    let (mut p, a) = player(&diag("303_slide_accent"));
    send(&mut p, Command::Play);
    let mut out = Output::new(a, 2, 48_000).unwrap();
    let mut buf = vec![0f32; 2 * 4096];
    for _ in 0..10 {
        out.fill(&mut buf);
    }
    assert!(buf.iter().any(|&x| x.abs() > 0.01));
    assert_eq!(p.snapshot().output_errors, 0);
    assert!(out.inject_resampler_error());
    out.fill(&mut buf);
    assert!(
        buf.iter().all(|&x| x == 0.0),
        "the failed callback is not silent"
    );
    out.fill(&mut buf);
    assert!(
        buf.iter().any(|&x| x.abs() > 0.01),
        "playback did not resume"
    );
    for _ in 0..2 {
        out.fill(&mut buf);
    }
    assert_eq!(p.snapshot().output_errors, 1);
}

#[test]
fn the_resampled_output_follows_the_engine() {
    let s = diag("303_slide_accent");
    // the engine's own output at 44.1 kHz
    let (mut p, mut a) = player(&s);
    send(&mut p, Command::Play);
    let direct = run(&mut a, 43);
    let rms = |x: &mut dyn Iterator<Item = f64>| {
        let (mut sum, mut n) = (0.0, 0.0);
        for v in x {
            sum += v * v;
            n += 1.0;
        }
        (sum / n).sqrt()
    };
    let want = rms(&mut direct.iter().map(|&v| v as f64 / 32768.0));
    // the same through the resampler at 48 kHz
    let (mut p, a) = player(&s);
    send(&mut p, Command::Play);
    let mut out = Output::new(a, 2, 48_000).unwrap();
    let mut buf = vec![0f32; 2 * 48_000];
    out.fill(&mut buf);
    // one second at 48 kHz took one second of engine output (plus the first call and the
    // resampler's look-ahead)
    let frames = out.audio().current().frame.blocks * 64;
    assert!(
        (44_100..44_100 + 6 * CALL_FRAMES as u64).contains(&frames),
        "{frames}"
    );
    let got = rms(&mut buf.iter().map(|&v| v as f64));
    assert!((got - want).abs() < 0.05 * want, "rms {got} against {want}");
}
