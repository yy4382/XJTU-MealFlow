use std::collections::HashMap;

use chrono::NaiveTime;
use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Layout},
    style::{Style, Stylize, palette::tailwind},
    symbols,
    text::Line,
    widgets::{Bar, BarChart, BarGroup, Block, Padding, Tabs},
};
use strum::{Display, EnumIter, IntoEnumIterator};
use tracing::info;

use crate::{
    actions::{Action, ActionSender, LayerManageAction},
    libs::transactions::{Transaction, TransactionManager},
    tui::Event,
    utils::help_msg::{HelpEntry, HelpMsg},
};

use super::{EventLoopParticipant, Layer, WidgetExt};

pub(crate) struct Analysis {
    manager: crate::libs::transactions::TransactionManager,
    tx: ActionSender,

    analysis_type: AnalysisType,
    data: Vec<Transaction>,
}
/// false for left, true for right
type MoveDirection = bool;
#[derive(Clone, Debug)]
pub(crate) enum AnalysisAction {
    MoveTypeFocus(MoveDirection),
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

#[derive(Debug, Default, Clone)]
struct TimePeriodData {
    /// 5am - 10:30am
    breakfast: u32,
    /// 10:30am - 1:30pm
    lunch: u32,
    /// 4:30pm - 7:30pm
    dinner: u32,
    /// other
    unknown: u32,
}

impl TimePeriodData {
    fn new(data: &Vec<Transaction>) -> Self {
        data.iter().fold(Self::default(), |acc, entry| {
            let time = entry.time.time();
            if Self::check_time_in(
                time,
                NaiveTime::from_hms_opt(5, 0, 0).unwrap(),
                NaiveTime::from_hms_opt(10, 30, 0).unwrap(),
            ) {
                return Self {
                    breakfast: acc.breakfast + 1,
                    ..acc
                };
            }
            if Self::check_time_in(
                time,
                NaiveTime::from_hms_opt(10, 30, 0).unwrap(),
                NaiveTime::from_hms_opt(13, 30, 0).unwrap(),
            ) {
                return Self {
                    lunch: acc.lunch + 1,
                    ..acc
                };
            }

            if Self::check_time_in(
                time,
                NaiveTime::from_hms_opt(16, 30, 0).unwrap(),
                NaiveTime::from_hms_opt(19, 30, 0).unwrap(),
            ) {
                return Self {
                    dinner: acc.dinner + 1,
                    ..acc
                };
            }
            Self {
                unknown: acc.unknown + 1,
                ..acc
            }
        })
    }

    fn check_time_in(time: NaiveTime, start: NaiveTime, end: NaiveTime) -> bool {
        if time >= start && time < end {
            return true;
        }
        false
    }
}
impl IntoIterator for TimePeriodData {
    type Item = (&'static str, u32);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        vec![
            ("Breakfast", self.breakfast),
            ("Lunch", self.lunch),
            ("Dinner", self.dinner),
            ("Other", self.unknown),
        ]
        .into_iter()
    }
}

#[derive(Default, Debug, Clone)]
struct MerchantData {
    data: Vec<(String, f64)>,
}
impl MerchantData {
    fn new(data: &Vec<Transaction>) -> Self {
        let mut hash_map: HashMap<&str, f64> = HashMap::new();
        data.iter().for_each(|entry| {
            match hash_map.get(entry.merchant.as_str()) {
                Some(v) => hash_map.insert(&entry.merchant, *v + entry.amount),
                None => hash_map.insert(&entry.merchant, entry.amount),
            };
        });
        let mut entries: Vec<(&&str, &f64)> = hash_map.iter().collect();
        entries.sort_by(|a, b| a.1.total_cmp(&b.1));
        MerchantData {
            data: entries.iter().map(|e| ((*e.0).to_string(), *e.1)).collect(),
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

        let block = Block::bordered()
            .border_set(symbols::border::PROPORTIONAL_TALL)
            .padding(Padding::horizontal(1))
            .border_style(self.analysis_type.get_palette().c600);

        let widget = match &self.analysis_type {
            AnalysisType::TimePeriod(data) => {
                let style = Style::default().fg(tailwind::BLUE.c300);
                let bars: Vec<Bar> = data
                    .clone()
                    .into_iter()
                    .map(|(name, value)| {
                        Bar::default()
                            .value(u64::from(value))
                            .label(Line::from(name))
                            .style(style)
                            .value_style(style.reversed())
                    })
                    .collect();
                let bar_chart = BarChart::default()
                    .block(block)
                    .data(BarGroup::default().bars(&bars))
                    .bar_width(1)
                    .bar_gap(1)
                    .direction(ratatui::layout::Direction::Horizontal);
                bar_chart
            }

            AnalysisType::Merchant(data) => {
                let style = Style::default().fg(tailwind::BLUE.c300);
                let bars: Vec<Bar> = data
                    .clone()
                    .data
                    .into_iter()
                    .map(|(name, value)| {
                        info!("{}{}", value, value as u64);
                        Bar::default()
                            .value(((value.abs() * 100.0).round() as u64) / 100)
                            .text_value(format!("{:.2}", value.abs()))
                            .label(Line::from(name))
                            .style(style)
                            .value_style(style.reversed())
                    })
                    .collect();
                let bar_chart = BarChart::default()
                    .block(block)
                    .data(BarGroup::default().bars(&bars))
                    .bar_width(1)
                    .bar_gap(1)
                    .direction(ratatui::layout::Direction::Horizontal);
                bar_chart
            }
        };
        frame.render_widget(widget, main_area);

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
