use std::collections::HashMap;

use ratatui::{
    Frame,
    layout::{Rect, Size},
    style::{Style, Stylize as _, palette::tailwind},
    symbols,
    text::Line,
    widgets::{Bar, BarChart, BarGroup, Block, Clear, Padding, Paragraph},
};
use tui_scrollview::{ScrollView, ScrollViewState, ScrollbarVisibility};

use crate::libs::transactions::Transaction;

#[derive(Debug, Default, Clone)]
pub(super) struct MerchantData {
    data: Vec<(String, f64)>,
    pub scroll_state: ScrollViewState,
}
impl MerchantData {
    pub fn new(data: &[Transaction]) -> Self {
        let hash_map = data.iter().fold(HashMap::new(), |mut acc, entry| {
            match acc.get(&entry.merchant) {
                Some(v) => acc.insert(&entry.merchant, *v + entry.amount),
                None => acc.insert(&entry.merchant, entry.amount),
            };
            acc
        });
        let mut entries: Vec<_> = hash_map.iter().collect();
        entries.sort_by(|a, b| a.1.total_cmp(b.1));
        MerchantData {
            data: entries.iter().map(|e| ((*e.0).to_string(), *e.1)).collect(),
            scroll_state: ScrollViewState::default(),
        }
    }
}

impl MerchantData {
    pub(super) fn render(
        &mut self,
        main_area: ratatui::prelude::Rect,
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
                main_area,
            );
            return;
        }

        let style = Style::default().fg(tailwind::BLUE.c300);
        let bars: Vec<Bar> = self
            .data
            .clone()
            .into_iter()
            .map(|(name, value)| {
                Bar::default()
                    .value(((value.abs() * 100.0).round() as u64) / 100)
                    .text_value(format!("{:.2}", value.abs()))
                    .label(Line::from(name))
                    .style(style)
                    .value_style(style.reversed())
            })
            .collect();
        let bar_chart = BarChart::default()
            .block(Block::default().padding(Padding::horizontal(1)))
            .data(BarGroup::default().bars(&bars))
            .bar_width(1)
            .bar_gap(1)
            .direction(ratatui::layout::Direction::Horizontal);

        // inset 1
        let chart_area = Rect {
            x: main_area.x + 1,
            y: main_area.y + 1,
            width: main_area.width - 2,
            height: main_area.height - 2,
        };
        frame.render_widget(block.clone(), main_area);
        frame.render_widget(Clear, chart_area);

        let chart_height = ((self.data.len()) * 2 - 1) as u16;

        let mut scroll_view = ScrollView::new(Size::new(chart_area.width, chart_height))
            .horizontal_scrollbar_visibility(ScrollbarVisibility::Never);

        scroll_view.render_widget(
            bar_chart,
            Rect::new(0, 0, chart_area.width - 1, chart_height),
        );

        frame.render_stateful_widget(scroll_view, chart_area, &mut self.scroll_state);
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
        let mut data = MerchantData::default();
        terminal
            .draw(|f| data.render(f.area(), f, tailwind::BLUE))
            .unwrap();
        assert_snapshot!(terminal.backend())
    }
}