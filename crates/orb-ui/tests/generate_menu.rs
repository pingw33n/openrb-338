//! The GENERATE menu of the bass synths (`generate.rs`), headless: synthetic pointer and key
//! events run through `App::show` on the panel at 1:1 (panel pixels are screen points).

use eframe::egui::{self, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2};
use orb_ui::app::App;
use eframe::egui::{MouseWheelSource, MouseWheelUnit, TouchPhase};
use orb_ui::generate::{self, CELL_H, COL_W, Field, LIST_ROWS, Op, ROWS};
use orb_ui::scale::{KEYS, SCALES};
use orb_ui::layout::{self, PANEL_H, PANEL_W};
use orb_ui::model::Panel;

struct Harness {
    ctx: egui::Context,
    app: App,
    time: f64,
}

impl Harness {
    fn new() -> Self {
        let ctx = egui::Context::default();
        egui_extras::install_image_loaders(&ctx);
        let mut h = Self {
            ctx,
            app: App::new(Panel::new(), None),
            time: 0.0,
        };
        h.frame(vec![]);
        h
    }

    fn frame(&mut self, events: Vec<Event>) {
        self.time += 0.016;
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

    fn click_at(&mut self, p: Pos2) {
        self.frame(vec![Event::PointerMoved(p)]);
        for pressed in [true, false] {
            self.frame(vec![Event::PointerButton {
                pos: p,
                button: PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            }]);
        }
    }

    /// The mouse wheel over `p`, `lines` lines down (negative: up), then frames for egui's
    /// smooth scrolling to pass it all on.
    fn wheel_at(&mut self, p: Pos2, lines: f32) {
        self.frame(vec![Event::PointerMoved(p)]);
        self.frame(vec![Event::MouseWheel {
            unit: MouseWheelUnit::Point,
            delta: Vec2::new(0.0, -lines * CELL_H),
            phase: TouchPhase::Move,
            source: MouseWheelSource::Wheel,
            modifiers: Modifiers::NONE,
        }]);
        for _ in 0..60 {
            self.frame(vec![]);
        }
    }

    /// A click in the middle of a field of row 1.
    fn click_field(&mut self, synth: usize, field: Field) {
        let (x, y, w, h) = self.app.field_rect(synth, field).unwrap();
        self.click_at(Pos2::new(x + w / 2.0, y + h / 2.0));
    }

    /// A click on the open list's visible row `row` (0 is the top), left of the slider.
    fn click_list_row(&mut self, row: usize) {
        let (synth, field) = self.app.generate_list().unwrap();
        let (x, y, ..) = self.app.list_rect(synth, field).unwrap();
        self.click_at(Pos2::new(x + 8.0, y + CELL_H * (row as f32 + 0.5)));
    }

    fn key(&mut self, key: Key) {
        for pressed in [true, false] {
            self.frame(vec![Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: Modifiers::NONE,
            }]);
        }
    }
}

fn button(synth: usize) -> Pos2 {
    let id = format!("tb303_{}.generate", synth + 1);
    let c = layout::controls().into_iter().find(|c| c.id == id).unwrap();
    Pos2::new(c.x + c.w / 2.0, c.y + c.h / 2.0)
}

#[test]
fn the_button_opens_and_closes_its_synths_menu() {
    let mut h = Harness::new();
    assert_eq!(h.app.generate_open(), None);
    h.click_at(button(0));
    assert_eq!(h.app.generate_open(), Some(0));
    h.click_at(button(0));
    assert_eq!(h.app.generate_open(), None);
    // the lower synth has its own menu, and opening it closes the upper one's
    h.click_at(button(0));
    h.click_at(button(1));
    assert_eq!(h.app.generate_open(), Some(1));
}

#[test]
fn escape_and_a_click_outside_close_the_menu_and_a_click_inside_does_not() {
    let mut h = Harness::new();
    h.click_at(button(0));
    let (x, y, w, h_) = h.app.generate_rect(0).unwrap();
    h.click_at(Pos2::new(x + w / 2.0, y + h_ / 2.0));
    assert_eq!(h.app.generate_open(), Some(0));
    h.click_at(Pos2::new(20.0, 20.0));
    assert_eq!(h.app.generate_open(), None);
    h.click_at(button(0));
    h.key(Key::Escape);
    assert_eq!(h.app.generate_open(), None);
}

#[test]
fn the_menu_is_under_its_button_inside_the_panel_with_column_b_twice_as_wide() {
    assert_eq!(ROWS, ["SCALE/KEY", "RANDOM", "ALTER", "MOVE"]);
    assert_eq!(COL_W[1], 2.0 * COL_W[0]);
    let h = Harness::new();
    for synth in 0..2 {
        let (x, y, w, h_) = h.app.generate_rect(synth).unwrap();
        assert_eq!((w, h_), (COL_W[0] + COL_W[1], CELL_H * ROWS.len() as f32));
        let b = button(synth);
        assert!(y > b.y, "the menu is under the button");
        assert!(x >= 0.0 && y >= 0.0 && x + w <= PANEL_W && y + h_ <= PANEL_H);
    }
}

fn scale(name: &str) -> usize {
    SCALES.iter().position(|(n, _)| *n == name).unwrap()
}

fn key(name: &str) -> usize {
    KEYS.iter().position(|n| *n == name).unwrap()
}

#[test]
fn baze_1_chooses_a_scale_then_a_key_and_its_keys_glow() {
    let mut h = Harness::new();
    h.click_at(button(0));
    // the key note waits for the scale
    h.click_field(0, Field::Key);
    assert_eq!(h.app.generate_list(), None);
    h.click_field(0, Field::Scale);
    assert_eq!(h.app.generate_list(), Some((0, Field::Scale)));
    // a second press on the field closes its list, a third opens it again
    h.click_field(0, Field::Scale);
    assert_eq!(h.app.generate_list(), None);
    h.click_field(0, Field::Scale);
    h.click_list_row(1);
    assert_eq!(h.app.generate_list(), None);
    assert_eq!(h.app.generate_open(), Some(0), "choosing keeps the menu open");
    assert_eq!(h.app.scale_keys().get(0).scale, Some(scale("MINOR")));
    assert!(!generate::glows(h.app.scale_keys(), "tb303_1.key0"));
    // SCALE/KEY turns green only with the key note too
    assert!(!h.app.row_done(0, 0));
    h.click_field(0, Field::Key);
    assert_eq!(h.app.generate_list(), Some((0, Field::Key)));
    // A is the tenth key note: the wheel brings it into view
    let (x, y, w, _) = h.app.list_rect(0, Field::Key).unwrap();
    h.wheel_at(Pos2::new(x + w / 2.0, y + 10.0), 7.0);
    h.click_list_row(key("A") - (KEYS.len() - LIST_ROWS));
    assert_eq!(h.app.scale_keys().get(0).key, Some(key("A")));
    // the row is complete: SCALE/KEY is green on BAZE 1, not on BAZE 2; the other rows stay red
    assert!(h.app.row_done(0, 0));
    assert!(!h.app.row_done(1, 0));
    assert!((1..ROWS.len()).all(|row| !h.app.row_done(0, row)));
    // A minor: the white keys glow, the black ones and the LEDs do not; BAZE 2's do not
    let sk = h.app.scale_keys();
    for n in [0, 2, 4, 5, 7, 9, 11, 12] {
        assert!(generate::glows(sk, &format!("tb303_1.key{n}")), "key{n}");
    }
    for n in [1, 3, 6, 8, 10] {
        assert!(!generate::glows(sk, &format!("tb303_1.key{n}")), "key{n}");
    }
    assert!(!generate::glows(sk, "tb303_1.key0_led"));
    assert!(!generate::glows(sk, "tb303_2.key0"));
}

#[test]
fn baze_2_waits_for_baze_1_and_lists_only_what_fits() {
    let mut h = Harness::new();
    h.click_at(button(1));
    h.click_field(1, Field::Scale);
    assert_eq!(h.app.generate_list(), None, "nothing to offer before BAZE 1");
    // BAZE 1: C major
    h.click_at(button(0));
    h.click_field(0, Field::Scale);
    h.click_list_row(0);
    h.click_field(0, Field::Key);
    h.click_list_row(0);
    assert_eq!(h.app.scale_keys().get(0).mask().unwrap(), 0b1010_1011_0101);
    // BAZE 2: its list has the 9 scales that fit C major, slider and all
    h.click_at(button(1));
    h.click_field(1, Field::Scale);
    let (_, _, _, lh) = h.app.list_rect(1, Field::Scale).unwrap();
    assert_eq!(lh, CELL_H * LIST_ROWS as f32);
    // the second item is MINOR, whose only fitting key is A
    h.click_list_row(1);
    assert_eq!(h.app.scale_keys().get(1).scale, Some(scale("MINOR")));
    h.click_field(1, Field::Key);
    let (_, _, _, lh) = h.app.list_rect(1, Field::Key).unwrap();
    assert_eq!(lh, CELL_H, "one key note fits");
    h.click_list_row(0);
    assert_eq!(h.app.scale_keys().get(1).key, Some(key("A")));
    assert!(generate::glows(h.app.scale_keys(), "tb303_2.key9"));
    // a list's Escape closes the list only, the next closes the menu
    h.click_field(1, Field::Scale);
    h.key(Key::Escape);
    assert_eq!((h.app.generate_list(), h.app.generate_open()), (None, Some(1)));
    h.key(Key::Escape);
    assert_eq!(h.app.generate_open(), None);
}

/// The middle of RANDOM's name, or of the RANDOM row's button `param`.
fn random_label(h: &Harness, synth: usize) -> Pos2 {
    op_label(h, synth, Op::Random)
}

fn random_button(h: &Harness, synth: usize, param: usize) -> Pos2 {
    op_button(h, synth, Op::Random, param)
}

/// The middle of a row's name, or of the row's button `param`.
fn op_label(h: &Harness, synth: usize, op: Op) -> Pos2 {
    let (x, y, w, hh) = h.app.op_label_rect(synth, op).unwrap();
    Pos2::new(x + w / 2.0, y + hh / 2.0)
}

fn op_button(h: &Harness, synth: usize, op: Op, param: usize) -> Pos2 {
    let (x, y, w, hh) = h.app.op_button_rect(synth, op, param).unwrap();
    Pos2::new(x + w / 2.0, y + hh / 2.0)
}

#[test]
fn random_chooses_its_fields_while_blinking_then_randomizes_the_pattern() {
    use orb_ui::random::{ACCENT, NOTE, SLIDE};
    let mut h = Harness::new();
    h.app.set_random_seed(5);
    h.click_at(button(0));
    let before = h.app.panel.pattern_303(0);
    // the buttons do nothing until RANDOM is pressed
    let p = random_button(&h, 0, ACCENT);
    h.click_at(p);
    assert_eq!(h.app.op_selection(0, Op::Random), [false; 5]);
    let l = random_label(&h, 0);
    h.click_at(l);
    assert_eq!(h.app.op_picking(), Some((0, Op::Random)));
    // NOTE waits for the scale and key
    let p = random_button(&h, 0, NOTE);
    h.click_at(p);
    assert!(!h.app.op_selection(0, Op::Random)[NOTE]);
    let p = random_button(&h, 0, ACCENT);
    h.click_at(p);
    let p = random_button(&h, 0, SLIDE);
    h.click_at(p);
    // a second click turns a field off again
    h.click_at(p);
    h.click_at(p);
    assert_eq!(h.app.op_selection(0, Op::Random), [false, false, true, true, false]);
    assert!(!h.app.row_done(0, 1));
    h.click_at(l);
    assert_eq!(h.app.op_picking(), None);
    assert!(h.app.row_done(0, 1), "RANDOM is green after a run");
    let after = h.app.panel.pattern_303(0);
    assert_ne!(after, before);
    // only the chosen fields changed (a cleared pattern: rests on low C)
    for (a, b) in after.iter().zip(before.iter()) {
        assert_eq!((a.note, a.rest, a.up, a.down), (b.note, b.rest, b.up, b.down));
    }
    // the engine is told every change
    assert!(h.app.panel.events.iter().any(|e| matches!(
        e,
        orb_ui::model::Event::Step303 { synth: 0, field: 2, .. }
    )));
    // the choice is kept for the next time; Escape stops choosing without a run
    h.click_at(l);
    assert_eq!(h.app.op_selection(0, Op::Random), [false, false, true, true, false]);
    let kept = h.app.panel.pattern_303(0);
    h.key(Key::Escape);
    assert_eq!((h.app.op_picking(), h.app.generate_open()), (None, None));
    assert_eq!(h.app.panel.pattern_303(0), kept);
    // BAZE 2's pattern was left alone
    assert_eq!(h.app.panel.pattern_303(1), orb_ui::model::Panel::new().pattern_303(1));
}

#[test]
fn random_notes_follow_the_scale_and_key() {
    use orb_ui::random::NOTE;
    let mut h = Harness::new();
    h.app.set_random_seed(9);
    h.click_at(button(0));
    // C major pentatonic: C D E G A
    h.click_field(0, Field::Scale);
    let (x, y, w, _) = h.app.list_rect(0, Field::Scale).unwrap();
    h.wheel_at(Pos2::new(x + w / 2.0, y + 10.0), 8.0);
    h.click_list_row(scale("MAJ PENTA") - (SCALES.len() - LIST_ROWS));
    h.click_field(0, Field::Key);
    h.click_list_row(0);
    let mask = h.app.scale_keys().get(0).mask().unwrap();
    let l = random_label(&h, 0);
    h.click_at(l);
    let p = random_button(&h, 0, NOTE);
    h.click_at(p);
    h.click_at(l);
    let steps = h.app.panel.pattern_303(0);
    assert!(steps.iter().any(|s| !s.rest), "some steps became notes");
    for s in steps {
        assert!(orb_ui::scale::key_in(mask, s.note as usize), "note {}", s.note);
    }
}

#[test]
fn alter_keeps_its_own_choice_and_moves_what_random_made() {
    use orb_ui::random::{ACCENT, NOTE};
    let mut h = Harness::new();
    h.app.set_random_seed(13);
    h.click_at(button(0));
    // C major, then RANDOM makes notes and accents
    h.click_field(0, Field::Scale);
    h.click_list_row(0);
    h.click_field(0, Field::Key);
    h.click_list_row(0);
    let l = random_label(&h, 0);
    h.click_at(l);
    for param in [ACCENT, NOTE] {
        let p = random_button(&h, 0, param);
        h.click_at(p);
    }
    h.click_at(l);
    let made = h.app.panel.pattern_303(0);
    // ALTER starts with nothing chosen: RANDOM's choice is its own
    let a = op_label(&h, 0, Op::Alter);
    h.click_at(a);
    assert_eq!(h.app.op_picking(), Some((0, Op::Alter)));
    assert_eq!(h.app.op_selection(0, Op::Alter), [false; 5]);
    // pressing a RANDOM button while ALTER is chosen does nothing
    let p = random_button(&h, 0, ACCENT);
    h.click_at(p);
    assert!(h.app.op_selection(0, Op::Random)[ACCENT]);
    assert_eq!(h.app.op_selection(0, Op::Alter), [false; 5]);
    for param in [ACCENT, NOTE] {
        let p = op_button(&h, 0, Op::Alter, param);
        h.click_at(p);
    }
    assert!(!h.app.row_done(0, 2));
    h.click_at(a);
    assert!(h.app.row_done(0, 2), "ALTER is green after a run");
    let moved = h.app.panel.pattern_303(0);
    assert_ne!(moved, made);
    // the same notes, rests and accents, moved together; the slides stayed in place
    let key = |s: &orb_ui::model::Step303| (s.note, s.rest, s.accent);
    let mut a1: Vec<_> = made.iter().map(key).collect();
    let mut b1: Vec<_> = moved.iter().map(key).collect();
    a1.sort();
    b1.sort();
    assert_eq!(a1, b1);
    for (m, n) in made.iter().zip(moved.iter()) {
        assert_eq!((m.slide, m.up, m.down), (n.slide, n.up, n.down));
    }
    // both rows keep their choice
    assert_eq!(h.app.op_selection(0, Op::Alter), [false, false, true, false, true]);
    assert_eq!(h.app.op_selection(0, Op::Random), [false, false, true, false, true]);
}
