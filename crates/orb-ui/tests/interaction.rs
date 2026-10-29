//! The widgets' interaction and the art through egui, headless: synthetic pointer events are
//! run through `App::show` frame by frame on the panel at 1:1 (window 624 x 510 points,
//! so panel pixels are screen points), and the SVG parts are loaded and rasterized by
//! egui_extras' loader at 1x, 2x and 3x.

use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2};
use orb_ui::app::App;
use orb_ui::art;
use orb_ui::layout::{self, PANEL_H, PANEL_W};
use orb_ui::model::{BAR, Event as Out, Panel};

struct Harness {
    ctx: egui::Context,
    app: App,
    time: f64,
    pos: Pos2,
}

impl Harness {
    fn new() -> Self {
        let ctx = egui::Context::default();
        egui_extras::install_image_loaders(&ctx);
        let mut h = Self {
            ctx,
            app: App::new(Panel::new(), None),
            time: 0.0,
            pos: Pos2::ZERO,
        };
        h.frame(vec![]);
        h
    }

    fn frame(&mut self, events: Vec<Event>) {
        self.frame_mods(events, Modifiers::NONE);
    }

    fn frame_mods(&mut self, mut events: Vec<Event>, modifiers: Modifiers) {
        self.time += 0.016;
        events.insert(0, Event::ModifiersChanged(modifiers));
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(PANEL_W, PANEL_H))),
            time: Some(self.time),
            events,
            ..Default::default()
        };
        let app = &mut self.app;
        let mut out = self.ctx.run_ui(input, |ui| app.show(ui));
        out.textures_delta.clear();
    }

    /// The center of a control, panel pixels = points.
    fn center(id: &str) -> Pos2 {
        // `tb303_N.accent` names both a knob and a button: the button is meant here only
        // via `center_of`.
        let c = layout::controls().into_iter().find(|c| c.id == id).unwrap();
        Pos2::new(c.x + c.w / 2.0, c.y + c.h / 2.0)
    }

    fn center_of(kind: &str, id: &str) -> Pos2 {
        let c = layout::controls()
            .into_iter()
            .find(|c| c.id == id && c.kind == kind)
            .unwrap();
        Pos2::new(c.x + c.w / 2.0, c.y + c.h / 2.0)
    }

    fn move_to(&mut self, p: Pos2) {
        self.pos = p;
        self.frame(vec![Event::PointerMoved(p)]);
    }

    fn down(&mut self, mods: Modifiers) {
        let pos = self.pos;
        self.frame_mods(
            vec![Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: mods,
            }],
            mods,
        );
    }

    fn up(&mut self) {
        let pos = self.pos;
        self.frame(vec![Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }]);
    }

    fn click(&mut self, id: &str) {
        self.move_to(Self::center(id));
        self.down(Modifiers::NONE);
        self.up();
    }
}

#[test]
fn knob_drag_up_increases_and_down_decreases() {
    let mut h = Harness::new();
    h.app.panel.set_knob("tb303_1.cutoff", 0);
    h.move_to(Harness::center("tb303_1.cutoff"));
    h.down(Modifiers::NONE);
    // Mouse down grabs without a jump.
    assert_eq!(h.app.panel.knob("tb303_1.cutoff"), 0);
    let start = h.pos;
    h.move_to(start - Vec2::new(0.0, 31.0));
    assert_eq!(h.app.panel.knob("tb303_1.cutoff"), 500);
    // Horizontal motion is ignored.
    h.move_to(start - Vec2::new(-20.0, 31.0));
    assert_eq!(h.app.panel.knob("tb303_1.cutoff"), 500);
    h.move_to(start - Vec2::new(0.0, 15.5));
    assert_eq!(h.app.panel.knob("tb303_1.cutoff"), 250);
    h.up();
    h.move_to(start - Vec2::new(0.0, 100.0));
    assert_eq!(
        h.app.panel.knob("tb303_1.cutoff"),
        250,
        "released knobs stay"
    );
}

#[test]
fn knob_with_shift_at_mouse_down_is_five_times_finer() {
    let mut h = Harness::new();
    h.app.panel.set_knob("tb303_1.reso", 0);
    h.move_to(Harness::center("tb303_1.reso"));
    let start = h.pos;
    h.down(Modifiers::SHIFT);
    h.move_to(start - Vec2::new(0.0, 31.0));
    assert_eq!(h.app.panel.knob("tb303_1.reso"), 100);
    let sent = h.app.panel.events.iter().rev().find_map(|e| match e {
        Out::Set { id, value } if id == "tb303_1.reso" => Some(*value),
        _ => None,
    });
    assert_eq!(sent, Some(13));
}

#[test]
fn fader_jumps_to_the_pointer_and_drags() {
    let mut h = Harness::new();
    h.app.panel.set_fader("master.fader", 0);
    // Track top is y = 75: pressing near the top jumps the handle's center there.
    h.move_to(Pos2::new(566.5, 75.0 + 6.0));
    h.down(Modifiers::NONE);
    assert!(h.app.panel.fader("master.fader") >= 990);
    h.up();
    // Grab the handle (now at the top) and drag down 21.5 px: about half.
    h.move_to(Pos2::new(566.5, 75.0 + 5.0));
    h.down(Modifiers::NONE);
    let p0 = h.app.panel.fader("master.fader");
    h.move_to(Pos2::new(566.5, 75.0 + 5.0 + 21.5));
    let p = h.app.panel.fader("master.fader");
    assert!(p < p0 && (450..=550).contains(&p), "p0 {p0} p {p}");
    h.up();
}

#[test]
fn two_position_switch_flips_on_a_click_anywhere() {
    let mut h = Harness::new();
    // The 303 waveform switch is horizontal, 16 x 12 at (119, 71). A click on the half
    // that is already selected flips it too.
    let left = Pos2::new(119.0 + 3.0, 77.0);
    h.move_to(left);
    h.down(Modifiers::NONE);
    assert_eq!(h.app.panel.switch("tb303_1.wave"), 1);
    // Dragging does not flip it back.
    h.move_to(Pos2::new(119.0 + 13.0, 77.0));
    assert_eq!(h.app.panel.switch("tb303_1.wave"), 1);
    h.up();
    h.move_to(left);
    h.down(Modifiers::NONE);
    h.up();
    assert_eq!(h.app.panel.switch("tb303_1.wave"), 0);
    // The delay's triplet switch is vertical: a click on its top half flips it.
    h.move_to(Pos2::new(579.0, 281.0 + 3.0));
    h.down(Modifiers::NONE);
    h.up();
    assert_eq!(h.app.panel.switch("delay.triplet"), 1);
}

#[test]
fn toggles_flip_at_mouse_down_and_triggers_fire_once() {
    let mut h = Harness::new();
    let c = Harness::center("transport.loop");
    h.move_to(c);
    h.down(Modifiers::NONE);
    assert!(h.app.panel.transport.looping, "toggle acts at mouse down");
    h.up();
    h.click("transport.loop");
    assert!(!h.app.panel.transport.looping);
    h.click("transport.play");
    assert!(h.app.panel.transport.playing);
    // The mode switch is a toggle too.
    h.click("transport.mode");
    assert!(h.app.panel.transport.song_mode);
    h.app.panel.events.clear();
    h.move_to(Harness::center_of("trigger", "tb303_1.accent"));
    h.down(Modifiers::NONE);
    h.up();
    assert_eq!(h.app.panel.events.len(), 1);
    assert!(h.app.panel.synth[0].step(0).accent);
}

#[test]
fn repeat_button_steps_once_then_repeats_while_held() {
    let mut h = Harness::new();
    h.move_to(Harness::center("transport.tempo_up"));
    h.down(Modifiers::NONE);
    assert_eq!(h.app.panel.transport.tempo_display(), 141);
    // Held for 0.3 s: still one step (repeat starts at 400 ms).
    for _ in 0..18 {
        h.frame(vec![]);
    }
    assert_eq!(h.app.panel.transport.tempo_display(), 141);
    // Held past 1.4 s: 1 + 10 steps.
    h.time += 1.1;
    h.frame(vec![]);
    let t = h.app.panel.transport.tempo_display();
    assert!((150..=153).contains(&t), "tempo {t}");
    h.up();
    let after = h.app.panel.transport.tempo_display();
    h.time += 2.0;
    h.frame(vec![]);
    assert_eq!(h.app.panel.transport.tempo_display(), after);
    h.click("transport.tempo_down");
    assert_eq!(h.app.panel.transport.tempo_display(), after - 1);
}

#[test]
fn bar_arrows_and_wind_move_the_song_position() {
    let mut h = Harness::new();
    // pattern mode: no effect (the song-mode block of `sub_42A058`, 0x42A074)
    h.click("transport.ff");
    assert_eq!(h.app.panel.transport.position, 0);
    h.click("transport.mode");
    h.click("transport.ff");
    assert_eq!(h.app.panel.transport.position, 10 * BAR);
    h.click("transport.bar_up");
    assert_eq!(h.app.panel.transport.position, 11 * BAR);
    h.click("transport.rewind");
    assert_eq!(h.app.panel.transport.position, BAR);
}

#[test]
fn pattern_buttons_select_and_give_the_row_the_focus() {
    let mut h = Harness::new();
    assert_eq!(h.app.panel.focus, 0);
    h.click("pattern3.pattern6");
    assert_eq!(h.app.panel.rows[2].pattern, 5);
    assert_eq!(h.app.panel.focus, 2);
    h.click("pattern3.bankB");
    assert_eq!(h.app.panel.rows[2].bank, 1);
    assert_eq!(
        h.app.panel.rows[2].pattern, 5,
        "a bank click changes no pattern"
    );
    h.click("pattern3.pattern1");
    assert_eq!(h.app.panel.rows[2].pattern, 8);
    // A click anywhere in a row (here a drum machine knob area) moves the focus.
    h.click("tr909.bd_tune");
    assert_eq!(h.app.panel.focus, 3);
}

#[test]
fn tr808_steps_paint_across_a_drag_and_erase_from_a_lit_step() {
    let mut h = Harness::new();
    assert_eq!(h.app.panel.inst808, 1);
    // Steps 1-4 at x = 106 + 16 i, y = 371 + 8.
    h.move_to(Pos2::new(106.0 + 8.0, 379.0));
    h.down(Modifiers::NONE);
    h.move_to(Pos2::new(106.0 + 16.0 * 3.0 + 8.0, 379.0));
    h.up();
    assert_eq!(
        &h.app.panel.steps808[1][..5],
        &[true, true, true, true, false]
    );
    // Starting on a lit step erases every step crossed.
    h.move_to(Pos2::new(106.0 + 16.0 * 3.0 + 8.0, 379.0));
    h.down(Modifiers::NONE);
    h.move_to(Pos2::new(106.0 + 16.0 + 8.0, 379.0));
    h.up();
    assert_eq!(
        &h.app.panel.steps808[1][..5],
        &[true, false, false, false, false]
    );
}

#[test]
fn tr909_step_click_cycles_and_modifiers_pick_the_value() {
    let mut h = Harness::new();
    let step = |i: usize| Pos2::new(119.0 + 21.0 * i as f32 + 10.0, 481.0 + 8.0);
    h.move_to(step(0));
    h.down(Modifiers::NONE);
    h.up();
    assert_eq!(h.app.panel.steps909[1][0], 1);
    h.down(Modifiers::NONE);
    h.up();
    assert_eq!(h.app.panel.steps909[1][0], 2);
    h.down(Modifiers::NONE);
    h.up();
    assert_eq!(h.app.panel.steps909[1][0], 0);
    h.move_to(step(2));
    h.down(Modifiers::CTRL | Modifiers::ALT);
    h.up();
    assert_eq!(h.app.panel.steps909[1][2], 3);
    // A drag sets every crossed step to the new value.
    h.move_to(step(5));
    h.down(Modifiers::NONE);
    h.move_to(step(8));
    h.up();
    assert_eq!(&h.app.panel.steps909[1][5..10], &[1, 1, 1, 1, 0]);
}

#[test]
fn instrument_legends_select_and_drag_keeps_selecting() {
    let mut h = Harness::new();
    // The 808 legend: 12 cells of 22 px from x 100, y 351..365.
    h.move_to(Pos2::new(100.0 + 22.0 * 9.0 + 5.0, 358.0));
    h.down(Modifiers::NONE);
    assert_eq!(h.app.panel.inst808, 9);
    h.move_to(Pos2::new(100.0 + 22.0 * 11.0 + 5.0, 358.0));
    assert_eq!(h.app.panel.inst808, 11);
    h.up();
    // The 909 legend, AC included; the area under AC's level knob no longer selects.
    h.move_to(Pos2::new(90.0 + 10.0, 395.0 + 50.0));
    h.down(Modifiers::NONE);
    h.up();
    assert_eq!(h.app.panel.inst909, 1);
    h.move_to(Pos2::new(90.0 + 10.0, 395.0 + 10.0));
    h.down(Modifiers::NONE);
    h.up();
    assert_eq!(h.app.panel.inst909, 0);
    h.move_to(Pos2::new(90.0 + 26.0 + 42.0 * 2.0 + 5.0, 395.0 + 10.0));
    h.down(Modifiers::NONE);
    h.up();
    assert_eq!(h.app.panel.inst909, 3);
}

#[test]
fn space_toggles_play_and_arrows_move_the_focus() {
    let mut h = Harness::new();
    // (a key goes down and up: egui marks a second press of a key still down as a repeat,
    // which the original ignores)
    let key = |k, pressed| Event::Key {
        key: k,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: Modifiers::NONE,
    };
    let tap = |h: &mut Harness, k| {
        h.frame(vec![key(k, true)]);
        h.frame(vec![key(k, false)]);
    };
    tap(&mut h, egui::Key::Space);
    assert!(h.app.panel.transport.playing);
    tap(&mut h, egui::Key::Space);
    assert!(!h.app.panel.transport.playing);
    tap(&mut h, egui::Key::ArrowUp);
    assert_eq!(h.app.panel.focus, 3);
    tap(&mut h, egui::Key::ArrowDown);
    assert_eq!(h.app.panel.focus, 0);
}

/// Every part loads through egui_extras' SVG loader and is rasterized at the displayed
/// physical pixel size, at 1x, 2x and 3x (the check task-042 could not make).
#[test]
fn every_part_loads_and_rasterizes_at_1x_2x_3x() {
    for ppp in [1.0_f32, 2.0, 3.0] {
        let ctx = egui::Context::default();
        egui_extras::install_image_loaders(&ctx);
        ctx.set_pixels_per_point(ppp);
        let mut count = 0;
        for _ in 0..2 {
            let input = RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(100.0, 100.0))),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                count = 0;
                for part in art::all() {
                    let (w, h) = svg_size(part.svg);
                    let size = Vec2::new(w, h);
                    let image = egui::Image::from_bytes(part.uri, part.svg).fit_to_exact_size(size);
                    let poll = image
                        .load_for_size(ui.ctx(), size)
                        .unwrap_or_else(|e| panic!("{} at {ppp}x: {e}", part.name));
                    let id = poll
                        .texture_id()
                        .unwrap_or_else(|| panic!("{}: pending", part.name));
                    let meta = ui.ctx().tex_manager().read().meta(id).map(|m| m.size);
                    let want = [(w * ppp).round() as usize, (h * ppp).round() as usize];
                    assert_eq!(meta, Some(want), "{} at {ppp}x", part.name);
                    count += 1;
                }
            });
            out.textures_delta.clear();
        }
        assert!(count >= 160, "{count} parts");
    }
}

fn svg_size(svg: &[u8]) -> (f32, f32) {
    let s = std::str::from_utf8(svg).unwrap();
    let attr = |name: &str| -> f32 {
        let key = format!(" {name}=\"");
        let i = s.find(&key).unwrap() + key.len();
        s[i..i + s[i..].find('"').unwrap()].parse().unwrap()
    };
    (attr("width"), attr("height"))
}

#[test]
fn the_window_draws_at_any_size_and_density() {
    for (w, h, ppp) in [
        (312.0, 255.0, 2.0),
        (1000.0, 600.0, 1.0),
        (700.0, 900.0, 3.0),
    ] {
        let ctx = egui::Context::default();
        egui_extras::install_image_loaders(&ctx);
        ctx.set_pixels_per_point(ppp);
        let mut app = App::new(Panel::demo(), None);
        // The first pass of a headless context has no screen size yet.
        let warmup = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(w, h))),
            ..Default::default()
        };
        let mut out = ctx.run_ui(warmup, |_| {});
        out.textures_delta.clear();
        for _ in 0..2 {
            let input = RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(w, h))),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| app.show(ui));
            assert!(!out.shapes.is_empty());
            out.textures_delta.clear();
        }
    }
}

#[test]
fn view_keeps_the_aspect_ratio_and_letterboxes() {
    use orb_ui::app::View;
    let wide = View::fit(
        Rect::from_min_size(Pos2::ZERO, Vec2::new(1248.0, 510.0)),
        1.0,
    );
    assert_eq!(wide.scale, 1.0);
    assert_eq!(wide.origin, Pos2::new(312.0, 0.0));
    let tall = View::fit(
        Rect::from_min_size(Pos2::ZERO, Vec2::new(312.0, 600.0)),
        1.0,
    );
    assert_eq!(tall.scale, 0.5);
    assert_eq!(tall.origin.x, 0.0);
    assert_eq!(tall.origin.y, ((600.0_f32 - 255.0) / 2.0).round());
    let p = wide.to_panel(wide.origin + Vec2::new(100.0, 50.0));
    assert_eq!(p, Pos2::new(100.0, 50.0));
}
