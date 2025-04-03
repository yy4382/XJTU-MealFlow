use chrono::NaiveTime;
use ratatui::{
    Frame,
    style::{Style, Stylize as _, palette::tailwind},
    symbols,
    text::Line,
    widgets::{Bar, BarChart, BarGroup, Block, Padding},
};

use crate::libs::transactions::Transaction;

#[derive(Debug, Default, Clone)]
pub(super) struct TimePeriodData {
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
    pub(super) fn new(data: &[Transaction]) -> Self {
        data.iter().fold(Self::default(), |mut acc, entry| {
            let time = entry.time.time();
            if Self::check_time_in(time, (5, 0), (10, 30)) {
                acc.breakfast += 1;
            } else if Self::check_time_in(time, (10, 30), (13, 30)) {
                acc.lunch += 1;
            } else if Self::check_time_in(time, (16, 30), (19, 30)) {
                acc.dinner += 1;
            } else {
                acc.unknown += 1;
            }
            acc
        })
    }
    fn check_time_in(time: NaiveTime, start: (u32, u32), end: (u32, u32)) -> bool {
        if time >= NaiveTime::from_hms_opt(start.0, start.1, 0).unwrap()
            && time < NaiveTime::from_hms_opt(end.0, end.1, 0).unwrap()
        {
            return true;
        }
        false
    }
}
impl IntoIterator for &TimePeriodData {
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
impl TimePeriodData {
    pub(super) fn render(
        &self,
        area: ratatui::prelude::Rect,
        frame: &mut Frame,
        color: tailwind::Palette,
    ) {
        let style = Style::default().fg(color.c300);
        let bars: Vec<Bar> = self
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
            .block(
                Block::bordered()
                    .border_set(symbols::border::PROPORTIONAL_TALL)
                    .padding(Padding::horizontal(1))
                    .border_style(color.c600),
            )
            .data(BarGroup::default().bars(&bars))
            .bar_width(1)
            .bar_gap(1)
            .direction(ratatui::layout::Direction::Horizontal);
        frame.render_widget(bar_chart, area);
    }
}
