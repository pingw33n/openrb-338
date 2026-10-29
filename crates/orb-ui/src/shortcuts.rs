//! The keyboard shortcuts of the menu and the browser (task-092): one definition each, used
//! both to catch the key ([`consume`]) and to write the text next to a menu item ([`text`]), so
//! the two cannot drift. The command modifier is Cmd on macOS and Ctrl elsewhere (egui's
//! `Modifiers::COMMAND`).

use eframe::egui::{Context, Key, KeyboardShortcut, ModifierNames, Modifiers, Ui};

pub const NEW: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::N);
pub const OPEN: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::O);
pub const SAVE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::S);
/// "Program Synth from Keyboard".
pub const KEYS_SYNTH: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::F);
/// "Select Patterns from Keyboard".
pub const KEYS_PATTERNS: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::G);
/// The file browser's path field.
pub const EDIT_PATH: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::L);

/// The text of a shortcut for a platform: "Cmd+N" on macOS, "Ctrl+N" elsewhere (the macOS
/// symbol form is not in egui's default font).
pub fn text_for(sc: &KeyboardShortcut, is_mac: bool) -> String {
    sc.format(&ModifierNames::NAMES, is_mac)
}

/// The text of a shortcut on the running platform.
pub fn text(ctx: &Context, sc: &KeyboardShortcut) -> String {
    text_for(sc, ctx.os().is_mac())
}

/// Takes the shortcut's key press from the input, true if it was pressed.
pub fn consume(ui: &Ui, sc: &KeyboardShortcut) -> bool {
    ui.input_mut(|i| i.consume_shortcut(sc))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_names_cmd_on_mac_and_ctrl_elsewhere() {
        let all = [NEW, OPEN, SAVE, KEYS_SYNTH, KEYS_PATTERNS, EDIT_PATH];
        let keys = ["N", "O", "S", "F", "G", "L"];
        for (sc, k) in all.iter().zip(keys) {
            assert_eq!(text_for(sc, true), format!("Cmd+{k}"));
            assert_eq!(text_for(sc, false), format!("Ctrl+{k}"));
        }
    }
}
