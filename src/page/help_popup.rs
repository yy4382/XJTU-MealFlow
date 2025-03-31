use std::cmp::{max, min};

use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style, palette::tailwind},
    text::{Line, Text},
    widgets::{Block, BorderType, Borders, Clear, HighlightSpacing, List, ListItem, Padding},
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

    longest_entry_size: u16,
    list_state: ratatui::widgets::ListState,

    tx: ActionSender,
}

impl HelpPopup {
    pub fn new(tx: ActionSender, msg: HelpMsg) -> Option<Self> {
        if msg.is_empty() {
            return None;
        }
        // FIXME use real size
        let longest = msg
            .iter()
            .map(|entry| UnicodeWidthStr::width(String::from(entry.clone()).as_str()))
            .max()
            .unwrap();

        Some(Self {
            help_msg: msg,
            longest_entry_size: longest as u16,
            list_state: ratatui::widgets::ListState::default(),
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
        match event {
            Event::Key(key) => match key.code {
                KeyCode::Esc => {
                    self.tx.send(LayerManageAction::PopPage);
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
                self.list_state.select_previous();
            }
            HelpPopupAction::Down => {
                self.list_state.select_next();
            }
            HelpPopupAction::Start => {
                self.list_state.select_first();
            }
            HelpPopupAction::End => {
                self.list_state.select_last();
            }
        }
    }
}

impl Layer for HelpPopup {}

impl WidgetExt for HelpPopup {
    fn render(&mut self, frame: &mut ratatui::Frame, area: ratatui::prelude::Rect) {
        let width = max(self.longest_entry_size + 8, min(50, frame.area().width - 4));
        let show_area = Rect {
            // FIXME deal with subtract overflow
            x: (area.width - width) / 2,
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
            .fg(LIST_COLORS.selected_row_style_fg);

        let block = Block::new()
            .title(Line::raw("Help").centered())
            .border_type(BorderType::Rounded)
            .borders(Borders::ALL)
            .padding(Padding::horizontal(1))
            .padding(Padding::vertical(1));

        let items: Vec<ListItem> = self
            .help_msg
            .iter()
            .map(|entry| ListItem::from(Text::raw(format!("  {}  ", entry))))
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(selected_row_style)
            .highlight_spacing(HighlightSpacing::Always);

        frame.render_stateful_widget(list, area, &mut self.list_state);
    }
}

struct ListColors {
    // buffer_bg: Color,
    // header_bg: Color,
    // header_fg: Color,
    // row_fg: Color,
    selected_row_style_fg: Color,
    // selected_column_style_fg: Color,
    // selected_cell_style_fg: Color,
    // normal_row_color: Color,
    // alt_row_color: Color,
    // footer_border_color: Color,
}

impl Default for ListColors {
    fn default() -> Self {
        Self {
            // buffer_bg: tailwind::GRAY.c950,
            // header_bg: tailwind::INDIGO.c950,
            // header_fg: tailwind::GRAY.c100,
            // row_fg: tailwind::INDIGO.c200,
            selected_row_style_fg: tailwind::INDIGO.c400,
            // selected_column_style_fg: tailwind::INDIGO.c400,
            // selected_cell_style_fg: tailwind::INDIGO.c600,
            // normal_row_color: tailwind::SLATE.c950,
            // alt_row_color: tailwind::SLATE.c900,
            // footer_border_color: tailwind::INDIGO.c400,
        }
    }
}

lazy_static::lazy_static! {
    static ref LIST_COLORS: ListColors = ListColors::default();
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
        assert_eq!(help_popup.longest_entry_size, 7);
        assert_eq!(help_popup.list_state.selected(), None);
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
            assert_eq!(help_popup.list_state.selected(), expected);
        };

        test_loop('j', Some(0));
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
