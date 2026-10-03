//! The single source of truth for keybindings advertised in the help popup.
//!
//! The table is static data so that the popup's line/selection math can be
//! answered without building any `Line`s. `ui::help` renders it, and
//! `runtime::input::keyboard` is checked against it by a drift test.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// One advertised keybinding.
pub struct Keymap {
    label: &'static str,
    ascii_label: Option<&'static str>,
    pub description: &'static str,
    code: KeyCode,
    modifiers: KeyModifiers,
}

impl Keymap {
    /// The label to show, falling back to the glyph label when the ASCII
    /// variant is identical.
    pub fn label(&self, icons_ascii: bool) -> &'static str {
        match self.ascii_label {
            Some(ascii) if icons_ascii => ascii,
            _ => self.label,
        }
    }

    /// The event replayed when this command is run from the help popup.
    pub fn key_event(&self) -> KeyEvent {
        KeyEvent::new(self.code, self.modifiers)
    }
}

pub struct Section {
    pub title: &'static str,
    pub commands: &'static [Keymap],
}

const fn key(label: &'static str, description: &'static str, code: KeyCode) -> Keymap {
    Keymap {
        label,
        ascii_label: None,
        description,
        code,
        modifiers: KeyModifiers::NONE,
    }
}

/// A binding whose label contains a glyph, with an ASCII fallback.
const fn glyph_key(
    label: &'static str,
    ascii_label: &'static str,
    description: &'static str,
    code: KeyCode,
) -> Keymap {
    Keymap {
        label,
        ascii_label: Some(ascii_label),
        description,
        code,
        modifiers: KeyModifiers::NONE,
    }
}

const fn modified_key(
    label: &'static str,
    description: &'static str,
    code: KeyCode,
    modifiers: KeyModifiers,
) -> Keymap {
    Keymap {
        label,
        ascii_label: None,
        description,
        code,
        modifiers,
    }
}

pub static SECTIONS: &[Section] = &[
    Section {
        title: "Navigation",
        commands: &[
            glyph_key("j / ↓", "j / down", "Move down", KeyCode::Char('j')),
            glyph_key("k / ↑", "k / up", "Move up", KeyCode::Char('k')),
            key("Tab", "Next panel", KeyCode::Tab),
            key("S-Tab", "Previous panel", KeyCode::BackTab),
            glyph_key(
                "l / ←",
                "l / left",
                "Previous status tab",
                KeyCode::Char('l'),
            ),
            glyph_key("; / →", "; / right", "Next status tab", KeyCode::Char(';')),
        ],
    },
    Section {
        title: "Search",
        commands: &[
            key("/", "Search installed list", KeyCode::Char('/')),
            key("f", "Find packages", KeyCode::Char('f')),
            key("C", "Toggle formula/cask list", KeyCode::Char('C')),
            key(
                "L",
                "Toggle leaves / all installed formulae",
                KeyCode::Char('L'),
            ),
            key("O", "Cycle sort: name / size / recent", KeyCode::Char('O')),
        ],
    },
    Section {
        title: "Actions",
        commands: &[
            key("Enter", "Load details", KeyCode::Enter),
            key("d", "Load deps/uses", KeyCode::Char('d')),
            key("i", "Install selected (confirm)", KeyCode::Char('i')),
            key("u", "Uninstall selected (confirm)", KeyCode::Char('u')),
            key(
                "U",
                "Upgrade selected or all outdated (confirm)",
                KeyCode::Char('U'),
            ),
            key("p", "Pin / unpin selected formula", KeyCode::Char('p')),
            key("g", "Open homepage in browser", KeyCode::Char('g')),
            key(
                "P",
                "Update Brewery via cargo (confirm)",
                KeyCode::Char('P'),
            ),
            key("S", "Start selected service (confirm)", KeyCode::Char('S')),
            key("X", "Stop selected service (confirm)", KeyCode::Char('X')),
            key(
                "R",
                "Restart selected service (confirm)",
                KeyCode::Char('R'),
            ),
            key("I", "Show selected service info", KeyCode::Char('I')),
            key("F", "Filter failed services", KeyCode::Char('F')),
            key("A", "Filter auto-start services", KeyCode::Char('A')),
            key("K", "Cycle service kind filter", KeyCode::Char('K')),
            key(
                "o",
                "Toggle outdated-only formula filter",
                KeyCode::Char('o'),
            ),
        ],
    },
    Section {
        title: "Data",
        commands: &[
            key("r", "Refresh formulae + casks", KeyCode::Char('r')),
            key("s", "Load sizes", KeyCode::Char('s')),
            key("h", "Status check", KeyCode::Char('h')),
            key("e", "Run brew update", KeyCode::Char('e')),
        ],
    },
    Section {
        title: "Other",
        commands: &[
            key("t", "Cycle theme", KeyCode::Char('t')),
            key("m", "Toggle mouse", KeyCode::Char('m')),
            modified_key(
                "Alt+i",
                "Toggle icons",
                KeyCode::Char('i'),
                KeyModifiers::ALT,
            ),
            key("c", "Cleanup", KeyCode::Char('c')),
            key("a", "Autoremove (confirm)", KeyCode::Char('a')),
            key("b", "Bundle dump", KeyCode::Char('b')),
            key("v", "Toggle view", KeyCode::Char('v')),
            key("q", "Quit", KeyCode::Char('q')),
            key("Esc", "Cancel action", KeyCode::Esc),
        ],
    },
];

/// Every advertised command, in popup order.
pub fn commands() -> impl Iterator<Item = &'static Keymap> {
    SECTIONS.iter().flat_map(|section| section.commands.iter())
}

pub fn command_count() -> usize {
    SECTIONS.iter().map(|section| section.commands.len()).sum()
}

pub fn command_at(index: usize) -> Option<&'static Keymap> {
    commands().nth(index)
}

/// Total rendered lines: one title per section, one per command, and a blank
/// separator between sections.
pub fn line_count() -> usize {
    let sections = SECTIONS.len();
    sections + command_count() + sections.saturating_sub(1)
}

/// The rendered line a command occupies.
pub fn command_line(command_index: usize) -> Option<usize> {
    let mut line = 0;
    let mut remaining = command_index;

    for (index, section) in SECTIONS.iter().enumerate() {
        line += 1; // section title
        if remaining < section.commands.len() {
            return Some(line + remaining);
        }
        remaining -= section.commands.len();
        line += section.commands.len();
        if index + 1 < SECTIONS.len() {
            line += 1; // blank separator
        }
    }

    None
}

/// The command occupying a rendered line, if that line is a command line.
pub fn command_index_at_line(line_index: usize) -> Option<usize> {
    let mut line = 0;
    let mut command_index = 0;

    for (index, section) in SECTIONS.iter().enumerate() {
        line += 1; // section title
        if line_index >= line && line_index < line + section.commands.len() {
            return Some(command_index + (line_index - line));
        }
        command_index += section.commands.len();
        line += section.commands.len();
        if index + 1 < SECTIONS.len() {
            line += 1; // blank separator
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advertises_each_key_once() {
        let mut seen: Vec<KeyEvent> = Vec::new();
        for command in commands() {
            let event = command.key_event();
            assert!(
                !seen.contains(&event),
                "{} is advertised more than once",
                command.label(true)
            );
            seen.push(event);
        }
    }

    #[test]
    fn command_lines_round_trip() {
        for index in 0..command_count() {
            let line = command_line(index).expect("command should have a line");
            assert!(line < line_count(), "line {line} is outside the popup");
            assert_eq!(command_index_at_line(line), Some(index));
        }
    }

    #[test]
    fn section_titles_and_separators_are_not_command_lines() {
        let command_lines: Vec<usize> = (0..command_count())
            .map(|index| command_line(index).expect("command should have a line"))
            .collect();

        for line in 0..line_count() {
            assert_eq!(
                command_index_at_line(line).is_some(),
                command_lines.contains(&line),
                "line {line} disagrees about being a command line"
            );
        }
    }

    #[test]
    fn rejects_out_of_range_lookups() {
        assert!(command_line(command_count()).is_none());
        assert!(command_index_at_line(line_count()).is_none());
    }
}
