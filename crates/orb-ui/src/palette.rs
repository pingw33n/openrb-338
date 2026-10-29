//! The colors the windows share (task-128): the ink of the texts, the panel's background, the
//! slate of the boxes and the amber of the lamps. A color that one window alone uses stays in
//! that window's module.

use eframe::egui::Color32;

/// The text ink: the main texts of the windows.
pub const INK: Color32 = Color32::from_rgb(0xe9, 0xed, 0xf2);
/// The dimmed ink: the labels and the secondary texts.
pub const INK_DIM: Color32 = Color32::from_rgb(0x9a, 0xa7, 0xb8);
/// The panel's background and the letterbox color.
pub const BG: Color32 = Color32::from_rgb(0x1d, 0x21, 0x27);
/// The dark slate of the boxes' fills.
pub const SLATE_DARK: Color32 = Color32::from_rgb(0x2c, 0x35, 0x42);
/// The light slate of the boxes' outlines and the selection.
pub const SLATE_LINE: Color32 = Color32::from_rgb(0x56, 0x64, 0x7a);
/// The amber of the panel's lamps: the accent of the windows (a title, a warning).
pub const AMBER: Color32 = Color32::from_rgb(0xff, 0xb0, 0x00);
