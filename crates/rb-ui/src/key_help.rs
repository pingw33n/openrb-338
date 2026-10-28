//! Help > Keyboard Shortcuts (task-105): a window that lists every key rb-ui handles, by group,
//! with the current keyboard mode marked.
//!
//! The list is one table ([`groups`]). Each row carries, besides its text, the [`Bind`]s it
//! stands for: a key, the modes and focus rows it is active in and the [`keys::Action`] it
//! must give. The tests run every bind through [`keys::map`] and run every key `keys::map`
//! gives an action through the binds, so the window cannot drift from the bindings: a changed
//! action fails the first, a new or removed binding the second. The menu shortcuts come from
//! [`crate::shortcuts`], the one definition of them.

use eframe::egui::{self, Color32, Context, Key, KeyboardShortcut, Modifiers, RichText};

use crate::app::App;
use crate::keys::{Action, KeyMode};
use crate::shortcuts;

const INK: Color32 = Color32::from_rgb(0xe9, 0xed, 0xf2);
const INK_DIM: Color32 = Color32::from_rgb(0x9a, 0xa7, 0xb8);
const CURRENT: Color32 = Color32::from_rgb(0xf0, 0xb4, 0x4c);

/// Where a bind is active: the keyboard mode and the focused row (0 and 1 are the 303s, 2 and
/// 3 the 808 and the 909).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// Both modes, any row.
    Always,
    /// Program Synth mode, any row.
    Synth,
    /// Select Patterns mode, any row.
    Patterns,
    /// Program Synth mode, a 303 focused.
    Synth303,
    /// Program Synth mode, a drum machine focused.
    SynthDrums,
    /// Select Patterns mode, or Program Synth mode with a drum machine focused (where Enter
    /// and `/` are the transport's).
    NotSynth303,
}

impl Scope {
    /// Every (mode, focused row) the scope covers.
    pub fn cases(self) -> Vec<(KeyMode, usize)> {
        let mut v = Vec::new();
        for mode in [KeyMode::Synth, KeyMode::Patterns] {
            for focus in 0..4 {
                let on = match self {
                    Scope::Always => true,
                    Scope::Synth => mode == KeyMode::Synth,
                    Scope::Patterns => mode == KeyMode::Patterns,
                    Scope::Synth303 => mode == KeyMode::Synth && focus < 2,
                    Scope::SynthDrums => mode == KeyMode::Synth && focus >= 2,
                    Scope::NotSynth303 => !(mode == KeyMode::Synth && focus < 2),
                };
                if on {
                    v.push((mode, focus));
                }
            }
        }
        v
    }
}

/// Which Shift states a bind holds for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shift {
    /// With and without Shift.
    Any,
    /// Without Shift.
    No,
    /// With Shift.
    Yes,
}

#[cfg(test)]
impl Shift {
    fn states(self) -> &'static [bool] {
        match self {
            Shift::Any => &[false, true],
            Shift::No => &[false],
            Shift::Yes => &[true],
        }
    }
}

/// What a bind must give from [`keys::map`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Want {
    /// A press of this layout id; `{r}` is the focused 303 number (1 or 2).
    Id(String),
    Arrow(&'static str),
    PlayStop,
    Locate(bool),
    Focus(i32),
    Tap303(bool),
    Drum(usize),
    /// No action: the key does nothing there.
    Nothing,
}

impl Want {
    /// The action for a focused row (`None`: the key does nothing).
    pub fn action(&self, focus: usize) -> Option<Action> {
        Some(match self {
            Want::Id(id) => Action::Press(id.replace("{r}", &(focus + 1).to_string())),
            Want::Arrow(a) => Action::Arrow(a),
            Want::PlayStop => Action::PlayStop,
            Want::Locate(end) => Action::Locate(*end),
            Want::Focus(d) => Action::Focus(*d),
            Want::Tap303(rest) => Action::Tap303(*rest),
            Want::Drum(i) => Action::TapDrum(*i),
            Want::Nothing => return None,
        })
    }
}

/// One key and what [`keys::map`] must make of it.
#[derive(Clone, Debug)]
pub struct Bind {
    pub key: Key,
    pub shift: Shift,
    pub scope: Scope,
    pub want: Want,
}

fn bind(key: Key, scope: Scope, want: Want) -> Bind {
    Bind {
        key,
        shift: Shift::Any,
        scope,
        want,
    }
}

fn id(s: &str) -> Want {
    Want::Id(s.into())
}

impl Bind {
    fn shift(mut self, shift: Shift) -> Self {
        self.shift = shift;
        self
    }
}

/// A line of the window.
#[derive(Clone, Debug)]
pub struct Row {
    /// The keys, as written in the window.
    pub keys: String,
    /// What they do.
    pub what: &'static str,
    /// The bindings the line stands for (empty for a menu shortcut and for a plain note).
    pub binds: Vec<Bind>,
}

/// A titled block of lines.
#[derive(Clone, Debug)]
pub struct Group {
    pub title: &'static str,
    /// The keyboard mode the group is about: it is marked when it is the current one.
    pub mode: Option<KeyMode>,
    pub note: &'static str,
    pub rows: Vec<Row>,
}

fn row(keys: &str, what: &'static str, binds: Vec<Bind>) -> Row {
    Row {
        keys: keys.into(),
        what,
        binds,
    }
}

fn menu_row(is_mac: bool, sc: &KeyboardShortcut, what: &'static str) -> Row {
    row(&shortcuts::text_for(sc, is_mac), what, vec![])
}

/// The 303 pitch keys in note order (`keys.rs` `PITCH`).
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

/// The drum instrument keys in instrument order, 1..=11 (`keys.rs` `DRUM`).
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

/// The pattern keys of the four rows (`keys.rs` `PATTERN`).
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

/// All the groups of the window, for a platform (`is_mac`: Cmd instead of Ctrl).
pub fn groups(is_mac: bool) -> Vec<Group> {
    let s303 = Scope::Synth303;
    let pitch = PITCH
        .iter()
        .enumerate()
        .map(|(n, &k)| bind(k, s303, Want::Id(format!("tb303_{{r}}.key{n}"))))
        .collect();
    let drums = DRUM
        .iter()
        .enumerate()
        .map(|(n, &k)| bind(k, Scope::SynthDrums, Want::Drum(n + 1)))
        .collect();
    let patterns = |r: usize, what: &'static str, keys: &str| {
        let binds = PATTERN[r]
            .iter()
            .enumerate()
            .map(|(b, &k)| {
                bind(
                    k,
                    Scope::Patterns,
                    Want::Id(format!("pattern{}.pattern{}", r + 1, b + 1)),
                )
            })
            .collect();
        row(keys, what, binds)
    };
    let note_pause = |scope, want: Want| {
        vec![
            bind(Key::Minus, scope, want.clone()),
            bind(Key::Equals, scope, want).shift(Shift::No),
        ]
    };
    vec![
        Group {
            title: "Menu shortcuts",
            mode: None,
            note: "Keys with the command key held are left to the menu.",
            rows: vec![
                menu_row(is_mac, &shortcuts::NEW, "File > New"),
                menu_row(is_mac, &shortcuts::OPEN, "File > Open..."),
                menu_row(is_mac, &shortcuts::SAVE, "File > Save"),
                menu_row(
                    is_mac,
                    &shortcuts::KEYS_SYNTH,
                    "Options > Program Synth from Keyboard",
                ),
                menu_row(
                    is_mac,
                    &shortcuts::KEYS_PATTERNS,
                    "Options > Select Patterns from Keyboard",
                ),
            ],
        },
        Group {
            title: "Always active",
            mode: None,
            note: "Both keyboard modes, whatever row has the focus.",
            rows: vec![
                row(
                    "Space",
                    "Play if stopped, Stop otherwise",
                    vec![bind(Key::Space, Scope::Always, Want::PlayStop)],
                ),
                row(
                    "Up / Down",
                    "Move the focus to the previous / next row (wraps)",
                    vec![
                        bind(Key::ArrowUp, Scope::Always, Want::Focus(-1)),
                        bind(Key::ArrowDown, Scope::Always, Want::Focus(1)),
                    ],
                ),
                row(
                    "Insert",
                    "Record",
                    vec![bind(Key::Insert, Scope::Always, id("transport.record"))],
                ),
                row(
                    "Numpad 0 (and the main-row 0)",
                    "Stop",
                    vec![bind(Key::Num0, Scope::Always, id("transport.stop"))],
                ),
                row(
                    "Numpad +  or  Shift+=",
                    "Tempo up (held keys repeat)",
                    vec![
                        bind(Key::Plus, Scope::Always, Want::Arrow("transport.tempo_up")),
                        bind(
                            Key::Equals,
                            Scope::Always,
                            Want::Arrow("transport.tempo_up"),
                        )
                        .shift(Shift::Yes),
                    ],
                ),
                row(
                    "PgUp / PgDn",
                    "Nothing: the panel does not scroll",
                    vec![
                        bind(Key::PageUp, Scope::Always, Want::Nothing),
                        bind(Key::PageDown, Scope::Always, Want::Nothing),
                    ],
                ),
            ],
        },
        Group {
            title: "Transport keys that depend on the mode",
            mode: None,
            note: "egui cannot tell the numpad from the main keyboard, so these keys are \
                   resolved by the mode and the focus.",
            rows: vec![
                row(
                    "Enter",
                    "Play (in Program Synth mode only with a drum machine focused: with a \
                     303 focused it is Step)",
                    vec![bind(Key::Enter, Scope::NotSynth303, id("transport.play"))],
                ),
                row(
                    "/",
                    "Loop on / off (in Program Synth mode only with a drum machine \
                     focused: with a 303 focused it is the high C)",
                    vec![bind(Key::Slash, Scope::NotSynth303, id("transport.loop"))],
                ),
                row(
                    "-",
                    "Tempo down (held keys repeat). Select Patterns mode only: in \
                     Program Synth mode `-` is Note/Pause or AC, so tempo down has no key",
                    vec![bind(
                        Key::Minus,
                        Scope::Patterns,
                        Want::Arrow("transport.tempo_down"),
                    )],
                ),
                row(
                    "4 / 5",
                    "Rewind / Fast forward (Program Synth mode only)",
                    vec![
                        bind(Key::Num4, Scope::Synth, Want::Arrow("transport.rewind")),
                        bind(Key::Num5, Scope::Synth, Want::Arrow("transport.ff")),
                    ],
                ),
                row(
                    "7 / 8",
                    "Bar down / Bar up (Program Synth mode only)",
                    vec![
                        bind(Key::Num7, Scope::Synth, Want::Arrow("transport.bar_down")),
                        bind(Key::Num8, Scope::Synth, Want::Arrow("transport.bar_up")),
                    ],
                ),
                row(
                    "1 / 2",
                    "Locate to the loop start / end (Program Synth mode only)",
                    vec![
                        bind(Key::Num1, Scope::Synth, Want::Locate(false)),
                        bind(Key::Num2, Scope::Synth, Want::Locate(true)),
                    ],
                ),
                row(
                    "Numpad *",
                    "Nothing: egui gives no key for it (Insert records)",
                    vec![],
                ),
            ],
        },
        Group {
            title: "Program Synth mode, a TB-303 focused",
            mode: Some(KeyMode::Synth),
            note: "The keys press the focused 303's panel keys.",
            rows: vec![
                row(
                    "C F V G B N J M K , L . /",
                    "Piano keys C, C#, D, D#, E, F, F#, G, G#, A, A#, B, high C",
                    pitch,
                ),
                row(
                    "Enter",
                    "Step",
                    vec![bind(Key::Enter, s303, id("tb303_{r}.step"))],
                ),
                row(
                    "Backspace",
                    "Back",
                    vec![bind(Key::Backspace, s303, id("tb303_{r}.back"))],
                ),
                row(
                    "P",
                    "Accent",
                    vec![bind(Key::P, s303, id("tb303_{r}.accent"))],
                ),
                row(
                    "[",
                    "Slide",
                    vec![bind(Key::OpenBracket, s303, id("tb303_{r}.slide"))],
                ),
                row(
                    ";",
                    "Octave down",
                    vec![bind(Key::Semicolon, s303, id("tb303_{r}.down"))],
                ),
                row(
                    "'",
                    "Octave up",
                    vec![bind(Key::Quote, s303, id("tb303_{r}.up"))],
                ),
                row(
                    "-  or  =",
                    "Note / Pause",
                    note_pause(s303, id("tb303_{r}.note_pause")),
                ),
                row(
                    "Tab",
                    "The playing step becomes a note (tap recording)",
                    vec![bind(Key::Tab, s303, Want::Tap303(false)).shift(Shift::No)],
                ),
                row(
                    "Shift+Tab",
                    "The playing step becomes a rest",
                    vec![bind(Key::Tab, s303, Want::Tap303(true)).shift(Shift::Yes)],
                ),
            ],
        },
        Group {
            title: "Program Synth mode, a drum machine focused",
            mode: Some(KeyMode::Synth),
            note: "A key sets that instrument on the playing step (tap recording).",
            rows: vec![
                row(
                    "-  or  =",
                    "AC (accent)",
                    note_pause(Scope::SynthDrums, Want::Drum(0)),
                ),
                row(
                    "A S D F G H J K L ; '",
                    "Instruments 1..11. TR-808: BD SD LT MT HT RS CP CB CY OH CH. \
                     TR-909: BD SD LT MT HT RS CP CH OH CC RC",
                    drums,
                ),
            ],
        },
        Group {
            title: "Select Patterns mode",
            mode: Some(KeyMode::Patterns),
            note: "Each key row selects patterns 1..8 of one section, as a click on the \
                   pattern button.",
            rows: vec![
                patterns(0, "TB-303 #1", "1 2 3 4 5 6 7 8"),
                patterns(1, "TB-303 #2", "Q W E R T Y U I"),
                patterns(2, "TR-808", "A S D F G H J K"),
                patterns(3, "TR-909", "Z X C V B N M ,"),
            ],
        },
        Group {
            title: "Windows and the file browser",
            mode: None,
            note: "Listed by hand from the browser's key handler; no test checks these lines.",
            rows: vec![
                row(
                    "Escape",
                    "Close the Info, About and this window; cancel the browser or the \
                     unsaved-changes prompt",
                    vec![],
                ),
                row(
                    "Up / Down, PgUp / PgDn",
                    "Move the browser's selection",
                    vec![],
                ),
                row("Home / End", "First / last entry of the browser", vec![]),
                row("Enter", "Open the selected entry", vec![]),
                row(
                    "Backspace  or  Alt+Up",
                    "The browser goes to the parent folder",
                    vec![],
                ),
                menu_row(is_mac, &shortcuts::EDIT_PATH, "Edit the browser's path"),
            ],
        },
        Group {
            title: "Prompts and questions",
            mode: None,
            note: "Listed by hand from the code (the prompt in menu.rs, the browser's question \
                   in browser.rs); no test checks these lines.",
            rows: vec![
                row("S  Y  Enter", "Unsaved-changes prompt: save", vec![]),
                row("D  N", "Unsaved-changes prompt: do not save", vec![]),
                row("Escape", "Unsaved-changes prompt: cancel", vec![]),
                row(
                    "Y",
                    "Overwrite question of the browser: replace the file",
                    vec![],
                ),
                row(
                    "Enter  Escape  N",
                    "Overwrite question of the browser: go back",
                    vec![],
                ),
            ],
        },
    ]
}

/// Whether a group is marked "(current mode)" while `current` is the keyboard mode.
pub fn is_current(g: &Group, current: KeyMode) -> bool {
    g.mode == Some(current)
}

/// The name of a mode as the window writes it.
fn mode_name(mode: KeyMode) -> &'static str {
    match mode {
        KeyMode::Synth => "Program Synth",
        KeyMode::Patterns => "Select Patterns",
    }
}

/// The id of the window of an open (`App::key_help_serial`).
pub fn key_help_window_id(serial: u32) -> egui::Id {
    egui::Id::unique(("rb_key_help_window", serial))
}

impl App {
    /// Help > Keyboard Shortcuts.
    pub fn show_key_help(&mut self) {
        if !self.key_help_open {
            // a new id: no stored place, so the window opens at its default, the center
            self.key_help_serial += 1;
        }
        self.key_help_open = true;
    }

    /// The id of the window now (for tests, to look its place up).
    pub fn key_help_id(&self) -> egui::Id {
        key_help_window_id(self.key_help_serial)
    }

    /// Whether the window is open.
    pub fn key_help_is_open(&self) -> bool {
        self.key_help_open
    }

    /// Draws the window when it is open. Escape and the close button close it.
    pub fn key_help_ui(&mut self, ctx: &Context) {
        if !self.key_help_open {
            return;
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            self.key_help_open = false;
            return;
        }
        let mut open = true;
        let screen = ctx.content_rect();
        let size = egui::vec2(
            (screen.width() - 40.0).clamp(200.0, 520.0),
            (screen.height() - 40.0).clamp(150.0, 440.0),
        );
        let current = self.key_mode;
        // centered the way File > Info is (info.rs): pivot, default place, an id per open
        egui::Window::new("Keyboard Shortcuts")
            .id(self.key_help_id())
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(screen.center())
            .default_size(size)
            .show(ctx, |ui| {
                ui.label(
                    RichText::new(format!("Keyboard mode now: {}", mode_name(current)))
                        .strong()
                        .color(CURRENT),
                );
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for (n, g) in groups(ctx.os().is_mac()).iter().enumerate() {
                            ui.add_space(8.0);
                            let is_current = is_current(g, current);
                            let mut title = RichText::new(g.title).strong().size(15.0);
                            title = title.color(if is_current { CURRENT } else { INK });
                            ui.horizontal(|ui| {
                                ui.label(title);
                                if is_current {
                                    ui.label(RichText::new("(current mode)").color(CURRENT));
                                }
                            });
                            if !g.note.is_empty() {
                                ui.label(RichText::new(g.note).color(INK_DIM));
                            }
                            egui::Grid::new(("rb_key_help_grid", n))
                                .num_columns(2)
                                .show(ui, |ui| {
                                    for r in &g.rows {
                                        ui.label(RichText::new(&r.keys).monospace().color(INK));
                                        ui.add(egui::Label::new(r.what).wrap());
                                        ui.end_row();
                                    }
                                });
                        }
                    });
            });
        if !open {
            self.key_help_open = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys;

    fn binds(is_mac: bool) -> Vec<Bind> {
        groups(is_mac)
            .into_iter()
            .flat_map(|g| g.rows)
            .flat_map(|r| r.binds)
            .collect()
    }

    /// Every listed key does what the window says, in every mode and row the bind covers.
    #[test]
    fn every_listed_key_maps_to_its_action() {
        let all = binds(false);
        assert!(all.len() > 80, "{} binds", all.len());
        for b in &all {
            for &shift in b.shift.states() {
                for (mode, focus) in b.scope.cases() {
                    assert_eq!(
                        keys::map(b.key, shift, mode, focus),
                        b.want.action(focus),
                        "{:?} shift={shift} {mode:?} focus={focus}",
                        b.key
                    );
                }
            }
        }
    }

    /// Every key `keys::map` gives an action is listed with that action.
    #[test]
    fn every_mapped_key_is_listed() {
        let all = binds(false);
        let mut mapped = 0;
        for &key in Key::ALL {
            for shift in [false, true] {
                for mode in [KeyMode::Synth, KeyMode::Patterns] {
                    for focus in 0..4 {
                        let Some(action) = keys::map(key, shift, mode, focus) else {
                            continue;
                        };
                        mapped += 1;
                        let listed = all.iter().any(|b| {
                            b.key == key
                                && b.shift.states().contains(&shift)
                                && b.scope.cases().contains(&(mode, focus))
                                && b.want.action(focus).as_ref() == Some(&action)
                        });
                        assert!(
                            listed,
                            "{key:?} shift={shift} {mode:?} focus={focus} -> {action:?} is not \
                             listed in the Keyboard Shortcuts window"
                        );
                    }
                }
            }
        }
        assert!(mapped > 400, "{mapped} mapped cases");
    }

    /// No two binds claim the same (key, shift, mode, row) with different actions.
    #[test]
    fn the_binds_do_not_contradict_each_other() {
        let all = binds(false);
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                if a.key != b.key {
                    continue;
                }
                if !a
                    .shift
                    .states()
                    .iter()
                    .any(|s| b.shift.states().contains(s))
                {
                    continue;
                }
                for (mode, focus) in a.scope.cases() {
                    if b.scope.cases().contains(&(mode, focus)) {
                        assert_eq!(a.want.action(focus), b.want.action(focus), "{:?}", a.key);
                    }
                }
            }
        }
    }

    /// The menu rows carry the text of `shortcuts` for the platform.
    #[test]
    fn menu_rows_use_the_platform_text() {
        let text = |is_mac| -> Vec<String> {
            groups(is_mac)[0]
                .rows
                .iter()
                .map(|r| r.keys.clone())
                .collect()
        };
        assert_eq!(text(true), ["Cmd+N", "Cmd+O", "Cmd+S", "Cmd+F", "Cmd+G"]);
        assert_eq!(
            text(false),
            ["Ctrl+N", "Ctrl+O", "Ctrl+S", "Ctrl+F", "Ctrl+G"]
        );
    }

    /// The current-mode mark: Synth marks exactly the two Program Synth groups, Patterns only
    /// Select Patterns.
    #[test]
    fn the_current_mode_marks_its_groups() {
        let marked = |mode| -> Vec<&'static str> {
            groups(false)
                .iter()
                .filter(|g| is_current(g, mode))
                .map(|g| g.title)
                .collect()
        };
        assert_eq!(
            marked(KeyMode::Synth),
            [
                "Program Synth mode, a TB-303 focused",
                "Program Synth mode, a drum machine focused"
            ]
        );
        assert_eq!(marked(KeyMode::Patterns), ["Select Patterns mode"]);
    }
}
