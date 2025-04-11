use std::collections::HashMap;

use chrono::Datelike;
use ratatui::{
    Frame,
    style::{Style, Stylize as _, palette::tailwind},
    symbols,
    text::Line,
    widgets::{Bar, BarChart, BarGroup, Block, Padding, Paragraph},
};
use tracing::info;

use crate::libs::transactions::Transaction;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct YearMonth {
    year: u16,
    month: u16,
}
impl YearMonth {
    fn new(year: u16, month: u16) -> Self {
        Self { year, month }
    }
    // fn relative_to(&self, other: &Self) -> f64 {
    //     let year_diff = self.year as f64 - other.year as f64;
    //     let month_diff = self.month as f64 - other.month as f64;
    //     year_diff * 12.0 + month_diff
    // }
    // fn relative_to_default(&self) -> f64 {
    //     let default = YearMonth::new(2015, 1);
    //     self.relative_to(&default)
    // }
    fn next(&self) -> Self {
        if self.month == 12 {
            YearMonth::new(self.year + 1, 1)
        } else {
            YearMonth::new(self.year, self.month + 1)
        }
    }
}
impl std::fmt::Display for YearMonth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02}-{:02}", self.year, self.month)
    }
}

#[derive(Debug, Default, Clone)]
pub(super) struct TimeSeriesData {
    data: Vec<(YearMonth, f64)>,
}

impl TimeSeriesData {
    pub(super) fn new(data: &[Transaction]) -> Self {
        let mut processed_data: Vec<(YearMonth, f64)> = data
            .iter()
            .fold(HashMap::new(), |mut acc, entry| {
                let year = entry.time.year() as u16;
                let month = entry.time.month() as u16;
                match acc.get(&(year, month)) {
                    Some(v) => acc.insert((year, month), *v + entry.amount.abs()),
                    None => acc.insert((year, month), entry.amount.abs()),
                };
                acc
            })
            .iter()
            .map(|((year, month), amount)| (YearMonth::new(*year, *month), *amount))
            .collect();

        processed_data.sort_by(|a, b| a.0.cmp(&b.0));

        let processed_data = processed_data
            .into_iter()
            .fold(Vec::new(), |mut acc, entry| {
                let mut last_ym = acc.last().unwrap_or(&entry).0;
                while last_ym.next() < entry.0 {
                    info!("Missing data for {:?}, {:?}", last_ym, entry.0);
                    acc.push((last_ym, 0.0));
                    last_ym = last_ym.next();
                }
                acc.push(entry);
                acc
            });

        info!("{:?}", processed_data);

        Self {
            data: processed_data,
        }
    }
    pub(super) fn render(
        &self,
        area: ratatui::prelude::Rect,
        frame: &mut Frame,
        color: tailwind::Palette,
    ) {
        let block = Block::bordered()
            .border_set(symbols::border::PROPORTIONAL_TALL)
            .border_style(color.c600)
            .padding(Padding::horizontal(1));

        if self.data.len() == 0 {
            frame.render_widget(
                Paragraph::new("No data available yet").block(block.clone()),
                area,
            );
            return;
        }

        let bars = (area.width as u64 - 6) / 8;

        let style = Style::default().fg(color.c300);
        let bars: Vec<Bar> = self.data[self.data.len().saturating_sub(bars as usize)..]
            .iter()
            .map(|(ym, value)| {
                Bar::default()
                    .value(value.round() as u64)
                    .label(Line::from(ym.to_string()))
                    .style(style)
                    .value_style(style.reversed())
            })
            .collect();

        let bar_chart = BarChart::default()
            .block(block)
            .data(BarGroup::default().bars(&bars))
            .bar_width(7)
            .bar_gap(1)
            .bar_style(style);

        frame.render_widget(bar_chart, area);
    }
}

#[cfg(test)]
mod test {
    use insta::assert_snapshot;
    use ratatui::backend::TestBackend;

    use super::*;

    #[test]
    fn test_empty_render() {
        let mut terminal = ratatui::Terminal::new(TestBackend::new(80, 20)).unwrap();
        let data = TimeSeriesData::default();
        terminal
            .draw(|f| data.render(f.area(), f, tailwind::BLUE))
            .unwrap();
        assert_snapshot!(terminal.backend())
    }
}
