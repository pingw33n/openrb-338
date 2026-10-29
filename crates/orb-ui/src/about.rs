//! Help > About (task-102, laid out in task-115): the icon, the wordmark, the version and the
//! project link in a small centered window. The version string is computed by `build.rs`
//! (`crate::version`).

use eframe::egui::{self, Context, Key, Modifiers, RichText};

use crate::app::App;
use crate::art;
use crate::palette::INK_DIM;

/// The wordmark part of the panel (`assets/ui`), its native size and the scale it is shown at.
pub const LOGO_PART: &str = "logo";
const LOGO_W: f32 = 134.0;
const LOGO_H: f32 = 48.0;
const LOGO_SCALE: f32 = 2.0;
/// The app icon part and the size it is shown at.
pub const ICON_PART: &str = "app_icon";
const ICON_SIZE: f32 = 64.0;
/// The content area below the title bar and the x of the version and the link. The mockup's
/// window is 360 wide; the link needs about 20 more at this alignment.
const CONTENT_W: f32 = 380.0;
const CONTENT_H: f32 = 156.0;
const TEXT_X: f32 = 108.0;

/// The name of the app.
pub const NAME: &str = "OpenRB-338";
/// The project page.
pub const URL: &str = "https://github.com/pingw33n/open-rb-338";
/// The version: `YYYY.MMDD.HHMM (<[dev-]hash[-dirty]>)` (`src/version.rs`).
pub const VERSION: &str = env!("ORB_UI_VERSION");

/// The id of the About window of an open (`App::about_serial`).
pub fn about_window_id(serial: u32) -> egui::Id {
    egui::Id::unique(("orb_about_window", serial))
}

impl App {
    /// Help > About.
    pub fn show_about(&mut self) {
        if !self.about_open {
            // a new id: no stored place, so the window opens at its default, the center
            self.about_serial += 1;
        }
        self.about_open = true;
    }

    /// The id of the About window now (for tests, to look its place up).
    pub fn about_id(&self) -> egui::Id {
        about_window_id(self.about_serial)
    }

    /// Whether the About window is open.
    pub fn about_is_open(&self) -> bool {
        self.about_open
    }

    /// Draws the About window when it is open. Escape and the close button close it.
    pub fn about_ui(&mut self, ctx: &Context) {
        if !self.about_open {
            return;
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            self.about_open = false;
            return;
        }
        let mut open = true;
        // centered the way File > Info is (info.rs): pivot, default place, an id per open
        egui::Window::new(format!("About {NAME}"))
            .id(self.about_id())
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .frame(egui::Frame::window(&ctx.global_style()).inner_margin(0.0))
            .show(ctx, |ui| {
                // The layout of the mockup session (task-115), points from the top left of
                // the content: the icon, the wordmark at 2x, and under its subtitle the
                // version and the link, left-aligned. The images are SVG, so the loader
                // rasterizes them at the window's pixel density.
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(CONTENT_W, CONTENT_H), egui::Sense::hover());
                let at = |x: f32, y: f32| rect.min + egui::vec2(x, y);
                let image = |ui: &mut egui::Ui, name: &str, pos: egui::Pos2, size: egui::Vec2| {
                    let part = art::part(name).expect("the part");
                    let r = egui::Rect::from_min_size(pos, size);
                    egui::Image::from_bytes(part.uri, part.svg)
                        .fit_to_exact_size(size)
                        .paint_at(ui, r);
                };
                image(
                    ui,
                    ICON_PART,
                    at(18.0, 18.0),
                    egui::vec2(ICON_SIZE, ICON_SIZE),
                );
                let logo_size = egui::vec2(LOGO_W, LOGO_H) * LOGO_SCALE;
                image(ui, LOGO_PART, at(92.0, 14.0), logo_size);
                let line = |ui: &mut egui::Ui, y: f32, f: &dyn Fn(&mut egui::Ui)| {
                    let r = egui::Rect::from_min_size(at(TEXT_X, y), egui::vec2(CONTENT_W, 18.0));
                    ui.scope_builder(egui::UiBuilder::new().max_rect(r), f);
                };
                line(ui, 104.0, &|ui| {
                    ui.label(RichText::new(format!("Version {VERSION}")).color(INK_DIM));
                });
                line(ui, 126.0, &|ui| {
                    ui.hyperlink_to(URL, URL);
                });
            });
        if !open {
            self.about_open = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The About window shows the panel's own wordmark part, at its aspect ratio.
    #[test]
    fn the_logo_part_is_the_panels_wordmark() {
        let part = art::part(LOGO_PART).expect("the logo part");
        let svg = std::str::from_utf8(part.svg).unwrap();
        let want =
            format!("width=\"{LOGO_W}\" height=\"{LOGO_H}\" viewBox=\"0 0 {LOGO_W} {LOGO_H}\"");
        assert!(svg.contains(&want), "{want} in the part's header");
        // the panel draws the same part over its transport background
        assert!(
            crate::layout::backgrounds()
                .iter()
                .any(|b| b.0 == LOGO_PART)
        );
    }
}
