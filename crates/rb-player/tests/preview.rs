//! The preview player (task-074, `docs/export_state.md` section 8): top-level msg 58 plays a
//! mod's startup sound, mixed after the song in each call; msg 57 clears it.

mod common;

use std::sync::Arc;

use common::{manifest, run, samples, send, song, tables};
use rb_engine::samples::{STARTUP_ID, Sample};
use rb_player::{CALL_FRAMES, Command, Player, Preview, mix_preview};

/// `base` with the preview `audio` mixed into each of its calls from its start, as the
/// callback does.
fn mixed(base: &[i16], audio: &[i16]) -> Vec<i16> {
    let mut out = base.to_vec();
    let mut pos = 0;
    for c in out.chunks_mut(2 * CALL_FRAMES) {
        pos = mix_preview(c, audio, pos);
    }
    out
}

fn preview(v: &[i16]) -> Command {
    Command::Preview(Box::new(Preview::new(Arc::new(Sample::from_audio(v)))))
}

/// The built-in startup sound over the new empty song and over a playing song: the output is
/// the song's plus the preview, call by call, until the sample ends.
#[test]
fn the_startup_sound_is_mixed_after_the_song() {
    let s = samples();
    let audio = s.get(STARTUP_ID).audio().to_vec();
    assert!(
        audio.len() > CALL_FRAMES,
        "a startup sound longer than a call"
    );
    let calls = audio.len().div_ceil(CALL_FRAMES) + 3;
    let c = 2 * CALL_FRAMES;

    let (_, mut a) = Player::new(tables(), Some(s.clone()));
    let base = run(&mut a, 1 + calls);
    let (mut p, mut b) = Player::new(tables(), Some(s.clone()));
    let mut got = run(&mut b, 1);
    assert!(p.startup_sound(&s).is_ok());
    got.extend(run(&mut b, calls));
    assert_eq!(b.preview_position(), None, "cleared at the end");
    assert_eq!(got[..c], base[..c]);
    assert_eq!(got[c..], mixed(&base[c..], &audio));
    assert_ne!(got, base);

    let mix = song(&manifest()["diag"]["full_mix"]);
    let playing = |with_preview: bool| {
        let (mut p, mut a) = Player::new(tables(), Some(s.clone()));
        assert!(p.load(&mix, Some(s.clone())).is_ok());
        send(&mut p, Command::Play);
        let mut out = run(&mut a, 5);
        if with_preview {
            assert!(p.startup_sound(&s).is_ok());
        }
        out.extend(run(&mut a, calls));
        out
    };
    let base = playing(false);
    let got = playing(true);
    assert_eq!(got[..5 * c], base[..5 * c]);
    assert_eq!(got[5 * c..], mixed(&base[5 * c..], &audio));
}

/// Msg 58 again restarts from the start with the new sample; msg 57 clears it at once.
#[test]
fn a_new_preview_restarts_and_stop_clears() {
    let s = samples();
    let c = 2 * CALL_FRAMES;
    let (_, mut a) = Player::new(tables(), Some(s.clone()));
    let base = run(&mut a, 7);
    let one: Vec<i16> = (0..3 * CALL_FRAMES as i32)
        .map(|i| (i % 2000) as i16)
        .collect();
    let two: Vec<i16> = vec![-1000; 5000];
    let (mut p, mut b) = Player::new(tables(), Some(s.clone()));
    send(&mut p, preview(&one));
    let mut got = run(&mut b, 2);
    assert_eq!(b.preview_position(), Some(2 * CALL_FRAMES));
    // the second replaces the first, from its start; the first's box comes back
    send(&mut p, preview(&two));
    got.extend(run(&mut b, 2));
    assert_eq!(p.collect_garbage(), 1);
    assert_eq!(b.preview_position(), Some(2 * CALL_FRAMES));
    send(&mut p, Command::PreviewStop);
    got.extend(run(&mut b, 3));
    assert_eq!(b.preview_position(), None);

    let mut want = mixed(&base[..2 * c], &one);
    want.extend(mixed(&base[2 * c..4 * c], &two));
    want.extend_from_slice(&base[4 * c..]);
    assert_eq!(got, want);
}
