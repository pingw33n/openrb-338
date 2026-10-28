//! The theme is always dark (task-127): the colors of the windows assume a dark background, so
//! a light system theme (a GTK light theme, the macOS light appearance) must not switch egui to
//! its light visuals, where the Song Information values were pale grey on near-white.

use eframe::egui::{self, Color32, RawInput, Rect, Theme, Vec2};
use rb_ui::app::App;
use rb_ui::layout::{PANEL_H, PANEL_W};
use rb_ui::menu::MENU_H;
use rb_ui::model::Panel;
use rb_ui::palette::{BG, INK, INK_DIM};

/// The relative luminance (WCAG) of a color.
fn luminance(c: Color32) -> f64 {
    let lin = |v: u8| {
        let v = f64::from(v) / 255.0;
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(c.r()) + 0.7152 * lin(c.g()) + 0.0722 * lin(c.b())
}

/// The WCAG contrast ratio of two colors.
fn contrast(a: Color32, b: Color32) -> f64 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// A frame; the window themes it asks the native window for (egui's `sync_window_theme`: on
/// Windows winit then sets the title bar's dark mode).
fn frame(
    ctx: &egui::Context,
    app: &mut App,
    system: Option<Theme>,
    t: f64,
) -> Vec<egui::SystemTheme> {
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(
            egui::Pos2::ZERO,
            Vec2::new(PANEL_W, PANEL_H + MENU_H),
        )),
        time: Some(t),
        system_theme: system,
        ..Default::default()
    };
    let mut out = ctx.run_ui(input, |ui| app.show(ui));
    out.textures_delta.clear();
    out.viewport_output
        .values()
        .flat_map(|v| v.commands.iter())
        .filter_map(|c| match c {
            egui::ViewportCommand::SetTheme(t) => Some(*t),
            _ => None,
        })
        .collect()
}

fn check(ctx: &egui::Context) {
    let v = ctx.global_style().visuals.clone();
    assert!(v.dark_mode, "light visuals under a light system theme");
    assert_eq!(ctx.theme(), Theme::Dark);
    // the fills under the texts of the windows: the window, egui's panel and the panel
    // background the windows sit on
    for (what, fill) in [
        ("window", v.window_fill),
        ("panel", v.panel_fill),
        ("BG", BG),
    ] {
        let ink = contrast(INK, fill);
        let dim = contrast(INK_DIM, fill);
        assert!(ink >= 7.0, "{what} fill {fill:?}: INK contrast {ink}");
        assert!(dim >= 4.5, "{what} fill {fill:?}: INK_DIM contrast {dim}");
    }
}

#[test]
fn light_system_theme_still_draws_dark() {
    let ctx = egui::Context::default();
    egui_extras::install_image_loaders(&ctx);
    let mut app = App::new(Panel::demo(), None);
    app.menu = true;
    app.show_info();
    // the system reports light from the first frame
    let mut window = frame(&ctx, &mut app, Some(Theme::Light), 0.0);
    window.extend(frame(&ctx, &mut app, Some(Theme::Light), 0.1));
    check(&ctx);
    // a system theme that changes at run time does not alter the visuals: the preference is
    // Dark, and the window theme is set to Dark (winit then sends no theme event at all)
    window.extend(frame(&ctx, &mut app, Some(Theme::Dark), 0.2));
    window.extend(frame(&ctx, &mut app, Some(Theme::Light), 0.3));
    check(&ctx);
    // no system theme reported (the platform has none)
    window.extend(frame(&ctx, &mut app, None, 0.4));
    check(&ctx);
    // the native window (the title bar on Windows) is told dark, never light or the system's
    assert_eq!(window.last(), Some(&egui::SystemTheme::Dark), "{window:?}");
    assert!(
        window.iter().all(|t| *t == egui::SystemTheme::Dark),
        "{window:?}"
    );
}
