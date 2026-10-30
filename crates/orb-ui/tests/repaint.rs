//! The repaint rule (task-153, docs/ui_spec.md section 16): the window repaints at the meter
//! rate while the engine's side of the panel moves (`Link::moving`) and not at all otherwise.

use std::sync::Arc;
use std::time::Duration;

use eframe::egui::{self, Pos2, RawInput, Rect, Vec2};
use orb_engine::tables::Tables;
use orb_player::output::Health;
use orb_player::{Audio, CALL_FRAMES};
use orb_ui::app::App;
use orb_ui::layout::{PANEL_H, PANEL_W};
use orb_ui::link::Link;
use orb_ui::model::Panel;

/// Runs `n` engine calls of the audio callback.
fn calls(audio: &mut Audio, n: usize) {
    let mut out = vec![0i16; 2 * CALL_FRAMES];
    for _ in 0..n {
        audio.render_i16(&mut out);
    }
}

/// A link on the new empty song with the built-in kit, as File > New makes it, after two
/// engine calls.
fn new_song() -> (Link, Audio, Panel) {
    let samples = Some(rbm::standard::builtin_samples());
    let (mut link, mut audio) = Link::new(Arc::new(Tables::new()), samples.clone());
    let mut panel = Panel::new();
    link.load_new(samples, &mut panel);
    calls(&mut audio, 2);
    (link, audio, panel)
}

#[test]
fn the_link_moves_while_playing_and_until_the_engine_took_the_commands() {
    let (mut link, mut audio, mut panel) = new_song();
    link.sync(&mut panel);
    // the first look at the gain-reduction LEDs counts as a change
    link.moving(&panel);
    assert!(!link.moving(&panel), "stopped, silent, nothing sent");

    panel.press("transport.play", 0.0);
    link.sync(&mut panel);
    assert!(link.moving(&panel), "Play sent, not yet in the snapshot");
    calls(&mut audio, 2);
    link.sync(&mut panel);
    assert!(panel.transport.playing);
    assert!(link.moving(&panel), "playing");

    panel.press("transport.stop", 0.0);
    link.sync(&mut panel);
    calls(&mut audio, 2);
    link.sync(&mut panel);
    assert!(!panel.transport.playing);
    // the empty song is silent: no meter is lit once stopped
    link.moving(&panel);
    assert!(!link.moving(&panel), "stopped again");
}

/// The repaint the app asks for after a frame (Duration::MAX: none).
fn repaint_after(ctx: &egui::Context, app: &mut App, t: f64) -> Duration {
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(PANEL_W, PANEL_H))),
        time: Some(t),
        ..Default::default()
    };
    let mut out = ctx.run_ui(input, |ui| app.show(ui));
    out.textures_delta.clear();
    out.viewport_output
        .get(&egui::ViewportId::ROOT)
        .map_or(Duration::MAX, |v| v.repaint_delay)
}

#[test]
fn a_still_panel_asks_for_no_repaint() {
    let ctx = egui::Context::default();
    egui_extras::install_image_loaders(&ctx);
    let (link, _audio, panel) = new_song();
    // no audio stream (`App::audio_health` None): the snapshot never moves, so the panel
    // does not wait for it
    let mut app = App::with_link(panel, link, None);
    let mut last = Duration::ZERO;
    for i in 0..6 {
        last = repaint_after(&ctx, &mut app, i as f64);
    }
    assert!(last > Duration::from_secs(60), "{last:?}");
}

/// An app on `link` with a working audio stream (a healthy `Health`), whose callback the test
/// runs itself.
fn sounding(link: Link, panel: Panel) -> App {
    let mut app = App::with_link(panel, link, None);
    app.audio_health = Some(Health::started());
    app
}

#[test]
fn a_stopped_sounding_panel_asks_for_no_repaint_and_a_playing_one_does() {
    let ctx = egui::Context::default();
    egui_extras::install_image_loaders(&ctx);
    let (link, mut audio, panel) = new_song();
    let mut app = sounding(link, panel);
    let mut last = Duration::ZERO;
    for i in 0..6 {
        calls(&mut audio, 1);
        last = repaint_after(&ctx, &mut app, i as f64);
    }
    assert!(last > Duration::from_secs(60), "stopped: {last:?}");

    app.panel.press("transport.play", 6.0);
    // Play sent: the frame waits for the snapshot to show it
    let d = repaint_after(&ctx, &mut app, 6.0);
    assert_eq!(d, orb_ui::app::METER_FRAME, "sent");
    for i in 0..4 {
        calls(&mut audio, 1);
        let d = repaint_after(&ctx, &mut app, 7.0 + i as f64);
        assert_eq!(d, orb_ui::app::METER_FRAME, "playing");
    }
    assert!(app.panel.transport.playing);
}

#[test]
fn the_panel_follows_a_playing_song_frame_by_frame() {
    let ctx = egui::Context::default();
    egui_extras::install_image_loaders(&ctx);
    let (mut link, mut audio, mut panel) = new_song();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let song = orb_ui::session::load_song(&root.join("tests/songs/shipped/Trakka Rekka.rbs"))
        .expect("the song opens");
    link.load(&song, Some(rbm::standard::builtin_samples()), &mut panel)
        .unwrap();
    calls(&mut audio, 2);
    let mut app = sounding(link, panel);
    repaint_after(&ctx, &mut app, 0.0);
    app.panel.press("transport.play", 0.0);
    // a frame each 33 ms of audio (about 1.4 engine calls of 1024 frames), for 3 s
    let (mut positions, mut steps, mut lit) = (vec![], vec![], 0);
    let mut sent = 0.0;
    for f in 0..90 {
        let due = (f as f64 + 1.0) * 0.033 * 44100.0 / CALL_FRAMES as f64;
        while sent < due {
            calls(&mut audio, 1);
            sent += 1.0;
        }
        let d = repaint_after(&ctx, &mut app, f as f64 * 0.033);
        // (the first frames of a loaded song may ask egui for more: a new texture, the title)
        assert!(d <= orb_ui::app::METER_FRAME, "{d:?}");
        positions.push(app.panel.transport.position);
        steps.push(app.panel.playing_step909);
        lit += app.panel.meters.values().any(|&m| m > 0) as usize;
    }
    // the song position moves forward (song mode, no loop back in 3 s)
    assert!(positions.windows(2).all(|w| w[0] <= w[1]), "{positions:?}");
    assert!(positions.last() > positions.first());
    // the 909's running light takes each sixteenth: at 139 BPM, 9.3 a second
    let changes = steps.windows(2).filter(|w| w[0] != w[1]).count();
    assert!(changes >= 20, "{changes} changes: {steps:?}");
    assert!(lit > 60, "the meters show the sound in {lit} of 90 frames");
}

#[test]
fn a_lit_303_note_repaints_when_it_goes_out() {
    let ctx = egui::Context::default();
    egui_extras::install_image_loaders(&ctx);
    let mut app = App::new(Panel::new(), None);
    for i in 0..4 {
        repaint_after(&ctx, &mut app, i as f64);
    }
    // pitch mode on, then a key: the note's LED is lit for a moment
    app.panel.press("tb303_1.pitch_button", 4.0);
    app.panel.press("tb303_1.key4", 4.0);
    let t = app.panel.next_change(4.0).expect("the flash ends");
    assert!(t > 4.0);
    let d = repaint_after(&ctx, &mut app, 4.0);
    assert!(
        d <= Duration::from_secs_f64(t - 4.0) + Duration::from_millis(1),
        "{d:?}"
    );
}
