//! The Mods menu's mod change (task-056, `docs/rbm.md` section 14): song msg 19 swaps the drum
//! machines' sample table of the song in place.

mod common;

use common::{kit, manifest, run, samples, send, song, tables};
use rb_engine::song::Song;
use rb_player::{Audio, Command, Player};

fn diag(name: &str) -> Song {
    song(&manifest()["diag"][name])
}

fn peak(x: &[i16]) -> i32 {
    x.iter().map(|&s| (s as i32).abs()).max().unwrap_or(0)
}

/// `s` loaded with the built-in kit and the first call made.
fn player(s: &Song) -> (Player, Audio) {
    let (mut p, mut a) = Player::new(tables(), Some(samples()));
    assert!(p.load(s, Some(samples())).is_ok());
    run(&mut a, 1);
    (p, a)
}

/// On a new, stopped song the change is the same as building the song with the new mod: slot 6
/// on idle voices changes nothing, and the drums then play the new samples.
#[test]
fn a_mod_set_on_a_stopped_song_equals_building_with_it() {
    let s = diag("full_mix");
    let half = kit(|x| x / 2);
    let (mut p, mut a) = player(&s);
    assert!(p.set_mod(half.clone()).is_ok());
    send(&mut p, Command::Play);
    let changed = run(&mut a, 80);

    let (mut q, mut b) = Player::new(tables(), Some(samples()));
    assert!(q.load(&s, Some(half)).is_ok());
    run(&mut b, 1);
    send(&mut q, Command::Play);
    let built = run(&mut b, 80);
    assert_eq!(changed, built);

    // and the kit matters: the built-in one plays otherwise
    let (mut r, mut c) = player(&s);
    send(&mut r, Command::Play);
    assert_ne!(run(&mut c, 80), built);
}

/// On a playing song: the song plays on from where it is (the tick runs on as without the
/// change), the voices that sounded stop at once, and the drums play the new samples from the
/// next step on. The change hands back the old table and the voices' samples.
#[test]
fn a_mod_change_on_a_playing_song_keeps_its_place() {
    let s = diag("drums_kit");
    let (mut p, mut a) = player(&s);
    let (mut q, mut b) = player(&s);
    send(&mut p, Command::Play);
    send(&mut q, Command::Play);
    let x = run(&mut a, 40);
    assert_eq!(x, run(&mut b, 40));
    assert!(peak(&x) > 1000);

    // a silent kit: from the call that applies it on, the drums output nothing
    assert!(p.set_mod(kit(|_| 0)).is_ok());
    let mut ticks = Vec::new();
    let mut out = Vec::new();
    for _ in 0..40 {
        out.extend(run(&mut a, 1));
        run(&mut b, 1);
        let (sp, sq) = (*p.snapshot(), *q.snapshot());
        assert!(sp.playing && sp.generation == 1 && !sp.pending);
        ticks.push((sp.tick, sq.tick));
    }
    assert!(ticks.iter().all(|&(t, u)| t == u), "{ticks:?}");
    assert_eq!(peak(&out), 0, "the song has no effect that would ring on");

    // the command's box came back (with the old tables and the voices' samples in it: the
    // engine's unit tests check its contents)
    assert_eq!(p.collect_garbage(), 1);

    // back to the built-in kit: the drums sound again
    assert!(p.set_mod(samples()).is_ok());
    assert!(peak(&run(&mut a, 40)) > 1000);
    assert_eq!(p.snapshot().generation, 1);
}
