use crate::{RootState, actions::Action};

use super::Page;
use color_eyre::eyre::Result;
use ratatui::{
    Frame,
    layout::Alignment,
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
};

#[derive(Default, Clone, Debug)]
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

    fn update(&mut self, _root_state: &RootState, _action: Action) {}

    fn get_name(&self) -> String {
        "Home".to_string()
    }

    fn handle_events(&self, _app: &RootState, _event: crate::tui::Event) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;
    use crate::{app::RootState, tui::test_utils::get_char_evt};

    fn get_test_page() -> (Home, RootState) {
        let app = RootState::new(None);

        let mut home = Home::default();
        home.init(&app);
        (home, app)
    }

    #[test]
    fn test_render() {
        let (page, app) = get_test_page();
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        terminal.draw(|frame| page.render(frame, &app)).unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_events() {
        let (mut page, mut app) = get_test_page();
        app.handle_event_and_update(&mut page, get_char_evt('q'));
        page.update(&app, Action::Tick);
        // nothing should happen. No panic means success.
    }

    #[test]
    fn test_name() {
        assert_eq!(Home::default().get_name(), "Home");
    }
}
