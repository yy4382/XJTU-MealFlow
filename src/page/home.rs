use crate::{RootState, actions::Action};

use super::Page;
use color_eyre::eyre::Result;
use ratatui::{
    Frame,
    layout::Alignment,
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
};

#[derive(Default, Clone)]
pub struct Home {}

impl Page for Home {
    fn render(&self, frame: &mut Frame, _app: &RootState) {
        let area = frame.area();
        frame.render_widget(
            Paragraph::new(
                "Welcome to XJTU MealFlow\n\nPress 'T' (Capitalized) to view transactions\nPress 'q' to quit"
            )
            .block(
                Block::default()
                    .title("XJTU MealFlow")
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
