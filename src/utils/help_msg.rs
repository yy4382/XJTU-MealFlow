use std::ops::{Deref, DerefMut};

use ratatui::widgets::{Block, BorderType, Borders, Padding};

use super::key_events::KeyEvent;

#[derive(Debug, Clone)]

enum HelpKeyEvent {
    Key(KeyEvent),
    Plain(String),
}
// TODO add a long_desc field to HelpEntry to show in popup

#[derive(Debug, Clone)]
pub(crate) struct HelpEntry {
    key: HelpKeyEvent,
    desc: String,
}

impl HelpEntry {
    pub(crate) fn new<T: Into<String>, K: Into<KeyEvent>>(event: K, desc: T) -> Self {
        Self {
            key: HelpKeyEvent::Key(event.into()),
            desc: desc.into(),
        }
    }
    pub(crate) fn new_plain<T: Into<String>>(event: T, desc: T) -> Self {
        Self {
            key: HelpKeyEvent::Plain(event.into()),
            desc: desc.into(),
        }
    }

    pub(crate) fn key(&self) -> String {
        match &self.key {
            HelpKeyEvent::Key(key) => key.to_string(),
            HelpKeyEvent::Plain(key) => key.clone(),
        }
    }

    pub(crate) fn desc(&self) -> &str {
        &self.desc
    }
}

impl std::fmt::Display for HelpEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.desc(), self.key())
    }
}

impl From<HelpEntry> for String {
    fn from(val: HelpEntry) -> Self {
        format!("{}", val)
    }
}

#[derive(Default, Clone, Debug)]
pub(crate) struct HelpMsg {
    slices: Vec<HelpEntry>,
}

impl From<Vec<HelpEntry>> for HelpMsg {
    fn from(slices: Vec<HelpEntry>) -> Self {
        Self { slices }
    }
}

impl HelpMsg {
    pub(crate) fn extend(&mut self, other: &HelpMsg) {
        self.slices.extend(other.slices.clone());
    }

    pub(crate) fn extend_ret(mut self, other: &HelpMsg) -> Self {
        self.slices.extend(other.slices.clone());
        self
    }
    pub(crate) fn push(&mut self, entry: HelpEntry) {
        self.slices.push(entry);
    }

    pub(crate) fn render(&mut self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        let help_msg: String = self.into();
        let paragraph = ratatui::widgets::Paragraph::new(help_msg).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .padding(Padding::horizontal(1)),
        );
        frame.render_widget(paragraph, area);
    }
}

impl Deref for HelpMsg {
    type Target = Vec<HelpEntry>;

    fn deref(&self) -> &Self::Target {
        &self.slices
    }
}

impl DerefMut for HelpMsg {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.slices
    }
}

impl From<HelpMsg> for String {
    fn from(val: HelpMsg) -> Self {
        val.slices
            .into_iter()
            .map(|s| s.into())
            .collect::<Vec<String>>()
            .join(" | ")
    }
}

impl From<&mut HelpMsg> for String {
    fn from(val: &mut HelpMsg) -> Self {
        val.slices
            .clone()
            .into_iter()
            .map(|s| s.into())
            .collect::<Vec<String>>()
            .join(" | ")
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_help_entry_key() {
        let entry = HelpEntry::new('c', "Create a new transaction");
        assert_eq!(entry.key(), "c");
        assert_eq!(entry.desc(), "Create a new transaction");
        assert_eq!(entry.to_string(), "Create a new transaction: c");
    }
    #[test]
    fn test_help_entry_plain() {
        let entry = HelpEntry::new_plain("hjkl", "Move cursor");
        assert_eq!(entry.key(), "hjkl");
        assert_eq!(entry.desc(), "Move cursor");
        assert_eq!(entry.to_string(), "Move cursor: hjkl");
    }
}
