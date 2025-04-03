use crossterm::event::KeyCode;
use merchant::MerchantData;
use ratatui::{
    layout::{Constraint, Layout},
    style::{Stylize, palette::tailwind},
    widgets::Tabs,
};
use strum::{Display, EnumIter, IntoEnumIterator};
use time_period::TimePeriodData;

use crate::{
    actions::{Action, ActionSender, LayerManageAction},
    libs::transactions::{Transaction, TransactionManager},
    tui::Event,
    utils::help_msg::{HelpEntry, HelpMsg},
};

use super::{EventLoopParticipant, Layer, WidgetExt};

mod merchant;
mod time_period;

pub(crate) struct Analysis {
    manager: crate::libs::transactions::TransactionManager,
    tx: ActionSender,

    analysis_type: AnalysisType,
    data: Vec<Transaction>,
}
/// false for left/up, true for right/down
type MoveDirection = bool;
#[derive(Clone, Debug)]
pub(crate) enum AnalysisAction {
    MoveTypeFocus(MoveDirection),
    Scroll(MoveDirection),
}
impl From<AnalysisAction> for Action {
    fn from(value: AnalysisAction) -> Self {
        Action::Analysis(value)
    }
}

#[derive(Display, EnumIter)]
enum AnalysisType {
    #[strum(to_string = "Time Period")]
    TimePeriod(TimePeriodData),
    #[strum(to_string = "Merchant")]
    Merchant(MerchantData),
    // MerchantCategory,
}

impl AnalysisType {
    fn next(&self, data: &Vec<Transaction>) -> Self {
        match self {
            Self::TimePeriod(_) => Self::Merchant(MerchantData::new(data)),
            Self::Merchant(_) => Self::TimePeriod(TimePeriodData::new(data)),
        }
    }
    fn previous(&self, data: &Vec<Transaction>) -> Self {
        match self {
            Self::TimePeriod(_) => Self::Merchant(MerchantData::new(data)),
            Self::Merchant(_) => Self::TimePeriod(TimePeriodData::new(data)),
        }
    }
    fn to_index(&self) -> usize {
        match self {
            AnalysisType::TimePeriod(_) => 0,
            AnalysisType::Merchant(_) => 1,
        }
    }
    fn get_palette(&self) -> tailwind::Palette {
        match self {
            AnalysisType::TimePeriod(_) => tailwind::BLUE,
            AnalysisType::Merchant(_) => tailwind::INDIGO,
        }
    }
}

impl Analysis {
    pub fn new(tx: ActionSender, manager: TransactionManager) -> Self {
        let mut new = Self {
            manager,
            tx,
            analysis_type: AnalysisType::TimePeriod(Default::default()),
            data: vec![],
        };
        new.data = new
            .manager
            .fetch_all()
            .expect("Failed to load transactions");
        new.analysis_type = AnalysisType::TimePeriod(TimePeriodData::new(&new.data));
        new
    }
}

impl EventLoopParticipant for Analysis {
    fn handle_events(&self, event: crate::tui::Event) -> color_eyre::eyre::Result<()> {
        match event {
            Event::Key(key) => match key.code {
                KeyCode::Esc => {
                    self.tx.send(LayerManageAction::PopPage);
                }
                KeyCode::Char('h') | KeyCode::Left => {
                    self.tx.send(AnalysisAction::MoveTypeFocus(false));
                }
                KeyCode::Char('l') | KeyCode::Right => {
                    self.tx.send(AnalysisAction::MoveTypeFocus(true));
                }
                KeyCode::Char('j') | KeyCode::Down => self.tx.send(AnalysisAction::Scroll(true)),
                KeyCode::Char('k') | KeyCode::Up => self.tx.send(AnalysisAction::Scroll(false)),
                _ => {}
            },
            _ => {}
        };
        Ok(())
    }

    fn update(&mut self, action: crate::actions::Action) {
        match action {
            Action::Analysis(action) => match action {
                AnalysisAction::MoveTypeFocus(dir) => {
                    if dir {
                        self.analysis_type = self.analysis_type.next(&self.data)
                    } else {
                        self.analysis_type = self.analysis_type.previous(&self.data)
                    }
                }
                AnalysisAction::Scroll(dir) => {
                    if let AnalysisType::Merchant(ref mut data) = self.analysis_type {
                        if dir {
                            data.scroll_state.scroll_down();
                        } else {
                            data.scroll_state.scroll_up();
                        }
                    }
                }
            },
            _ => (),
        }
    }
}

impl Layer for Analysis {}

impl WidgetExt for Analysis {
    fn render(&mut self, frame: &mut ratatui::Frame, area: ratatui::prelude::Rect) {
        let [header_area, main_area, help_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(3),
        ])
        .areas(area);

        let tabs = Tabs::new(AnalysisType::iter().map(|e| {
            format!(" {} ", e.to_string())
                .fg(tailwind::GRAY.c500)
                .bg(e.get_palette().c950)
        }))
        .select(self.analysis_type.to_index())
        .highlight_style((tailwind::GRAY.c200, self.analysis_type.get_palette().c600))
        .divider(" ")
        .padding("", "");

        frame.render_widget(tabs, header_area);

        let palette = self.analysis_type.get_palette();

        match &mut self.analysis_type {
            AnalysisType::TimePeriod(data) => data.render(main_area, frame, palette),
            AnalysisType::Merchant(data) => data.render(main_area, frame, palette),
        };

        self.get_help_message().render(frame, help_area);
    }
}

impl Analysis {
    fn get_help_message(&self) -> HelpMsg {
        let mut help = HelpMsg::default();
        help.push(HelpEntry::new('h', "Last tab"));
        help.push(HelpEntry::new('l', "Next Tab"));
        help.push(HelpEntry::new(KeyCode::Esc, "Go back"));
        help
    }
}
