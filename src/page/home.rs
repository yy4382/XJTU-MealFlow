use std::vec;

use crate::{
    RootState,
    actions::Action,
    utils::help_msg::{HelpEntry, HelpMsg},
};

use super::Page;
use color_eyre::eyre::Result;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Flex, Layout},
    style::{Color, Style},
    widgets::Paragraph,
};

#[derive(Default, Clone, Debug)]
pub struct Home {}

impl Page for Home {
    fn render(&self, frame: &mut Frame, _app: &RootState) {
        let area = &Layout::default()
            .constraints([Constraint::Fill(1), Constraint::Length(3)])
            .split(frame.area());

        // TODO use different ascii art for different screen sizes
        let ascii_art = include_str!("../../ascii-arts/mealflow.txt");
        let height = ascii_art.lines().count() as u16;
        let [v_align_area] = &Layout::vertical([Constraint::Length(height + 1)])
            .flex(Flex::Center)
            .areas(area[0]);

        frame.render_widget(
            Paragraph::new(include_str!("../../ascii-arts/mealflow.txt"))
                .style(Style::default().fg(Color::Cyan))
                .alignment(Alignment::Center),
            *v_align_area,
        );

        let help_msg: HelpMsg = vec![
            HelpEntry::new('T', "Go to transactions page"),
            HelpEntry::new('q', "Quit"),
        ]
        .into();

        help_msg.render(frame, area[1]);
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
    use crate::{app::RootState, utils::key_events::test_utils::get_char_evt};

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
