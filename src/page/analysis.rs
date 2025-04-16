use crossterm::event::KeyCode;
use merchant::MerchantData;
use merchant_type::MerchantCategoryData;
use ratatui::{
    layout::{Constraint, Layout},
    style::{Stylize, palette::tailwind},
    widgets::Tabs,
};
use strum::{Display, EnumIter, IntoEnumIterator};
use time_period::TimePeriodData;
use time_series::TimeSeriesData;

use crate::{
    actions::{ActionSender, LayerManageAction},
    app::layer_manager::EventHandlingStatus,
    libs::transactions::{Transaction, TransactionManager},
    tui::Event,
    utils::help_msg::{HelpEntry, HelpMsg},
};

use super::{EventLoopParticipant, Layer, WidgetExt};

mod merchant;
mod merchant_type;
mod time_period;
mod time_series;

pub(crate) struct Analysis {
    manager: crate::libs::transactions::TransactionManager,
    tx: ActionSender,

    analysis_type: AnalysisType,
    data: Vec<Transaction>,
}

#[derive(Display, EnumIter)]
enum AnalysisType {
    #[strum(to_string = "Time Period")]
    TimePeriod(TimePeriodData),
    #[strum(to_string = "Time Series")]
    TimeSeries(TimeSeriesData),
    #[strum(to_string = "Merchant")]
    Merchant(MerchantData),
    #[strum(to_string = "MerchantCategory")]
    MerchantCategory(MerchantCategoryData),
}

impl AnalysisType {
    fn next(&self, data: &[Transaction]) -> Self {
        match self {
            Self::TimePeriod(_) => Self::TimeSeries(TimeSeriesData::new(data)),
            Self::TimeSeries(_) => Self::Merchant(MerchantData::new(data)),
            Self::Merchant(_) => Self::MerchantCategory(MerchantCategoryData::new(data)),
            Self::MerchantCategory(_) => Self::TimePeriod(TimePeriodData::new(data)),
        }
    }
    fn previous(&self, data: &[Transaction]) -> Self {
        match self {
            Self::TimePeriod(_) => Self::MerchantCategory(MerchantCategoryData::new(data)),
            Self::MerchantCategory(_) => Self::Merchant(MerchantData::new(data)),
            Self::TimeSeries(_) => Self::TimePeriod(TimePeriodData::new(data)),
            Self::Merchant(_) => Self::TimeSeries(TimeSeriesData::new(data)),
        }
    }
    fn to_index(&self) -> usize {
        match self {
            AnalysisType::TimePeriod(_) => 0,
            AnalysisType::TimeSeries(_) => 1,
            AnalysisType::Merchant(_) => 2,
            AnalysisType::MerchantCategory(_) => 3,
        }
    }
    fn get_palette(&self) -> tailwind::Palette {
        match self {
            AnalysisType::TimePeriod(_) => tailwind::BLUE,
            AnalysisType::TimeSeries(_) => tailwind::GREEN,
            AnalysisType::Merchant(_) => tailwind::INDIGO,
            AnalysisType::MerchantCategory(_) => tailwind::YELLOW,
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
    fn handle_events(&mut self, event: &crate::tui::Event) -> EventHandlingStatus {
        let mut status = EventHandlingStatus::default();
        #[allow(clippy::single_match)]
        match event {
            Event::Key(key) => match key.code {
                KeyCode::Esc => {
                    self.tx.send(LayerManageAction::Pop);
                    status.consumed();
                }
                KeyCode::Char('h') | KeyCode::Left => {
                    self.analysis_type = self.analysis_type.previous(&self.data);
                    status.consumed();
                }
                KeyCode::Char('l') | KeyCode::Right => {
                    self.analysis_type = self.analysis_type.next(&self.data);
                    status.consumed();
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    if let AnalysisType::Merchant(ref mut data) = self.analysis_type {
                        data.scroll_state.scroll_down();
                        status.consumed();
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    if let AnalysisType::Merchant(ref mut data) = self.analysis_type {
                        data.scroll_state.scroll_up();
                        status.consumed();
                    }
                }
                _ => {}
            },
            _ => {}
        };
        status
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
            AnalysisType::TimeSeries(data) => data.render(main_area, frame, palette),
            AnalysisType::Merchant(data) => data.render(main_area, frame, palette),
            AnalysisType::MerchantCategory(data) => data.render(main_area, frame, palette),
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
        let (_, mut page) = get_test_objs();
        // Initial state should be TimePeriod
        assert!(matches!(page.analysis_type, AnalysisType::TimePeriod(_)));

        // Test moving to next tab
        page.handle_event_with_status_check(&'l'.into());
        assert!(matches!(page.analysis_type, AnalysisType::TimeSeries(_)));

        // Test moving back to TimePeriod
        page.handle_event_with_status_check(&'h'.into());
        assert!(matches!(page.analysis_type, AnalysisType::TimePeriod(_)));

        // Test wrapping around
        page.handle_event_with_status_check(&'h'.into());
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
        let (_, mut page) = get_test_objs();

        // First switch to Merchant tab
        page.handle_event_with_status_check(&'h'.into());

        let initial_offset = get_merchant_data(&page.analysis_type)
            .scroll_state
            .offset()
            .y;

        // Test scrolling down
        page.handle_event_with_status_check(&'j'.into());
        assert_eq!(
            get_merchant_data(&page.analysis_type)
                .scroll_state
                .offset()
                .y,
            initial_offset + 1
        );

        // Test scrolling up
        page.handle_event_with_status_check(&'k'.into());
        assert_eq!(
            get_merchant_data(&page.analysis_type)
                .scroll_state
                .offset()
                .y,
            initial_offset
        );
    }

    #[test]
    fn test_render_time_period() {
        let (_, mut page) = get_test_objs();
        let mut terminal = ratatui::Terminal::new(TestBackend::new(80, 20)).unwrap();

        terminal
            .draw(|f| {
                page.render(f, f.area());
            })
            .unwrap();

        assert_snapshot!(terminal.backend());
    }
    #[test]
    fn test_render_merchant() {
        let (_, mut page) = get_test_objs();
        let mut terminal = ratatui::Terminal::new(TestBackend::new(80, 20)).unwrap();
        // Switch to Merchant tab and render again
        page.handle_event_with_status_check(&'h'.into());

        terminal
            .draw(|f| {
                page.render(f, f.area());
            })
            .unwrap();

        assert_snapshot!(terminal.backend());

        let seq = "j".repeat(10);
        seq.chars().for_each(|c| {
            page.handle_event_with_status_check(&c.into());
        });
        terminal
            .draw(|f| {
                page.render(f, f.area());
            })
            .unwrap();
        assert_snapshot!(terminal.backend());
    }
    #[test]
    fn test_render_time_series() {
        let (_, mut page) = get_test_objs();
        let mut terminal = ratatui::Terminal::new(TestBackend::new(80, 20)).unwrap();

        // Switch to TimeSeries tab and render again
        page.handle_event_with_status_check(&'l'.into());

        terminal
            .draw(|f| {
                page.render(f, f.area());
            })
            .unwrap();

        assert_snapshot!(terminal.backend());
    }
}
