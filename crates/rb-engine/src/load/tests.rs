//! Unit tests of the mappings of `docs/song_load.md`, on edited copies of a shipped song.

use super::*;

const SILENT: &[u8] = include_bytes!("../../../../tests/songs/shipped/Silent Default Song.rbs");

/// The silent default song (file version 4.1.0) at version `minor`.`revision`.
fn song(minor: u8, revision: u8) -> rbs::Song {
    let mut s = rbs::Song::parse(SILENT).unwrap();
    assert_eq!((s.head.major, s.head.minor, s.head.revision), (4, 1, 0));
    s.head.minor = minor;
    s.head.revision = revision;
    s
}

fn ev(pos: u32, ctrl: u8, value: u8) -> rbs::Event {
    rbs::Event { pos, ctrl, value }
}

fn te(tick: u32, ctrl: u8, value: u8) -> TrackEvent {
    TrackEvent { tick, ctrl, value }
}

/// All accepted events of one track.
fn events(track: u8, evs: &[rbs::Event], reader: TrackReader) -> Vec<TrackEvent> {
    track_batches(track, evs, reader)
        .into_iter()
        .flat_map(|b| b.events)
        .collect()
}

#[test]
fn globals_from_glob() {
    let mut s = song(2, 0);
    s.glob.song_mode = 1;
    s.glob.loop_on = 1;
    s.glob.tempo = 133_250;
    s.glob.loop_start = 2 * 768;
    s.glob.loop_end = 10 * 768;
    s.glob.shuffle = 77;
    s.glob.vintage = 1;
    let g = convert(&s).unwrap().globals;
    assert_eq!(
        g,
        Globals {
            song_mode: 1,
            play: 0,
            byte2: 0,
            loop_on: 1,
            tempo: 133_250,
            locate: 0,
            loop_start: 1536,
            loop_end: 7680,
            shuffle: 77,
            vintage: 1,
            byte22: 1,
        }
    );
    // The vintage byte is GLOB[482] from 4.2.0 on (any revision), 0 before.
    s.head.revision = 3;
    assert_eq!(convert(&s).unwrap().globals.vintage, 1);
    s.glob.vintage = 7;
    assert_eq!(convert(&s).unwrap().globals.vintage, 7);
    for minor in [0, 1] {
        s.head.minor = minor;
        s.head.revision = 0;
        assert_eq!(convert(&s).unwrap().globals.vintage, 0);
    }
}

#[test]
fn version_major_must_be_4() {
    let mut s = song(2, 0);
    s.head.major = 3;
    assert_eq!(
        convert(&s),
        Err(LoadError::Version {
            major: 3,
            minor: 2,
            revision: 0
        })
    );
}

fn set_channels(s: &mut rbs::Song) {
    s.mixer.master = 90;
    s.mixer.comp_id = 1;
    s.mixer.pcf_id = 3;
    for (c, ch) in s.mixer.channels.iter_mut().enumerate() {
        let c = c as u8;
        *ch = rbs::MixChannel {
            enabled: c & 1,
            level: 10 + c,
            pan: 20 + c,
            delay_send: 30 + c,
            dist: (c == 2) as u8,
        };
    }
}

#[test]
fn mixer_4_2_as_stored() {
    let mut s = song(2, 0);
    set_channels(&mut s);
    let m = convert(&s).unwrap().mixer;
    assert_eq!((m.master, m.comp_id, m.pcf_id), (90, 1, 3));
    let en: Vec<_> = m.channels.iter().map(|c| c.enabled).collect();
    assert_eq!(en, [0, 1, 0, 1]);
    assert_eq!(
        m.channels[2],
        MixerChannel {
            enabled: 0,
            level: 12,
            pan: 22,
            delay_send: 32,
            dist: 1
        }
    );
}

#[test]
fn mixer_4_1_enables_forced() {
    let mut s = song(1, 0);
    set_channels(&mut s);
    let m = convert(&s).unwrap().mixer;
    assert!(m.channels.iter().all(|c| c.enabled == 1));
    assert_eq!(m.channels[3].level, 13);
    // Only exactly 4.1.0 (sub_47DA82 is an equality).
    s.head.revision = 1;
    let m = convert(&s).unwrap().mixer;
    let en: Vec<_> = m.channels.iter().map(|c| c.enabled).collect();
    assert_eq!(en, [0, 1, 0, 1]);
}

/// Writes `s`, patches the MIXR body bytes, and parses it back.
fn patch_mixr(s: &rbs::Song, patches: &[(usize, u8)]) -> rbs::Song {
    let mut b = s.write().unwrap();
    let at = b.windows(4).position(|w| w == b"MIXR").unwrap() + 8;
    for &(i, v) in patches {
        b[at + i] = v;
    }
    rbs::Song::parse(&b).unwrap()
}

/// The PCF and comp flag bytes (5, 6) of the four 7-byte channels of a 4.0 MIXR, all 0.
const NO_FLAGS: [(usize, u8); 8] = [
    (16 + 5, 0),
    (16 + 6, 0),
    (28 + 5, 0),
    (28 + 6, 0),
    (40 + 5, 0),
    (40 + 6, 0),
    (52 + 5, 0),
    (52 + 6, 0),
];

#[test]
fn mixer_4_0_converter() {
    let mut s = song(0, 0);
    set_channels(&mut s);
    s.mixer.comp_id = 0;
    // Channel bytes 5 (PCF flag) and 6 (comp flag) of the 7-byte channels at 16 + 12c.
    let mut flags = NO_FLAGS;
    flags[0].1 = 1; // channel 0 PCF
    flags[4].1 = 1; // channel 2 PCF
    flags[3].1 = 1; // channel 1 comp
    let s = patch_mixr(&s, &flags);
    let m = convert(&s).unwrap().mixer;
    assert_eq!(m.master, 90);
    // The last channel with a PCF flag wins (channel 2, id 3); channel 1's comp flag gives id 3.
    assert_eq!((m.comp_id, m.pcf_id), (3, 3));
    assert!(m.channels.iter().all(|c| c.enabled == 1));
    assert_eq!(
        m.channels[1],
        MixerChannel {
            enabled: 1,
            level: 11,
            pan: 21,
            delay_send: 31,
            dist: 0
        }
    );
    // No flags: comp id = (MIXR[1] != 0), PCF id 0.
    let mut s = song(0, 0);
    set_channels(&mut s);
    s.mixer.comp_id = 9;
    let mut s = patch_mixr(&s, &NO_FLAGS);
    let m = convert(&s).unwrap().mixer;
    assert_eq!((m.comp_id, m.pcf_id), (1, 0));
    s.mixer.comp_id = 0;
    assert_eq!(convert(&s).unwrap().mixer.comp_id, 0);
    // 4.0.1 is not "4.0": the 5-byte path, enables as stored.
    s.head.revision = 1;
    let m = convert(&s).unwrap().mixer;
    assert_eq!((m.comp_id, m.pcf_id, m.channels[0].enabled), (0, 3, 0));
}

#[test]
fn effect_blocks() {
    let mut s = song(2, 0);
    s.delay.values = vec![1, 3, 0, 80, 64];
    s.pcf.values = vec![1, 70, 64, 40, 24, 90, 1];
    s.dist.values = vec![1, 70, 5];
    s.comp.values = vec![1, 70, 64];
    let x = convert(&s).unwrap();
    assert_eq!(x.delay.payload(), [1, 3, 0, 80, 64, 0]);
    assert_eq!(x.pcf.payload(), [1, 70, 64, 40, 24, 90, 1, 0]);
    assert_eq!(x.dist.payload(), [1, 70, 5, 0]);
    assert_eq!(x.comp.payload(), [1, 70, 64, 127, 0]);
    s.comp.values.pop();
    assert!(matches!(convert(&s), Err(LoadError::Model(_))));
}

#[test]
fn tb303_block() {
    let mut s = song(2, 0);
    s.tb303[1].params = [1, 5, 64, 70, 80, 90, 100, 110, 1];
    let p = &mut s.tb303[1].patterns[31];
    p.shuffle = 1;
    p.length = 13;
    p.steps[15] = rbs::Step303 {
        note: 12,
        slide: 1,
        accent: 0,
        up: 1,
        down: 0,
        rest: 1,
        flags_hi: 7,
    };
    let t = &convert(&s).unwrap().tb303[1];
    let b = t.payload();
    assert_eq!(b.len(), 3148);
    assert_eq!(&b[..12], &[1, 5, 64, 70, 80, 90, 100, 110, 1, 0, 0, 0]);
    let o = 12 + 98 * 31;
    assert_eq!(&b[o..o + 2], &[1, 13]);
    assert_eq!(&b[o + 2 + 6 * 15..o + 98], &[12, 1, 0, 1, 0, 1]);
}

#[test]
fn tr808_block() {
    let mut s = song(1, 0);
    s.tr808.params[10] = 100;
    s.tr808.params[13] = 127;
    s.tr808.params[16] = 1;
    s.tr808.params[28] = 99;
    s.tr808.patterns[2].steps[3] = [2, 1, 0, 255, 0, 0, 0, 0, 0, 0, 0, 1];
    s.tr808.patterns[2].length = 9;
    let t = convert(&s).unwrap().tr808;
    let b = t.payload();
    assert_eq!(b.len(), 6240);
    assert_eq!((b[10], b[13], b[16]), (82, 95, 32));
    assert_eq!(&b[28..32], &[99, 0, 0, 0]);
    let o = 32 + 194 * 2;
    assert_eq!(b[o + 1], 9);
    assert_eq!(
        &b[o + 2 + 12 * 3..o + 2 + 12 * 4],
        &[1, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1]
    );
    // 4.2: the tunings are as stored.
    s.head.minor = 2;
    let t = convert(&s).unwrap().tr808;
    assert_eq!((t.params[10], t.params[13], t.params[16]), (100, 127, 1));
}

#[test]
fn tr909_block() {
    let mut s = song(2, 0);
    s.tr909.params[13] = 33;
    s.tr909.params[16] = 99;
    s.tr909.params[20] = 1;
    s.tr909.params[21] = 2;
    s.tr909.params[22] = 3;
    s.tr909.params[29] = 44;
    s.tr909.patterns[31].steps[15] = [2, 3, 1, 0, 0, 0, 0, 0, 0, 0, 0, 3];
    let t = convert(&s).unwrap().tr909;
    let b = t.payload();
    assert_eq!(b.len(), 6241);
    // MidTomDecay (16) is LowTomDecay (13); 20..22 keep the RBS order.
    assert_eq!((b[13], b[16]), (33, 33));
    assert_eq!(&b[20..23], &[1, 2, 3]);
    assert_eq!(&b[29..33], &[44, 0, 0, 0]);
    let o = 33 + 194 * 31 + 2 + 12 * 15;
    assert_eq!(&b[o..o + 12], &[2, 3, 1, 0, 0, 0, 0, 0, 0, 0, 0, 3]);
}

#[test]
fn ranges() {
    // docs/song_load.md section 5.
    let mut mixer = vec![(1, (0, 5)), (2, (0, 4))];
    for c in 0..4u8 {
        let b = 6 + 6 * c;
        mixer.extend([
            (b, (0, 127)),
            (b + 1, (0, 127)),
            (b + 2, (0, 127)),
            (b + 3, (0, 1)),
        ]);
    }
    let mut t303 = vec![(0, (0, 1)), (1, (0, 31)), (8, (0, 1))];
    t303.extend((2..=7).map(|c| (c, (0, 127))));
    let drums = |last: u8| {
        let mut v = vec![(0, (0, 1)), (1, (0, 31))];
        v.extend((2..=last).map(|c| (c, (0, 127))));
        v
    };
    let table: [Vec<(u8, (u8, u8))>; 9] = [
        mixer,
        t303.clone(),
        t303,
        drums(28),
        drums(29),
        vec![
            (0, (0, 1)),
            (1, (1, 32)),
            (2, (0, 1)),
            (3, (0, 127)),
            (4, (0, 127)),
        ],
        vec![(0, (0, 1)), (1, (0, 127)), (2, (0, 127))],
        vec![
            (0, (0, 1)),
            (1, (0, 127)),
            (2, (0, 127)),
            (3, (0, 127)),
            (4, (0, 127)),
            (5, (0, 127)),
            (6, (0, 1)),
        ],
        vec![(0, (0, 1)), (1, (0, 127)), (2, (0, 127))],
    ];
    for (t, want) in table.iter().enumerate() {
        for ctrl in 0..=255u8 {
            let w = want.iter().find(|x| x.0 == ctrl).map(|x| x.1);
            assert_eq!(control_range(t as u8, ctrl), w, "track {t} ctrl {ctrl}");
        }
    }
    for ctrl in 0..=255 {
        assert_eq!(control_range(9, ctrl), None);
    }
}

#[test]
fn ticks_are_pos_times_24() {
    let e = events(
        5,
        &[ev(0, 0, 1), ev(8, 1, 3), ev(32, 3, 80), ev(31999, 4, 64)],
        TrackReader::Plain,
    );
    assert_eq!(
        e,
        [
            te(0, 0, 1),
            te(192, 1, 3),
            te(768, 3, 80),
            te(767_976, 4, 64)
        ]
    );
}

#[test]
fn invalid_tick_dropped_without_advancing() {
    // pos 32005 is at tick 768120 >= 768000: dropped, and its delta is not added, so the next
    // event (delta 5) lands at position 15.
    let e = events(
        6,
        &[ev(10, 1, 1), ev(32005, 1, 2), ev(32010, 1, 3)],
        TrackReader::Plain,
    );
    assert_eq!(e, [te(240, 1, 1), te(360, 1, 3)]);
}

#[test]
fn tick_wraps_in_u32() {
    // (abs * 192) >> 3 in u32: 2^26 * 192 wraps to 0; 22369622 * 192 wraps to 128 -> tick 16,
    // not a multiple of 24.
    let e = events(
        8,
        &[ev(1 << 26, 1, 5), ev((1 << 26) + 1, 1, 6)],
        TrackReader::Plain,
    );
    assert_eq!(e, [te(0, 1, 5), te(24, 1, 6)]);
    let e = events(8, &[ev(22_369_622, 1, 5)], TrackReader::Plain);
    assert!(e.is_empty());
}

#[test]
fn controller_and_range_checks() {
    let e = events(
        5,
        &[
            ev(0, 1, 0),   // steps below 1
            ev(0, 1, 33),  // steps above 32
            ev(0, 1, 32),  // ok
            ev(0, 5, 0),   // no controller 5
            ev(0, 2, 2),   // mode above 1
            ev(1, 3, 127), // ok
        ],
        TrackReader::Plain,
    );
    assert_eq!(e, [te(0, 1, 32), te(24, 3, 127)]);
    // A dropped event with a valid tick still advances the position.
    let e = events(0, &[ev(4, 5, 0), ev(6, 6, 1)], TrackReader::Plain);
    assert_eq!(e, [te(144, 6, 1)]);
}

#[test]
fn old_808_tom_tune_before_range_check() {
    let evs = [
        ev(0, 10, 190),
        ev(0, 13, 192),
        ev(0, 16, 0),
        ev(0, 11, 190),
        ev(0, 12, 100),
    ];
    let e = events(3, &evs, TrackReader::Old808);
    assert_eq!(e, [te(0, 10, 127), te(0, 16, 32), te(0, 12, 100)]);
    let e = events(3, &evs, TrackReader::Plain);
    assert_eq!(e, [te(0, 16, 0), te(0, 12, 100)]);
    // The reader follows the file version.
    let mut s = song(1, 0);
    s.tracks[3].events = evs.to_vec();
    assert_eq!(convert(&s).unwrap().tracks[3][0], te(0, 10, 127));
    s.head.minor = 2;
    assert_eq!(convert(&s).unwrap().tracks[3][0], te(0, 16, 0));
}

#[test]
fn mixer_4_0_track_converter() {
    let e = events(
        0,
        &[
            ev(0, 10, 1),  // 303 #1 comp flag -> comp id 2
            ev(0, 18, 0),  // comp off at tick 0, not after the last comp event: dropped
            ev(1, 1, 0),   // master comp off at 24 > 0 -> comp id 0
            ev(1, 9, 0),   // PCF off at 24 > 0 -> PCF id 0
            ev(1, 33, 5),  // 909 PCF flag -> PCF id 4
            ev(1, 1, 3),   // master comp flag -> comp id 1
            ev(1, 34, 1),  // 909 comp flag -> comp id 5
            ev(2, 5, 100), // level 303 #1 -> 6
            ev(2, 8, 1),   // dist 303 #1 -> 9
            ev(2, 29, 50), // level 909 -> 24
            ev(2, 32, 1),  // dist 909 -> 27
            ev(2, 4, 1),   // old "enabled" -> 5: not a controller
            ev(2, 19, 64), // unchanged: 19 is the 808 pan
            ev(2, 3, 0),   // unchanged, not a controller
            ev(2, 0, 0),   // 0, not a controller
        ],
        TrackReader::Mixer40,
    );
    assert_eq!(
        e,
        [
            te(0, 1, 2),
            te(24, 1, 0),
            te(24, 2, 0),
            te(24, 2, 4),
            te(24, 1, 1),
            te(24, 1, 5),
            te(48, 6, 100),
            te(48, 9, 1),
            te(48, 24, 50),
            te(48, 27, 1),
            te(48, 19, 64),
        ]
    );
}

#[test]
fn mixer_4_0_trackers_see_dropped_ticks() {
    // The comp event at an invalid tick still sets the last comp tick (768024), so the later
    // comp off at tick 48 is dropped; the PCF tracker is separate.
    let e = events(
        0,
        &[
            ev(1, 10, 1),
            ev(32001, 1, 1),
            ev(32002, 1, 0),
            ev(32002, 9, 0),
        ],
        TrackReader::Mixer40,
    );
    assert_eq!(e, [te(24, 1, 2), te(48, 2, 0)]);
    // The 4.0 file uses this reader for track 0 only.
    let mut s = song(0, 0);
    s.tracks[0].events = vec![ev(0, 5, 100)];
    s.tracks[1].events = vec![ev(0, 5, 100)];
    let x = convert(&s).unwrap();
    assert_eq!(x.tracks[0], [te(0, 6, 100)]);
    assert_eq!(x.tracks[1], [te(0, 5, 100)]);
}

#[test]
fn batches_of_1284_file_events() {
    let mut evs: Vec<_> = (0..1300).map(|i| ev(i, 1, (i % 128) as u8)).collect();
    // Two dropped events in the first batch.
    evs[3].ctrl = 99;
    evs[1000].value = 200;
    let b = track_batches(7, &evs, TrackReader::Plain);
    assert_eq!(b.len(), 2);
    assert_eq!((b[0].track, b[0].first, b[0].last), (7, 0, 1282));
    assert_eq!(b[0].events.len(), 1282);
    assert_eq!((b[1].first, b[1].last), (1282, 1298));
    assert_eq!(b[1].events[0], te(1284 * 24, 1, (1284 % 128) as u8));
    // A batch without an accepted event is not sent; the counts go on.
    let mut evs: Vec<_> = (0..1284).map(|i| ev(i, 99, 0)).collect();
    evs.extend((0..3).map(|i| ev(2000 + i, 2, 1)));
    let b = track_batches(7, &evs, TrackReader::Plain);
    assert_eq!(b.len(), 1);
    assert_eq!((b[0].first, b[0].last), (0, 3));
    assert!(track_batches(7, &[], TrackReader::Plain).is_empty());
}

#[test]
fn batch_payload() {
    let b = TrackBatch {
        track: 4,
        first: 0x0102,
        last: 0x0104,
        events: vec![te(0x0001_0203, 7, 8), te(24, 9, 10)],
    };
    assert_eq!(
        b.payload(),
        [
            4, 0, 0, 1, 2, 0, 0, 1, 4, 99, 0, 1, 2, 3, 7, 8, 99, 0, 0, 0, 24, 9, 10
        ]
    );
}

#[test]
fn message_stream() {
    let s = song(1, 0);
    let x = load(&s).unwrap();
    let m = x.messages(5);
    assert_eq!(m[0][..3], [78, 5, 36]);
    assert_eq!(m[0].len(), 3 + 23);
    let dev: Vec<_> = m[1..10]
        .iter()
        .map(|m| (m[2], m[3], m[4], m.len()))
        .collect();
    assert_eq!(
        dev,
        [
            (41, 0, 143, 5 + 29),
            (41, 5, 86, 5 + 6),
            (41, 7, 116, 5 + 8),
            (41, 6, 94, 5 + 4),
            (41, 8, 104, 5 + 5),
            (41, 1, 39, 5 + 3148),
            (41, 2, 39, 5 + 3148),
            (41, 3, 56, 5 + 6240),
            (41, 4, 73, 5 + 6241),
        ]
    );
    // One batch per track (the song has a few events in every track), in track order.
    let tracks: Vec<_> = m[10..m.len() - 1].iter().map(|m| (m[2], m[3])).collect();
    assert_eq!(tracks, (0..9).map(|t| (37, t)).collect::<Vec<_>>());
    assert_eq!(m.last().unwrap(), &vec![78, 5, 32]);
    assert_eq!(load_messages(&s, 5).unwrap(), m);
    // The model's tracks are the batches' events.
    for t in 0..9 {
        assert_eq!(x.song.tracks[t], x.batches[t][0].events);
        assert_eq!(x.song.tracks[t].len(), s.tracks[t].events.len());
    }
    assert_eq!(x.song.mod_name, "Standard ReBirth");
}
