//! Keyboard controls for the interactive garden.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::flower::Species;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    NewGarden,
    PreviousGarden,
    NextGarden,
    ReplayBloom,
    ToggleSpecies(Species),
    AllSpecies,
    MoreFlowers,
    FewerFlowers,
    ToggleGrass,
    Quit,
}

/// Reminders shown in the status bar, as (key, what it does). Species toggles aren't
/// listed because the species row already shows each one's number.
pub const KEY_HINTS: [(&str, &str); 7] = [
    ("space", "new"),
    ("←/→", "history"),
    ("b", "bloom"),
    ("0", "all species"),
    ("+/-", "flowers"),
    ("g", "grass"),
    ("q", "quit"),
];

pub fn command_for(key: KeyEvent) -> Option<Command> {
    // Some platforms also report key releases; acting on those would double every press.
    if key.kind == KeyEventKind::Release {
        return None;
    }
    // Raw mode swallows the interrupt signal, so Ctrl-C has to be handled as a key.
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return (key.code == KeyCode::Char('c')).then_some(Command::Quit);
    }
    match key.code {
        KeyCode::Char(' ') | KeyCode::Enter => Some(Command::NewGarden),
        KeyCode::Left => Some(Command::PreviousGarden),
        KeyCode::Right => Some(Command::NextGarden),
        KeyCode::Up => Some(Command::MoreFlowers),
        KeyCode::Down => Some(Command::FewerFlowers),
        KeyCode::Esc => Some(Command::Quit),
        KeyCode::Char(character) => command_for_character(character),
        _ => None,
    }
}

fn command_for_character(character: char) -> Option<Command> {
    match character {
        'n' => Some(Command::NewGarden),
        'b' => Some(Command::ReplayBloom),
        'g' => Some(Command::ToggleGrass),
        'q' => Some(Command::Quit),
        '0' => Some(Command::AllSpecies),
        // `=` and `_` share keys with `+` and `-`, so they work without Shift.
        '+' | '=' => Some(Command::MoreFlowers),
        '-' | '_' => Some(Command::FewerFlowers),
        digit => species_numbered(digit).map(Command::ToggleSpecies),
    }
}

/// Species are numbered from 1, in the order the status bar lists them.
fn species_numbered(digit: char) -> Option<Species> {
    let number = digit.to_digit(10)? as usize;
    Species::ALL.get(number.checked_sub(1)?).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(code: KeyCode) -> Option<Command> {
        command_for(KeyEvent::new(code, KeyModifiers::NONE))
    }

    #[test]
    fn digits_toggle_species_in_listed_order() {
        for (number, species) in (1..).zip(Species::ALL) {
            let digit = char::from_digit(number, 10).expect("fewer than ten species");
            assert_eq!(
                press(KeyCode::Char(digit)),
                Some(Command::ToggleSpecies(species))
            );
        }
        assert_eq!(press(KeyCode::Char('9')), None);
        assert_eq!(press(KeyCode::Char('0')), Some(Command::AllSpecies));
    }

    #[test]
    fn control_c_quits_but_plain_c_does_nothing() {
        let control_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(command_for(control_c), Some(Command::Quit));
        assert_eq!(press(KeyCode::Char('c')), None);
    }

    #[test]
    fn releases_are_ignored() {
        let release = KeyEvent::new_with_kind(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
            KeyEventKind::Release,
        );
        assert_eq!(command_for(release), None);
    }
}
