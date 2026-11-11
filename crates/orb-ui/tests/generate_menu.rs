//! The GENERATE menu of the bass synths (`generate.rs`), headless: synthetic pointer and key
//! events run through `App::show` on the panel at 1:1 (panel pixels are screen points).

use eframe::egui::{self, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2};
use orb_ui::app::App;
use orb_ui::generate::{CELL_H, CELL_W, COLS, ROWS};
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
fn the_menu_is_a_table_of_equal_cells_under_its_button_inside_the_panel() {
    assert_eq!(ROWS, ["SCALE/KEY", "RANDOM", "ALTER", "MOVE"]);
    let h = Harness::new();
    for synth in 0..2 {
        let (x, y, w, h_) = h.app.generate_rect(synth).unwrap();
        assert_eq!((w, h_), (CELL_W * COLS as f32, CELL_H * ROWS.len() as f32));
        let b = button(synth);
        assert!(y > b.y, "the menu is under the button");
        assert!(x >= 0.0 && y >= 0.0 && x + w <= PANEL_W && y + h_ <= PANEL_H);
    }
}
