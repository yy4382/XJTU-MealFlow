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
    fn next(&self, data: &[Transaction]) -> Self {
        match self {
            Self::TimePeriod(_) => Self::Merchant(MerchantData::new(data)),
            Self::Merchant(_) => Self::TimePeriod(TimePeriodData::new(data)),
        }
    }
    fn previous(&self, data: &[Transaction]) -> Self {
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
        #[allow(clippy::single_match)]
        match event {
            Event::Key(key) => match key.code {
                KeyCode::Esc => {
                    self.tx.send(LayerManageAction::Pop);
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
        if let Action::Analysis(action) = action {
            match action {
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
            }
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
            format!(" {} ", e)
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
#[cfg(test)]
mod test {
    use super::*;
    use crate::{actions::Action, libs::fetcher};
    use insta::assert_snapshot;
    use ratatui::backend::TestBackend;
    use tokio::sync::mpsc::{self, UnboundedReceiver};

    fn get_test_objs() -> (UnboundedReceiver<Action>, Analysis) {
        let (tx, _rx) = mpsc::unbounded_channel();
        let manager = TransactionManager::new(None).unwrap();
        let data = fetcher::test_utils::get_mock_data(50);
        manager.insert(&data).unwrap();
        let page = Analysis::new(tx.clone().into(), manager);
        (_rx, page)
    }

    #[test]
    fn test_initial_state() {
        let (_, page) = get_test_objs();
        assert!(matches!(page.analysis_type, AnalysisType::TimePeriod(_)));
        assert!(!page.data.is_empty());
    }

    #[test]
    fn test_tab_navigation() {
        let (mut rx, mut page) = get_test_objs();
        // TODO
        // Initial state should be TimePeriod
        assert!(matches!(page.analysis_type, AnalysisType::TimePeriod(_)));

        // Test moving to next tab (Merchant)
        page.event_loop_once(&mut rx, 'l'.into());
        assert!(matches!(page.analysis_type, AnalysisType::Merchant(_)));

        // Test moving back to TimePeriod
        page.event_loop_once(&mut rx, 'h'.into());
        assert!(matches!(page.analysis_type, AnalysisType::TimePeriod(_)));

        // Test wrapping around
        page.event_loop_once(&mut rx, 'h'.into());
        assert!(matches!(page.analysis_type, AnalysisType::Merchant(_)));
    }

    fn get_merchant_data(analysis_type: &AnalysisType) -> MerchantData {
        if let AnalysisType::Merchant(data) = analysis_type {
            return data.clone();
        } else {
            panic!("Should be merchant")
        }
    }

    #[test]
    fn test_scroll_merchant_data() {
        let (mut rx, mut page) = get_test_objs();

        // First switch to Merchant tab
        page.event_loop_once(&mut rx, 'l'.into());

        let initial_offset = get_merchant_data(&page.analysis_type)
            .scroll_state
            .offset()
            .y;

        // Test scrolling down
        page.event_loop_once(&mut rx, 'j'.into());
        assert_eq!(
            get_merchant_data(&page.analysis_type)
                .scroll_state
                .offset()
                .y,
            initial_offset + 1
        );

        // Test scrolling up
        page.event_loop_once(&mut rx, 'k'.into());
        assert_eq!(
            get_merchant_data(&page.analysis_type)
                .scroll_state
                .offset()
                .y,
            initial_offset
        );
    }

    #[test]
    fn test_render() {
        let (mut rx, mut page) = get_test_objs();
        let mut terminal = ratatui::Terminal::new(TestBackend::new(80, 20)).unwrap();

        terminal
            .draw(|f| {
                page.render(f, f.area());
            })
            .unwrap();

        assert_snapshot!(terminal.backend());

        // Switch to Merchant tab and render again
        page.event_loop_once(&mut rx, 'l'.into());

        terminal
            .draw(|f| {
                page.render(f, f.area());
            })
            .unwrap();

        assert_snapshot!(terminal.backend());

        let seq = "j".repeat(10);
        seq.chars().for_each(|c| {
            page.event_loop_once(&mut rx, c.into());
        });
        terminal
            .draw(|f| {
                page.render(f, f.area());
            })
            .unwrap();
        assert_snapshot!(terminal.backend());
    }
}
