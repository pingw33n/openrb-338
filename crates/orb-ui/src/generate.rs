//! The drop-down menu of the bass synths' GENERATE buttons: the parameters of the random note
//! generator. A press of the button opens its synth's menu under it and a second press closes
//! it; Escape and a click outside close it too.
//!
//! The menu is a table thought of as a spreadsheet: columns A and B, rows 1 to 4, every cell of
//! one size, black with white lines. Column A names the rows in the red of the step displays;
//! column B is empty for now.

use eframe::egui::{
    self, Align2, Color32, FontId, Key, Modifiers, PointerButton, Pos2, Rect, Sense, Stroke,
    StrokeKind, Ui,
};

use crate::app::{App, View};

/// The row names in column A (A1 to A4).
pub const ROWS: [&str; 4] = ["SCALE/KEY", "RANDOM", "ALTER", "MOVE"];
/// The number of columns (A and B).
pub const COLS: usize = 2;
/// One cell in panel pixels; every cell has the same width and height.
pub const CELL_W: f32 = 48.0;
pub const CELL_H: f32 = 14.0;
/// The space between the button's bottom edge and the menu, in panel pixels.
const GAP: f32 = 2.0;
/// The red of the step displays (the digits of STEPS and EDIT STEP, `LCD_ON` of orb-ui-art).
pub const RED: Color32 = Color32::from_rgb(0xff, 0x3b, 0x2f);
const LINE: Color32 = Color32::WHITE;
const FILL: Color32 = Color32::BLACK;

/// The synth (0 for the upper, 1 for the lower) whose GENERATE button `id` is, if it is one.
pub fn synth_of(id: &str) -> Option<usize> {
    match id {
        "tb303_1.generate" => Some(0),
        "tb303_2.generate" => Some(1),
        _ => None,
    }
}

impl App {
    /// A press of a GENERATE button: opens its synth's menu, or closes it when it is open.
    pub fn toggle_generate(&mut self, synth: usize) {
        self.generate_open = if self.generate_open == Some(synth) {
            None
        } else {
            Some(synth)
        };
    }

    /// The synth whose menu is open, if any.
    pub fn generate_open(&self) -> Option<usize> {
        self.generate_open
    }

    /// The panel rectangle (x, y, w, h) of a synth's GENERATE button.
    fn generate_button(&self, synth: usize) -> Option<(f32, f32, f32, f32)> {
        let id = format!("tb303_{}.generate", synth + 1);
        let b = self.ctls.iter().find(|c| c.id == id)?;
        Some((b.x, b.y, b.w, b.h))
    }

    /// The menu's panel rectangle (x, y, w, h): under the button, the right edges aligned.
    pub fn generate_rect(&self, synth: usize) -> Option<(f32, f32, f32, f32)> {
        let (bx, by, bw, bh) = self.generate_button(synth)?;
        let w = CELL_W * COLS as f32;
        let h = CELL_H * ROWS.len() as f32;
        Some((bx + bw - w, by + bh + GAP, w, h))
    }

    /// Draws the open menu above the panel, after the panel's controls (so a press of the
    /// button has already opened or closed it this frame).
    pub(crate) fn generate_ui(&mut self, ui: &Ui, view: &View) {
        let Some(k) = self.generate_open else {
            return;
        };
        let (Some((x, y, w, h)), Some((bx, by, bw, bh))) =
            (self.generate_rect(k), self.generate_button(k))
        else {
            return;
        };
        let ctx = ui.ctx().clone();
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            self.generate_open = None;
            return;
        }
        let rect = view.rect(x, y, w, h);
        let button = view.rect(bx, by, bw, bh);
        // a click outside the menu and its button closes the menu
        let pressed = ctx.input(|i| {
            i.pointer
                .button_pressed(PointerButton::Primary)
                .then(|| i.pointer.press_origin())
                .flatten()
        });
        if let Some(p) = pressed
            && !rect.contains(p)
            && !button.contains(p)
        {
            self.generate_open = None;
            return;
        }
        egui::Area::new(egui::Id::unique(("orb_generate_menu", k)))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.min)
            .constrain(false)
            .show(&ctx, |ui| {
                let (r, _) = ui.allocate_exact_size(rect.size(), Sense::click());
                paint_table(ui, r, view.scale);
            });
    }
}

/// The table: the black fill, the white lines around and between the cells, the row names.
fn paint_table(ui: &Ui, r: Rect, scale: f32) {
    let p = ui.painter();
    p.rect_filled(r, 0.0, FILL);
    let stroke = Stroke::new((0.6 * scale).max(1.0), LINE);
    let cw = r.width() / COLS as f32;
    let ch = r.height() / ROWS.len() as f32;
    for i in 1..COLS {
        let x = r.min.x + cw * i as f32;
        p.line_segment([Pos2::new(x, r.min.y), Pos2::new(x, r.max.y)], stroke);
    }
    for j in 1..ROWS.len() {
        let y = r.min.y + ch * j as f32;
        p.line_segment([Pos2::new(r.min.x, y), Pos2::new(r.max.x, y)], stroke);
    }
    p.rect_stroke(r, 0.0, stroke, StrokeKind::Inside);
    let font = FontId::proportional(CELL_H * 0.5 * scale);
    for (j, name) in ROWS.into_iter().enumerate() {
        let c = Pos2::new(r.min.x + cw * 0.5, r.min.y + ch * (j as f32 + 0.5));
        p.text(c, Align2::CENTER_CENTER, name, font.clone(), RED);
    }
}
