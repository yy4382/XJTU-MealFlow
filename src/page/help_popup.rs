use std::cmp::{max, min};

use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Modifier, Style, palette::tailwind},
    text::{Line, Text},
    widgets::{Block, BorderType, Borders, Cell, Clear, HighlightSpacing, Padding, Row, Table},
};
use unicode_width::UnicodeWidthStr;

use crate::{
    actions::{Action, ActionSender, LayerManageAction},
    tui::Event,
    utils::help_msg::{HelpEntry, HelpMsg},
};

use super::{EventLoopParticipant, Layer, WidgetExt};

pub(crate) struct HelpPopup {
    help_msg: HelpMsg,

    longest_key: u16,
    longest_desc: u16,

    table_state: ratatui::widgets::TableState,

    tx: ActionSender,
}

impl HelpPopup {
    pub fn new(tx: ActionSender, msg: HelpMsg) -> Option<Self> {
        if msg.is_empty() {
            return None;
        }

        let longest_key = msg
            .iter()
            .map(|entry| UnicodeWidthStr::width(entry.key().as_str()))
            .max()
            .unwrap();

        let longest_desc = msg
            .iter()
            .map(|entry| UnicodeWidthStr::width(entry.desc()))
            .max()
            .unwrap();

        Some(Self {
            help_msg: msg,
            longest_key: longest_key as u16,
            longest_desc: longest_desc as u16,
            table_state: ratatui::widgets::TableState::default().with_selected(0),
            tx,
        })
    }
}

#[derive(Clone, Debug)]
pub enum HelpPopupAction {
    Up,
    Down,
    Start,
    End,
}
impl From<HelpPopupAction> for Action {
    fn from(value: HelpPopupAction) -> Self {
        Action::HelpPopup(value)
    }
}

impl EventLoopParticipant for HelpPopup {
    fn handle_events(&self, event: crate::tui::Event) -> color_eyre::eyre::Result<()> {
        #[allow(clippy::single_match)]
        match event {
            Event::Key(key) => match key.code {
                KeyCode::Esc => {
                    self.tx.send(LayerManageAction::Pop);
                }
                KeyCode::Char('j') => {
                    self.tx.send(HelpPopupAction::Down);
                }
                KeyCode::Char('k') => {
                    self.tx.send(HelpPopupAction::Up);
                }
                KeyCode::Char('g') => {
                    self.tx.send(HelpPopupAction::Start);
                }
                KeyCode::Char('G') => {
                    self.tx.send(HelpPopupAction::End);
                }
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }

    fn update(&mut self, action: crate::actions::Action) {
        let Action::HelpPopup(actions) = action else {
            return;
        };
        match actions {
            HelpPopupAction::Up => {
                self.table_state.select_previous();
            }
            HelpPopupAction::Down => {
                self.table_state.select_next();
            }
            HelpPopupAction::Start => {
                self.table_state.select_first();
            }
            HelpPopupAction::End => {
                self.table_state.select_last();
            }
        }
    }
}

impl Layer for HelpPopup {}

impl WidgetExt for HelpPopup {
    fn render(&mut self, frame: &mut ratatui::Frame, area: ratatui::prelude::Rect) {
        let width = min(
            max(self.longest_desc + self.longest_key + 8, 50),
            frame.area().width - 4,
        );
        let show_area = Rect {
            x: (area.width.saturating_sub(width)) / 2,
            y: area.height / 6,
            width,
            height: area.height * 2 / 3,
        };
        let bottom_help_area = Rect {
            x: 0,
            y: area.height - 3,
            width: area.width,
            height: 3,
        };

        frame.render_widget(Clear, bottom_help_area);
        HelpPopup::get_self_help_msg().render(frame, bottom_help_area);

        frame.render_widget(Clear, show_area);
        self.render_list(frame, show_area);
    }
}

impl HelpPopup {
    pub fn get_self_help_msg() -> HelpMsg {
        let help_msg = vec![
            HelpEntry::new('j', "Go Down"),
            HelpEntry::new('k', "Go Up"),
            HelpEntry::new('g', "Go to Top"),
            HelpEntry::new('G', "Go to Bottom"),
            HelpEntry::new(KeyCode::Esc, "Close help"),
        ];
        help_msg.into()
    }

    fn render_list(&mut self, frame: &mut Frame, area: Rect) {
        let selected_row_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(tailwind::INDIGO.c300);

        let block = Block::new()
            .title(Line::raw("Help").centered())
            .border_type(BorderType::Rounded)
            .borders(Borders::ALL)
            .padding(Padding::horizontal(1))
            .padding(Padding::vertical(1));

        let items = self.help_msg.iter().map(|entry| {
            Row::new([
                Cell::new(Text::raw(format!("  {}", entry.key())).right_aligned())
                    .style(Style::default().fg(tailwind::BLUE.c400)),
                Cell::new(entry.desc().to_string()),
            ])
        });

        let list = Table::new(
            items,
            [
                Constraint::Length(self.longest_key + 2),
                Constraint::Min(self.longest_desc),
            ],
        )
        .block(block)
        .row_highlight_style(selected_row_style)
        .highlight_spacing(HighlightSpacing::Always)
        .column_spacing(2);

        frame.render_stateful_widget(list, area, &mut self.table_state);
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;
    use ratatui::Terminal;

    use super::*;

    #[test]
    fn test_help_popup_new() {
        let (tx, _) = tokio::sync::mpsc::unbounded_channel();
        let help_popup =
            HelpPopup::new(tx.into(), vec![HelpEntry::new('a', "test")].into()).unwrap();
        assert_eq!(help_popup.help_msg.len(), 1);
        assert_eq!(help_popup.longest_desc, 4);
        assert_eq!(help_popup.longest_key, 1);
        assert_eq!(help_popup.table_state.selected(), Some(0));
    }

    #[test]
    fn test_navigation() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let mut help_popup = HelpPopup::new(
            tx.into(),
            vec![
                HelpEntry::new('a', "test"),
                HelpEntry::new('b', "test2"),
                HelpEntry::new('c', "test3"),
            ]
            .into(),
        )
        .unwrap();
        let mut terminal = Terminal::new(ratatui::backend::TestBackend::new(80, 25)).unwrap();

        let mut test_loop = |key: char, expected: Option<usize>| {
            help_popup.event_loop_once(&mut rx, key.into());
            terminal
                .draw(|f| {
                    help_popup.render(f, f.area());
                })
                .unwrap();
            assert_eq!(help_popup.table_state.selected(), expected);
        };

        test_loop('j', Some(1));
        test_loop('k', Some(0));
        test_loop('G', Some(2));
        test_loop('k', Some(1));
        test_loop('j', Some(2));
        test_loop('g', Some(0));
    }

    #[test]
    fn test_help_popup_render() {
        let (tx, _) = tokio::sync::mpsc::unbounded_channel();
        let mut help_popup = HelpPopup::new(
            tx.into(),
            vec![
                HelpEntry::new('a', "test"),
                HelpEntry::new('b', "test2"),
                HelpEntry::new('c', "test3"),
            ]
            .into(),
        )
        .unwrap();
        let mut terminal = Terminal::new(ratatui::backend::TestBackend::new(80, 25)).unwrap();
        terminal
            .draw(|f| {
                help_popup.render(f, f.area());
            })
            .unwrap();
        assert_snapshot!(terminal.backend());
    }
}
