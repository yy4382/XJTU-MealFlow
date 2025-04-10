use std::vec;

use crate::{
    actions::{Action, ActionSender, LayerManageAction, Layers},
    app::layer_manager::EventHandlingStatus,
    tui::Event,
    utils::help_msg::{HelpEntry, HelpMsg},
};

use super::{EventLoopParticipant, Layer, WidgetExt};
use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Flex, Layout},
    style::{Color, Style},
    widgets::Paragraph,
};

#[derive(Clone, Debug)]
pub struct Home {
    pub tx: ActionSender,
}

impl Home {
    fn get_help_msg(&self) -> HelpMsg {
        let help_msg: HelpMsg = vec![
            HelpEntry::new('T', "Go to transactions page"),
            HelpEntry::new('q', "Quit"),
            HelpEntry::new('?', "Show help"),
        ]
        .into();
        help_msg
    }
}

#[cfg(test)]
impl Default for Home {
    fn default() -> Self {
        let (tx, _) = tokio::sync::mpsc::unbounded_channel();
        Self {
            tx: ActionSender(tx),
        }
    }
}

impl WidgetExt for Home {
    fn render(&mut self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let ascii_art = if area.width >= 100 {
            include_str!("../../ascii-arts/xjtu-mealflow.txt")
        } else if area.width >= 60 {
            include_str!("../../ascii-arts/mealflow.txt")
        } else {
            "XJTU MealFlow"
        };

        let area = &Layout::default()
            .constraints([Constraint::Fill(1), Constraint::Length(3)])
            .split(area);

        let height = ascii_art.lines().count() as u16;
        let [v_align_area] = &Layout::vertical([Constraint::Length(height + 1)])
            .flex(Flex::Center)
            .areas(area[0]);

        frame.render_widget(
            Paragraph::new(ascii_art)
                .style(Style::default().fg(Color::Cyan))
                .alignment(Alignment::Center),
            *v_align_area,
        );

        self.get_help_msg().render(frame, area[1]);
    }
}

impl EventLoopParticipant for Home {
    fn handle_events(&mut self, _event: &crate::tui::Event) -> EventHandlingStatus {
        let mut status = EventHandlingStatus::default();
        if let Event::Key(key) = _event {
            match key.code {
                KeyCode::Char('?') => {
                    self.tx.send(LayerManageAction::Push(
                        Layers::Help(self.get_help_msg()).into_push_config(true),
                    ));
                    status.consumed();
                }
                KeyCode::Char('a') => {
                    // TODO add help msg for this
                    self.tx.send(LayerManageAction::Push(
                        Layers::Analysis.into_push_config(false),
                    ));
                    status.consumed();
                }
                KeyCode::Char('T') => {
                    self.tx.send(LayerManageAction::Push(
                        Layers::Transaction(None).into_push_config(false),
                    ));
                    status.consumed();
                }
                KeyCode::Char('q') => {
                    self.tx.send(Action::Quit);
                    status.consumed();
                }
                _ => {}
            }
        }
        status
    }
}

impl Layer for Home {}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;
    use ratatui::{Terminal, backend::TestBackend};
    use tokio::sync::mpsc;

    use crate::actions::Action;

    use super::*;

    fn get_test_page() -> Home {
        let mut home = Home {
            tx: ActionSender(tokio::sync::mpsc::unbounded_channel().0),
        };
        home.init();
        home
    }

    #[test]
    fn test_render() {
        let mut page = get_test_page();
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        terminal
            .draw(|frame| page.render(frame, frame.area()))
            .unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_render_large() {
        let mut page = get_test_page();
        let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
        terminal
            .draw(|frame| page.render(frame, frame.area()))
            .unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_render_small() {
        let mut page = get_test_page();
        let mut terminal = Terminal::new(TestBackend::new(40, 25)).unwrap();
        terminal
            .draw(|frame| page.render(frame, frame.area()))
            .unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_events() {
        let (tx, mut _rx) = mpsc::unbounded_channel::<Action>();
        let mut home = Home { tx: tx.into() };
        home.handle_event_with_status_check(&'?'.into());
        let mut should_receive_layer_opt = false;
        while let Ok(action) = _rx.try_recv() {
            if let Action::Layer(LayerManageAction::Push(act)) = action {
                assert!(matches!(act.layer, Layers::Help(_)));
                should_receive_layer_opt = true;
            }
        }
        assert!(should_receive_layer_opt);
    }
    #[test]
    fn test_event_nav_to_analysis() {
        let (tx, mut rx) = mpsc::unbounded_channel::<Action>();
        let mut home = Home { tx: tx.into() };
        home.handle_event_with_status_check(&'a'.into());
        let mut should_receive_layer_opt = false;
        while let Ok(action) = rx.try_recv() {
            if let Action::Layer(LayerManageAction::Push(act)) = action {
                assert!(matches!(act.layer, Layers::Analysis));
                should_receive_layer_opt = true;
            }
        }
        assert!(should_receive_layer_opt);
    }
    #[test]
    fn test_event_nav_to_transactions() {
        let (tx, mut rx) = mpsc::unbounded_channel::<Action>();
        let mut home = Home { tx: tx.into() };
        home.handle_event_with_status_check(&'T'.into());
        let mut should_receive_layer_opt = false;
        while let Ok(action) = rx.try_recv() {
            if let Action::Layer(LayerManageAction::Push(act)) = action {
                assert!(matches!(act.layer, Layers::Transaction(_)));
                should_receive_layer_opt = true;
            }
        }
        assert!(should_receive_layer_opt);
    }
}
