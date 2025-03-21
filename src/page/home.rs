use crate::{RootState, actions::Action};

use super::Page;
use color_eyre::eyre::Result;
use ratatui::{
    Frame,
    layout::Alignment,
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
};

#[derive(Default)]
pub struct Home {}

impl Page for Home {
    fn render(&self, frame: &mut Frame) {
        let area = frame.area();
        frame.render_widget(
            Paragraph::new(format!(
                "Press j or k to increment or decrement.\n\nCounter: {}",
                1
            ))
            .block(
                Block::default()
                    .title("ratatui async counter app")
                    .title_alignment(Alignment::Center)
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Center),
            area,
        );
    }

    fn handle_events(&mut self, _event: Option<crate::tui::Event>) -> Result<Action> {
        Ok(Action::None)
    }

    fn update(&mut self, _root_state: &mut RootState, _action: Action) {}

    fn get_name(&self) -> String {
        "Home".to_string()
    }
}
