//! The window: draws the panel from the SVG parts at the displayed size and runs the
//! original's interaction (docs/ui_spec.md section 2) on the [`Panel`] model.
//!
//! Rendering follows decision task-040: every part is an `egui::Image` of its SVG, loaded by
//! egui_extras' SVG loader (usvg + resvg) with a size hint of the displayed size in physical
//! pixels; a scale change gives a new hint and so a new, crisp raster. The panel keeps its
//! 624 x 510 aspect ratio and is letterboxed.

use std::path::PathBuf;
use std::time::Duration;

use eframe::egui::{
    self, Color32, Image, Key, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2,
    emath::GuiRounding,
};

use crate::art::{self, KnobDesign};
use crate::behavior::{self, Mods};
use crate::layout::{self, Ctl, PANEL_H, PANEL_W};
use crate::link::Link;
use crate::model::Panel;

/// The panel's background and the letterbox color.
const BG: Color32 = Color32::from_rgb(0x1d, 0x21, 0x27);

/// Where the panel is on screen: the point of the panel's origin and points per panel pixel.
#[derive(Clone, Copy, Debug)]
pub struct View {
    pub origin: Pos2,
    pub scale: f32,
}

impl View {
    /// The largest panel-shaped area centered in `avail`, snapped to whole pixels.
    pub fn fit(avail: Rect, ppp: f32) -> Self {
        let scale = (avail.width() / PANEL_W).min(avail.height() / PANEL_H);
        let size = Vec2::new(PANEL_W, PANEL_H) * scale;
        Self {
            origin: (avail.center() - size / 2.0).round_to_pixels(ppp),
            scale,
        }
    }

    pub fn rect(&self, x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(
            self.origin + Vec2::new(x, y) * self.scale,
            Vec2::new(w, h) * self.scale,
        )
    }

    /// Screen point to panel pixels.
    pub fn to_panel(&self, p: Pos2) -> Pos2 {
        ((p - self.origin) / self.scale).to_pos2()
    }
}

/// Per-control gesture state, kept in egui's memory.
#[derive(Clone, Default)]
struct Gesture {
    down: bool,
    p0: i32,
    y0: f32,
    fine: bool,
    t0: f64,
    taken: u32,
    new_value: u8,
    last_step: i32,
}

/// Where to save a screenshot and quit (`--screenshot`).
pub struct Shot {
    pub path: PathBuf,
    pub frames: u32,
    requested: bool,
}

impl Shot {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            frames: 0,
            requested: false,
        }
    }
}

pub struct App {
    pub panel: Panel,
    /// The player the panel drives (task-045); without one the model's events pile up in
    /// `Panel::events`.
    pub link: Option<Link>,
    ctls: Vec<Ctl>,
    shot: Option<Shot>,
}

impl App {
    pub fn new(panel: Panel, shot: Option<Shot>) -> Self {
        Self {
            panel,
            link: None,
            ctls: layout::controls(),
            shot,
        }
    }

    /// An app whose panel drives `link`.
    pub fn with_link(panel: Panel, link: Link, shot: Option<Shot>) -> Self {
        let mut app = Self::new(panel, shot);
        app.link = Some(link);
        app
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        egui_extras::install_image_loaders(ui.ctx());
        self.show(ui);
        self.screenshot(ui.ctx());
    }
}

/// An image of the part `name` painted into `rect`.
fn paint_part(ui: &Ui, name: &str, rect: Rect) {
    paint_part_rotated(ui, name, rect, None);
}

fn paint_part_rotated(ui: &Ui, name: &str, rect: Rect, rot: Option<(f32, Vec2)>) {
    let part = art::part(name).unwrap_or_else(|| panic!("missing art part {name}"));
    let mut image = Image::from_bytes(part.uri, part.svg).fit_to_exact_size(rect.size());
    if let Some((angle, origin)) = rot {
        image = image.rotate(angle, origin);
    }
    image.paint_at(ui, rect);
}

impl App {
    /// Draws the whole panel in the free area of `ui` and handles the input.
    pub fn show(&mut self, ui: &mut Ui) {
        let avail = ui.ctx().content_rect();
        ui.painter().rect_filled(avail, 0.0, BG);
        let view = View::fit(avail, ui.ctx().pixels_per_point());
        let now = ui.input(|i| i.time);
        self.panel.held = None;

        self.keyboard(ui);
        self.focus_click(ui, &view);

        for (name, x, y) in layout::backgrounds() {
            let (w, h) = part_size(name);
            paint_part(ui, name, view.rect(x, y, w, h));
        }
        for r in 0..4 {
            let (x, y, w, h) = layout::focus_bar(r);
            let name = if self.panel.focus == r {
                "focus_on"
            } else {
                "focus_off"
            };
            paint_part(ui, name, view.rect(x, y, w, h));
        }

        let ctls = std::mem::take(&mut self.ctls);
        for c in &ctls {
            self.control(ui, &view, c, now);
        }
        self.ctls = ctls;
        self.drum_strips(ui, &view);
        self.instrument_selection(ui, &view);

        // Send what was done, take what the engine did (the panel follows playback).
        if let Some(link) = &mut self.link {
            link.sync(&mut self.panel);
            ui.ctx().request_repaint_after(Duration::from_millis(16));
        }
    }

    fn keyboard(&mut self, ui: &Ui) {
        let (space, up, down) = ui.input(|i| {
            (
                i.key_pressed(Key::Space),
                i.key_pressed(Key::ArrowUp),
                i.key_pressed(Key::ArrowDown),
            )
        });
        if space {
            let id = if self.panel.transport.playing {
                "transport.stop"
            } else {
                "transport.play"
            };
            self.panel.press(id, 0.0);
        }
        // Up from row 1 goes to row 4 and Down from row 4 to row 1 (`sub_42D1D3`).
        if up {
            self.panel.focus = (self.panel.focus + 3) % 4;
        }
        if down {
            self.panel.focus = (self.panel.focus + 1) % 4;
        }
    }

    /// A click anywhere inside a row gives the row the focus (`sub_42ADBB`).
    fn focus_click(&mut self, ui: &Ui, view: &View) {
        let pressed = ui.input(|i| {
            i.pointer
                .button_pressed(egui::PointerButton::Primary)
                .then(|| i.pointer.press_origin())
                .flatten()
        });
        if let Some(p) = pressed {
            let p = view.to_panel(p);
            if let Some(r) = layout::row_at(p.x, p.y) {
                self.panel.focus = r;
            }
        }
    }

    fn control(&mut self, ui: &mut Ui, view: &View, c: &Ctl, now: f64) {
        let rect = view.rect(c.x, c.y, c.w, c.h);
        match c.kind.as_str() {
            "area" | "steps" => {}
            "knob" => self.knob(ui, view, c, rect),
            "fader" => self.fader(ui, view, c, rect),
            "display" => {
                let n = (c.w / 9.0) as usize;
                let frames = behavior::digit_frames(self.panel.display(&c.id), n);
                for (i, f) in frames.into_iter().enumerate() {
                    let x = c.x + c.w - 9.0 * (n - i) as f32;
                    paint_part(ui, f, view.rect(x, c.y, 9.0, 16.0));
                }
            }
            "led" | "meter" => {
                let name = self.panel.part_name(c, false, now);
                paint_part(ui, &name, rect);
            }
            kind => {
                let r = self.hit(ui, c, rect);
                let down = r.is_pointer_button_down_on();
                match kind {
                    "repeat" | "wind" => self.repeat(ui, c, &r),
                    "switch" | "toggle" | "radio" | "trigger" | "grid" => {
                        if self.first_press(ui, c, &r) {
                            self.panel.press(&c.id, now);
                        }
                    }
                    "hswitch" | "vswitch" => self.two_position(ui, view, c, &r),
                    _ => {}
                }
                let name = self.panel.part_name(c, down, now);
                paint_part(ui, &name, rect);
            }
        }
    }

    fn hit(&self, ui: &mut Ui, c: &Ctl, rect: Rect) -> Response {
        // The kind is part of the key: the 303's accent knob and accent flag button share
        // the layout id `tb303_N.accent`.
        ui.interact(
            rect,
            egui::Id::unique(("hit", &c.kind, &c.id)),
            Sense::click_and_drag(),
        )
    }

    fn gesture(ui: &Ui, c: &Ctl) -> (egui::Id, Gesture) {
        let id = egui::Id::unique(("gesture", &c.kind, &c.id));
        (id, ui.data(|d| d.get_temp(id)).unwrap_or_default())
    }

    /// True on the frame the mouse goes down on the control (the action runs at mouse down).
    fn first_press(&self, ui: &Ui, c: &Ctl, r: &Response) -> bool {
        let (id, mut g) = Self::gesture(ui, c);
        let down = r.is_pointer_button_down_on();
        let first = down && !g.down;
        g.down = down;
        ui.data_mut(|d| d.insert_temp(id, g));
        first
    }

    /// Repeat and wind buttons: one step at mouse down, then the held-button schedule.
    fn repeat(&mut self, ui: &Ui, c: &Ctl, r: &Response) {
        let (id, mut g) = Self::gesture(ui, c);
        let down = r.is_pointer_button_down_on();
        let time = ui.input(|i| i.time);
        if down && !g.down {
            g.t0 = time;
            g.taken = 0;
        }
        g.down = down;
        if down {
            let t = ((time - g.t0) * 1000.0) as u32;
            let total = 1 + behavior::repeat_extra_steps(t);
            while g.taken < total {
                self.panel.arrow(&c.id);
                g.taken += 1;
            }
            ui.ctx().request_repaint_after(Duration::from_millis(16));
        }
        ui.data_mut(|d| d.insert_temp(id, g));
    }

    /// Where the pointer is, in panel pixels.
    fn pointer(ui: &Ui, view: &View) -> Option<Pos2> {
        ui.input(|i| i.pointer.latest_pos())
            .map(|p| view.to_panel(p))
    }

    fn press_origin(ui: &Ui, view: &View) -> Option<Pos2> {
        ui.input(|i| i.pointer.press_origin().or(i.pointer.latest_pos()))
            .map(|p| view.to_panel(p))
    }

    /// The horizontal and vertical two-position switches: the half the pointer is in.
    fn two_position(&mut self, ui: &Ui, view: &View, c: &Ctl, r: &Response) {
        if !r.is_pointer_button_down_on() {
            return;
        }
        let Some(p) = Self::pointer(ui, view) else {
            return;
        };
        let v = if c.kind == "hswitch" {
            behavior::switch_value(p.x - c.x, c.w)
        } else {
            behavior::switch_value(p.y - c.y, c.h)
        };
        if self.panel.switch(&c.id) != v {
            self.panel.set_switch(&c.id, v);
        }
    }

    fn knob(&mut self, ui: &mut Ui, view: &View, c: &Ctl, rect: Rect) {
        let r = self.hit(ui, c, rect);
        let (id, mut g) = Self::gesture(ui, c);
        let down = r.is_pointer_button_down_on();
        if r.hovered() || down {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
        }
        if down {
            self.panel.held = Some(c.id.clone());
            if !g.down {
                // Mouse down grabs the knob where it is (no jump); Shift now means fine.
                g.p0 = self.panel.knob(&c.id);
                g.y0 = Self::press_origin(ui, view).map_or(0.0, |p| p.y);
                g.fine = ui.input(|i| i.modifiers.shift);
            }
            if let Some(p) = Self::pointer(ui, view) {
                let v = behavior::knob_drag(g.p0, g.y0, p.y, behavior::knob_span(&c.id), g.fine);
                self.panel.set_knob(&c.id, v);
            }
        }
        g.down = down;
        ui.data_mut(|d| d.insert_temp(id, g));

        let k = art::knob_design(&c.part);
        let p = self.panel.knob(&c.id);
        paint_part(ui, &format!("{}_body", k.name), rect);
        if k.ring_r > 0.0 {
            let bipolar = c.name() == "pan";
            let a0 = if bipolar { 0.0 } else { -135.0 };
            paint_arc(ui, view, c, k, a0, behavior::knob_angle(p));
        }
        let angle = behavior::knob_angle(p).to_radians();
        let pivot = Vec2::new(k.cx / k.w, k.cy / k.h);
        paint_part_rotated(
            ui,
            &format!("{}_pointer", k.name),
            rect,
            Some((angle, pivot)),
        );
    }

    fn fader(&mut self, ui: &mut Ui, view: &View, c: &Ctl, rect: Rect) {
        let r = self.hit(ui, c, rect);
        let (id, mut g) = Self::gesture(ui, c);
        let down = r.is_pointer_button_down_on();
        if down {
            self.panel.held = Some(c.id.clone());
            if !g.down {
                let y = Self::press_origin(ui, view).map_or(0.0, |p| p.y) - c.y;
                let p = self.panel.fader(&c.id);
                if !behavior::fader_on_handle(p, y) {
                    self.panel.set_fader(&c.id, behavior::fader_jump(y));
                }
                g.p0 = self.panel.fader(&c.id);
                g.y0 = y + c.y;
            }
            if let Some(p) = Self::pointer(ui, view) {
                self.panel
                    .set_fader(&c.id, behavior::fader_drag(g.p0, g.y0, p.y));
            }
        }
        g.down = down;
        ui.data_mut(|d| d.insert_temp(id, g));
        paint_part(ui, "fader_track", rect);
        let top = behavior::fader_handle_top(self.panel.fader(&c.id));
        paint_part(ui, "fader_handle", view.rect(c.x, c.y + top, 15.0, 11.0));
    }

    /// The step strips of the two drum machines: press sets the step to the opposite (808)
    /// or the cycled value (909) and a drag paints every crossed step with that value.
    fn drum_strips(&mut self, ui: &mut Ui, view: &View) {
        for is808 in [true, false] {
            let (origin, x0, y0, w, h) = if is808 {
                (layout::TR808_ORIGIN, 16.0, 91.0, 256.0, 17.0)
            } else {
                (layout::TR909_ORIGIN, 29.0, 86.0, 336.0, 16.0)
            };
            let rect = view.rect(origin.0 + x0, origin.1 + y0, w, h);
            let name = if is808 { "strip808" } else { "strip909" };
            let r = ui.interact(
                rect,
                egui::Id::unique(("hit", name)),
                Sense::click_and_drag(),
            );
            let gid = egui::Id::unique(("gesture", name));
            let mut g: Gesture = ui.data(|d| d.get_temp(gid)).unwrap_or_default();
            let down = r.is_pointer_button_down_on();
            if down {
                let Some(p) = (if g.down {
                    Self::pointer(ui, view)
                } else {
                    Self::press_origin(ui, view)
                }) else {
                    continue;
                };
                let x = p.x - origin.0;
                let step = if is808 {
                    behavior::tr808_step(x)
                } else {
                    behavior::tr909_step(x)
                };
                if let Some(step) = step {
                    let step = step as i32;
                    if !g.down {
                        g.new_value = if is808 {
                            u8::from(!self.panel.steps808[self.panel.inst808][step as usize])
                        } else {
                            let m = ui.input(|i| Mods {
                                ctrl: i.modifiers.ctrl,
                                alt: i.modifiers.alt,
                            });
                            self.panel.step909_new_value(step as usize, m)
                        };
                        g.last_step = step;
                    }
                    // Every step crossed since the last frame.
                    let (a, b) = (g.last_step.min(step), g.last_step.max(step));
                    for s in a..=b {
                        if is808 {
                            self.panel.set_step808(s as usize, g.new_value != 0);
                        } else {
                            self.panel.set_step909(s as usize, g.new_value);
                        }
                    }
                    g.last_step = step;
                }
            }
            g.down = down;
            ui.data_mut(|d| d.insert_temp(gid, g));
        }
        // The step parts are drawn here, after the hit tests, so a press shows at once.
        let ctls = std::mem::take(&mut self.ctls);
        for c in ctls.iter().filter(|c| c.kind == "steps") {
            let name = self.panel.part_name(c, false, 0.0);
            paint_part(ui, &name, view.rect(c.x, c.y, c.w, c.h));
        }
        self.ctls = ctls;
    }

    /// The legends, name rows and AC area select the instrument; dragging keeps selecting.
    /// Also draws the selection frames.
    fn instrument_selection(&mut self, ui: &mut Ui, view: &View) {
        let (o8, o9) = (layout::TR808_ORIGIN, layout::TR909_ORIGIN);
        /// (is the 808, section origin, [x y w h] in the section, name)
        type Area = (bool, (f32, f32), [f32; 4], &'static str);
        let areas: [Area; 5] = [
            (true, o8, [10.0, 71.0, 264.0, 14.0], "legend808"),
            (true, o8, [78.0, 43.0, 106.0, 12.0], "upper808"),
            (false, o9, [4.0, 4.0, 362.0, 14.0], "legend909"),
            (false, o9, [26.0, 70.0, 340.0, 15.0], "names909"),
            (false, o9, [4.0, 44.0, 22.0, 18.0], "ac909"),
        ];
        for (is808, o, [x, y, w, h], name) in areas {
            let rect = view.rect(o.0 + x, o.1 + y, w, h);
            let r = ui.interact(
                rect,
                egui::Id::unique(("hit", name)),
                Sense::click_and_drag(),
            );
            if !r.is_pointer_button_down_on() {
                continue;
            }
            let Some(p) = Self::pointer(ui, view) else {
                continue;
            };
            let (sx, sy) = (p.x - o.0, p.y - o.1);
            let hit = if is808 {
                behavior::tr808_hit(sx, sy)
            } else {
                behavior::tr909_hit(sx, sy)
            };
            if let Some(i) = hit {
                self.panel.select_instrument(is808, i);
            }
        }
        // The selection frames.
        let i = self.panel.inst808;
        let upper =
            (3..=7).contains(&i) && self.panel.switch(&format!("tr808.slot{i}_switch")) == 0;
        let (fx, fy) = if upper {
            (o8.0 + 78.0 + 22.0 * (i - 3) as f32, o8.1 + 42.0)
        } else {
            (o8.0 + 10.0 + 22.0 * i as f32, o8.1 + 71.0)
        };
        paint_part(ui, "sel_22x14", view.rect(fx, fy, 22.0, 14.0));
        let i = self.panel.inst909;
        let (name, x, y, w, h) = match i {
            0 => ("sel_22x18", 4.0, 44.0, 22.0, 18.0),
            1..=5 => ("sel_42x14", 26.0 + 42.0 * (i - 1) as f32, 4.0, 42.0, 14.0),
            _ => ("sel_21x14", 236.0 + 21.0 * (i - 6) as f32, 4.0, 21.0, 14.0),
        };
        paint_part(ui, name, view.rect(o9.0 + x, o9.1 + y, w, h));
    }

    /// With `--screenshot`: after a few frames (the SVGs are loaded and rasterized) save the
    /// window to the file and quit.
    fn screenshot(&mut self, ctx: &egui::Context) {
        let Some(shot) = &mut self.shot else {
            return;
        };
        shot.frames += 1;
        ctx.request_repaint();
        if shot.frames == 8 && !shot.requested {
            shot.requested = true;
            let path = shot.path.clone();
            ctx.request_screenshot(Box::new(move |image: std::sync::Arc<egui::ColorImage>| {
                save_png(&path, &image).expect("writing the screenshot");
                std::process::exit(0);
            }));
        }
    }
}

fn part_size(name: &str) -> (f32, f32) {
    match name {
        "wood" => (16.0, 510.0),
        "bg_transport" => (592.0, 50.0),
        "bg_pattern" | "bg_mixer" => (70.0, 115.0),
        "bg_tb303" | "bg_tr808" | "bg_tr909" => (366.0, 115.0),
        "bg_master" => (82.0, 98.0),
        "bg_pcf" => (82.0, 111.0),
        "bg_delay" | "bg_comp" => (82.0, 93.0),
        "bg_dist" => (82.0, 65.0),
        _ => panic!("no size for background {name}"),
    }
}

/// The value arc of a knob from angle `a0` to `a` degrees, painted over the ring track.
fn paint_arc(ui: &Ui, view: &View, c: &Ctl, k: &KnobDesign, a0: f32, a: f32) {
    if (a - a0).abs() < 0.5 {
        return;
    }
    let center = view.origin + Vec2::new(c.x + k.cx, c.y + k.cy) * view.scale;
    let r = k.ring_r * view.scale;
    let n = (((a - a0).abs() / 3.0).ceil() as usize).max(2);
    let points: Vec<Pos2> = (0..=n)
        .map(|i| {
            let t = (a0 + (a - a0) * i as f32 / n as f32).to_radians();
            center + Vec2::new(t.sin(), -t.cos()) * r
        })
        .collect();
    let color = Color32::from_rgb(k.arc[0], k.arc[1], k.arc[2]);
    ui.painter().add(Shape::line(
        points,
        Stroke::new(k.ring_w * view.scale, color),
    ));
}

fn save_png(path: &PathBuf, image: &egui::ColorImage) -> Result<(), image::ImageError> {
    let [w, h] = image.size;
    let mut raw = Vec::with_capacity(w * h * 4);
    for p in &image.pixels {
        raw.extend_from_slice(&p.to_array());
    }
    image::save_buffer(path, &raw, w as u32, h as u32, image::ColorType::Rgba8)
}
