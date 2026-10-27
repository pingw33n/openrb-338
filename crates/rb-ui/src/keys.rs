//! The computer keyboard (docs/ui_spec.md section 10, mapping decided in task-054): keys by
//! their physical US position as the manual pictures them, whatever the layout. The window
//! gives each key press as an egui key; this module maps it to what the panel does.
//!
//! egui 0.37 cannot tell a numpad key from the key it shares a name with (egui-winit maps
//! `Numpad4` and `Digit4` both to `Key::Num4`, `NumpadEnter` to `Enter`, `NumpadSubtract` to
//! `Minus`, `NumpadDivide` to `Slash`; `NumpadMultiply` gives no key at all). So the keys the
//! original keeps apart on its own codes are resolved by the mode and the focus, see [`map`].

use eframe::egui::Key;

/// The Options menu's keyboard mode (docs/ui_spec.md section 10): one setting, two values.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KeyMode {
    /// "Program Synth from Keyboard" (Ctrl+F): the focused row's synth and tap keys.
    #[default]
    Synth,
    /// "Select Patterns from Keyboard" (Ctrl+G): the four rows' pattern buttons.
    Patterns,
}

/// What a key does to the panel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// The same as a mouse down on the trigger, toggle or grid button of this layout id.
    Press(String),
    /// A repeat or wind button (tempo, bar, rewind, fast forward): a step at the key press, then
    /// the held-button schedule. The layout id of the arrow.
    Arrow(&'static str),
    /// Space: Play if stopped, Stop otherwise.
    PlayStop,
    /// Numpad 1 / 2: locate to the loop start (false) or the loop end (true).
    Locate(bool),
    /// Up / Down: the focus moves by -1 / +1 row, wrapping.
    Focus(i32),
    /// Tab (false) / Shift+Tab (true) on a 303: the playing step becomes a note / a rest.
    Tap303(bool),
    /// A drum tap key: instrument 0 (AC) to 11 of the focused drum machine.
    TapDrum(usize),
}

/// The 303 pitch keys, `C F V G B N J M K , L . /` = notes 0..=12.
const PITCH: [Key; 13] = [
    Key::C,
    Key::F,
    Key::V,
    Key::G,
    Key::B,
    Key::N,
    Key::J,
    Key::M,
    Key::K,
    Key::Comma,
    Key::L,
    Key::Period,
    Key::Slash,
];

/// The drum tap keys, `A S D F G H J K L ; '` = instruments 1..=11.
const DRUM: [Key; 11] = [
    Key::A,
    Key::S,
    Key::D,
    Key::F,
    Key::G,
    Key::H,
    Key::J,
    Key::K,
    Key::L,
    Key::Semicolon,
    Key::Quote,
];

/// The pattern keys of the four rows: `1`..`8`, `Q`..`I`, `A`..`K`, `Z`..`,`.
const PATTERN: [[Key; 8]; 4] = [
    [
        Key::Num1,
        Key::Num2,
        Key::Num3,
        Key::Num4,
        Key::Num5,
        Key::Num6,
        Key::Num7,
        Key::Num8,
    ],
    [
        Key::Q,
        Key::W,
        Key::E,
        Key::R,
        Key::T,
        Key::Y,
        Key::U,
        Key::I,
    ],
    [
        Key::A,
        Key::S,
        Key::D,
        Key::F,
        Key::G,
        Key::H,
        Key::J,
        Key::K,
    ],
    [
        Key::Z,
        Key::X,
        Key::C,
        Key::V,
        Key::B,
        Key::N,
        Key::M,
        Key::Comma,
    ],
];

/// The key an event stands for: its physical position, else (a platform that does not report
/// it, or a scancode egui does not know) the logical key, which then follows the layout.
pub fn position(key: Key, physical: Option<Key>) -> Key {
    physical.unwrap_or(key)
}

/// What the key `k` (a [`position`]) does with `shift` held, in `mode`, with the focus on row
/// `focus` (0..=3: the two 303s, the 808, the 909). `None`: the key does nothing.
///
/// A key the original keeps on two codes (the numpad's and the main keyboard's) is resolved:
/// - digits `1`..`8` select patterns in [`KeyMode::Patterns`] (and then `4 5 7 8 1 2` are no
///   transport keys); in [`KeyMode::Synth`] all digits are the numpad's;
/// - Enter is the 303's Step when a 303 has the focus in Synth mode, else Play;
/// - `/` is the 303's high C there, else the loop switch;
/// - `-` is Note/Pause (303) or AC (drums) in Synth mode, else tempo down.
///
/// `=` is the alias of Note/Pause and AC (the main keyboard's OEM plus key), only in Synth mode.
pub fn map(k: Key, shift: bool, mode: KeyMode, focus: usize) -> Option<Action> {
    let synth = mode == KeyMode::Synth;
    let is303 = focus < 2;
    let press = |id: &str| Some(Action::Press(id.into()));
    // Always active (the window's handler).
    match k {
        Key::Space => return Some(Action::PlayStop),
        Key::ArrowUp => return Some(Action::Focus(-1)),
        Key::ArrowDown => return Some(Action::Focus(1)),
        Key::Insert => return press("transport.record"),
        // numpad + ; a shifted `=` (`+` on a US keyboard) is no alias of Note/Pause/AC
        Key::Plus => return Some(Action::Arrow("transport.tempo_up")),
        Key::Equals if shift => return Some(Action::Arrow("transport.tempo_up")),
        _ => {}
    }
    if synth {
        // The focused row's synth keys.
        let row = format!("tb303_{}", focus + 1);
        if is303 {
            if let Some(n) = PITCH.iter().position(|&p| p == k) {
                return press(&format!("{row}.key{n}"));
            }
            let name = match k {
                Key::Enter => "step",
                Key::Backspace => "back",
                Key::P => "accent",
                Key::OpenBracket => "slide",
                Key::Semicolon => "down",
                Key::Quote => "up",
                Key::Minus | Key::Equals => "note_pause",
                Key::Tab => return Some(Action::Tap303(shift)),
                _ => "",
            };
            if !name.is_empty() {
                return press(&format!("{row}.{name}"));
            }
        } else {
            if matches!(k, Key::Minus | Key::Equals) {
                return Some(Action::TapDrum(0));
            }
            if let Some(n) = DRUM.iter().position(|&p| p == k) {
                return Some(Action::TapDrum(n + 1));
            }
        }
    } else if let Some((r, b)) = PATTERN
        .iter()
        .enumerate()
        .find_map(|(r, row)| row.iter().position(|&p| p == k).map(|b| (r, b)))
    {
        return press(&format!("pattern{}.pattern{}", r + 1, b + 1));
    }
    // The numpad's transport keys.
    match k {
        Key::Enter => press("transport.play"),
        Key::Num0 => press("transport.stop"),
        Key::Slash => press("transport.loop"),
        Key::Minus => Some(Action::Arrow("transport.tempo_down")),
        Key::Num4 if synth => Some(Action::Arrow("transport.rewind")),
        Key::Num5 if synth => Some(Action::Arrow("transport.ff")),
        Key::Num7 if synth => Some(Action::Arrow("transport.bar_down")),
        Key::Num8 if synth => Some(Action::Arrow("transport.bar_up")),
        Key::Num1 if synth => Some(Action::Locate(false)),
        Key::Num2 if synth => Some(Action::Locate(true)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_physical_key_falls_back_to_the_logical_one() {
        assert_eq!(position(Key::A, None), Key::A);
        assert_eq!(position(Key::Q, Some(Key::A)), Key::A);
    }

    #[test]
    fn minus_and_equals_are_note_pause_or_ac() {
        for k in [Key::Minus, Key::Equals] {
            assert_eq!(
                map(k, false, KeyMode::Synth, 1),
                Some(Action::Press("tb303_2.note_pause".into()))
            );
            assert_eq!(map(k, false, KeyMode::Synth, 3), Some(Action::TapDrum(0)));
        }
        assert_eq!(
            map(Key::Minus, false, KeyMode::Patterns, 0),
            Some(Action::Arrow("transport.tempo_down"))
        );
        assert_eq!(map(Key::Equals, false, KeyMode::Patterns, 0), None);
    }

    #[test]
    fn patterns_mode_digits_select_and_synth_mode_digits_are_the_numpad() {
        assert_eq!(
            map(Key::Num5, false, KeyMode::Patterns, 3),
            Some(Action::Press("pattern1.pattern5".into()))
        );
        assert_eq!(
            map(Key::Comma, false, KeyMode::Patterns, 0),
            Some(Action::Press("pattern4.pattern8".into()))
        );
        assert_eq!(
            map(Key::Num5, false, KeyMode::Synth, 0),
            Some(Action::Arrow("transport.ff"))
        );
    }
}
