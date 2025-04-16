use ratatui::{
    Frame,
    style::{Style, Stylize as _, palette::tailwind},
    symbols,
    text::Line,
    widgets::{Bar, BarChart, BarGroup, Block, Padding, Paragraph},
};

use crate::libs::transactions::Transaction;
use crate::utils::merchant_class::MerchantType; // 引入商家分类

#[derive(Debug, Default, Clone)]
pub(super) struct MerchantCategoryData {
    canteen_food: u32,
    canteen_drink: u32,
    supermarket: u32,
    bathhouse: u32,
    other: u32,
    unknown: u32,
}

impl MerchantCategoryData {
    pub(super) fn new(data: &[Transaction]) -> Self {
        data.iter().fold(Self::default(), |mut acc, entry| {
            let merchant_type = MerchantType::from_str(&entry.merchant);
            match merchant_type {
                MerchantType::CanteenFood => acc.canteen_food += 1,
                MerchantType::CanteenDrink => acc.canteen_drink += 1,
                MerchantType::Supermarket => acc.supermarket += 1,
                MerchantType::Bathhouse => acc.bathhouse += 1,
                MerchantType::Other => acc.other += 1,
                MerchantType::Unknown => acc.unknown += 1,
            }
            acc
        })
    }

    fn all_zero(&self) -> bool {
        self.canteen_food == 0
            && self.canteen_drink == 0
            && self.supermarket == 0
            && self.bathhouse == 0
            && self.other == 0
            && self.unknown == 0
    }
}

impl IntoIterator for &MerchantCategoryData {
    type Item = (&'static str, u32);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        vec![
            ("食堂食物", self.canteen_food),
            ("食堂饮品", self.canteen_drink),
            ("超市", self.supermarket),
            ("浴室", self.bathhouse),
            ("其他", self.other),
            ("未知", self.unknown),
        ]
        .into_iter()
    }
}

impl MerchantCategoryData {
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

        if self.all_zero() {
            frame.render_widget(Paragraph::new("暂无数据").block(block.clone()), area);
            return;
        }

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
            .block(block)
            .data(BarGroup::default().bars(&bars))
            .bar_width(1)
            .bar_gap(1)
            .direction(ratatui::layout::Direction::Horizontal);
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
        let data = MerchantCategoryData::default();
        terminal
            .draw(|f| data.render(f.area(), f, tailwind::BLUE))
            .unwrap();
        assert_snapshot!(terminal.backend())
    }
}
