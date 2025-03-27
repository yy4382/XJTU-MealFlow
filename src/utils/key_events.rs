use std::ops::Deref;

use crossterm::event::{KeyCode, KeyEvent as crosstermKeyEvent, KeyModifiers};

#[cfg(not(tarpaulin_include))]
pub fn key_event_to_string(key_event: &crosstermKeyEvent) -> String {
    let char;
    let key_code = match key_event.code {
        KeyCode::Backspace => "backspace",
        KeyCode::Enter => "enter",
        KeyCode::Left => "left",
        KeyCode::Right => "right",
        KeyCode::Up => "up",
        KeyCode::Down => "down",
        KeyCode::Home => "home",
        KeyCode::End => "end",
        KeyCode::PageUp => "pageup",
        KeyCode::PageDown => "pagedown",
        KeyCode::Tab => "tab",
        KeyCode::BackTab => "backtab",
        KeyCode::Delete => "delete",
        KeyCode::Insert => "insert",
        KeyCode::F(c) => {
            char = format!("f({c})");
            &char
        }
        KeyCode::Char(' ') => "space",
        KeyCode::Char(c) => {
            char = c.to_string();
            &char
        }
        KeyCode::Esc => "esc",
        KeyCode::Null => "",
        KeyCode::CapsLock => "",
        KeyCode::Menu => "",
        KeyCode::ScrollLock => "",
        KeyCode::Media(_) => "",
        KeyCode::NumLock => "",
        KeyCode::PrintScreen => "",
        KeyCode::Pause => "",
        KeyCode::KeypadBegin => "",
        KeyCode::Modifier(_) => "",
    };

    let mut modifiers = Vec::with_capacity(3);

    if key_event.modifiers.intersects(KeyModifiers::CONTROL) {
        modifiers.push("ctrl");
    }

    if key_event.modifiers.intersects(KeyModifiers::SHIFT) {
        modifiers.push("shift");
    }

    if key_event.modifiers.intersects(KeyModifiers::ALT) {
        modifiers.push("alt");
    }

    let mut key = modifiers.join("-");

    if !key.is_empty() {
        key.push('-');
    }
    key.push_str(key_code);

    key
}

#[derive(Clone, Debug, PartialEq)]
pub struct KeyEvent(pub crosstermKeyEvent);

impl From<crosstermKeyEvent> for KeyEvent {
    fn from(key_event: crosstermKeyEvent) -> Self {
        Self(key_event)
    }
}
impl From<KeyCode> for KeyEvent {
    fn from(key_code: KeyCode) -> Self {
        Self(crosstermKeyEvent::new(key_code, KeyModifiers::NONE))
    }
}
impl From<char> for KeyEvent {
    fn from(c: char) -> Self {
        Self(crosstermKeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE))
    }
}
impl AsRef<crosstermKeyEvent> for KeyEvent {
    fn as_ref(&self) -> &crosstermKeyEvent {
        &self.0
    }
}
impl From<KeyEvent> for crosstermKeyEvent {
    fn from(val: KeyEvent) -> Self {
        val.0
    }
}
impl Deref for KeyEvent {
    type Target = crosstermKeyEvent;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<KeyEvent> for String {
    fn from(val: KeyEvent) -> Self {
        key_event_to_string(&val.0)
    }
}
impl std::fmt::Display for KeyEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", key_event_to_string(self))
    }
}

#[cfg(test)]
pub mod test_utils {
    use crate::tui::Event;

    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};

    pub fn get_key_evt(key: KeyCode) -> Event {
        Event::Key(crosstermKeyEvent::new(key, KeyModifiers::NONE))
    }
    pub fn get_char_evt(key: char) -> Event {
        Event::Key(crosstermKeyEvent::new(
            KeyCode::Char(key),
            KeyModifiers::NONE,
        ))
    }
}
