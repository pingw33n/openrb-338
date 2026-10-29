//! Help > About (task-102): the logo, the name, the version and the project link in a small
//! centered window. The version string is computed by `build.rs` (`crate::version`).

use eframe::egui::{self, Context, Key, Modifiers, RichText};

use crate::app::App;
use crate::art;
use crate::palette::{INK, INK_DIM};

/// The wordmark part of the panel (`assets/ui`), its native size and the scale it is shown at.
pub const LOGO_PART: &str = "logo";
const LOGO_W: f32 = 134.0;
const LOGO_H: f32 = 48.0;
const LOGO_SCALE: f32 = 2.0;

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
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    // the panel's wordmark, 2x its native size; it is SVG, so the loader
                    // rasterizes it at the window's pixel density
                    let logo = art::part(LOGO_PART).expect("logo part");
                    ui.add(
                        egui::Image::from_bytes(logo.uri, logo.svg)
                            .fit_to_exact_size(egui::vec2(LOGO_W, LOGO_H) * LOGO_SCALE),
                    );
                    ui.add_space(6.0);
                    ui.label(RichText::new(NAME).size(20.0).strong().color(INK));
                    ui.label(RichText::new(format!("Version {VERSION}")).color(INK_DIM));
                    ui.add_space(6.0);
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
