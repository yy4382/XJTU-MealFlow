use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style, palette::tailwind},
    text::{Line, Text},
    widgets::{Block, BorderType, Borders, Clear, HighlightSpacing, List, ListItem, Padding},
};
use tracing::info;
use unicode_width::UnicodeWidthStr;

use crate::{
    actions::{Action, ActionSender, LayerManageAction},
    tui::Event,
    utils::help_msg::{HelpEntry, HelpMsg},
};

use super::{EventLoopParticipant, Page, WidgetExt};

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

impl Page for HelpPopup {}

impl WidgetExt for HelpPopup {
    fn render(&mut self, frame: &mut ratatui::Frame, area: ratatui::prelude::Rect) {
        info!("Rendering help popup, {:?}", area);
        let show_area = Rect {
            // FIXME deal with subtract overflow
            x: (area.width - self.longest_entry_size - 6) / 2,
            y: area.height / 6,
            width: self.longest_entry_size + 8,
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
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .padding(Padding::horizontal(1));

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
            // buffer_bg: tailwind::SLATE.c950,
            // header_bg: tailwind::INDIGO.c900,
            // header_fg: tailwind::SLATE.c200,
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
