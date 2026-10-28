//! Maps a key press to a message. The mode decides first, so a key can only ever do what the
//! current screen lists.

use crate::model::{Message, Mode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn key_to_message(mode: Mode, key: KeyEvent) -> Option<Message> {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return (key.code == KeyCode::Char('c')).then_some(Message::Quit);
    }

    match mode {
        Mode::Normal => normal_keys(key.code),
        Mode::EditingTask => editing_keys(key.code),
        Mode::ConfirmingQuit => confirming_keys(key.code),
        Mode::Help | Mode::Stats => overlay_keys(mode, key.code),
        Mode::Settings => settings_keys(key.code),
    }
}

fn normal_keys(code: KeyCode) -> Option<Message> {
    match code {
        KeyCode::Char(' ') => Some(Message::ToggleTimer),
        KeyCode::Char('s') => Some(Message::Skip),
        KeyCode::Char('r') => Some(Message::Reset),
        KeyCode::Char('+') | KeyCode::Char('=') => Some(Message::AdjustTime(1)),
        KeyCode::Char('-') => Some(Message::AdjustTime(-1)),
        KeyCode::Char('t') => Some(Message::StartEditing),
        KeyCode::Char('?') => Some(Message::OpenHelp),
        KeyCode::Char(',') => Some(Message::OpenSettings),
        KeyCode::Tab => Some(Message::OpenStats),
        KeyCode::Char('q') => Some(Message::AskToQuit),
        _ => None,
    }
}

fn editing_keys(code: KeyCode) -> Option<Message> {
    match code {
        KeyCode::Enter => Some(Message::FinishEditing),
        KeyCode::Esc => Some(Message::CancelEditing),
        KeyCode::Backspace => Some(Message::Backspace),
        KeyCode::Char(c) => Some(Message::TypeChar(c)),
        _ => None,
    }
}

fn confirming_keys(code: KeyCode) -> Option<Message> {
    match code {
        KeyCode::Char('y') => Some(Message::Quit),
        KeyCode::Char('n') | KeyCode::Esc => Some(Message::CancelQuit),
        _ => None,
    }
}

/// Help and stats are read-only: the key that opened them, `q` or Esc closes them.
fn overlay_keys(mode: Mode, code: KeyCode) -> Option<Message> {
    let opener = match mode {
        Mode::Help => KeyCode::Char('?'),
        _ => KeyCode::Tab,
    };
    (code == opener || code == KeyCode::Esc || code == KeyCode::Char('q'))
        .then_some(Message::CloseOverlay)
}

fn settings_keys(code: KeyCode) -> Option<Message> {
    match code {
        KeyCode::Up | KeyCode::Char('k') => Some(Message::SettingsMove(-1)),
        KeyCode::Down | KeyCode::Char('j') => Some(Message::SettingsMove(1)),
        KeyCode::Left | KeyCode::Char('h') => Some(Message::SettingsAdjust(-1)),
        KeyCode::Right | KeyCode::Char('l') => Some(Message::SettingsAdjust(1)),
        KeyCode::Esc | KeyCode::Enter | KeyCode::Char(',') | KeyCode::Char('q') => {
            Some(Message::CloseOverlay)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_MODES: [Mode; 6] = [
        Mode::Normal,
        Mode::EditingTask,
        Mode::ConfirmingQuit,
        Mode::Help,
        Mode::Settings,
        Mode::Stats,
    ];

    fn press(mode: Mode, code: KeyCode) -> Option<Message> {
        key_to_message(mode, KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn ch(mode: Mode, c: char) -> Option<Message> {
        press(mode, KeyCode::Char(c))
    }

    #[test]
    fn test_normal_mode_keys() {
        assert_eq!(ch(Mode::Normal, ' '), Some(Message::ToggleTimer));
        assert_eq!(ch(Mode::Normal, 's'), Some(Message::Skip));
        assert_eq!(ch(Mode::Normal, 'r'), Some(Message::Reset));
        assert_eq!(ch(Mode::Normal, '+'), Some(Message::AdjustTime(1)));
        assert_eq!(ch(Mode::Normal, '-'), Some(Message::AdjustTime(-1)));
        assert_eq!(ch(Mode::Normal, 't'), Some(Message::StartEditing));
        assert_eq!(ch(Mode::Normal, '?'), Some(Message::OpenHelp));
        assert_eq!(ch(Mode::Normal, ','), Some(Message::OpenSettings));
        assert_eq!(press(Mode::Normal, KeyCode::Tab), Some(Message::OpenStats));
        assert_eq!(ch(Mode::Normal, 'q'), Some(Message::AskToQuit));
    }

    #[test]
    fn test_escape_does_nothing_in_normal_mode() {
        assert_eq!(press(Mode::Normal, KeyCode::Esc), None);
    }

    #[test]
    fn test_timer_keys_do_nothing_outside_normal_mode() {
        for mode in ALL_MODES.into_iter().filter(|m| *m != Mode::Normal) {
            for c in [' ', 's', 'r', '+', '-', 't'] {
                let message = ch(mode, c);

                assert!(
                    !matches!(
                        message,
                        Some(
                            Message::ToggleTimer
                                | Message::Skip
                                | Message::Reset
                                | Message::AdjustTime(_)
                                | Message::StartEditing
                        )
                    ),
                    "{c:?} fired {message:?} in {mode:?}"
                );
            }
        }
    }

    #[test]
    fn test_typing_sends_every_character_including_command_letters() {
        for c in ['a', 'q', 's', ' ', 'r', '?'] {
            assert_eq!(ch(Mode::EditingTask, c), Some(Message::TypeChar(c)));
        }
    }

    #[test]
    fn test_editing_special_keys() {
        assert_eq!(
            press(Mode::EditingTask, KeyCode::Enter),
            Some(Message::FinishEditing)
        );
        assert_eq!(
            press(Mode::EditingTask, KeyCode::Esc),
            Some(Message::CancelEditing)
        );
        assert_eq!(
            press(Mode::EditingTask, KeyCode::Backspace),
            Some(Message::Backspace)
        );
    }

    #[test]
    fn test_confirming_quit_keys() {
        assert_eq!(ch(Mode::ConfirmingQuit, 'y'), Some(Message::Quit));
        assert_eq!(ch(Mode::ConfirmingQuit, 'n'), Some(Message::CancelQuit));
        assert_eq!(
            press(Mode::ConfirmingQuit, KeyCode::Esc),
            Some(Message::CancelQuit)
        );
        assert_eq!(ch(Mode::ConfirmingQuit, 'q'), None);
    }

    #[test]
    fn test_overlays_close_with_their_key_q_or_escape() {
        assert_eq!(ch(Mode::Help, '?'), Some(Message::CloseOverlay));
        assert_eq!(press(Mode::Help, KeyCode::Esc), Some(Message::CloseOverlay));
        assert_eq!(ch(Mode::Stats, 'q'), Some(Message::CloseOverlay));
        assert_eq!(
            press(Mode::Stats, KeyCode::Tab),
            Some(Message::CloseOverlay)
        );
        assert_eq!(ch(Mode::Help, 'x'), None);
    }

    #[test]
    fn test_settings_keys() {
        assert_eq!(
            press(Mode::Settings, KeyCode::Up),
            Some(Message::SettingsMove(-1))
        );
        assert_eq!(ch(Mode::Settings, 'j'), Some(Message::SettingsMove(1)));
        assert_eq!(
            press(Mode::Settings, KeyCode::Left),
            Some(Message::SettingsAdjust(-1))
        );
        assert_eq!(ch(Mode::Settings, 'l'), Some(Message::SettingsAdjust(1)));
        assert_eq!(
            press(Mode::Settings, KeyCode::Esc),
            Some(Message::CloseOverlay)
        );
    }

    #[test]
    fn test_ctrl_c_quits_from_every_mode_and_other_ctrl_keys_do_nothing() {
        for mode in ALL_MODES {
            let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
            let ctrl_s = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL);

            assert_eq!(key_to_message(mode, ctrl_c), Some(Message::Quit));
            assert_eq!(key_to_message(mode, ctrl_s), None);
        }
    }
}
