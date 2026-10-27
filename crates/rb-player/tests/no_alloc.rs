//! AC #2 of task-043: the audio callback allocates and frees nothing.
//!
//! This test binary's global allocator counts the allocations, reallocations and frees made on
//! a thread while that thread is "armed"; the test arms itself only around the callback's work
//! ([`Audio::render_i16`] and [`Output::fill`]) and sends every kind of command from the
//! unarmed UI side in between: transport, song and pattern mode, loop, tempo, shuffle, Vintage,
//! device controllers, the mixer's master and mutes, a data block, the panel's device and
//! pattern messages (task-055, a whole pattern's box included), the mod change (task-056),
//! the preview player's startup sound, its replacement, stop and end (task-074), songs loaded over a stopped
//! song, over a playing one (pending) and switched to by Play, through the direct 44.1 kHz path
//! and the resampled one, with stereo, mono and six-channel devices. The callback's songs play
//! the drum machines, the 303s and every effect (the corpus song `full_mix`).
//!
//! Locks: the callback's code uses no `Mutex`, `RwLock` or `Condvar` (the engine's process
//! globals are atomics, `rb_engine::process`; the queues are `rtrb`'s and `triple_buffer`'s
//! lock-free ones), which `the_callback_crates_hold_no_lock` checks in the sources. That check is
//! textual and its scope is narrow: it greps `crates/rb-player/src` and `crates/rb-engine/src`
//! (skipping the unit-test files `tests.rs` and `test_cell.rs`) for the words `Mutex`, `RwLock`,
//! `Condvar`, `Barrier`, `Once` (also `OnceLock`), `mpsc` and `parking_lot`; it does not see a
//! lock under another name, a spin loop, or the dependencies' code (rtrb, triple_buffer and
//! rubato's `process_into_buffer` are lock-free by their documentation).

mod common;

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

use common::{kit, manifest, samples, send, song, tables};
use rb_engine::samples::Sample;
use rb_engine::song::Device;
use rb_player::output::Output;
use rb_player::{Audio, CALL_FRAMES, Command, Player, Preview};

struct Counting;

static EVENTS: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    static ARMED: Cell<bool> = const { Cell::new(false) };
}

fn note() {
    if ARMED.try_with(|a| a.get()).unwrap_or(false) {
        EVENTS.fetch_add(1, Ordering::Relaxed);
    }
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        note();
        unsafe { System.alloc(l) }
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        note();
        unsafe { System.alloc_zeroed(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        note();
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        note();
        unsafe { System.realloc(p, l, n) }
    }
}

#[global_allocator]
static A: Counting = Counting;

/// Runs `f` armed and returns the allocator events it caused.
fn armed(f: impl FnOnce()) -> usize {
    let before = EVENTS.load(Ordering::Relaxed);
    ARMED.with(|a| a.set(true));
    f();
    ARMED.with(|a| a.set(false));
    EVENTS.load(Ordering::Relaxed) - before
}

/// The counter works: an allocation made armed is counted.
#[test]
fn the_counter_counts() {
    let n = armed(|| drop(std::hint::black_box(vec![1u8; 10])));
    assert_eq!(n, 2);
}

/// `calls` engine calls through `render_i16`, armed; returns the events.
fn render(a: &mut Audio, buf: &mut [i16], calls: usize) -> usize {
    armed(|| {
        for _ in 0..calls {
            a.render_i16(&mut buf[..2 * CALL_FRAMES]);
        }
    })
}

#[test]
fn the_callback_does_not_allocate() {
    let m = manifest();
    let mix = song(&m["diag"]["full_mix"]);
    let other = song(&m["diag"]["delay_automated"]);
    let (mut p, mut a) = Player::new(tables(), Some(samples()));
    let mut buf = vec![0i16; 2 * CALL_FRAMES];
    let mut events = render(&mut a, &mut buf, 4);
    // a song over the stopped empty song: it becomes current at once
    assert!(p.load(&mix, Some(samples())).is_ok());
    send(&mut p, Command::Play);
    events += render(&mut a, &mut buf, 40);
    assert!(p.snapshot().playing && p.snapshot().generation == 1);
    let mixer = |ctrl, value| Command::Param {
        device: Device::Mixer,
        ctrl,
        value,
    };
    let cmds = vec![
        Command::Tempo(150_000),
        Command::Shuffle(100),
        Command::Vintage(true),
        Command::LoopOn(true),
        Command::LoopStart(768),
        Command::LoopEnd(3 * 768),
        Command::Param {
            device: Device::Tb303_1,
            ctrl: 3,
            value: 20,
        },
        Command::Param {
            device: Device::Tr808,
            ctrl: 1,
            value: 2,
        },
        Command::Param {
            device: Device::Delay,
            ctrl: 3,
            value: 100,
        },
        mixer(7, 0),
        mixer(1, 1),
        Command::Master(90),
        Command::ChannelEnabled {
            channel: 2,
            on: false,
        },
        Command::DeviceData {
            device: Device::Tb303_2,
            payload: other.device_payload(Device::Tb303_2).into_boxed_slice(),
        },
        // the panel's device messages (task-055)
        Command::DeviceParam {
            device: Device::Tb303_1,
            ctrl: 4,
            value: 30,
        },
        Command::DeviceParam {
            device: Device::Pcf,
            ctrl: 1,
            value: 50,
        },
        Command::MixerGlobal { id: 0, value: 80 },
        Command::MixerGlobal { id: 2, value: 1 },
        Command::MixerChannel {
            channel: 1,
            field: 2,
            value: 20,
        },
        Command::PatternField {
            device: Device::Tr909,
            pattern: 0,
            field: 1,
            value: 8,
        },
        Command::PatternHeader {
            device: Device::Tb303_2,
            pattern: 1,
            header: [1, 12],
        },
        Command::Pattern {
            device: Device::Tr808,
            pattern: 0,
            bytes: vec![1u8; 194].into_boxed_slice(),
        },
        Command::StepField {
            device: Device::Tb303_1,
            pattern: 0,
            step: 2,
            field: 0,
            value: 7,
        },
        Command::Step {
            device: Device::Tr909,
            pattern: 0,
            step: 3,
            bytes: [2; 12],
        },
        Command::Locate(2 * 768),
    ];
    for c in cmds {
        send(&mut p, c);
        events += render(&mut a, &mut buf, 3);
    }
    // recording (task-057) in song mode over the two-bar loop above: knobs, switches and the
    // mixer recorded while playing, through two loop wraps (each merges), then record off
    // (merges and chases)
    send(&mut p, Command::SongMode(true));
    send(&mut p, Command::Record(true));
    events += render(&mut a, &mut buf, 1);
    assert!(p.snapshot().recording);
    let before = a.current().frame.seq.events(1).len();
    for k in 0..300u32 {
        let v = (k % 128) as u8;
        send(
            &mut p,
            Command::DeviceParam {
                device: Device::Tb303_1,
                ctrl: 2 + (k % 6) as u8,
                value: v,
            },
        );
        send(
            &mut p,
            Command::DeviceParam {
                device: Device::Tr909,
                ctrl: (k % 30) as u8,
                value: v % 4,
            },
        );
        send(
            &mut p,
            Command::MixerChannel {
                channel: (k % 4) as u8,
                field: 1 + (k % 4) as u8,
                value: v,
            },
        );
        events += render(&mut a, &mut buf, 1);
    }
    send(&mut p, Command::Record(false));
    events += render(&mut a, &mut buf, 1);
    assert!(!p.snapshot().recording);
    let seq = &a.current().frame.seq;
    assert!(seq.events(1).len() > before, "nothing was recorded");
    assert!((0..9).all(|t| seq.recorded(t).is_empty()));
    // the callback copied the recorded tracks for the UI (`Player::tracks`)
    let copy = p.tracks();
    assert!(copy.merges > 0);
    for t in 0..9 {
        assert_eq!(&copy.events[t], seq.events(t));
    }
    // the Mods menu's change on the playing song (task-056), twice: the second lets go of a
    // kit only the song held (its table and the samples its voices played), which would be
    // freed if the callback dropped it
    assert!(p.set_mod(kit(|x| x / 2)).is_ok());
    events += render(&mut a, &mut buf, 20);
    assert!(p.set_mod(kit(|x| x / 3)).is_ok());
    events += render(&mut a, &mut buf, 1);
    // the first kit is freed here, on the UI side; the voices that played it must not hold it
    // when the next renders retrigger them
    assert_eq!(p.collect_garbage(), 1);
    events += render(&mut a, &mut buf, 40);
    // the preview player (task-074): the startup sound over the playing song, then another one
    // replacing it (its box goes back), msg 57's stop, and one played to its end
    let short = Sample::from_audio(&[1000; 3000]);
    assert!(p.startup_sound(&samples()).is_ok());
    events += render(&mut a, &mut buf, 2);
    assert!(a.preview_position().is_some());
    assert!(p.startup_sound(&kit(|x| x / 2)).is_ok());
    events += render(&mut a, &mut buf, 2);
    assert_eq!(p.collect_garbage(), 1);
    send(&mut p, Command::PreviewStop);
    events += render(&mut a, &mut buf, 1);
    assert!(a.preview_position().is_none());
    send(
        &mut p,
        Command::Preview(Box::new(Preview::new(std::sync::Arc::new(short)))),
    );
    events += render(&mut a, &mut buf, 4);
    assert!(a.preview_position().is_none());
    // a song over the playing one: pending, then Play switches
    assert!(p.load(&other, Some(samples())).is_ok());
    events += render(&mut a, &mut buf, 3);
    assert!(p.snapshot().pending);
    send(&mut p, Command::Play);
    events += render(&mut a, &mut buf, 20);
    assert!(!p.snapshot().pending && p.snapshot().generation == 2);
    // pattern mode and back, stop, a song over the stopped one
    send(&mut p, Command::SongMode(false));
    events += render(&mut a, &mut buf, 20);
    send(&mut p, Command::SongMode(true));
    send(&mut p, Command::Stop);
    events += render(&mut a, &mut buf, 3);
    assert!(p.load(&mix, Some(samples())).is_ok());
    send(&mut p, Command::Play);
    events += render(&mut a, &mut buf, 20);
    assert_eq!(p.snapshot().generation, 3);
    assert_eq!(events, 0, "the callback allocated or freed {events} times");
    // what it handed back (the last replaced song) is dropped here; the boxes of the data block
    // and the pattern went back while the commands were sent
    assert_eq!(p.collect_garbage(), 1);

    // the device paths: resampled to 48 kHz and direct, odd sizes, other formats and layouts
    let mut out = Output::new(a, 2, 48_000).unwrap();
    let mut f = vec![0f32; 2 * 3000];
    let n = armed(|| {
        for k in [1, 512, 3000, 7, 2048, 1000] {
            out.fill(&mut f[..2 * k]);
        }
    });
    assert_eq!(n, 0, "the resampled callback allocated or freed {n} times");
    // the resampler's error path
    assert!(out.inject_resampler_error());
    let n = armed(|| out.fill(&mut f[..2 * 512]));
    assert_eq!(
        n, 0,
        "the failing resampled callback allocated or freed {n} times"
    );
    let (mut p2, a2) = Player::new(tables(), Some(samples()));
    assert!(p2.load(&mix, Some(samples())).is_ok());
    send(&mut p2, Command::Play);
    let mut six = Output::new(a2, 6, 44_100).unwrap();
    let mut s = vec![0i16; 6 * 5000];
    let n = armed(|| {
        for k in [5000, 1, 4096, 333] {
            six.fill(&mut s[..6 * k]);
        }
    });
    assert_eq!(
        n, 0,
        "the six-channel callback allocated or freed {n} times"
    );
    let (_p3, a3) = Player::new(tables(), Some(samples()));
    let mut mono = Output::new(a3, 1, 22_050).unwrap();
    let mut u = vec![0u16; 4000];
    let n = armed(|| mono.fill(&mut u[..]));
    assert_eq!(n, 0, "the mono callback allocated or freed {n} times");
}

/// No lock type in the code the callback runs: the player's sources and the engine's.
#[test]
fn the_callback_crates_hold_no_lock() {
    let root = common::root();
    let mut bad = Vec::new();
    for dir in ["crates/rb-player/src", "crates/rb-engine/src"] {
        let mut stack = vec![root.join(dir)];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(&d).unwrap() {
                let p = e.unwrap().path();
                if p.is_dir() {
                    stack.push(p);
                    continue;
                }
                let name = p.file_name().unwrap().to_string_lossy();
                // the unit tests' shared logs are a Mutex, outside the render path
                if name == "tests.rs" || name == "test_cell.rs" {
                    continue;
                }
                let text = std::fs::read_to_string(&p).unwrap();
                for w in [
                    "Mutex",
                    "RwLock",
                    "Condvar",
                    "Barrier",
                    "Once",
                    "mpsc",
                    "parking_lot",
                ] {
                    if text.contains(w) {
                        bad.push(format!("{}: {w}", p.display()));
                    }
                }
            }
        }
    }
    assert!(bad.is_empty(), "{bad:?}");
}
