//! Task-084: the merge's cost on a large song, and its equivalence with the insert-by-insert
//! merge it replaced.
//!
//! The timing test is `#[ignore]`d; run it in release:
//! `cargo test --release -p orb-engine --lib merge_timing -- --ignored --nocapture`.

use super::*;
use std::time::Instant;

/// A small deterministic generator (xorshift64*).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// In `0..n`.
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// The merge as task-057 wrote it, one `Vec::insert` / `Vec::remove` per node: the reference
/// the one-pass [`Track::merge`] must match node for node.
fn merge_reference(tr: &mut Track, pool: &mut Pool, t: i32) {
    let (ev, rec) = (tr.list, tr.rec());
    let mut marked = [false; 128];
    let mut value = [0xFFu8; 128];
    let mut r = 0;
    let mut i = 0;
    while i < pool.list(ev).len() && t >= pool.list(ev)[i].tick {
        let m = pool.list(ev)[i];
        let recorded = pool.list(rec);
        while r < recorded.len() && recorded[r].tick <= m.tick {
            marked[recorded[r].ctrl as usize & 127] = true;
            r += 1;
        }
        let c = m.ctrl as usize & 127;
        value[c] = m.value;
        if marked[c] && m.tick != 0 {
            pool.remove(ev, i);
            if i < tr.next {
                tr.next -= 1;
            }
        } else {
            i += 1;
        }
    }
    let recorded = pool.list(rec);
    while r < recorded.len() && t >= recorded[r].tick {
        marked[recorded[r].ctrl as usize & 127] = true;
        r += 1;
    }
    for c in 0..128u8 {
        if !marked[c as usize] {
            continue;
        }
        let back = tr.quantise(t.wrapping_add(1), c);
        if !pool.list(ev).iter().any(|e| e.tick == back && e.ctrl == c) {
            tr.insert_pooled(pool, back, c, value[c as usize]);
        }
    }
    for k in 0..r {
        let e = pool.list(rec)[k];
        tr.insert_pooled(pool, e.tick, e.ctrl, e.value);
    }
    pool.clear(rec);
    tr.hold = [false; 128];
}

/// [`Sequencer::merge_all`] with [`merge_reference`].
fn merge_all_reference(s: &mut Sequencer, t: i32) {
    if (0..TRACKS).any(|i| !s.recorded(i).is_empty()) {
        s.merges += 1;
    }
    for tr in &mut s.tracks {
        merge_reference(tr, &mut s.pool, t);
    }
    s.revision += 1;
}

/// A song of about `n` events on the nine tracks over `bars` bars (ticks on the 24-tick grid,
/// controllers `0..ctrls`), the 303s' controllers 0 and 1 quantised to a bar.
fn song(rng: &mut Rng, n: usize, bars: u64, ctrls: u64) -> Sequencer {
    let mut s = Sequencer::default();
    for i in [1, 2] {
        s.tracks[i].set_quant(0, 32);
        s.tracks[i].set_quant(1, 32);
    }
    for tr in 0..TRACKS {
        let events: Vec<_> = (0..n / TRACKS)
            .map(|_| song::TrackEvent {
                tick: 24 * rng.below(bars * 32) as u32,
                ctrl: rng.below(ctrls) as u8,
                value: rng.below(128) as u8,
            })
            .collect();
        for batch in events.chunks(1284) {
            let _ = s.add_events(tr, batch);
        }
    }
    s.globals.song_mode = 1;
    s.globals.byte2 = 1;
    s
}

/// `n` panel moves through the record hook at ticks up to `t`, on random tracks and
/// controllers below `ctrls`, playing or stopped, the loop `wrap` on or off.
fn record(
    s: &mut Sequencer,
    rng: &mut Rng,
    n: usize,
    t: i32,
    ctrls: u64,
    wrap: Option<(u32, u32)>,
) {
    for _ in 0..n {
        s.globals.play = rng.below(4).min(1) as u8;
        match wrap {
            Some((start, end)) => {
                s.globals.loop_on = 1;
                s.globals.loop_start = start;
                s.globals.loop_end = end;
            }
            None => s.globals.loop_on = 0,
        }
        s.pos = rng.below(t as u64 + 1) as f64;
        let d = Device::ALL[rng.below(TRACKS as u64) as usize];
        s.record_hook(d, rng.below(ctrls) as u8, rng.below(256) as u8);
    }
    s.globals.play = 1;
}

/// Chases every track to a random tick, so that the applied pointers are anywhere.
fn chase_somewhere(s: &mut Sequencer, rng: &mut Rng, ticks: u64) {
    for tr in &mut s.tracks {
        tr.chase(&s.pool, rng.below(ticks) as i32);
    }
}

/// The one-pass merge gives what the insert-by-insert one gives: the same pool (every list,
/// node for node), applied pointers, hold masks and counters; over random songs, recordings
/// (playing or stopped, the loop wrap's tick rule), merge ticks, and pools down to their last
/// nodes, where the merge's inserts are refused. 400 cases in release, 40 in a debug build.
#[test]
fn the_merge_matches_the_reference() {
    let mut rng = Rng(0x0123_4567_89AB_CDEF);
    let cases = if cfg!(debug_assertions) { 40 } else { 400 };
    for case in 0..cases {
        let ctrls = [3, 8, 40, 128][case % 4];
        let n = rng.below(3000) as usize;
        let bars = 1 + rng.below(16);
        let mut s = song(&mut rng, n, bars, ctrls);
        let t = rng.below(16 * 768) as i32;
        let wrap = (rng.below(2) == 0).then(|| {
            let start = rng.below(8) as u32 * 24 + rng.below(30) as u32;
            (start, start + rng.below(4096) as u32)
        });
        let moves = rng.below(2500) as usize;
        record(&mut s, &mut rng, moves, t + 400, ctrls, wrap);
        if case % 5 == 4 {
            // fill the pool (track 8's list, behind the others) down to a few nodes
            let left = rng.below(40) as usize;
            let fill: Vec<_> = (0..s.free_nodes().saturating_sub(left) as u32)
                .map(|i| song::TrackEvent {
                    tick: 24 * (30_000 + i / 128),
                    ctrl: (i % 128) as u8,
                    value: 1,
                })
                .collect();
            for batch in fill.chunks(1284) {
                let _ = s.add_events(8, batch);
            }
        }
        chase_somewhere(&mut s, &mut rng, 16 * 768);
        let mut want = s.clone();
        merge_all_reference(&mut want, t);
        s.merge_all(t);
        assert_eq!(s, want, "case {case}");
        assert_eq!(s.pool.nodes.capacity(), POOL_NODES, "case {case}");
    }
}

/// The same with more new nodes on one track than [`MERGE_CHUNK`], so that the merge moves
/// them into the list in several chunks, later chunks replacing values in earlier ones; the
/// pool running out of nodes before, within and after the second chunk.
#[test]
fn a_merge_of_several_chunks_matches_the_reference() {
    let mut rng = Rng(0xFEED_FACE_0000_0084);
    for case in 0..8 {
        let mut s = song(&mut rng, 3000, 16, 128);
        let t = 12 * 768 + rng.below(768) as i32;
        s.globals.play = 0;
        for _ in 0..12_000 {
            s.pos = rng.below(16 * 768) as f64;
            let d = [Device::Mixer, Device::Tb303_1][rng.below(2) as usize];
            s.record_hook(d, rng.below(128) as u8, rng.below(256) as u8);
        }
        if case >= 4 {
            let left = [MERGE_CHUNK + 50, MERGE_CHUNK / 2, 2 * MERGE_CHUNK + 7, 3][case - 4];
            let fill: Vec<_> = (0..s.free_nodes().saturating_sub(left) as u32)
                .map(|i| song::TrackEvent {
                    tick: 24 * (30_000 + i / 128),
                    ctrl: (i % 128) as u8,
                    value: 1,
                })
                .collect();
            for batch in fill.chunks(1284) {
                let _ = s.add_events(8, batch);
            }
        }
        chase_somewhere(&mut s, &mut rng, 16 * 768);
        let mut want = s.clone();
        merge_all_reference(&mut want, t);
        s.merge_all(t);
        assert_eq!(s, want, "case {case}");
    }
}

/// Times `merge_all(t)` on copies of `s`: the best and the median of `runs`, in microseconds.
fn time_merge(s: &Sequencer, t: i32, runs: usize, reference: bool) -> (f64, f64) {
    let mut us: Vec<f64> = (0..runs)
        .map(|_| {
            let mut c = s.clone();
            let t0 = Instant::now();
            if reference {
                merge_all_reference(&mut c, t);
            } else {
                c.merge_all(t);
            }
            let dt = t0.elapsed().as_secs_f64() * 1e6;
            std::hint::black_box(&c);
            dt
        })
        .collect();
    us.sort_by(f64::total_cmp);
    (us[0], us[runs / 2])
}

/// The cost of a merge on a song of ~10000 events and on a nearly full pool, ~1000 nodes
/// recorded across the tracks over an 8-bar loop, merged at its end (the loop wrap's
/// `loopEnd - 1`); and a stress case, ~20000 nodes recorded on one track (more than a
/// [`MERGE_CHUNK`]) of an ~70000-event song. Prints the time per merge and per recorded node.
#[test]
#[ignore = "timing; run in release with --ignored --nocapture"]
fn merge_timing() {
    let t = 8 * 768 - 1;
    for (name, n, bars, moves) in [
        ("~10000 events", 10_000, 64, 1000),
        ("nearly full", 98_000, 999, 1000),
        ("stress", 70_000, 999, 0),
    ] {
        let mut rng = Rng(0x0BAD_5EED_1234_5678);
        let mut s = song(&mut rng, n, bars, 48);
        record(&mut s, &mut rng, moves, t, 48, None);
        if moves == 0 {
            // ~20000 distinct moves on the mixer's track (list 0, the whole pool behind it)
            s.globals.play = 0;
            for k in 0..20_000 {
                s.pos = (24 * (k / 80) % (t + 1)) as f64;
                s.record_hook(Device::Mixer, (k % 80) as u8, 1);
            }
        }
        let recorded: usize = (0..TRACKS).map(|i| s.recorded(i).len()).sum();
        let events: usize = (0..TRACKS).map(|i| s.events(i).len()).sum();
        let mut after = s.clone();
        after.merge_all(t);
        let merged: usize = (0..TRACKS).map(|i| after.events(i).len()).sum();
        for (label, reference) in [("insert-by-insert", true), ("one-pass", false)] {
            let (best, median) = time_merge(&s, t, 21, reference);
            println!(
                "{name}: {events} events, {recorded} recorded, {merged} after, free {}: \
                 {label} merge_all best {best:.1} us, median {median:.1} us, \
                 {:.3} us per recorded node",
                s.free_nodes(),
                median / recorded as f64
            );
        }
    }
}
