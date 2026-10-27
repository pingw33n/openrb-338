//! task-088: a song above the node pool (100000 events) is refused by `Player::build` and
//! `Player::load`, and nothing is queued (docs/song_load.md section 5.2).

mod common;

use common::{run, samples, tables};
use rb_engine::frame::LoadRefused;
use rb_player::{LoadError, Player};

/// A song of 101000 events on two tracks, generated (values alternate per position, so that no
/// stop prune frees nodes; as `a_file_above_the_pool_is_refused` in rb-engine).
fn big_song() -> rb_engine::song::Song {
    let events = |n: u32, ctrls: [u8; 2]| -> Vec<rbs::Event> {
        (0..n)
            .map(|i| rbs::Event {
                pos: i / 2,
                ctrl: ctrls[(i % 2) as usize],
                value: (i / 2 % 2) as u8 + 1,
            })
            .collect()
    };
    let mut f = rbs::Song::empty();
    f.tracks[5].events = events(60_000, [3, 4]);
    f.tracks[6].events = events(41_000, [1, 2]);
    rb_engine::load::convert(&f).unwrap()
}

#[test]
fn a_song_above_the_pool_is_refused_and_not_queued() {
    let s = big_song();
    assert!(s.tracks.iter().map(|t| t.len()).sum::<usize>() > 100_000);
    let (mut p, mut a) = Player::new(tables(), Some(samples()));
    run(&mut a, 2);
    let generation = p.snapshot().generation;
    let r = p.build(&s, Some(samples())).err().expect("refused");
    assert_eq!((r.track, r.first, r.count), (6, 31 * 1284, 41_000 % 1284));
    assert_eq!(
        LoadRefused::ALERT,
        "There is not enough memory for opening the Song."
    );
    match p.load(&s, Some(samples())) {
        Err(LoadError::Refused(r2)) => assert_eq!(r2, r),
        other => panic!("{other:?}"),
    }
    // nothing was queued: the callback keeps the song it had
    run(&mut a, 2);
    assert_eq!(p.snapshot().generation, generation);
}
