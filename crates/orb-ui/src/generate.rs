//! The drop-down menu of the bass synths' GENERATE buttons: the parameters of the random note
//! generator. A press of the button opens its synth's menu under it and a second press closes
//! it; Escape and a click outside close it too.
//!
//! The menu is a table thought of as a spreadsheet: columns A and B, rows 1 to 4 of one height,
//! column B twice as wide as column A. It is black with white lines between the cells (no
//! outer frame; the lines stop short of the menu's edges). Column A names the rows in the red
//! of the step displays; a row's name turns green once its choice in column B is complete.
//!
//! Row 1 (SCALE/KEY): column B is split into the scale (the wider part) and the key note. A
//! press on one opens its list under it, scrolled with the mouse wheel or its slider; a press on
//! an item chooses it, shown in green. The key note is chosen after the scale, and BAZE 2 is
//! offered only what fits BAZE 1 (`scale.rs`). The 303's keys of the chosen scale glow green.
//!
//! Row 2 (RANDOM): column B is five buttons, the step fields to randomize: DOWN (down arrow),
//! UP (up arrow), ACCENT (+), SLIDE (underline) and NOTE (a note; it needs the scale and key).
//! A press on RANDOM starts choosing them (RANDOM blinks, the last choice is shown, a chosen
//! button glows green); the next press randomizes the chosen fields of the synth's pattern
//! (`random.rs`) and stops.
//!
//! Row 3 (ALTER): the same five buttons, chosen the same way (each row keeps its own choice),
//! but its run makes nothing new: it moves the chosen fields of the steps to other steps along
//! one random chain (`random::alter`), the chosen fields of a step together.
//!
//! Row 4 (MOVE): five buttons, -2, -1, ?, +1 and +2, one of them chosen at a time, the same
//! way; its run moves the whole pattern that many steps around in a ring (`?`: a random 1 to
//! 15 steps; `random::rotate`).

use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Key, Modifiers, PointerButton, Pos2, Rect,
    Sense, Stroke, Ui, Vec2,
};

use crate::app::{App, View};
use crate::palette::{INK, INK_DIM, SLATE_DARK};
use crate::random::{self, Rng};
use crate::scale::{self, KEYS, SCALES, ScaleKeys};

/// The row names in column A (A1 to A4).
pub const ROWS: [&str; 4] = ["SCALE/KEY", "RANDOM", "ALTER", "MOVE"];
/// The widths of columns A and B in panel pixels: B is twice as wide as A.
pub const COL_W: [f32; 2] = [48.0, 96.0];
/// The height of every row (and of a list's item) in panel pixels.
pub const CELL_H: f32 = 14.0;
/// Row 1's column B: the width of the scale field; the key field takes the rest.
pub const SCALE_W: f32 = 64.0;
/// How far short of the menu's edges the lines between the cells stop, in panel pixels.
const LINE_INSET: f32 = 3.0;
/// The space between the button's bottom edge and the menu, in panel pixels.
const GAP: f32 = 2.0;
/// The items a list shows at once, and the width of its slider (panel pixels).
pub const LIST_ROWS: usize = 5;
const SLIDER_W: f32 = 5.0;
/// The red of the step displays (the digits of STEPS and EDIT STEP, `LCD_ON` of orb-ui-art).
pub const RED: Color32 = Color32::from_rgb(0xff, 0x3b, 0x2f);
/// A chosen scale or key note, and the glow of the scale's keys (the panel's `GREEN`).
pub const GREEN: Color32 = Color32::from_rgb(0x53, 0xd8, 0x6a);
const LINE: Color32 = Color32::WHITE;
const FILL: Color32 = Color32::BLACK;
/// A field that cannot be used yet.
const OFF: Color32 = Color32::from_gray(0x55);
const LIST_FILL: Color32 = Color32::from_gray(0x14);
/// The period of a row name's blink while its fields are chosen, in seconds (half shown, half
/// not).
const BLINK: f64 = 0.6;

/// The rows of five buttons: RANDOM makes new values, ALTER moves the values there are to other
/// steps, MOVE moves the whole pattern (one button at a time).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Random = 0,
    Alter = 1,
    Move = 2,
}

impl Op {
    pub const ALL: [Op; 3] = [Op::Random, Op::Alter, Op::Move];

    /// The menu row (0 is A1).
    pub fn row(self) -> usize {
        1 + self as usize
    }
}

/// Row 1's two fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Scale,
    Key,
}

/// An open list: whose field, how far it is scrolled (in items) and, while the slider's thumb
/// is dragged, where on the thumb it was grabbed (screen points from its top).
#[derive(Clone, Copy, Debug)]
pub struct List {
    pub synth: usize,
    pub field: Field,
    pub scroll: f32,
    grab: Option<f32>,
}

/// The synth (0 for the upper, 1 for the lower) whose GENERATE button `id` is, if it is one.
pub fn synth_of(id: &str) -> Option<usize> {
    match id {
        "tb303_1.generate" => Some(0),
        "tb303_2.generate" => Some(1),
        _ => None,
    }
}

/// Whether the control `id` is one of the 303's keys (`tb303_N.keyM`, not its LED) whose note is
/// in its synth's chosen scale.
pub fn glows(sk: &ScaleKeys, id: &str) -> bool {
    let Some((sec, name)) = id.split_once('.') else {
        return false;
    };
    let synth = match sec {
        "tb303_1" => 0,
        "tb303_2" => 1,
        _ => return false,
    };
    let Some(n) = name.strip_prefix("key").and_then(|n| n.parse::<usize>().ok()) else {
        return false;
    };
    sk.get(synth).mask().is_some_and(|m| scale::key_in(m, n))
}

/// The green glow around a key of the chosen scale, painted under the key.
pub fn paint_glow(ui: &Ui, key: Rect, scale: f32) {
    let p = ui.painter();
    for (grow, alpha) in [(3.5, 45), (2.0, 110)] {
        p.rect_filled(
            key.expand(grow * scale),
            CornerRadius::same((2.5 * scale) as u8),
            Color32::from_rgba_unmultiplied(GREEN.r(), GREEN.g(), GREEN.b(), alpha),
        );
    }
}

/// The items of a list: the indexes into `SCALES` or `KEYS`.
fn options(sk: &ScaleKeys, synth: usize, field: Field) -> Vec<usize> {
    match field {
        Field::Scale => sk.scale_options(synth),
        Field::Key => sk.key_options(synth),
    }
}

fn item_name(field: Field, i: usize) -> &'static str {
    match field {
        Field::Scale => SCALES[i].0,
        Field::Key => KEYS[i],
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
        self.generate_list = None;
        self.op_picking = None;
    }

    /// Whether row `row` (0 is A1) of a synth's menu has its choice in column B complete: its
    /// name in column A then turns green. SCALE/KEY needs both the scale and the key note,
    /// RANDOM, ALTER and MOVE a last run that changed something.
    pub fn row_done(&self, synth: usize, row: usize) -> bool {
        match row {
            0 => self.scale_keys.get(synth).mask().is_some(),
            _ => Op::ALL
                .into_iter()
                .any(|op| op.row() == row && self.op_done[op as usize][synth]),
        }
    }

    // ---- the RANDOM, ALTER and MOVE rows --------------------------------------------------

    /// The synth and row whose fields are being chosen (the row's name blinks), if any.
    pub fn op_picking(&self) -> Option<(usize, Op)> {
        self.op_picking
    }

    /// A synth's chosen buttons of a row, indexed by `random::PARAMS` (MOVE: `random::MOVES`).
    pub fn op_selection(&self, synth: usize, op: Op) -> [bool; 5] {
        self.op_sel[op as usize][synth]
    }

    /// Restarts the random numbers from `seed` (tests).
    pub fn set_random_seed(&mut self, seed: u64) {
        self.rng = Rng::new(seed);
    }

    /// Whether a row's field can be chosen: RANDOM's NOTE needs the synth's scale and key note
    /// (it makes notes of the scale); ALTER and MOVE only move what is there, so all can.
    pub fn op_param_usable(&self, synth: usize, op: Op, param: usize) -> bool {
        op != Op::Random || param != random::NOTE || self.scale_keys.get(synth).mask().is_some()
    }

    /// A press on RANDOM, ALTER or MOVE: starts choosing the row's buttons (its name blinks;
    /// its last choice is shown), or, while choosing, runs the row on the synth's pattern and
    /// stops.
    pub fn press_op(&mut self, synth: usize, op: Op) {
        self.generate_list = None;
        if self.op_picking != Some((synth, op)) {
            self.op_picking = Some((synth, op));
            return;
        }
        self.op_picking = None;
        let mut on = self.op_sel[op as usize][synth];
        for (p, o) in on.iter_mut().enumerate() {
            *o &= self.op_param_usable(synth, op, p);
        }
        let any = on.iter().any(|&o| o);
        self.op_done[op as usize][synth] = any;
        if any {
            let mut steps = self.panel.pattern_303(synth);
            match op {
                Op::Random => {
                    let mask = self.scale_keys.get(synth).mask();
                    random::randomize(&mut steps, on, mask, &mut self.rng);
                }
                Op::Alter => random::alter(&mut steps, on, &mut self.rng),
                Op::Move => {
                    let choice = on.iter().position(|&o| o).unwrap_or(random::MOVE_RANDOM);
                    random::rotate(&mut steps, random::move_offset(choice, &mut self.rng));
                }
            }
            self.panel.set_pattern_303(synth, steps);
        }
    }

    /// A press on one of a row's five buttons: while choosing, turns it on or off. MOVE's are
    /// one at a time: turning one on turns the others off.
    pub fn press_op_param(&mut self, synth: usize, op: Op, param: usize) {
        self.generate_list = None;
        if self.op_picking == Some((synth, op)) && self.op_param_usable(synth, op, param) {
            let sel = &mut self.op_sel[op as usize][synth];
            let on = !sel[param];
            if op == Op::Move {
                *sel = [false; 5];
            }
            sel[param] = on;
        }
    }

    /// The panel rectangle of a row's name (column A).
    pub fn op_label_rect(&self, synth: usize, op: Op) -> Option<(f32, f32, f32, f32)> {
        let (x, y, ..) = self.generate_rect(synth)?;
        Some((x, y + CELL_H * op.row() as f32, COL_W[0], CELL_H))
    }

    /// The panel rectangle of a row's button `param` (column B split in five).
    pub fn op_button_rect(
        &self,
        synth: usize,
        op: Op,
        param: usize,
    ) -> Option<(f32, f32, f32, f32)> {
        let (x, y, ..) = self.generate_rect(synth)?;
        let w = COL_W[1] / random::PARAMS.len() as f32;
        let y = y + CELL_H * op.row() as f32;
        Some((x + COL_W[0] + w * param as f32, y, w, CELL_H))
    }

    /// The synth whose menu is open, if any.
    pub fn generate_open(&self) -> Option<usize> {
        self.generate_open
    }

    /// The open list's synth and field, if any.
    pub fn generate_list(&self) -> Option<(usize, Field)> {
        self.generate_list.map(|l| (l.synth, l.field))
    }

    /// The scale and key note of both synths.
    pub fn scale_keys(&self) -> &ScaleKeys {
        &self.scale_keys
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
        let w: f32 = COL_W.iter().sum();
        let h = CELL_H * ROWS.len() as f32;
        Some((bx + bw - w, by + bh + GAP, w, h))
    }

    /// The panel rectangle of row 1's scale or key field.
    pub fn field_rect(&self, synth: usize, field: Field) -> Option<(f32, f32, f32, f32)> {
        let (x, y, ..) = self.generate_rect(synth)?;
        let b = x + COL_W[0];
        Some(match field {
            Field::Scale => (b, y, SCALE_W, CELL_H),
            Field::Key => (b + SCALE_W, y, COL_W[1] - SCALE_W, CELL_H),
        })
    }

    /// The panel rectangle of a field's list, under the field; None when it has no items.
    pub fn list_rect(&self, synth: usize, field: Field) -> Option<(f32, f32, f32, f32)> {
        let n = options(&self.scale_keys, synth, field).len();
        if n == 0 {
            return None;
        }
        let (x, y, w, h) = self.field_rect(synth, field)?;
        Some((x, y + h, w, CELL_H * n.min(LIST_ROWS) as f32))
    }

    /// A press on a field: opens its list (scrolled to the chosen item), or closes it when it
    /// is open. A field with nothing to offer (BAZE 2 before BAZE 1, a key before a scale) does
    /// nothing.
    pub fn press_field(&mut self, synth: usize, field: Field) {
        if self.generate_list.is_some_and(|l| l.synth == synth && l.field == field) {
            self.generate_list = None;
            return;
        }
        let items = options(&self.scale_keys, synth, field);
        if items.is_empty() {
            self.generate_list = None;
            return;
        }
        let chosen = match field {
            Field::Scale => self.scale_keys.get(synth).scale,
            Field::Key => self.scale_keys.get(synth).key,
        };
        let at = chosen.and_then(|c| items.iter().position(|&i| i == c));
        let max = items.len().saturating_sub(LIST_ROWS) as f32;
        let scroll = at.map_or(0.0, |a| (a as f32 - 2.0).clamp(0.0, max));
        self.generate_list = Some(List {
            synth,
            field,
            scroll,
            grab: None,
        });
    }

    /// Chooses the open list's item at `row` (0 is the list's first item) and closes the list.
    pub fn choose_item(&mut self, row: usize) {
        let Some(l) = self.generate_list else {
            return;
        };
        let items = options(&self.scale_keys, l.synth, l.field);
        if let Some(&i) = items.get(row) {
            match l.field {
                Field::Scale => self.scale_keys.set_scale(l.synth, i),
                Field::Key => self.scale_keys.set_key(l.synth, i),
            };
        }
        self.generate_list = None;
    }

    /// Draws the open menu (and its open list) above the panel, after the panel's controls (so
    /// a press of the button has already opened or closed it this frame), and takes its input.
    pub(crate) fn generate_ui(&mut self, ui: &Ui, view: &View) {
        let Some(k) = self.generate_open else {
            return;
        };
        // a change of the menu's settings is a change of the song (an .rbsx keeps them)
        let before = self.gen_state();
        self.generate_menu(view, k, ui.ctx().clone());
        if self.gen_state() != before {
            self.doc.modified = true;
        }
    }

    /// [`App::generate_ui`] for the open menu of synth `k`.
    fn generate_menu(&mut self, view: &View, k: usize, ctx: egui::Context) {
        let (Some((x, y, w, h)), Some((bx, by, bw, bh))) =
            (self.generate_rect(k), self.generate_button(k))
        else {
            return;
        };
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            if self.generate_list.is_some() {
                self.generate_list = None;
            } else {
                self.generate_open = None;
                self.op_picking = None;
            }
            return;
        }
        let menu = view.rect(x, y, w, h);
        let button = view.rect(bx, by, bw, bh);
        let screen = |r: Option<(f32, f32, f32, f32)>| r.map(|(x, y, w, h)| view.rect(x, y, w, h));
        let scale_field = screen(self.field_rect(k, Field::Scale)).unwrap_or(Rect::NOTHING);
        let key_field = screen(self.field_rect(k, Field::Key)).unwrap_or(Rect::NOTHING);
        let list_rect = self
            .generate_list
            .and_then(|l| screen(self.list_rect(l.synth, l.field)));
        let item_h = CELL_H * view.scale;
        let slider_w = SLIDER_W * view.scale;
        // RANDOM's, ALTER's and MOVE's names and buttons, indexed by `Op`
        let op_labels: [Rect; 3] = Op::ALL
            .map(|op| screen(self.op_label_rect(k, op)).unwrap_or(Rect::NOTHING));
        let op_buttons: [[Rect; 5]; 3] = Op::ALL.map(|op| {
            std::array::from_fn(|i| screen(self.op_button_rect(k, op, i)).unwrap_or(Rect::NOTHING))
        });

        let (pressed, down, hover, wheel) = ctx.input(|i| {
            (
                i.pointer
                    .button_pressed(PointerButton::Primary)
                    .then(|| i.pointer.press_origin())
                    .flatten(),
                i.pointer.primary_down(),
                i.pointer.hover_pos(),
                i.smooth_scroll_delta.y,
            )
        });

        if let Some(p) = pressed {
            let label = Op::ALL.into_iter().find(|&op| op_labels[op as usize].contains(p));
            let param = Op::ALL.into_iter().find_map(|op| {
                op_buttons[op as usize]
                    .iter()
                    .position(|b| b.contains(p))
                    .map(|i| (op, i))
            });
            match list_rect {
                Some(lr) if lr.contains(p) => self.press_list(lr, p, item_h, slider_w),
                _ if scale_field.contains(p) => self.press_field(k, Field::Scale),
                _ if key_field.contains(p) => self.press_field(k, Field::Key),
                _ if label.is_some() => self.press_op(k, label.unwrap_or(Op::Random)),
                _ if param.is_some() => {
                    let (op, i) = param.unwrap_or((Op::Random, 0));
                    self.press_op_param(k, op, i);
                }
                _ => {
                    self.generate_list = None;
                    if !menu.contains(p) && !button.contains(p) {
                        self.generate_open = None;
                        self.op_picking = None;
                        return;
                    }
                }
            }
        }
        if let (Some(l), Some(lr)) = (&mut self.generate_list, list_rect) {
            let n = options(&self.scale_keys, l.synth, l.field).len();
            let max = n.saturating_sub(LIST_ROWS) as f32;
            if let Some(grab) = l.grab {
                match (down, hover) {
                    (true, Some(p)) => {
                        let (track, thumb_h) = slider_track(lr, n, slider_w);
                        let room = (track.height() - thumb_h).max(1.0);
                        l.scroll = ((p.y - grab - track.min.y) / room * max).clamp(0.0, max);
                    }
                    _ => l.grab = None,
                }
            }
            if wheel != 0.0 && hover.is_some_and(|p| lr.contains(p)) {
                l.scroll = (l.scroll - wheel / item_h).clamp(0.0, max);
            }
        }

        let sk = self.scale_keys;
        // the row names: green once done, else red; a row's name blinks while its fields are
        // chosen
        let picking = self.op_picking.filter(|&(s, _)| s == k).map(|(_, op)| op.row());
        if picking.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
        let blink_on = (ctx.input(|i| i.time) / BLINK).fract() < 0.5;
        let names: [Option<Color32>; 4] = std::array::from_fn(|row| {
            let blinking = picking == Some(row);
            if blinking && !blink_on {
                None
            } else if self.row_done(k, row) && !blinking {
                Some(GREEN)
            } else {
                Some(RED)
            }
        });
        let buttons: [[(Rect, bool, Color32); 5]; 3] = Op::ALL.map(|op| {
            let choosing = picking == Some(op.row());
            std::array::from_fn(|i| {
                let usable = self.op_param_usable(k, op, i);
                let on = self.op_sel[op as usize][k][i] && usable;
                let ink = match (on, usable, choosing) {
                    (true, ..) => GREEN,
                    (false, false, _) => OFF,
                    (false, true, true) => INK,
                    (false, true, false) => INK_DIM,
                };
                (op_buttons[op as usize][i], on, ink)
            })
        });
        egui::Area::new(egui::Id::unique(("orb_generate_menu", k)))
            .order(egui::Order::Foreground)
            .fixed_pos(menu.min)
            .constrain(false)
            // solid from the first frame, like the panel's own parts
            .fade_in(false)
            .show(&ctx, |ui| {
                let (r, _) = ui.allocate_exact_size(menu.size(), Sense::click());
                paint_table(ui, r, view.scale, names);
                paint_fields(ui, &sk, k, scale_field, key_field, view.scale);
                for op in Op::ALL {
                    paint_op_buttons(ui, op, &buttons[op as usize], view.scale);
                }
            });
        if let (Some(l), Some(lr)) = (self.generate_list, list_rect) {
            egui::Area::new(egui::Id::unique(("orb_generate_list", k)))
                .order(egui::Order::Tooltip)
                .fixed_pos(lr.min)
                .constrain(false)
                .fade_in(false)
                .show(&ctx, |ui| {
                    let (r, _) = ui.allocate_exact_size(lr.size(), Sense::click());
                    paint_list(ui, &sk, &l, r, hover, view.scale);
                });
        }
    }

    /// A press inside the open list: on the slider it grabs the thumb (or jumps there), on an
    /// item it chooses the item.
    fn press_list(&mut self, lr: Rect, p: Pos2, item_h: f32, slider_w: f32) {
        let Some(l) = &mut self.generate_list else {
            return;
        };
        let n = options(&self.scale_keys, l.synth, l.field).len();
        if n > LIST_ROWS && p.x >= lr.max.x - slider_w {
            let (track, thumb_h) = slider_track(lr, n, slider_w);
            let max = (n - LIST_ROWS) as f32;
            let thumb_y = track.min.y + (track.height() - thumb_h) * l.scroll / max;
            let grab = if (thumb_y..thumb_y + thumb_h).contains(&p.y) {
                p.y - thumb_y
            } else {
                // a press on the track moves the thumb's middle there
                thumb_h / 2.0
            };
            let room = (track.height() - thumb_h).max(1.0);
            l.scroll = ((p.y - grab - track.min.y) / room * max).clamp(0.0, max);
            l.grab = Some(grab);
            return;
        }
        let row = ((p.y - lr.min.y) / item_h + l.scroll).floor().max(0.0) as usize;
        self.choose_item(row);
    }
}

/// The slider's track (the list's right edge) and the height of its thumb.
fn slider_track(lr: Rect, n: usize, slider_w: f32) -> (Rect, f32) {
    let track = Rect::from_min_max(Pos2::new(lr.max.x - slider_w, lr.min.y), lr.max);
    let thumb_h = track.height() * LIST_ROWS as f32 / n.max(1) as f32;
    (track, thumb_h)
}

/// The table: the black fill, the white lines between the cells (stopping short of the
/// edges), the row names in their colors (None: not shown, RANDOM's blink).
fn paint_table(ui: &Ui, r: Rect, scale: f32, names: [Option<Color32>; 4]) {
    let p = ui.painter();
    p.rect_filled(r, 0.0, FILL);
    let stroke = Stroke::new((0.6 * scale).max(1.0), LINE);
    let inset = LINE_INSET * scale;
    let col_a = COL_W[0] * scale;
    let ch = CELL_H * scale;
    // the line between columns A and B
    let x = r.min.x + col_a;
    p.line_segment(
        [Pos2::new(x, r.min.y + inset), Pos2::new(x, r.max.y - inset)],
        stroke,
    );
    // row 1: the line between the scale and the key note
    let x = x + SCALE_W * scale;
    p.line_segment(
        [Pos2::new(x, r.min.y + inset), Pos2::new(x, r.min.y + ch - inset)],
        stroke,
    );
    // the lines between the rows
    for j in 1..ROWS.len() {
        let y = r.min.y + ch * j as f32;
        p.line_segment(
            [Pos2::new(r.min.x + inset, y), Pos2::new(r.max.x - inset, y)],
            stroke,
        );
    }
    for (j, name) in ROWS.into_iter().enumerate() {
        let c = Pos2::new(r.min.x + col_a * 0.5, r.min.y + ch * (j as f32 + 0.5));
        if let Some(color) = names[j] {
            p.text(c, Align2::CENTER_CENTER, name, font(scale), color);
        }
    }
}

fn font(scale: f32) -> FontId {
    FontId::proportional(CELL_H * 0.5 * scale)
}

/// Row 1's scale and key note: the choice in green, else the field's name, dimmed while it
/// cannot be used.
fn paint_fields(ui: &Ui, sk: &ScaleKeys, synth: usize, sf: Rect, kf: Rect, scale: f32) {
    let p = ui.painter();
    let c = sk.get(synth);
    let shown = |chosen: Option<&'static str>, name: &'static str, usable: bool| match chosen {
        Some(t) => (t, GREEN),
        None => (name, if usable { INK_DIM } else { OFF }),
    };
    let (t, color) = shown(
        c.scale.map(|s| SCALES[s].0),
        "SCALE",
        !sk.scale_options(synth).is_empty(),
    );
    p.text(sf.center(), Align2::CENTER_CENTER, t, font(scale), color);
    let (t, color) = shown(
        c.key.map(|k| KEYS[k]),
        "KEY",
        !sk.key_options(synth).is_empty(),
    );
    p.text(kf.center(), Align2::CENTER_CENTER, t, font(scale), color);
}

/// An open list: its items (the chosen one green, the one under the pointer highlighted) and,
/// when it has more items than it shows, the slider at its right edge.
fn paint_list(ui: &Ui, sk: &ScaleKeys, l: &List, r: Rect, hover: Option<Pos2>, scale: f32) {
    let items = options(sk, l.synth, l.field);
    let chosen = match l.field {
        Field::Scale => sk.get(l.synth).scale,
        Field::Key => sk.get(l.synth).key,
    };
    let item_h = CELL_H * scale;
    let slider_w = SLIDER_W * scale;
    let has_slider = items.len() > LIST_ROWS;
    let text_w = r.width() - if has_slider { slider_w } else { 0.0 };
    let p = ui.painter().with_clip_rect(r);
    p.rect_filled(r, 0.0, LIST_FILL);
    for (row, &i) in items.iter().enumerate() {
        let top = r.min.y + (row as f32 - l.scroll) * item_h;
        let item = Rect::from_min_size(Pos2::new(r.min.x, top), Vec2::new(text_w, item_h));
        if !item.intersects(r) {
            continue;
        }
        if l.grab.is_none() && hover.is_some_and(|h| item.contains(h) && r.contains(h)) {
            p.rect_filled(item, 0.0, SLATE_DARK);
        }
        let color = if chosen == Some(i) { GREEN } else { INK };
        p.text(
            item.center(),
            Align2::CENTER_CENTER,
            item_name(l.field, i),
            font(scale),
            color,
        );
    }
    if has_slider {
        let (track, thumb_h) = slider_track(r, items.len(), slider_w);
        let max = (items.len() - LIST_ROWS) as f32;
        let thumb_y = track.min.y + (track.height() - thumb_h) * l.scroll / max;
        p.rect_filled(track, 0.0, Color32::from_gray(0x2a));
        let thumb = Rect::from_min_size(
            Pos2::new(track.min.x + scale * 0.5, thumb_y),
            Vec2::new(track.width() - scale, thumb_h),
        );
        p.rect_filled(thumb, CornerRadius::same((1.5 * scale) as u8), INK_DIM);
    }
    p.rect_stroke(
        r,
        0.0,
        Stroke::new((0.6 * scale).max(1.0), LINE),
        egui::StrokeKind::Inside,
    );
}

/// A row's five buttons (RANDOM, ALTER, MOVE): (rect, on, ink of the symbol). A chosen button
/// glows green; thin lines part the buttons, stopping short of the row's edges like the table's.
fn paint_op_buttons(ui: &Ui, op: Op, buttons: &[(Rect, bool, Color32); 5], scale: f32) {
    let p = ui.painter();
    let inset = LINE_INSET * scale;
    let line = Stroke::new((0.6 * scale).max(1.0), LINE);
    for (r, ..) in &buttons[1..] {
        p.line_segment(
            [
                Pos2::new(r.min.x, r.min.y + inset),
                Pos2::new(r.min.x, r.max.y - inset),
            ],
            line,
        );
    }
    for (param, &(r, on, ink)) in buttons.iter().enumerate() {
        if on {
            let glow = r.shrink(1.5 * scale);
            for (grow, alpha) in [(1.5, 40), (0.0, 90)] {
                p.rect_filled(
                    glow.expand(grow * scale),
                    CornerRadius::same((2.0 * scale) as u8),
                    Color32::from_rgba_unmultiplied(GREEN.r(), GREEN.g(), GREEN.b(), alpha),
                );
            }
        }
        if op == Op::Move {
            // MOVE's steps as text, bold: drawn twice, a hair apart
            let f = FontId::proportional(CELL_H * 0.6 * scale);
            for dx in [0.0, 0.35 * scale] {
                let at = r.center() + Vec2::new(dx, 0.0);
                p.text(at, Align2::CENTER_CENTER, random::MOVES[param], f.clone(), ink);
            }
        } else {
            paint_symbol(ui, param, r.center(), CELL_H * 0.27 * scale, ink, scale);
        }
    }
}

/// A RANDOM or ALTER button's symbol, bold: half its height is `s` around `c`.
fn paint_symbol(ui: &Ui, param: usize, c: Pos2, s: f32, ink: Color32, scale: f32) {
    let p = ui.painter();
    let bold = Stroke::new(1.4 * scale, ink);
    let at = |dx: f32, dy: f32| Pos2::new(c.x + dx * s, c.y + dy * s);
    let line = |a: Pos2, b: Pos2| {
        p.line_segment([a, b], bold);
    };
    match param {
        random::DOWN => {
            line(at(0.0, -1.0), at(0.0, 1.0));
            line(at(-0.7, 0.3), at(0.0, 1.0));
            line(at(0.7, 0.3), at(0.0, 1.0));
        }
        random::UP => {
            line(at(0.0, 1.0), at(0.0, -1.0));
            line(at(-0.7, -0.3), at(0.0, -1.0));
            line(at(0.7, -0.3), at(0.0, -1.0));
        }
        random::ACCENT => {
            line(at(-0.9, 0.0), at(0.9, 0.0));
            line(at(0.0, -0.9), at(0.0, 0.9));
        }
        random::SLIDE => line(at(-1.0, 0.9), at(1.0, 0.9)),
        _ => {
            // an eighth note: the head, the stem and the flag
            p.circle_filled(at(-0.35, 0.65), 0.42 * s, ink);
            line(at(0.05, 0.6), at(0.05, -1.0));
            line(at(0.05, -1.0), at(0.75, -0.35));
        }
    }
}
