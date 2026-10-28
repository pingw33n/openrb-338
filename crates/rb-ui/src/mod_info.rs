//! Mods > Info (task-106): the splash of the kit that plays at its 2000-era physical size, the
//! mod's INFO chunk (docs/rbm.md section 2) and facts about its file, in a centered window.
//!
//! **The size of the splash.** Every known mod's splash.jpg is 256 x 256 px. A 17-inch CRT of
//! 2000 had about 16 inches (406 mm) viewable at 4:3, i.e. 325 x 244 mm, and typically ran at
//! 1024 x 768: 1024 px / 325 mm = 3.15 px per mm = about 80 px per inch = [`MM_PER_PX`] =
//! 0.317 mm per px. So a 256 px splash was 256 x 0.317 = 81.2 mm (3.2 in) wide. The window shows
//! it that wide on the screen it is on:
//!
//! - **Known display size** (macOS: `CGDisplayScreenSize`, in mm, with the pixel width of the
//!   display's current mode): `points = image_px x MM_PER_PX x (display_px / display_mm) /
//!   pixels_per_point`.
//! - **Fallback** when the size is not known (other systems, or a display that reports none):
//!   [`FALLBACK_PPI_MAC`] = 110 logical points per inch on macOS, [`FALLBACK_PPI_OTHER`] = 96
//!   elsewhere: `points = image_px x MM_PER_PX x ppi / 25.4`. The window says which was used.
//!
//! When the physical size does not fit the window, the splash is scaled down to fit and the
//! window says so ([`splash_width`]).

use eframe::egui::{self, Color32, Context, Key, Modifiers, RichText};
use rb_cli::mods::{Kit, STANDARD};

use crate::app::App;
use crate::info::link;
use crate::session::ModDetails;

/// The width of a value in the facts' grid; a longer text wraps.
const VALUE_WIDTH: f32 = 300.0;
const INK: Color32 = Color32::from_rgb(0xe9, 0xed, 0xf2);
const INK_DIM: Color32 = Color32::from_rgb(0x9a, 0xa7, 0xb8);

/// Millimetres per pixel of a 17-inch CRT of 2000 at 1024 x 768 (the derivation is above).
pub const MM_PER_PX: f32 = 0.317;
/// Logical points per inch assumed on macOS when the display's size is not known.
pub const FALLBACK_PPI_MAC: f32 = 110.0;
/// Logical points per inch assumed elsewhere when the display's size is not known.
pub const FALLBACK_PPI_OTHER: f32 = 96.0;
const MM_PER_INCH: f32 = 25.4;

/// How the physical size of the splash was found.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Basis {
    /// The display reported this many pixels per millimetre.
    Measured(f32),
    /// The display's size is not known: this many logical points per inch were assumed.
    Assumed(f32),
}

/// The fallback points per inch of this system.
pub fn fallback_ppi() -> f32 {
    if cfg!(target_os = "macos") {
        FALLBACK_PPI_MAC
    } else {
        FALLBACK_PPI_OTHER
    }
}

/// The width in logical points at which an `image_px` wide picture is physically as large as
/// it was on the CRT: with a measured `px_per_mm` of the display (physical pixels) and the
/// window's `pixels_per_point`, or else with the assumed points per inch.
pub fn physical_width(image_px: f32, basis: Basis, pixels_per_point: f32) -> f32 {
    let mm = image_px * MM_PER_PX;
    match basis {
        Basis::Measured(px_per_mm) => mm * px_per_mm / pixels_per_point,
        Basis::Assumed(ppi) => mm * ppi / MM_PER_INCH,
    }
}

/// The width the splash is drawn at and whether that is less than its physical width: the
/// physical one, or `max_width` when that is smaller.
pub fn splash_width(physical: f32, max_width: f32) -> (f32, bool) {
    if physical > max_width {
        (max_width, true)
    } else {
        (physical, false)
    }
}

/// The size of a file for the window: bytes, with KB or MB (1024 based) from 1 KB up.
pub fn format_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    let b = bytes as f64;
    if bytes < 1024 {
        format!("{bytes} bytes")
    } else if b < KB * KB {
        format!("{bytes} bytes ({:.1} KB)", b / KB)
    } else {
        format!("{bytes} bytes ({:.2} MB)", b / (KB * KB))
    }
}

/// The version of a mod as the window shows it.
pub fn format_version(v: Option<(u8, u8, u8)>) -> String {
    match v {
        Some((a, b, c)) => format!("{a}.{b}.{c}"),
        None => "the app's version".into(),
    }
}

/// The physical size of a display as the system reports it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DisplaySize {
    /// The width in millimetres.
    pub mm: f64,
    /// The width in physical pixels of the display's current mode.
    pub px: f64,
}

impl DisplaySize {
    pub fn px_per_mm(&self) -> Option<f32> {
        (self.mm > 0.0 && self.px > 0.0).then(|| (self.px / self.mm) as f32)
    }
}

#[cfg(target_os = "macos")]
mod display {
    //! CoreGraphics, by a minimal binding (no crate): `CGDisplayScreenSize` is the EDID size of
    //! the display in mm (0 when unknown).
    use super::DisplaySize;
    use std::ffi::c_void;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGPoint {
        x: f64,
        y: f64,
    }
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGSize {
        width: f64,
        height: f64,
    }

    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGMainDisplayID() -> u32;
        fn CGGetDisplaysWithPoint(
            point: CGPoint,
            max: u32,
            displays: *mut u32,
            count: *mut u32,
        ) -> i32;
        fn CGDisplayScreenSize(display: u32) -> CGSize;
        fn CGDisplayCopyDisplayMode(display: u32) -> *mut c_void;
        fn CGDisplayModeGetPixelWidth(mode: *mut c_void) -> usize;
        fn CGDisplayModeRelease(mode: *mut c_void);
    }

    /// The display with the global point (in points, top-left origin: the window's outer
    /// position divided by the pixels per point), else the main display.
    pub fn at(x: f64, y: f64) -> Option<DisplaySize> {
        let mut id = 0u32;
        let mut n = 0u32;
        // SAFETY: both out pointers point to live locals, and max is 1.
        let err = unsafe { CGGetDisplaysWithPoint(CGPoint { x, y }, 1, &mut id, &mut n) };
        if err != 0 || n == 0 {
            // SAFETY: no arguments.
            id = unsafe { CGMainDisplayID() };
        }
        size_of(id)
    }

    fn size_of(id: u32) -> Option<DisplaySize> {
        // SAFETY: plain value calls on a display id; the mode is released once.
        unsafe {
            let mm = CGDisplayScreenSize(id).width;
            let mode = CGDisplayCopyDisplayMode(id);
            if mode.is_null() {
                return None;
            }
            let px = CGDisplayModeGetPixelWidth(mode) as f64;
            CGDisplayModeRelease(mode);
            Some(DisplaySize { mm, px })
        }
    }

    /// The main display.
    pub fn main() -> Option<DisplaySize> {
        // SAFETY: no arguments.
        size_of(unsafe { CGMainDisplayID() })
    }
}

/// The size of the display holding the global point (x, y) in points; None where it is not
/// known (not macOS).
pub fn display_at(x: f64, y: f64) -> Option<DisplaySize> {
    #[cfg(target_os = "macos")]
    {
        display::at(x, y)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (x, y);
        None
    }
}

/// The size of the main display (for the notes); None where it is not known.
pub fn main_display() -> Option<DisplaySize> {
    #[cfg(target_os = "macos")]
    {
        display::main()
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

/// What the window drew of the splash in its last frame (for tests).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplashShown {
    /// The width drawn, in logical points.
    pub width: f32,
    /// The physical width, in logical points.
    pub physical: f32,
    pub basis: Basis,
    pub scaled_down: bool,
}

/// The id of the Mods > Info window of an open (`App::mod_info_serial`).
pub fn mod_info_window_id(serial: u32) -> egui::Id {
    egui::Id::unique(("rb_mod_info_window", serial))
}

/// The details of a kit's file, kept until the kit or the name changes.
pub(crate) struct Cached {
    kit: Kit,
    name: String,
    details: Result<ModDetails, String>,
}

impl App {
    /// Mods > Info.
    pub fn show_mod_info(&mut self) {
        if !self.mod_info_open {
            // a new id: no stored place, so the window opens at its default, the center
            self.mod_info_serial += 1;
        }
        self.mod_info_open = true;
    }

    /// The id of the window now (for tests, to look its place up).
    pub fn mod_info_id(&self) -> egui::Id {
        mod_info_window_id(self.mod_info_serial)
    }

    /// Whether the window is open.
    pub fn mod_info_is_open(&self) -> bool {
        self.mod_info_open
    }

    /// What the window drew of the splash last (for tests).
    pub fn mod_info_splash(&self) -> Option<SplashShown> {
        self.mod_info_shown
    }

    /// How the splash's physical size is found in this window now.
    fn splash_basis(&self, ctx: &Context) -> Basis {
        let measured = ctx
            .input(|i| i.viewport().outer_rect.or(i.viewport().inner_rect))
            .map(|r| r.center())
            .and_then(|c| display_at(f64::from(c.x), f64::from(c.y)))
            .or_else(main_display)
            .and_then(|d| d.px_per_mm());
        match measured {
            Some(d) => Basis::Measured(d),
            None => Basis::Assumed(fallback_ppi()),
        }
    }

    /// Draws the window when it is open: it shows the kit of the moment. Escape and the close
    /// button close it.
    pub fn mod_info_ui(&mut self, ctx: &Context) {
        if !self.mod_info_open {
            return;
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            self.mod_info_open = false;
            self.mod_info_shown = None;
            return;
        }
        let stale = self
            .mod_details
            .as_ref()
            .is_none_or(|c| c.kit != self.doc.kit || c.name != self.doc.mod_name);
        if stale {
            self.mod_details = Some(Cached {
                kit: self.doc.kit.clone(),
                name: self.doc.mod_name.clone(),
                details: ModDetails::of(&self.doc.kit),
            });
        }
        let details = self
            .mod_details
            .as_ref()
            .expect("set above")
            .details
            .clone();
        let missing = (self.doc.kit == Kit::Standard && self.doc.mod_name != STANDARD)
            .then(|| self.doc.mod_name.clone());
        let tex = self.splash_texture(ctx);
        let basis = self.splash_basis(ctx);
        let screen = ctx.content_rect();
        let max_width = (screen.width() - 56.0)
            // the facts (about 330 high with the notes) go under the splash
            .min(screen.height() - 400.0)
            .max(64.0);
        let ppp = ctx.pixels_per_point();
        let mut shown = None;
        let mut open = true;
        egui::Window::new("Mod Information")
            .id(self.mod_info_id())
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(screen.center())
            .max_size(screen.size() - egui::vec2(16.0, 16.0))
            // else the content may use only the default area height (400)
            .default_height(screen.height() - 60.0)
            .show(ctx, |ui| {
                // a window with `vscroll` starts 400 high; this one is as high as its content
                egui::ScrollArea::vertical()
                    .auto_shrink([true, true])
                    .max_height(screen.height() - 70.0)
                    .show(ui, |ui| {
                        shown = body(ui, &missing, &tex, &details, basis, (ppp, max_width));
                    });
            });
        self.mod_info_shown = shown;
        if !open {
            self.mod_info_open = false;
            self.mod_info_shown = None;
        }
    }
}

/// The window's content: the missing notice, the splash with its note, the facts. Returns what
/// it drew of the splash. `(ppp, max_width)`: the pixels per point and the widest splash.
fn body(
    ui: &mut egui::Ui,
    missing: &Option<String>,
    tex: &Option<egui::TextureHandle>,
    details: &Result<ModDetails, String>,
    basis: Basis,
    (ppp, max_width): (f32, f32),
) -> Option<SplashShown> {
    if let Some(name) = missing {
        let text = format!("The mod \"{name}\" of the song is missing: the built-in kit plays.");
        ui.label(RichText::new(text).color(Color32::from_rgb(0xff, 0xb0, 0x60)));
        ui.add_space(4.0);
    }
    let shown = match tex {
        Some(tex) => {
            let s = splash(ui, tex, basis, ppp, max_width);
            ui.add_space(6.0);
            Some(s)
        }
        None => {
            ui.label(RichText::new("This mod has no splash picture.").color(INK_DIM));
            None
        }
    };
    match details {
        Err(e) => {
            ui.label(RichText::new(e).color(Color32::from_rgb(0xff, 0x80, 0x80)));
        }
        Ok(d) => details_grid(ui, d),
    }
    shown
}

/// The splash at its physical width (or scaled down to `max_width`) and the note under it.
fn splash(
    ui: &mut egui::Ui,
    tex: &egui::TextureHandle,
    basis: Basis,
    ppp: f32,
    max_width: f32,
) -> SplashShown {
    let px = tex.size()[0] as f32;
    let physical = physical_width(px, basis, ppp);
    let (w, scaled_down) = splash_width(physical, max_width);
    let h = w * tex.size()[1] as f32 / px;
    ui.vertical_centered(|ui| {
        ui.add(egui::Image::new((tex.id(), egui::vec2(w, h))));
        let how = match basis {
            Basis::Measured(_) => "the display's reported size".to_string(),
            Basis::Assumed(ppi) => format!("an assumed {ppi:.0} points per inch"),
        };
        let mm = px * MM_PER_PX;
        let text = if scaled_down {
            format!(
                "Scaled down to fit the window (it was {mm:.0} mm wide on a 2000-era 17-inch \
                 monitor)"
            )
        } else {
            format!("{mm:.0} mm wide, as on a 2000-era 17-inch monitor ({how})")
        };
        ui.add(egui::Label::new(RichText::new(text).small().color(INK_DIM)).wrap());
    });
    SplashShown {
        width: w,
        physical,
        basis,
        scaled_down,
    }
}

fn details_grid(ui: &mut egui::Ui, d: &ModDetails) {
    egui::Grid::new("rb_mod_info_grid")
        .num_columns(2)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            let row = |ui: &mut egui::Ui, k: &str, v: &str| {
                ui.label(RichText::new(k).color(INK_DIM));
                ui.scope(|ui| {
                    ui.set_max_width(VALUE_WIDTH);
                    ui.add(egui::Label::new(RichText::new(v).color(INK)).wrap());
                });
                ui.end_row();
            };
            row(ui, "Name", &d.name);
            row(ui, "Version", &format_version(d.version));
            row(ui, "Description", &d.text);
            ui.label(RichText::new("Web").color(INK_DIM));
            link(ui, &d.web);
            ui.end_row();
            row(ui, "FTP", &d.ftp);
            row(ui, "File", &d.file_name);
            if let Some(f) = &d.folder {
                row(ui, "Folder", f);
            }
            row(ui, "Size", &format_size(d.size));
            row(ui, "Embedded files", &d.files.to_string());
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 1 inch of a 110 points-per-inch display is 110 points: the splash is 256 x 0.317 mm.
    #[test]
    fn assumed_dpi_gives_81_mm() {
        for ppi in [96.0_f32, 110.0, 144.0] {
            let w = physical_width(256.0, Basis::Assumed(ppi), 2.0);
            let mm = w / ppi * MM_PER_INCH;
            assert!((mm - 256.0 * 0.317).abs() < 1e-3, "{ppi}: {mm}");
        }
        // the numbers: 81.152 mm = 3.195 in
        let w = physical_width(256.0, Basis::Assumed(110.0), 2.0);
        assert!((w - 351.4).abs() < 0.1, "{w}");
    }

    /// A display of 3456 px in 345.6 mm (10 px per mm) at 2 pixels per point.
    #[test]
    fn measured_size_gives_81_mm() {
        let d = DisplaySize {
            mm: 345.6,
            px: 3456.0,
        };
        let px_per_mm = d.px_per_mm().unwrap();
        assert!((px_per_mm - 10.0).abs() < 1e-4);
        let w = physical_width(256.0, Basis::Measured(px_per_mm), 2.0);
        // points -> pixels -> mm
        let mm = w * 2.0 / px_per_mm;
        assert!((mm - 81.152).abs() < 1e-3, "{mm}");
        assert!((w - 405.76).abs() < 1e-2, "{w}");
        // the same display at 1 pixel per point is twice as many points
        let w1 = physical_width(256.0, Basis::Measured(px_per_mm), 1.0);
        assert!((w1 - 2.0 * w).abs() < 1e-3);
    }

    #[test]
    fn unknown_display_size_is_no_density() {
        assert_eq!(
            DisplaySize {
                mm: 0.0,
                px: 3456.0
            }
            .px_per_mm(),
            None
        );
        assert_eq!(DisplaySize { mm: 300.0, px: 0.0 }.px_per_mm(), None);
        assert_eq!(
            fallback_ppi(),
            if cfg!(target_os = "macos") {
                110.0
            } else {
                96.0
            }
        );
    }

    #[test]
    fn too_wide_a_splash_scales_down() {
        assert_eq!(splash_width(300.0, 400.0), (300.0, false));
        assert_eq!(splash_width(300.0, 300.0), (300.0, false));
        assert_eq!(splash_width(300.0, 200.0), (200.0, true));
    }

    #[test]
    fn sizes_and_versions_are_formatted() {
        assert_eq!(format_size(0), "0 bytes");
        assert_eq!(format_size(1023), "1023 bytes");
        assert_eq!(format_size(1536), "1536 bytes (1.5 KB)");
        assert_eq!(format_size(3 * 1024 * 1024 / 2), "1572864 bytes (1.50 MB)");
        assert_eq!(format_version(Some((2, 0, 1))), "2.0.1");
        assert_eq!(format_version(None), "the app's version");
    }

    /// Prints the main display's size: `cargo test -p rb-ui -- --ignored --nocapture measure`.
    #[test]
    #[ignore = "measures the machine it runs on"]
    fn measure_main_display() {
        let d = main_display();
        eprintln!("main display: {d:?}");
        if let Some(d) = d {
            eprintln!("px per mm {:?}", d.px_per_mm());
        }
    }
}
