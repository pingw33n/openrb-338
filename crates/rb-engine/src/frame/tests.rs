use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, OnceLock};

use super::export::{ExportRange, chunk_sizes, frame_count};
use super::*;

fn tables() -> Arc<Tables> {
    static T: OnceLock<Arc<Tables>> = OnceLock::new();
    T.get_or_init(|| Arc::new(Tables::new())).clone()
}

type Log = Rc<RefCell<Vec<String>>>;

/// A source that writes a constant.
struct Const(f32);
impl SongDevice for Const {}
impl ChannelDevice for Const {
    fn process(&mut self, bus: &mut Block, _ctx: &BlockContext) {
        bus.fill(self.0);
    }
}

/// A source whose samples differ in every block: `(64k + i) / 8192` in block k.
struct Ramp(u32);
impl SongDevice for Ramp {}
impl ChannelDevice for Ramp {
    fn process(&mut self, bus: &mut Block, _ctx: &BlockContext) {
        for (i, x) in bus.iter_mut().enumerate() {
            *x = ((self.0 * 64 + i as u32) % 8192) as f32 / 8192.0;
        }
        self.0 += 1;
    }
}

/// An insert that applies `f` and logs its calls.
struct Op {
    name: &'static str,
    f: fn(f32) -> f32,
    log: Log,
}
impl InsertEffect for Op {
    fn process(&mut self, bus: &mut Block, _ctx: &BlockContext) {
        self.log.borrow_mut().push(self.name.to_string());
        for x in bus.iter_mut() {
            *x = (self.f)(*x);
        }
    }
    fn process_stereo(&mut self, _l: &mut Block, _r: &mut Block, _ctx: &BlockContext) {
        self.log.borrow_mut().push(format!("{}.stereo", self.name));
    }
    fn update(&mut self, _ctx: &BlockContext) {
        self.log.borrow_mut().push(format!("{}.update", self.name));
    }
}
impl SongDevice for Op {
    fn reset(&mut self) {
        self.log.borrow_mut().push(format!("{}.reset", self.name));
    }
}

/// A send effect that records the send bus it gets, then refills it like the delay.
struct Recorder {
    seen: Rc<RefCell<Vec<Block>>>,
    log: Log,
}
impl SongDevice for Recorder {}
impl SendEffect for Recorder {
    fn process(&mut self, send: &mut Block, _l: &mut Block, _r: &mut Block, _ctx: &BlockContext) {
        self.seen.borrow_mut().push(*send);
        self.log.borrow_mut().push("delay".into());
        send.fill(GUARD);
    }
}

fn engine_with(devices: Devices, state: EngineState) -> Engine {
    Engine::with_state(tables(), devices, state)
}

/// A state with every gain, target and send at zero and the master at a steady 1.0, outside the
/// update phase.
fn quiet_state() -> EngineState {
    let mut s = EngineState::default();
    s.mixer.counter = 1;
    for ch in &mut s.mixer.channels {
        *ch = ChannelState {
            target_l: 0.0,
            target_r: 0.0,
            gain_l: 0.0,
            gain_r: 0.0,
            send: 0.0,
            ..ChannelState::default()
        };
    }
    s
}

fn block_once(e: &mut Engine) -> (Block, Block) {
    let mut l = [0.0; BLOCK];
    let mut r = [0.0; BLOCK];
    e.frame.block(&mut l, &mut r);
    (l, r)
}

#[test]
fn constants_decode() {
    assert_eq!(GUARD, 1e-6f32);
    assert_eq!(KEEP, 0.93);
    assert_eq!(TAKE, 0.07);
    assert_eq!(RAMP, 1.0 / 32.0);
    assert_eq!(CLAMP_HI, 32767.0);
    assert_eq!(CLAMP_LO, -32768.0);
    assert_eq!(METER_DECAY, 0.92f32);
    assert_eq!(VINTAGE_OFFSET, 1e-6f32 as f64);
}

#[test]
fn int16_conversion() {
    let cases: [(f32, i16); 14] = [
        (0.0, 0),
        (GUARD, 0),
        (-GUARD, 0),
        (0.5, 16384),
        (-0.5, -16384),
        (1.0 / 32768.0, 1),
        (-1.0 / 32768.0, -1),
        (0.99999 / 32768.0, 0),
        (-1.99 / 32768.0, -1),
        (32767.5 / 32768.0, 32767),
        (1.0, 32767),
        (7.0, 32767),
        (-1.0, -32768),
        (-7.0, -32768),
    ];
    for (f, want) in cases {
        assert_eq!(to_i16(f), want, "{f:e}");
    }
    assert_eq!(to_i16(f32::NAN), -32768);
}

#[test]
fn constructor_state() {
    let e = Engine::new(tables(), Devices::placeholders());
    let t = &tables().level;
    let s = &e.frame.state;
    assert_eq!(s.counter, 10);
    assert_eq!(s.master, 1.0);
    assert_eq!(
        s.master_target,
        (2.0 * t[100] as f64 + 1e-6f32 as f64) as f32
    );
    for ch in &s.channels {
        let tgt = (1e-6f32 as f64 + t[100] as f64) as f32;
        assert_eq!((ch.target_l, ch.target_r), (tgt, tgt));
        assert_eq!((ch.gain_l, ch.gain_r), (GUARD, GUARD));
        assert_eq!(ch.send, GUARD);
        assert_eq!(ch.routing, Routing::default());
    }
    assert_eq!(e.output.leftover_pos, 0);
}

#[test]
fn pan_law_and_sends() {
    let mut e = engine_with(Devices::placeholders(), EngineState::default());
    let t = tables().level;
    let g = 1e-6f32 as f64;
    let p = &mut e.frame.devices.mixer.live;
    p.master = 127;
    p.channels[0] = ChannelParams {
        enabled: 1,
        level: 127,
        pan: 0,
        send: 127,
        dist: 0,
    };
    p.channels[1] = ChannelParams {
        enabled: 1,
        level: 127,
        pan: 127,
        send: 64,
        dist: 0,
    };
    p.channels[2] = ChannelParams {
        enabled: 0,
        level: 127,
        pan: 64,
        send: 127,
        dist: 0,
    };
    p.channels[3] = ChannelParams {
        enabled: 1,
        level: 100,
        pan: 96,
        send: 0,
        dist: 0,
    };
    e.frame.mixer_update();
    let c = &e.frame.state.channels;
    // hard left: d = -64, q = 128, R = T[0] = 0
    assert_eq!(c[0].target_l, (g + 1.0) as f32);
    assert_eq!(c[0].target_r, GUARD);
    assert_eq!(c[0].send, (1.0 + g) as f32);
    // hard right: d = 63, q = 124, L = T[127 - 124]
    assert_eq!(c[1].target_l, (g + t[3] as f64) as f32);
    assert_eq!(c[1].target_r, (g + 1.0) as f32);
    assert_eq!(c[1].send, (t[64] as f64 + g) as f32);
    // off: the level is 0
    assert_eq!(
        (c[2].target_l, c[2].target_r, c[2].send),
        (GUARD, GUARD, GUARD)
    );
    // d = 32, q = 32: L = T[100 - 32]
    assert_eq!(c[3].target_l, (g + t[68] as f64) as f32);
    assert_eq!(c[3].target_r, (g + t[100] as f64) as f32);
    assert_eq!(e.frame.state.master_target, (2.0 + g) as f32);

    // Vintage: (PAN.L + PAN.R) * T[send] * 0.5 + 1e-6
    e.frame.vintage = true;
    e.frame.mixer_update();
    let c = &e.frame.state.channels;
    assert_eq!(c[0].send, (1.0 * 0.5 + g) as f32);
    assert_eq!(
        c[1].send,
        ((t[3] as f64 + 1.0) * t[64] as f64 * 0.5 + g) as f32
    );
    assert_eq!(c[3].send, GUARD);
}

#[test]
fn smoothing_and_ramp() {
    let mut d = Devices::placeholders();
    d.channels[0] = Box::new(Const(1.0));
    let mut s = quiet_state();
    s.mixer.channels[0].target_l = 1.0;
    s.mixer.channels[0].target_r = 0.5;
    let mut e = engine_with(d, s);
    let (l, r) = block_once(&mut e);
    let ch = &e.frame.state.channels[0];
    // new = 0*0.93 + target*0.07, stored as f32
    assert_eq!(ch.gain_l, 0.07f64 as f32);
    assert_eq!(ch.gain_r, (0.5 * 0.07f64) as f32);
    // the master: 1*0.93 + 1*0.07, unrounded
    let m = 0.93f64 + 0.07;
    // pair p uses the gain after p steps of 0.07/32 (the ramp starts at the old gain)
    let (dl, dr) = (0.07 / 32.0, 0.5 * 0.07 / 32.0);
    let (mut gl, mut gr) = (0.0f64, 0.0f64);
    for p in 0..32 {
        let want_l = ((gl + 1e-6f32 as f64) as f32 as f64 * m) as f32;
        let want_r = ((gr + 1e-6f32 as f64) as f32 as f64 * m) as f32;
        assert_eq!((l[2 * p], l[2 * p + 1]), (want_l, want_l), "pair {p}");
        assert_eq!((r[2 * p], r[2 * p + 1]), (want_r, want_r), "pair {p}");
        gl += dl;
        gr += dr;
    }
    assert_eq!(l[0], GUARD);
    // the last pair: 31 steps, 0.0678125 (+ 1e-6), not yet the stored 0.07
    assert!((l[63] as f64 - 0.0678135).abs() < 1e-7);
    // the next block ramps from the stored f32, towards 0.07 + 0.07*0.93
    let (l2, _) = block_once(&mut e);
    assert_eq!(
        l2[0],
        ((0.07f32 as f64 + 1e-6f32 as f64) as f32 as f64 * m) as f32
    );
}

#[test]
fn smoothing_settles_on_an_f32_fixed_point() {
    let mut d = Devices::placeholders();
    d.channels[0] = Box::new(Const(0.25));
    let mut s = quiet_state();
    s.mixer.channels[0].target_l = 0.8;
    let mut e = engine_with(d, s);
    for _ in 0..2000 {
        block_once(&mut e);
    }
    // (the updates have replaced the target by the default level's)
    let ch = e.frame.state.channels[0];
    let g = ch.gain_l;
    assert_eq!(((g as f64) * 0.93 + ch.target_l as f64 * 0.07) as f32, g);
    assert!((g - ch.target_l).abs() < 1e-6);
}

#[test]
fn send_accumulation() {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let log: Log = Rc::default();
    let mut d = Devices::placeholders();
    d.channels[0] = Box::new(Const(0.5));
    d.channels[2] = Box::new(Const(-0.75));
    d.delay = Box::new(Recorder {
        seen: seen.clone(),
        log: log.clone(),
    });
    let mut s = quiet_state();
    s.mixer.channels[0].send = 0.25;
    s.mixer.channels[2].send = 0.5;
    let mut e = engine_with(d, s);
    block_once(&mut e);
    block_once(&mut e);
    let seen = seen.borrow();
    // from the guard: ((1e-6 + 0.5*0.25) + 0*1e-6) + (-0.75*0.5), each sum stored as f32
    let a = (0.125 + 1e-6f32 as f64) as f32;
    let want = (-0.375 + a as f64) as f32;
    assert_eq!(seen[0], [want; BLOCK]);
    // the send effect refilled the bus, so the second block starts from the guard again
    assert_eq!(seen[1], seen[0]);
}

#[test]
fn routing_order() {
    let log: Log = Rc::default();
    let seen = Rc::new(RefCell::new(Vec::new()));
    let mut d = Devices::placeholders();
    d.channels[1] = Box::new(Const(0.0));
    d.dist = Box::new(Op {
        name: "dist",
        f: |x| x + 1.0,
        log: log.clone(),
    });
    d.pcf = Box::new(Op {
        name: "pcf",
        f: |x| x * 2.0,
        log: log.clone(),
    });
    d.comp = Box::new(Op {
        name: "comp",
        f: |x| x + 3.0,
        log: log.clone(),
    });
    d.delay = Box::new(Recorder {
        seen: seen.clone(),
        log: log.clone(),
    });
    let mut e = engine_with(d, quiet_state());
    e.frame.devices.mixer.live.channels[1].dist = 1;
    e.frame.devices.mixer.live.pcf_id = 2; // channel 1
    e.frame.devices.mixer.live.comp_id = 3; // channel 1
    e.frame.mixer_update();
    assert_eq!(*log.borrow(), ["dist.update", "pcf.update", "comp.update"]);
    let r = e.frame.state.channels.map(|c| c.routing);
    assert_eq!(
        r[1],
        Routing {
            dist: true,
            pcf: true,
            comp: true
        }
    );
    assert_eq!(r[0], Routing::default());
    assert!(!e.frame.state.master_comp);
    // the update set the send gains; give channel 1 a unit send and nothing else
    for ch in &mut e.frame.state.channels {
        ch.send = 0.0;
    }
    e.frame.state.channels[1].send = 1.0;
    log.borrow_mut().clear();
    block_once(&mut e);
    assert_eq!(*log.borrow(), ["dist", "pcf", "comp", "delay"]);
    // Dist, then PCF, then Comp: ((0 + 1) * 2) + 3
    assert_eq!(seen.borrow()[0][0], (5.0 + 1e-6f32 as f64) as f32);

    // the compressor on the master runs after the send effect
    e.frame.devices.mixer.live.comp_id = 1;
    e.frame.devices.mixer.live.channels[1].dist = 0;
    e.frame.mixer_update();
    assert!(e.frame.state.master_comp);
    assert!(!e.frame.state.channels[1].routing.comp);
    log.borrow_mut().clear();
    block_once(&mut e);
    assert_eq!(*log.borrow(), ["pcf", "delay", "comp.stereo"]);
}

#[test]
fn update_phase() {
    let mut e = Engine::new(tables(), Devices::placeholders());
    let before = e.frame.state.channels[0].target_l;
    e.frame.devices.mixer.live.channels[0].level = 127;
    let mut buf = vec![0i16; 2 * 1024];
    // counter 10..15: no update in the first six blocks
    e.render(&mut buf, 6 * BLOCK);
    assert_eq!(e.frame.state.counter, 16);
    assert_eq!(e.frame.state.channels[0].target_l, before);
    // the seventh block runs it first
    e.render(&mut buf, BLOCK);
    assert_eq!(e.frame.state.counter, 17);
    assert_eq!(
        e.frame.state.channels[0].target_l,
        (1e-6f32 as f64 + 1.0) as f32
    );
    // then every 16 blocks
    e.frame.devices.mixer.live.channels[0].level = 0;
    e.render(&mut buf, 15 * BLOCK);
    assert_eq!(
        e.frame.state.channels[0].target_l,
        (1e-6f32 as f64 + 1.0) as f32
    );
    e.render(&mut buf, BLOCK);
    assert_eq!(e.frame.state.channels[0].target_l, GUARD);
    assert_eq!(e.frame.state.counter, 33);
}

fn loud_engine() -> Engine {
    let mut d = Devices::placeholders();
    d.channels[0] = Box::new(Ramp(0));
    d.channels[3] = Box::new(Const(-0.3));
    let mut e = Engine::new(tables(), d);
    for ch in &mut e.frame.state.channels {
        ch.gain_l = ch.target_l;
        ch.gain_r = ch.target_r;
    }
    e
}

#[test]
fn leftover_buffer() {
    let mut whole = loud_engine();
    let mut a = vec![0i16; 2 * 228];
    whole.render(&mut a, 228);
    assert_eq!(whole.output.leftover_pos, 36);
    assert!(a.iter().any(|&x| x != 0));

    let mut split = loud_engine();
    let mut b = Vec::new();
    let mut buf = vec![0i16; 2 * 228];
    for (n, lp) in [(100, 36), (28, 0), (100, 36)] {
        split.render(&mut buf, n);
        assert_eq!(split.output.leftover_pos, lp);
        b.extend_from_slice(&buf[..2 * n]);
    }
    assert_eq!(a, b);
    // the same blocks were rendered: four in each
    assert_eq!(whole.frame.state.counter, split.frame.state.counter);

    // a 0-frame call drops the rest of the leftover block, and the position goes negative
    split.render(&mut [], 0);
    assert_eq!(split.output.leftover_pos, 36 - 64);
    let c0 = split.frame.state.counter;
    split.render(&mut buf, 64);
    assert_eq!(split.output.leftover_pos, 0);
    assert_eq!(split.frame.state.counter, c0 + 1);
    // the copied part of the leftover block is refilled with the guard
    split.render(&mut buf, 10);
    assert_eq!(split.output.leftover[0][..10], [GUARD; 10]);
    // the clamp
    let mut big = vec![0i16; 2 * (MAX_FRAMES + 64)];
    let c0 = split.frame.state.counter;
    split.render(&mut big, MAX_FRAMES + 64);
    assert_eq!(split.frame.state.counter, c0 + 256);
}

/// Marks the global delay line: a value that is not the guard and the gate cleared.
fn dirty_delay_line(p: &ProcessState) {
    let mut l = p.delay_line();
    l.buf.fill(0.5);
    l.gate = false;
    p.set_delay_line(l);
}

fn assert_refilled(p: &ProcessState) {
    let l = p.delay_line();
    assert!(l.gate);
    assert!(l.buf.iter().all(|&x| x.to_bits() == GUARD.to_bits()));
}

#[test]
fn msg53_replacing_a_song_refills_the_delay_line() {
    for a in [Activation::ReplacesCurrent, Activation::NoneCurrent] {
        let mut e = Engine::new(tables(), Devices::implemented(None));
        let p = e.process_state().clone();
        dirty_delay_line(&p);
        e.activate(a);
        assert_refilled(&p);
    }
}

#[test]
fn msg33_reset() {
    let log: Log = Rc::default();
    let mut d = Devices::placeholders();
    d.comp = Box::new(Op {
        name: "comp",
        f: |x| x,
        log: log.clone(),
    });
    let mut e = loud_engine();
    e.frame.devices.comp = d.comp;
    let p = e.process_state().clone();
    e.frame.devices.delay = Box::new(crate::delay::Delay::new(p.clone()));
    let mut buf = vec![0i16; 2 * 64];
    e.render(&mut buf, 40);
    e.frame.state.master_peak = [0.5, 0.5];
    e.frame.state.send = [0.25; BLOCK];
    dirty_delay_line(&p);
    let before = e.state();
    e.reset();
    // the delay's slot 5 (0x1000F4F0) runs slot 6: the global line refilled, the gate set
    assert_refilled(&p);
    let s = e.state();
    assert_eq!(s.mixer.master, 0.0);
    assert_eq!(s.mixer.master_target, before.mixer.master_target);
    assert_eq!(s.mixer.master_peak, [0.0; 2]);
    assert_eq!(s.mixer.send, [GUARD; BLOCK]);
    assert_eq!(s.output.leftover, [[0.0; BLOCK]; 2]);
    assert_eq!(s.output.leftover_pos, 40);
    assert_eq!(s.mixer.counter, before.mixer.counter);
    assert_eq!(s.mixer.channels, before.mixer.channels);
    assert_eq!(*log.borrow(), ["comp.reset"]);
}

#[test]
fn export_chunks() {
    // 120 BPM, two bars: 4 s
    let r = ExportRange {
        tempo: 120_000,
        loop_start: 768,
        loop_end: 3 * 768,
    };
    let c = chunk_sizes(r);
    assert!(c[..c.len() - 1].iter().all(|&n| n == 2048));
    assert!(*c.last().unwrap() < 2048);
    assert_eq!(frame_count(r), 176_400);
    // an empty loop still makes one 0-frame call
    let r = ExportRange {
        tempo: 120_000,
        loop_start: 768,
        loop_end: 768,
    };
    assert_eq!(chunk_sizes(r), [0]);
    // 140 BPM, one bar: 60/140*4*44100 = 75600 frames, less one when the accumulation
    // truncates below it
    let r = ExportRange {
        tempo: 140_000,
        loop_start: 0,
        loop_end: 768,
    };
    let n = frame_count(r);
    assert!(n == 75_600 || n == 75_599, "{n}");
}

#[test]
fn export_of_silence_is_zero() {
    let mut e = Engine::new(tables(), Devices::placeholders());
    let r = ExportRange {
        tempo: 120_000,
        loop_start: 0,
        loop_end: 768,
    };
    let s = export::export(&mut e, r, 160);
    assert_eq!(s.len(), 2 * frame_count(r));
    assert!(s.iter().all(|&x| x == 0));
    // the final 0-frame call dropped the last chunk's leftover
    assert!(e.output.leftover_pos <= 0);
    // msg 33 ran after the chunks
    assert_eq!(e.frame.state.master, 0.0);
}
