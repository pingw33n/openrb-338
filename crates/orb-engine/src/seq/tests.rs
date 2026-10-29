//! The sequencer's unit tests. The expected block numbers are derived by hand from the position
//! step `inc = tempo * 64*192 / (60*44100*1000)`: a tick `n` is reached in the first block `k`
//! (1-based, counted from play) with `start + k*inc >= n`, i.e. `k = ceil((n - start) / inc)`.
//! Every case is chosen with the fraction far from an integer, so the f64 accumulation cannot
//! move it.

use super::*;
use crate::frame::{
    Block, BlockContext, ChannelDevice, ChannelParams, GUARD, InsertEffect, MixerDevice,
    MixerParams, SendEffect, SongDevice,
};
use crate::test_cell::{Rc, RefCell};

type Log = Rc<RefCell<Vec<String>>>;

/// A device that logs every sequencer-facing call.
struct Probe {
    name: &'static str,
    log: Log,
}

impl Probe {
    fn push(&self, s: String) {
        self.log.borrow_mut().push(format!("{} {s}", self.name));
    }
}

impl SongDevice for Probe {
    fn step(&mut self, tick: i32, shuffled: i32, tempo: i32) {
        self.push(format!("step {tick} {shuffled} {tempo}"));
    }
    fn relocate(&mut self, tick: i32, _now: i32) {
        self.push(format!("relocate {tick}"));
    }
    fn start(&mut self, _now: i32) {
        self.push("start".into());
    }
    fn stop(&mut self) {
        self.push("stop".into());
    }
    fn set_song_mode(&mut self, on: bool) {
        self.push(format!("mode {on}"));
    }
    fn set_param(&mut self, ctrl: u8, value: u8) {
        self.push(format!("p{ctrl}={value}"));
    }
}
impl ChannelDevice for Probe {
    fn process(&mut self, bus: &mut Block, _ctx: &BlockContext) {
        bus.fill(GUARD);
    }
}
impl InsertEffect for Probe {
    fn process(&mut self, _bus: &mut Block, _ctx: &BlockContext) {}
}
impl SendEffect for Probe {
    fn process(&mut self, send: &mut Block, _l: &mut Block, _r: &mut Block, _ctx: &BlockContext) {
        send.fill(GUARD);
    }
}

fn probes(log: &Log) -> Devices {
    let p = |name| Probe {
        name,
        log: log.clone(),
    };
    Devices {
        mixer: MixerDevice::default(),
        channels: [
            Box::new(p("303a")),
            Box::new(p("303b")),
            Box::new(p("808")),
            Box::new(p("909")),
        ],
        dist: Box::new(p("dist")),
        pcf: Box::new(p("pcf")),
        comp: Box::new(p("comp")),
        delay: Box::new(p("delay")),
        process: Default::default(),
    }
}

struct Rig {
    seq: Sequencer,
    dev: Devices,
    log: Log,
}

impl Rig {
    fn new(tempo: u32) -> Rig {
        let log = Log::default();
        let mut seq = Sequencer::default();
        seq.set_tempo(tempo);
        seq.set_shuffle(0);
        Rig {
            seq,
            dev: probes(&log),
            log,
        }
    }

    fn song_mode(tempo: u32) -> Rig {
        let mut r = Rig::new(tempo);
        r.seq.set_song_mode(1, &mut r.dev);
        r.take();
        r
    }

    fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.log.borrow_mut())
    }

    /// Runs `n` blocks and returns every logged call with its 1-based block number.
    fn run(&mut self, n: u32) -> Vec<(u32, String)> {
        let mut out = Vec::new();
        for k in 1..=n {
            self.seq.tick(&mut self.dev);
            out.extend(self.take().into_iter().map(|s| (k, s)));
        }
        out
    }

    fn add(&mut self, track: usize, tick: u32, ctrl: u8, value: u8) {
        assert!(
            self.seq
                .add_events(track, &[song::TrackEvent { tick, ctrl, value }])
        );
    }

    fn locate(&mut self, p: i32) {
        self.seq.locate(p, &mut self.dev);
    }

    fn play(&mut self) {
        self.seq.play(&mut self.dev);
    }
}

/// The 303 #1's step calls as `(block, tick)`.
fn steps(log: &[(u32, String)]) -> Vec<(u32, i32)> {
    log.iter()
        .filter_map(|(k, s)| {
            let s = s.strip_prefix("303a step ")?;
            Some((*k, s.split(' ').next()?.parse().ok()?))
        })
        .collect()
}

fn only(log: &[(u32, String)], k: u32) -> Vec<&str> {
    log.iter()
        .filter(|(b, _)| *b == k)
        .map(|(_, s)| s.as_str())
        .collect()
}

const ALL: [&str; 8] = ["303a", "303b", "808", "909", "delay", "dist", "pcf", "comp"];

fn each(what: &str) -> Vec<String> {
    ALL.iter().map(|d| format!("{d} {what}")).collect()
}

#[test]
fn constants_decode() {
    assert_eq!(TEMPO_STEP, 4.6439909297052154e-06);
    assert!((TEMPO_STEP - 64.0 * 192.0 / (60.0 * 44100.0 * 1000.0)).abs() < 1e-21);
    assert_eq!(SONG_END, 767_231.0);
    assert_eq!(trunc(1.999), 1);
    assert_eq!(trunc(-0.5), 0);
}

/// AC #2: the block of each step at four tempos. A tick n fires in block ceil(n / inc).
#[test]
fn step_blocks_at_four_tempos() {
    // (tempo, inc, the blocks of ticks 1, 48, 96 and 768)
    let cases = [
        (60_000, 0.2786394557823129, [4, 173, 345, 2757]),
        (120_000, 0.5572789115646258, [2, 87, 173, 1379]),
        (140_000, 0.6501587301587302, [2, 74, 148, 1182]),
        (300_000, 1.3931972789115645, [1, 35, 69, 552]),
    ];
    for (tempo, inc, blocks) in cases {
        let mut r = Rig::new(tempo);
        assert_eq!(tempo as f64 * TEMPO_STEP, inc);
        r.locate(0);
        r.play();
        assert_eq!(
            r.take(),
            each("relocate 0")
                .into_iter()
                .chain(each("start"))
                .collect::<Vec<_>>()
        );
        let log = r.run(blocks[3]);
        let s = steps(&log);
        for (n, k) in [1, 48, 96, 768].into_iter().zip(blocks) {
            // the first step at or past tick n is in block k
            let first = s.iter().find(|&&(_, t)| t >= n).unwrap();
            assert_eq!(first.0, k, "tempo {tempo} tick {n}");
            // and the block before reached only n - 1 or less
            assert!(s.iter().all(|&(b, t)| b >= k || t < n));
        }
        // one step per block whose tick moved: below 1 tick per block every tick appears once
        let ticks: Vec<i32> = s.iter().map(|&(_, t)| t).collect();
        if inc < 1.0 {
            assert_eq!(ticks, (1..=ticks.len() as i32).collect::<Vec<_>>());
        } else {
            // above it every block steps, and ticks are skipped (300 BPM: 1, 2, 4, 5, 6, 8, ...)
            assert_eq!(s.len(), blocks[3] as usize);
            assert_eq!(&ticks[..6], &[1, 2, 4, 5, 6, 8]);
        }
        // every device steps, in song order, with the song's tempo
        let k = blocks[1];
        let t = s.iter().find(|&&(b, _)| b == k).unwrap().1;
        assert_eq!(only(&log, k), each(&format!("step {t} {t} {tempo}")));
    }
}

/// AC #2: the block of each event at three tempos: an event fires in the first block whose tick
/// reaches it, before that block's steps.
#[test]
fn event_blocks_at_three_tempos() {
    for (tempo, b48, b768) in [(60_000, 173, 2757), (120_000, 87, 1379), (300_000, 35, 552)] {
        let mut r = Rig::song_mode(tempo);
        r.add(1, 0, 2, 10);
        r.add(1, 48, 2, 11);
        r.add(1, 768, 2, 12);
        r.locate(0);
        // the locate chases tick 0
        assert_eq!(r.take()[0], "303a p2=10");
        r.play();
        r.take();
        let log = r.run(b768);
        let ev: Vec<(u32, &str)> = log
            .iter()
            .filter(|(_, s)| s.starts_with("303a p"))
            .map(|(k, s)| (*k, s.as_str()))
            .collect();
        assert_eq!(
            ev,
            [(b48, "303a p2=11"), (b768, "303a p2=12")],
            "tempo {tempo}"
        );
        let blk = only(&log, b48);
        assert_eq!(blk[0], "303a p2=11");
        assert!(blk[1].starts_with("303a step 48 "));
    }
}

#[test]
fn shuffle_offsets() {
    // shuffle 127: 127/6 = 21 ticks
    let cases = [
        (0, 0),
        (20, 0),
        (21, 0),
        (22, 1),
        (47, 26),
        (48, 27),
        (60, 39),
        (95, 74),
        (96, 96),
        (117, 96),
        (144, 123),
        (768 + 48, 768 + 27),
    ];
    for (t, want) in cases {
        assert_eq!(shuffled(t, 127), want, "t {t}");
    }
    // the second 16th of an 8th (tick 48 + 96k) is reached only `shuffle/6` ticks later
    assert_eq!(shuffled(48 + 21, 127), 48);
    for s in [0, 5] {
        assert!((0..400).all(|t| shuffled(t, s) == t));
    }
    assert_eq!(shuffled(48, 64), 38);
    assert_eq!(shuffled(5, 64), 0);
    // the sequencer passes it with every step
    let mut r = Rig::new(300_000);
    r.seq.set_shuffle(127);
    r.locate(0);
    r.play();
    r.take();
    let log = r.run(35);
    assert_eq!(only(&log, 35)[0], "303a step 48 27 300000");
}

#[test]
fn events_before_steps_in_track_and_list_order() {
    let mut r = Rig::song_mode(120_000);
    // tick 48: the 808 first, then two 303 controllers (they end up in reverse arrival order),
    // then the mixer's channel 1 level
    r.add(3, 48, 2, 1);
    r.add(1, 48, 3, 2);
    r.add(1, 48, 5, 3);
    r.add(0, 48, Mixer::channel_ctrl(1, 0), 99);
    r.locate(0);
    r.play();
    r.take();
    let log = r.run(87);
    let mut want = vec![
        "303a p5=3".to_string(),
        "303a p3=2".into(),
        "808 p2=1".into(),
    ];
    want.extend(each("step 48 48 120000"));
    assert_eq!(only(&log, 87), want);
    assert_eq!(r.dev.mixer.live.channels[1].level, 99);
    assert!(only(&log, 86).iter().all(|s| !s.contains(" p")));
}

use crate::song::Mixer;

#[test]
fn insert_rules() {
    let mut t = Solo::new(Device::Tb303_1);
    assert!(t.insert(48, 2, 1));
    assert!(t.insert(0, 2, 0));
    assert!(t.insert(48, 3, 5));
    // same tick and controller: the value is replaced, no node added
    assert!(t.insert(48, 2, 7));
    // quantised up to a multiple of 24
    assert!(t.insert(1, 4, 9));
    assert!(t.insert(25, 4, 8));
    let ev: Vec<_> = t
        .events()
        .iter()
        .map(|e| (e.tick, e.ctrl, e.value))
        .collect();
    assert_eq!(
        ev,
        [(0, 2, 0), (24, 4, 9), (48, 4, 8), (48, 3, 5), (48, 2, 7)]
    );
    // above 767231 after quantising: dropped
    assert!(t.insert(767_208, 5, 1));
    assert!(!t.insert(767_231, 5, 1));
    assert!(!t.insert(767_232, 5, 1));
    assert_eq!(t.events().len(), 6);
    // an insert before the applied node moves the pointer with it
    t.chase(24);
    assert_eq!(t.next(), 2);
    t.insert(0, 6, 1);
    assert_eq!(t.next(), 3);
    // a new controller at the applied node's tick goes before it, so it is not applied
    t.insert(24, 7, 1);
    assert_eq!(t.next(), 4);
    assert_eq!(
        t.events()[2],
        Event {
            tick: 24,
            ctrl: 7,
            value: 1
        }
    );
    // one right after the applied node is the next to apply
    t.insert(48, 9, 1);
    assert_eq!(t.next(), 4);
    assert_eq!(
        t.events()[4],
        Event {
            tick: 48,
            ctrl: 9,
            value: 1
        }
    );
}

#[test]
fn prune_drops_repeated_values() {
    let mut t = Solo::new(Device::Tb303_1);
    for (tick, v) in [(0, 5), (48, 5), (96, 6), (144, 6), (192, 5)] {
        t.insert(tick, 2, v);
    }
    t.insert(48, 3, 1);
    t.insert(96, 3, 1);
    t.prune();
    let ev: Vec<_> = t
        .events()
        .iter()
        .map(|e| (e.tick, e.ctrl, e.value))
        .collect();
    assert_eq!(ev, [(0, 2, 5), (48, 3, 1), (96, 2, 6), (192, 2, 5)]);
}

#[test]
fn chase_and_advance() {
    let mut t = Solo::new(Device::Tb303_1);
    for (tick, c, v) in [(0, 2, 1), (0, 3, 1), (96, 2, 2), (192, 3, 3), (288, 2, 4)] {
        t.insert(tick, c, v);
    }
    let s = t.chase(200);
    assert_eq!((s[2], s[3], s[4]), (2, 3, 0xFF));
    assert_eq!(t.next(), 4);
    let mut got = Vec::new();
    t.advance(287, |e| got.push(e.value));
    assert!(got.is_empty());
    // a held controller is skipped by the clock
    t.hold[2] = true;
    t.advance(288, |e| got.push(e.value));
    assert!(got.is_empty());
    assert_eq!(t.next(), 5);
}

/// The loop wrap at 300 BPM over bars 1..2 (768..1536), and the chase at the loop start.
#[test]
fn loop_wrap_chases_the_loop_start() {
    let mut r = Rig::song_mode(300_000);
    r.seq.set_loop_on(1);
    r.seq.set_loop_start(768);
    r.seq.set_loop_end(1536);
    for (tick, v) in [(0, 1), (768, 2), (1152, 3), (1536, 4)] {
        r.add(1, tick, 2, v);
    }
    r.add(1, 1512, 3, 7);
    r.locate(768);
    let mut want = vec!["303a p2=2".to_string()];
    want.extend(each("relocate 768"));
    assert_eq!(r.take(), want);
    r.play();
    r.take();
    // locate sets lastTick = 768, so block 1 (pos 769.39) steps at 769
    let log = r.run(827);
    assert_eq!(steps(&log)[0], (1, 769));
    // tick 1512: (1512 - 768) / inc = 534.02, block 535 (pos 1513.36)
    assert_eq!(log.iter().find(|(_, s)| s == "303a p3=7").unwrap().0, 535);
    // block 552: pos 1537.04 crosses 1536 (pos - inc = 1535.65); the wrap moves it to 769.04,
    // chases to 768 (controller 2 only: no controller-3 event at or before 768), relocates,
    // and steps at 769. The event at the loop end itself never fires.
    let mut want = vec!["303a p2=2".to_string()];
    want.extend(each("relocate 768"));
    want.extend(each("step 769 769 300000"));
    assert_eq!(only(&log, 552), want);
    assert!(!log.iter().any(|(_, s)| s == "303a p2=4"));
    assert_eq!(steps(&log)[550..553], [(551, 1535), (552, 769), (553, 770)]);
    // tick 1152 again after the wrap: 768 + 384 / inc, block 552 + 276 - 1 = 827 (pos 1152.17)
    assert_eq!(only(&log, 827)[0], "303a p2=3");
}

#[test]
fn no_wrap_events_or_end_in_pattern_mode() {
    let mut r = Rig::new(300_000);
    r.seq.set_loop_on(1);
    r.seq.set_loop_end(96);
    r.add(1, 48, 2, 1);
    r.locate(0);
    r.play();
    r.take();
    let log = r.run(100);
    assert!(
        log.iter()
            .all(|(_, s)| !s.contains(" p") && !s.contains("relocate"))
    );
    assert_eq!(steps(&log).last(), Some(&(100, 139)));
}

#[test]
fn song_end() {
    let mut r = Rig::song_mode(300_000);
    r.locate(767_200);
    r.play();
    r.take();
    let log = r.run(23);
    // block 22: pos 767230.65 steps; block 23: pos 767232.04 > 767231 ends the song
    assert_eq!(steps(&log).last(), Some(&(22, 767_230)));
    let mut want = each("stop");
    want.extend(each("relocate 0"));
    assert_eq!(only(&log, 23), want);
    assert_eq!(
        (r.seq.globals.play, r.seq.pos, r.seq.last_tick),
        (0, 0.0, 0)
    );
    // stopped: nothing more happens
    assert!(r.run(10).is_empty());
}

#[test]
fn locate_play_stop() {
    let mut r = Rig::song_mode(120_000);
    r.add(1, 0, 2, 1);
    r.add(1, 960, 2, 2);
    // a negative tick is 0
    r.locate(-5);
    assert_eq!(r.seq.pos, 0.0);
    r.take();
    r.locate(768);
    assert_eq!((r.seq.pos, r.seq.last_tick), (768.0, 768));
    r.take();
    // stopped: no tick moves
    assert!(r.run(5).is_empty());
    assert_eq!(r.seq.pos, 768.0);
    r.play();
    assert_eq!(r.take(), each("start"));
    // a second play does nothing
    r.play();
    assert!(r.take().is_empty());
    // block 1 reaches 768.56 (no step), block 2 769.11
    let log = r.run(400);
    assert_eq!(steps(&log)[0], (2, 769));
    // (960 - 768) / inc = 344.53: the event in block 345
    assert_eq!(log.iter().find(|(_, s)| s == "303a p2=2").unwrap().0, 345);
    let t = r.seq.tick_pos();
    assert_eq!(t, 990);
    // stop: every device stops, the tracks are chased to 990, then the locate to the bar start
    // (768) chases again and relocates
    r.seq.stop(&mut r.dev);
    let mut want = each("stop");
    want.push("303a p2=2".into());
    want.push("303a p2=1".into());
    want.extend(each("relocate 768"));
    assert_eq!(r.take(), want);
    assert_eq!(
        (r.seq.globals.play, r.seq.pos, r.seq.last_tick),
        (0, 768.0, 768)
    );
    // a stop while stopped stops no device
    r.seq.stop(&mut r.dev);
    assert!(!r.take().iter().any(|s| s.ends_with(" stop")));
}

#[test]
fn locate_in_pattern_mode_does_not_chase() {
    let mut r = Rig::new(120_000);
    r.add(1, 0, 2, 1);
    r.locate(0);
    assert_eq!(r.take(), each("relocate 0"));
}

#[test]
fn song_mode_switch() {
    let mut r = Rig::new(120_000);
    r.add(1, 0, 2, 1);
    r.seq.pos = 1000.5;
    // entering song mode: every device is told, then the locate to the saved tick (0)
    r.seq.set_song_mode(1, &mut r.dev);
    let mut want = each("mode true");
    want.push("303a p2=1".into());
    want.extend(each("relocate 0"));
    assert_eq!(r.take(), want);
    // the same mode again: only the locate
    r.seq.set_song_mode(1, &mut r.dev);
    assert!(!r.take().iter().any(|s| s.contains("mode")));
    // leaving it: every device is told, relocated to 0, and the current tick saved
    r.seq.pos = 1234.9;
    r.seq.set_song_mode(0, &mut r.dev);
    let mut want = each("mode false");
    want.extend(each("relocate 0"));
    assert_eq!(r.take(), want);
    assert_eq!((r.seq.saved_tick, r.seq.pos), (1234, 1234.9));
    r.seq.set_song_mode(1, &mut r.dev);
    assert_eq!(r.seq.pos, 1234.0);
}

#[test]
fn mixer_device_rules() {
    let mut m = MixerDevice::default();
    m.set_param(1, 3);
    m.set_param(2, 4);
    m.set_param(Mixer::channel_ctrl(2, 0), 11);
    m.set_param(Mixer::channel_ctrl(2, 1), 12);
    m.set_param(Mixer::channel_ctrl(2, 2), 13);
    m.set_param(Mixer::channel_ctrl(2, 3), 1);
    // field 0 (enabled, controller 17 for channel 2) and controllers 0, 3, 4, 5 set nothing
    for c in [0, 3, 4, 5, 17] {
        m.set_param(c, 0);
    }
    assert_eq!((m.live.comp_id, m.live.pcf_id), (3, 4));
    assert_eq!(
        m.live.channels[2],
        ChannelParams {
            enabled: 1,
            level: 11,
            pan: 12,
            send: 13,
            dist: 1,
        }
    );
    assert_eq!(m.live.channels[0], ChannelParams::default());

    // data in song mode: all into the pattern copy, only master and enables live
    let mut b = [0u8; 29];
    b[0] = 50;
    b[1] = 2;
    for c in 0..4 {
        b[5 + 6 * c..10 + 6 * c].copy_from_slice(&[c as u8 & 1, 20, 30, 40, 1]);
    }
    let mut m = MixerDevice::default();
    m.data(&b, true);
    assert_eq!(m.live.master, 50);
    assert_eq!(m.live.comp_id, 0);
    assert_eq!(m.live.channels.map(|c| c.enabled), [0, 1, 0, 1]);
    assert_eq!(m.live.channels[1].level, 100);
    assert_eq!(m.pattern.channels[1].level, 20);
    // leaving song mode restores the pattern copy, keeping the live master and enables
    m.live.master = 77;
    m.live.channels[0].enabled = 1;
    m.live.channels[0].level = 5;
    m.set_song_mode(false);
    assert_eq!(m.live.master, 77);
    assert_eq!(m.live.channels[0].enabled, 1);
    assert_eq!(m.live.channels[0].level, 20);
    assert_eq!(m.live.comp_id, 2);
    // entering it saves the live set
    m.live.channels[3].pan = 1;
    m.set_song_mode(true);
    assert_eq!(m.pattern, m.live);
    // data in pattern mode goes live whole
    let mut m = MixerDevice::default();
    m.data(&b, false);
    assert_eq!(m.live, m.pattern);
    assert_eq!(m.live.channels[2].send, 40);
    assert_ne!(m.live, MixerParams::default());
}

// ---------------------------------------------------------------- recording (task-057)

fn triples(e: &[Event]) -> Vec<(i32, u8, u8)> {
    e.iter().map(|e| (e.tick, e.ctrl, e.value)).collect()
}

/// The record hook `sub_10021340`: only in song mode with record on; the tick rounded down
/// when stopped and up when playing, to the controller's quantisation of the first list; the
/// loop end becomes the loop start; the hold byte is set.
#[test]
fn record_hook_rounds_and_holds() {
    use Device as D;
    let mut r = Rig::new(120_000);
    r.seq.set_record(1, &mut r.dev);
    r.seq.record_hook(D::Tb303_1, 3, 50);
    assert!(r.seq.recorded(1).is_empty(), "pattern mode");
    let mut r = Rig::song_mode(120_000);
    r.seq.tracks[1].set_quant(1, 32);
    r.locate(100);
    r.seq.record_hook(D::Tb303_1, 3, 50);
    assert!(r.seq.recorded(1).is_empty(), "record off");
    r.seq.set_record(1, &mut r.dev);
    // stopped: down (100 → 96; controller 1's unit is a bar: 0)
    r.seq.record_hook(D::Tb303_1, 3, 50);
    r.seq.record_hook(D::Tb303_1, 1, 2);
    assert_eq!(triples(r.seq.recorded(1)), [(0, 1, 2), (96, 3, 50)]);
    let hold = r.seq.tracks[1].hold;
    assert!(hold[1] && hold[3] && !hold[2]);
    // playing: up (100 → 120, 768); the same tick and controller takes the value
    r.play();
    r.seq.record_hook(D::Tb303_1, 3, 51);
    r.seq.record_hook(D::Tb303_1, 3, 52);
    r.seq.record_hook(D::Tb303_1, 1, 3);
    // the loop on: the loop end is the loop start
    r.seq.set_loop_on(1);
    r.seq.set_loop_start(0);
    r.seq.set_loop_end(768);
    r.seq.record_hook(D::Tb303_1, 1, 4);
    assert_eq!(
        triples(r.seq.recorded(1)),
        [(0, 1, 4), (96, 3, 50), (120, 3, 52), (768, 1, 3)]
    );
    // each device records into its own track; nothing was applied or merged
    r.seq.record_hook(D::Mixer, 7, 9);
    assert_eq!(triples(r.seq.recorded(0)), [(120, 7, 9)]);
    assert!(r.seq.events(1).is_empty());
    r.take();
    // a controller above 127 is not recorded
    r.seq.record_hook(D::Delay, 200, 1);
    assert!(r.seq.recorded(D::Delay as usize).is_empty());
}

/// The merge `sub_1001F520` over a track with automation on controllers 2 and 3.
#[test]
fn merge_replaces_the_recorded_span() {
    let mut t = Solo::new(Device::Tb303_1);
    for (tick, c, v) in [
        (0, 2, 10),
        (0, 3, 20),
        (96, 2, 11),
        (192, 2, 12),
        (288, 2, 13),
        (192, 3, 21),
    ] {
        t.insert(tick, c, v);
    }
    let free = t.free();
    // controller 2 at 144 and 240, controller 4 (no automation) at 48, one above the merge
    t.record(144, false, None, 2, 50);
    t.record(240, false, None, 2, 51);
    t.record(48, false, None, 4, 7);
    t.record(300, true, None, 2, 60);
    assert_eq!(t.free(), free - 4);
    t.chase(250);
    t.merge(250);
    // controller 2: the node at 192 goes (a recorded node at or before it), 96 stays (before
    // the first recorded one), 288 is after the merge; it returns to 12, its value at 250, at
    // the next 24-tick unit 264. Controller 4 returns to 0xFF there (no node up to 250). The
    // recorded node at 312 is lost (OB-18).
    assert_eq!(
        triples(t.events()),
        [
            (0, 3, 20),
            (0, 2, 10),
            (48, 4, 7),
            (96, 2, 11),
            (144, 2, 50),
            (192, 3, 21),
            (240, 2, 51),
            (264, 4, 0xFF),
            (264, 2, 12),
            (288, 2, 13)
        ]
    );
    assert!(t.recorded().is_empty());
    assert_eq!(t.hold, [false; 128]);
    // 4 recorded, 1 removed, 2 returns, 3 copies, 4 freed
    assert_eq!(t.free(), free - 4 + 1 - 2 - 3 + 4);
    // the applied node (192, 2) was removed: orb moves to the one before it (OB-19)
    assert_eq!(t.next(), 6);
}

/// A tick-0 node is never removed, and a return that finds a node of its controller at its
/// tick leaves it; the unit is the controller's (a bar for the 303's pattern).
#[test]
fn merge_keeps_tick_zero_and_existing_returns() {
    let mut t = Solo::new(Device::Tb303_1);
    t.set_quant(1, 32);
    t.insert(0, 1, 5);
    t.insert(768, 1, 6);
    t.insert(48, 2, 1);
    t.insert(72, 2, 3);
    t.record(0, false, None, 1, 9);
    t.record(30, false, None, 2, 2);
    t.merge(50);
    // controller 1: tick 0 kept and given the recorded value, 768 (the return's tick) kept as
    // it is; controller 2: 48 removed, its return at 72 finds a node there, which keeps 3
    assert_eq!(
        triples(t.events()),
        [(0, 1, 9), (24, 2, 2), (72, 2, 3), (768, 1, 6)]
    );
}

/// Recording while playing: the held controller ignores its automation; record off merges
/// and chases.
#[test]
fn recording_while_playing_and_record_off() {
    let mut r = Rig::song_mode(300_000);
    for (tick, v) in [(0, 1), (384, 2), (768, 3)] {
        r.add(1, tick, 2, v);
    }
    r.locate(0);
    r.seq.set_record(1, &mut r.dev);
    r.play();
    r.take();
    r.run(100);
    // block 100: tick 139, up to 144
    r.seq.record_hook(Device::Tb303_1, 2, 50);
    let log = r.run(300);
    assert!(!log.iter().any(|(_, s)| s.starts_with("303a p")), "held");
    assert_eq!(r.seq.tick_pos(), 557);
    r.seq.set_record(0, &mut r.dev);
    // merged at 557 and chased there: 50
    assert_eq!(r.take(), ["303a p2=50"]);
    assert_eq!(
        triples(r.seq.events(1)),
        [(0, 2, 1), (144, 2, 50), (576, 2, 2), (768, 2, 3)]
    );
    let log = r.run(200);
    let p: Vec<_> = log
        .iter()
        .filter(|(_, s)| s.starts_with("303a p"))
        .map(|(_, s)| s.as_str())
        .collect();
    assert_eq!(p, ["303a p2=2", "303a p2=3"]);
    // record off while off does nothing
    r.seq.set_record(0, &mut r.dev);
    assert!(r.take().is_empty());
}

/// The loop wrap merges the recording up to loopEnd - 1 (0x1001F0C7): the next pass plays it.
#[test]
fn the_loop_wrap_merges() {
    let mut r = Rig::song_mode(300_000);
    r.seq.set_loop_on(1);
    r.seq.set_loop_end(768);
    r.add(1, 0, 2, 1);
    r.add(1, 384, 2, 2);
    r.locate(0);
    r.seq.set_record(1, &mut r.dev);
    r.play();
    r.take();
    r.run(100);
    r.seq.record_hook(Device::Tb303_1, 2, 50);
    // to the wrap (tick 768 at block 552) and on to tick 345 of the next pass
    let log = r.run(700);
    let p: Vec<_> = log
        .iter()
        .filter(|(_, s)| s.starts_with("303a p"))
        .map(|(_, s)| s.as_str())
        .collect();
    // held in the first pass; the wrap chases to 0 (1), then 50 at 144; 384 is gone and the
    // return at 768, the loop end, never fires in the loop (OB-4)
    assert_eq!(p, ["303a p2=1", "303a p2=50"]);
    assert_eq!(
        triples(r.seq.events(1)),
        [(0, 2, 1), (144, 2, 50), (768, 2, 2)]
    );
    assert_eq!(r.seq.tracks[1].hold, [false; 128]);
    assert_eq!(r.seq.globals.byte2, 1);
}

/// Song end turns record off through `sub_10021194`, which merges.
#[test]
fn song_end_merges_the_recording() {
    let mut r = Rig::song_mode(300_000);
    r.locate(767_000);
    r.seq.set_record(1, &mut r.dev);
    r.play();
    r.seq.record_hook(Device::Tb303_1, 3, 9);
    r.take();
    r.run(200);
    assert_eq!(r.seq.globals.byte2, 0);
    assert_eq!(triples(r.seq.events(1)), [(767_016, 3, 9)]);
}

/// The pool refuses a new node when one is free (`sub_10014434`); a replacement takes none,
/// and a list never outgrows its reservation.
#[test]
fn the_pool_refuses_the_last_node() {
    let mut s = Sequencer::default();
    assert_eq!(s.free_nodes(), POOL_NODES);
    let events: Vec<_> = (0..POOL_NODES as u32)
        .map(|i| song::TrackEvent {
            tick: i / 128 * 24,
            ctrl: (i % 128) as u8,
            value: 1,
        })
        .collect();
    // as many events as free nodes: the batch is taken, its last insert refused
    assert!(s.add_events(1, &events));
    assert_eq!(s.events(1).len(), POOL_NODES - 1);
    assert_eq!(s.free_nodes(), 1);
    assert!(s.add_events(
        1,
        &[song::TrackEvent {
            tick: 0,
            ctrl: 0,
            value: 7,
        }],
    ));
    assert_eq!(
        s.events(1)[127],
        Event {
            tick: 0,
            ctrl: 0,
            value: 7
        }
    );
    assert_eq!(s.pool.nodes.capacity(), POOL_NODES);
    // recording is refused too, the hold byte set all the same
    let mut d = probes(&Log::default());
    s.set_song_mode(1, &mut d);
    s.set_record(1, &mut d);
    s.record_hook(Device::Tb303_2, 3, 1);
    assert!(s.recorded(2).is_empty());
    assert!(s.tracks[2].hold[3]);
}

/// The lists of a song share one pool (task-082, `dword_10100230`): each list keeps its own
/// order while the others move around it, a node one list takes is one less for all the
/// others, and a node a list frees is one more for them.
#[test]
fn the_lists_share_one_pool() {
    let ev = |tick, ctrl| song::TrackEvent {
        tick,
        ctrl,
        value: 1,
    };
    let mut s = Sequencer::default();
    assert!(s.add_events(4, &[ev(48, 1), ev(0, 1)]));
    assert!(s.add_events(0, &[ev(24, 2)]));
    assert!(s.add_events(8, &[ev(96, 3)]));
    assert!(s.add_events(4, &[ev(24, 1)]));
    assert_eq!(triples(s.events(0)), [(24, 2, 1)]);
    assert_eq!(triples(s.events(4)), [(0, 1, 1), (24, 1, 1), (48, 1, 1)]);
    assert_eq!(triples(s.events(8)), [(96, 3, 1)]);
    assert!(
        (1..9)
            .filter(|&t| t != 4 && t != 8)
            .all(|t| s.events(t).is_empty())
    );
    assert_eq!(s.free_nodes(), POOL_NODES - 5);
    // track 1 takes all but one free node: track 2's load and track 3's recording are refused
    let fill: Vec<_> = (0..(POOL_NODES - 6) as u32)
        .map(|i| ev(i / 128 * 24, (i % 128) as u8))
        .collect();
    assert!(s.add_events(1, &fill));
    assert_eq!(s.free_nodes(), 1);
    // one event, one node free: the batch passes the check, the insert is refused
    assert!(s.add_events(2, &[ev(0, 0)]));
    assert!(s.events(2).is_empty());
    let mut d = probes(&Log::default());
    s.set_song_mode(1, &mut d);
    s.set_record(1, &mut d);
    s.record_hook(Device::Tr808, 5, 1);
    assert!(s.recorded(3).is_empty());
    // a stop prunes the repeated values (all but tick 0's, on tracks 1 and 4): the others get
    // the nodes
    s.halt(&mut d);
    assert_eq!(s.events(1).len(), 128);
    assert_eq!(triples(s.events(4)), [(0, 1, 1)]);
    assert_eq!(s.free_nodes(), POOL_NODES - 3 - 128);
    s.record_hook(Device::Tr808, 5, 9);
    assert_eq!(triples(s.recorded(3)), [(0, 5, 9)]);
    assert!(s.add_events(2, &[ev(0, 0)]));
    assert_eq!(triples(s.events(2)), [(0, 0, 1)]);
    assert_eq!(s.free_nodes(), POOL_NODES - 5 - 128);
    assert_eq!(triples(s.events(0)), [(24, 2, 1)]);
    assert_eq!(triples(s.events(8)), [(96, 3, 1)]);
    // the merge moves the recorded node into its track and frees the recording's
    s.set_record(0, &mut d);
    assert!(s.recorded(3).is_empty());
    assert!(s.events(3).contains(&Event {
        tick: 0,
        ctrl: 5,
        value: 9
    }));
}

/// Msg 37 refuses a whole batch of more events than free nodes (`sub_10021010`, `ja` at
/// 0x1002105B): nothing of it is inserted, even the events that would replace a value and
/// take no node; a batch of exactly the free count is taken.
#[test]
fn msg37_refuses_a_batch_above_the_free_count() {
    let ev = |tick, ctrl, value| song::TrackEvent { tick, ctrl, value };
    let mut s = Sequencer::default();
    let fill: Vec<_> = (0..(POOL_NODES - 10) as u32)
        .map(|i| ev(i / 128 * 24, (i % 128) as u8, 1))
        .collect();
    assert!(s.add_events(1, &fill));
    assert_eq!(s.free_nodes(), 10);
    let revision = s.revision;
    // 11 events, all of them replacements of track 1's tick-0 values
    let over: Vec<_> = (0..11).map(|c| ev(0, c, 9)).collect();
    assert!(!s.add_events(1, &over));
    assert_eq!(s.free_nodes(), 10);
    assert!(s.events(1)[..11].iter().all(|e| e.value == 1));
    assert_eq!(s.revision, revision);
    // 10 new events on track 3: taken, the last insert finds one node free and stores nothing
    let exact: Vec<_> = (0..10).map(|c| ev(24, c, 5)).collect();
    assert!(s.add_events(3, &exact));
    assert_eq!(s.events(3).len(), 9);
    assert_eq!(s.free_nodes(), 1);
}

/// A pool's clone and the callback's copy of the track lists keep the whole pool's capacity,
/// so neither grows in the callback.
#[test]
fn a_pool_copy_keeps_the_capacity() {
    let mut s = Sequencer::default();
    assert!(s.add_events(
        2,
        &[song::TrackEvent {
            tick: 0,
            ctrl: 1,
            value: 2,
        }],
    ));
    let c = s.clone();
    assert_eq!(c.pool.nodes.capacity(), POOL_NODES);
    assert_eq!(c, s);
    let mut copy = Pool::new();
    let mut d = probes(&Log::default());
    s.set_song_mode(1, &mut d);
    s.set_record(1, &mut d);
    s.record_hook(Device::Tr909, 3, 4);
    copy.copy_tracks_from(&s.pool);
    assert_eq!(copy.nodes.capacity(), POOL_NODES);
    for t in 0..TRACKS {
        assert_eq!(&copy[t], s.events(t));
        assert!(copy[TRACKS + t].is_empty());
    }
    assert_eq!(s.recorded(4).len(), 1);
}
