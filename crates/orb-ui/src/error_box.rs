//! The error message box (task-130): every error the app reports shows in a modal box over the
//! panel, closed by its OK button, its close button, Escape or Enter, not by a click outside. The
//! box blocks the input of everything else (the panel, the menu, the shortcuts and the other
//! windows). Several errors queue: one box at a time, in the order they came. What was done
//! (`App::status`) stays on the status line.

use eframe::egui::{
    self, Align, Color32, Context, Event, Frame, Id, Key, Label, Layout, Margin, Order, RichText,
    Sense, Stroke,
};

use crate::app::App;
use crate::palette::{AMBER, INK, INK_DIM, SLATE_DARK, SLATE_LINE};

/// The box's title.
pub const TITLE: &str = "Error";
/// The label of the box's one button.
pub const OK: &str = "OK";

/// The widest the message is wrapped.
const WIDTH: f32 = 380.0;

impl App {
    /// Where the OK button was drawn last (for tests, to click it).
    pub fn error_ok_rect(&self) -> Option<egui::Rect> {
        self.error_ok
    }

    /// Where the message text was drawn last (for tests, to select it).
    pub fn error_text_rect(&self) -> Option<egui::Rect> {
        self.error_text
    }

    /// Where the Copy button was drawn last (for tests).
    pub fn error_copy_rect(&self) -> Option<egui::Rect> {
        self.error_copy
    }

    /// Where the close button (the X of the title bar) was drawn last (for tests).
    pub fn error_close_rect(&self) -> Option<egui::Rect> {
        self.error_close
    }

    /// Reports an error: it is shown in the message box when the ones before it are closed.
    /// This is for the error of a one-time action the user asked for (Open, Save, a mod
    /// choice, Refresh List, the start-up song). A message that is already in the box or
    /// waiting is not queued again: the user is never spammed with the same box.
    pub fn fail(&mut self, message: impl Into<String>) {
        let message = message.into();
        // inside an action (`App::action`) the errors are collected into one box
        if let Some(batch) = &mut self.batch {
            batch.push(message);
        } else if !self.errors.contains(&message) {
            self.errors.push_back(message);
        }
    }

    /// Runs one action the user asked for (Open, Demo Song, a mod choice, Save, Export, Refresh
    /// List, the start-up song). The errors it reports, however many, are one box when it ends
    /// ([`App::fail_many`]): an Open with a missing mod and a refused load is one box, not two.
    /// An action inside an action is part of the outer one.
    pub fn action<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        if self.batch.is_some() {
            return f(self);
        }
        self.batch = Some(Vec::new());
        let r = f(self);
        let batch = self.batch.take().unwrap_or_default();
        self.fail_many(batch);
        r
    }

    /// The errors of one action, in one box: the distinct messages, one per line, each with
    /// how many times it came when that is more than once (many files failing in one Refresh
    /// List give one box that lists them, not one box per file).
    pub fn fail_many(&mut self, messages: impl IntoIterator<Item = String>) {
        let mut counted: Vec<(String, usize)> = Vec::new();
        for m in messages {
            match counted.iter_mut().find(|(k, _)| *k == m) {
                Some((_, n)) => *n += 1,
                None => counted.push((m, 1)),
            }
        }
        let lines: Vec<String> = counted
            .into_iter()
            .map(|(m, n)| if n > 1 { format!("{m} ({n} times)") } else { m })
            .collect();
        if !lines.is_empty() {
            self.fail(lines.join("\n"));
        }
    }

    /// A notification of an error that can repeat or come in the background (a failing audio
    /// output): never a box. The status line shows the latest one; one notice per `kind`, with
    /// "(repeated N times)" when it came again, so that they do not pile up.
    pub fn notify(&mut self, kind: &str, message: impl Into<String>) {
        let n = self.notified.entry(kind.to_owned()).or_insert(0);
        *n += 1;
        let message = message.into();
        self.status = Some(if *n > 1 {
            format!("{message} (repeated {n} times)")
        } else {
            message
        });
    }

    /// The error the box shows now (the oldest not closed), if any.
    pub fn error(&self) -> Option<&str> {
        self.errors.front().map(String::as_str)
    }

    /// Whether the box is open: the input of everything else is blocked.
    pub fn error_open(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Closes the box (OK): the next error, if any, shows.
    pub fn dismiss_error(&mut self) {
        self.errors.pop_front();
        self.clear_rects();
    }

    /// The places of the box's parts are not known when it is not there.
    fn clear_rects(&mut self) {
        self.error_ok = None;
        self.error_close = None;
        self.error_copy = None;
        self.error_text = None;
    }

    /// Draws the box when there is an error, above every other window. It is closed by OK
    /// (which has the focus, so Space presses it), by the close button of its title bar, by
    /// Escape and by Enter (whatever has the focus); a click outside does nothing, so that a
    /// misclick does not dismiss an error. The keys of the frame are taken away after the box has
    /// seen them, so that the windows drawn after it (the unsaved prompt, the info windows) do
    /// not act on them.
    pub fn error_box_ui(&mut self, ctx: &Context) {
        let Some(message) = self.error().map(str::to_owned) else {
            self.clear_rects();
            return;
        };
        let frame = Frame::new()
            .fill(SLATE_DARK)
            .stroke(Stroke::new(1.0, SLATE_LINE))
            .inner_margin(Margin::same(12))
            .corner_radius(3.0);
        // above the windows and the other modals (the prompt, the browser): Tooltip is the
        // highest order that is not a debug layer
        let id = Id::unique("orb_error_box");
        let area = egui::Modal::default_area(id)
            .order(Order::Tooltip)
            .fade_in(false);
        // Escape and Enter (whatever has the focus) close it, as OK does
        let mut ok = ctx.input(|i| {
            i.events.iter().any(|e| {
                matches!(
                    e,
                    Event::Key {
                        key: Key::Escape | Key::Enter,
                        pressed: true,
                        repeat: false,
                        ..
                    }
                )
            })
        });
        // (the response of the box is not looked at: a click on the backdrop closes nothing)
        egui::Modal::new(id)
            .area(area)
            .frame(frame)
            .backdrop_color(Color32::from_black_alpha(170))
            .show(ctx, |ui| {
                ui.set_width(WIDTH);
                // the title bar: the title, and the close button at the right
                ui.horizontal(|ui| {
                    ui.label(RichText::new(TITLE).color(AMBER).strong().size(15.0));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let (rect, r) =
                            ui.allocate_exact_size(egui::vec2(18.0, 18.0), Sense::click());
                        let color = if r.hovered() { AMBER } else { INK_DIM };
                        let c = rect.shrink(5.0);
                        let stroke = Stroke::new(1.5, color);
                        ui.painter()
                            .line_segment([c.left_top(), c.right_bottom()], stroke);
                        ui.painter()
                            .line_segment([c.left_bottom(), c.right_top()], stroke);
                        self.error_close = Some(rect);
                        ok |= r.clicked();
                    });
                });
                ui.add_space(6.0);
                // selectable: the text can be copied
                // (a long list scrolls instead of growing past the window)
                egui::ScrollArea::vertical()
                    .max_height((ctx.content_rect().height() - 200.0).max(80.0))
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        let r = ui.add(
                            Label::new(RichText::new(&message).color(INK))
                                .wrap()
                                .selectable(true),
                        );
                        self.error_text = Some(r.rect);
                    });
                let more = self.errors.len() - 1;
                if more > 0 {
                    ui.add_space(4.0);
                    ui.label(RichText::new(format!("{more} more")).color(INK_DIM).small());
                }
                ui.add_space(8.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let r = ui.add(egui::Button::new(OK).min_size(egui::vec2(72.0, 0.0)));
                    self.error_ok = Some(r.rect);
                    // Escape takes the focus away: OK gets it back
                    if ui.memory(|m| m.focused().is_none()) {
                        r.request_focus();
                    }
                    ok |= r.clicked();
                    // Copy: the whole message to the clipboard; the box stays
                    let c = ui.add(egui::Button::new("Copy").min_size(egui::vec2(72.0, 0.0)));
                    self.error_copy = Some(c.rect);
                    if c.clicked() {
                        ui.ctx().copy_text(message.clone());
                    }
                });
            });
        // no key and no paste or cut reaches what is under the box (Copy stays: it acts on the
        // selected text of the box)
        ctx.input_mut(|i| {
            i.events.retain(|e| {
                !matches!(
                    e,
                    Event::Key { .. } | Event::Text(_) | Event::Paste(_) | Event::Cut
                )
            })
        });
        // a menu that was open when the error came is closed
        egui::Popup::close_all(ctx);
        if ok {
            self.dismiss_error();
        }
    }
}
