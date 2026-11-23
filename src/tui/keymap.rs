//! Key binding tables — the single source for key dispatch and the key legend.
//!
//! Each input mode has one table. Handlers look up the pressed key in their
//! mode's table and match on the returned command; `ui.rs` builds the legend
//! from the same tables, so the two cannot disagree.
//!
//! Ctrl-C is not in any table: it always quits, as a terminal-wide emergency exit.

use crossterm::event::KeyCode;

use crate::tui::app::{App, Tab};
use crate::tui::types::ListEntry;

/// One legend entry and the keys behind it.
pub struct Binding<C: 'static> {
    /// Key glyph shown in the legend, e.g. `↑↓`. Empty hides the entry.
    pub keys_label: &'static str,
    pub label: &'static str,
    /// Each key and the command it triggers. Paired keys like `↑↓` share one
    /// legend entry but trigger opposite commands.
    pub keys: &'static [(KeyCode, C)],
}

/// A mode's bindings, in legend sections. The legend separates bindings with
/// two spaces and sections with three.
pub type Keymap<C> = &'static [&'static [Binding<C>]];

/// Find the command bound to `code`, if any.
pub fn lookup<C: Copy>(keymap: Keymap<C>, code: KeyCode) -> Option<C> {
    keymap
        .iter()
        .flat_map(|section| section.iter())
        .flat_map(|binding| binding.keys.iter())
        .find(|(key, _)| *key == code)
        .map(|(_, command)| *command)
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlobalCommand {
    Quit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabBarCommand {
    SwitchTab,
    /// Move focus from the tab bar into the screen.
    Leave,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpeakerCommand {
    Up,
    Down,
    VolumeDown,
    VolumeUp,
    ToggleMute,
    PickUp,
    PlayPause,
    Next,
    Previous,
    /// Satellite rows: switch the surround level `←→` edits (music or TV).
    SwitchLevel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickUpCommand {
    Up,
    Down,
    Drop,
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsCommand {
    Up,
    Down,
    Open,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropdownCommand {
    Up,
    Down,
    Confirm,
    Cancel,
}

// ---------------------------------------------------------------------------
// Tables
// ---------------------------------------------------------------------------

const UP_DOWN: &str = "\u{2191}\u{2193}";
const LEFT_RIGHT: &str = "\u{2190}\u{2192}";
const SPACE: &str = "\u{2423}";
const TAB: &str = "\u{21e5}";
const ESC: &str = "\u{238b}";

/// Keys that apply in every mode except pick-up, after the mode's own table.
pub const GLOBAL_KEYS: Keymap<GlobalCommand> = &[&[Binding {
    keys_label: ESC,
    label: "Quit",
    keys: &[
        (KeyCode::Esc, GlobalCommand::Quit),
        (KeyCode::Char('q'), GlobalCommand::Quit),
    ],
}]];

pub const TAB_BAR_KEYS: Keymap<TabBarCommand> = &[&[
    Binding {
        keys_label: LEFT_RIGHT,
        label: "Tabs",
        keys: &[
            (KeyCode::Left, TabBarCommand::SwitchTab),
            (KeyCode::Right, TabBarCommand::SwitchTab),
        ],
    },
    Binding {
        keys_label: "",
        label: "",
        keys: &[
            (KeyCode::Down, TabBarCommand::Leave),
            (KeyCode::Enter, TabBarCommand::Leave),
        ],
    },
]];

pub const SPEAKER_KEYS: Keymap<SpeakerCommand> = &[
    &[Binding {
        keys_label: UP_DOWN,
        label: "Navigate",
        keys: &[
            (KeyCode::Up, SpeakerCommand::Up),
            (KeyCode::Down, SpeakerCommand::Down),
        ],
    }],
    &[
        Binding {
            keys_label: LEFT_RIGHT,
            label: "Volume",
            keys: &[
                (KeyCode::Left, SpeakerCommand::VolumeDown),
                (KeyCode::Right, SpeakerCommand::VolumeUp),
            ],
        },
        Binding {
            keys_label: "m",
            label: "Mute",
            keys: &[(KeyCode::Char('m'), SpeakerCommand::ToggleMute)],
        },
    ],
    &[Binding {
        keys_label: SPACE,
        label: "Pick up/Drop",
        keys: &[(KeyCode::Char(' '), SpeakerCommand::PickUp)],
    }],
    &[
        Binding {
            keys_label: "p",
            label: "Play/Pause",
            keys: &[(KeyCode::Char('p'), SpeakerCommand::PlayPause)],
        },
        Binding {
            keys_label: "n",
            label: "Next",
            keys: &[(KeyCode::Char('n'), SpeakerCommand::Next)],
        },
        Binding {
            keys_label: "b",
            label: "Prev",
            keys: &[(KeyCode::Char('b'), SpeakerCommand::Previous)],
        },
    ],
];

/// A selected satellite row: `←→` and `m` act on the main speaker's surround
/// or sub settings instead of a volume. Space stays bound so it can explain
/// that satellites can't be picked up, but has no legend entry.
pub const SATELLITE_KEYS: Keymap<SpeakerCommand> = &[
    &[Binding {
        keys_label: UP_DOWN,
        label: "Navigate",
        keys: &[
            (KeyCode::Up, SpeakerCommand::Up),
            (KeyCode::Down, SpeakerCommand::Down),
        ],
    }],
    &[
        Binding {
            keys_label: LEFT_RIGHT,
            label: "Level",
            keys: &[
                (KeyCode::Left, SpeakerCommand::VolumeDown),
                (KeyCode::Right, SpeakerCommand::VolumeUp),
            ],
        },
        Binding {
            keys_label: TAB,
            label: "\u{266a}/TV",
            keys: &[(KeyCode::Tab, SpeakerCommand::SwitchLevel)],
        },
        Binding {
            keys_label: "m",
            label: "On/Off",
            keys: &[(KeyCode::Char('m'), SpeakerCommand::ToggleMute)],
        },
        Binding {
            keys_label: "",
            label: "",
            keys: &[(KeyCode::Char(' '), SpeakerCommand::PickUp)],
        },
    ],
    &[
        Binding {
            keys_label: "p",
            label: "Play/Pause",
            keys: &[(KeyCode::Char('p'), SpeakerCommand::PlayPause)],
        },
        Binding {
            keys_label: "n",
            label: "Next",
            keys: &[(KeyCode::Char('n'), SpeakerCommand::Next)],
        },
        Binding {
            keys_label: "b",
            label: "Prev",
            keys: &[(KeyCode::Char('b'), SpeakerCommand::Previous)],
        },
    ],
];

/// Pick-up mode is exclusive: keys outside this table are swallowed.
pub const PICK_UP_KEYS: Keymap<PickUpCommand> = &[&[
    Binding {
        keys_label: UP_DOWN,
        label: "Navigate",
        keys: &[
            (KeyCode::Up, PickUpCommand::Up),
            (KeyCode::Down, PickUpCommand::Down),
        ],
    },
    Binding {
        keys_label: SPACE,
        label: "Drop",
        keys: &[(KeyCode::Char(' '), PickUpCommand::Drop)],
    },
    Binding {
        keys_label: ESC,
        label: "Cancel",
        keys: &[(KeyCode::Esc, PickUpCommand::Cancel)],
    },
]];

pub const SETTINGS_KEYS: Keymap<SettingsCommand> = &[&[
    Binding {
        keys_label: UP_DOWN,
        label: "Navigate",
        keys: &[
            (KeyCode::Up, SettingsCommand::Up),
            (KeyCode::Down, SettingsCommand::Down),
        ],
    },
    Binding {
        keys_label: "Enter",
        label: "Open",
        keys: &[
            (KeyCode::Enter, SettingsCommand::Open),
            (KeyCode::Right, SettingsCommand::Open),
        ],
    },
]];

pub const DROPDOWN_KEYS: Keymap<DropdownCommand> = &[&[
    Binding {
        keys_label: UP_DOWN,
        label: "Select",
        keys: &[
            (KeyCode::Up, DropdownCommand::Up),
            (KeyCode::Down, DropdownCommand::Down),
        ],
    },
    Binding {
        keys_label: "Enter",
        label: "Confirm",
        keys: &[
            (KeyCode::Enter, DropdownCommand::Confirm),
            (KeyCode::Right, DropdownCommand::Confirm),
        ],
    },
    Binding {
        keys_label: ESC,
        label: "Cancel",
        keys: &[
            (KeyCode::Esc, DropdownCommand::Cancel),
            (KeyCode::Left, DropdownCommand::Cancel),
        ],
    },
]];

// ---------------------------------------------------------------------------
// Modes
// ---------------------------------------------------------------------------

/// Which table receives key presses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    TabBar,
    Speakers,
    /// The speaker list with a satellite row selected.
    Satellite,
    PickUp,
    Settings,
    Dropdown,
}

impl Mode {
    pub fn of(app: &App) -> Self {
        let nav = &app.navigation;
        if nav.speakers_state.pick_up.is_some() {
            Mode::PickUp
        } else if nav.tab_focused {
            Mode::TabBar
        } else {
            match nav.tab {
                Tab::Speakers
                    if matches!(
                        nav.speakers_state.selection.entry(),
                        Some(ListEntry::SatelliteRow { .. })
                    ) =>
                {
                    Mode::Satellite
                }
                Tab::Speakers => Mode::Speakers,
                Tab::Settings if nav.settings_state.dropdown_open => Mode::Dropdown,
                Tab::Settings => Mode::Settings,
            }
        }
    }

    /// True when the mode's own table binds `code`.
    pub fn binds(self, code: KeyCode) -> bool {
        match self {
            Mode::TabBar => lookup(TAB_BAR_KEYS, code).is_some(),
            Mode::Speakers => lookup(SPEAKER_KEYS, code).is_some(),
            Mode::Satellite => lookup(SATELLITE_KEYS, code).is_some(),
            Mode::PickUp => lookup(PICK_UP_KEYS, code).is_some(),
            Mode::Settings => lookup(SETTINGS_KEYS, code).is_some(),
            Mode::Dropdown => lookup(DROPDOWN_KEYS, code).is_some(),
        }
    }

    /// Exclusive modes swallow unbound keys instead of falling through to
    /// [`GLOBAL_KEYS`].
    pub fn is_exclusive(self) -> bool {
        self == Mode::PickUp
    }
}

// ---------------------------------------------------------------------------
// Legend
// ---------------------------------------------------------------------------

/// The key legend for the current tab, built from the binding tables.
///
/// The legend follows the tab rather than the mode: pick-up mode and a
/// focused tab bar keep their tab's legend.
pub fn legend(app: &App) -> String {
    let sections = match app.navigation.tab {
        Tab::Speakers if Mode::of(app) == Mode::Satellite => [
            legend_sections(SATELLITE_KEYS),
            legend_sections(GLOBAL_KEYS),
        ]
        .concat(),
        Tab::Speakers => [legend_sections(SPEAKER_KEYS), legend_sections(GLOBAL_KEYS)].concat(),
        Tab::Settings if app.navigation.settings_state.dropdown_open => {
            legend_sections(DROPDOWN_KEYS)
        }
        Tab::Settings => [
            legend_sections(SETTINGS_KEYS),
            legend_sections(TAB_BAR_KEYS),
            legend_sections(GLOBAL_KEYS),
        ]
        .concat(),
    };
    sections
        .iter()
        .map(|section| section.join("  "))
        .collect::<Vec<_>>()
        .join("   ")
}

fn legend_sections<C>(keymap: Keymap<C>) -> Vec<Vec<String>> {
    keymap
        .iter()
        .map(|section| {
            section
                .iter()
                .filter(|b| !b.keys_label.is_empty())
                .map(|b| format!("{} {}", b.keys_label, b.label))
                .collect::<Vec<_>>()
        })
        .filter(|section| !section.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::tui::theme::Theme;
    use sonos_sdk::SonosSystem;

    fn app() -> App {
        let system = SonosSystem::with_groups(&["Kitchen"]);
        App::with_system(system, Config::default(), Theme::default_theme(), None)
    }

    /// Every key in `keymap` resolves to the command it is listed with, so no
    /// key is shadowed by an earlier binding.
    fn assert_each_binding_dispatches<C: Copy + PartialEq + std::fmt::Debug>(keymap: Keymap<C>) {
        for binding in keymap.iter().flat_map(|s| s.iter()) {
            for (key, command) in binding.keys {
                assert_eq!(lookup(keymap, *key), Some(*command), "{key:?}");
            }
        }
    }

    #[test]
    fn each_binding_dispatches_its_command() {
        assert_each_binding_dispatches(GLOBAL_KEYS);
        assert_each_binding_dispatches(TAB_BAR_KEYS);
        assert_each_binding_dispatches(SPEAKER_KEYS);
        assert_each_binding_dispatches(SATELLITE_KEYS);
        assert_each_binding_dispatches(PICK_UP_KEYS);
        assert_each_binding_dispatches(SETTINGS_KEYS);
        assert_each_binding_dispatches(DROPDOWN_KEYS);
    }

    #[test]
    fn unbound_keys_look_up_nothing() {
        assert_eq!(lookup(SPEAKER_KEYS, KeyCode::Char('z')), None);
        assert_eq!(lookup(PICK_UP_KEYS, KeyCode::Char('p')), None);
        assert_eq!(lookup(PICK_UP_KEYS, KeyCode::Char('q')), None);
    }

    #[test]
    fn speakers_legend_is_built_from_the_table() {
        assert_eq!(
            legend(&app()),
            "\u{2191}\u{2193} Navigate   \u{2190}\u{2192} Volume  m Mute   \u{2423} Pick up/Drop   \
             p Play/Pause  n Next  b Prev   \u{238b} Quit"
        );
    }

    #[test]
    fn settings_legends_are_built_from_the_tables() {
        let mut app = app();
        app.navigation.tab = Tab::Settings;
        assert_eq!(
            legend(&app),
            "\u{2191}\u{2193} Navigate  Enter Open   \u{2190}\u{2192} Tabs   \u{238b} Quit"
        );
        app.navigation.settings_state.dropdown_open = true;
        assert_eq!(
            legend(&app),
            "\u{2191}\u{2193} Select  Enter Confirm  \u{238b} Cancel"
        );
    }

    #[test]
    fn mode_follows_navigation_state() {
        let mut app = app();
        assert_eq!(Mode::of(&app), Mode::Speakers);
        app.navigation.tab_focused = true;
        assert_eq!(Mode::of(&app), Mode::TabBar);
        app.navigation.tab_focused = false;
        app.navigation.tab = Tab::Settings;
        assert_eq!(Mode::of(&app), Mode::Settings);
        app.navigation.settings_state.dropdown_open = true;
        assert_eq!(Mode::of(&app), Mode::Dropdown);
    }
}
