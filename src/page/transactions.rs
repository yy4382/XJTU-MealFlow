use crate::{RootState, actions::Action, tui::Event};

use super::Page;
use color_eyre::eyre::Result;
use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout},
    style::{Color, Style},
    text::Text,
    widgets::{Block, BorderType, Borders, Paragraph},
};

#[derive(Default, Clone)]
pub struct Transactions {
    transactions: Vec<crate::libs::transactions::Transaction>,
}

#[derive(Clone)]
pub enum TransactionAction {
    LoadTransactions,
}

impl Page for Transactions {
    fn render(&self, frame: &mut Frame, _app: &RootState) {
        let area = frame.area();

        let vertical = &Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]);
        let rects = vertical.split(area);

        frame.render_widget(
            {
                let transactions_str = self
                    .transactions
                    .iter()
                    .fold(String::new(), |acc, x| acc + &format!("{:?}\n", x));
                Paragraph::new(transactions_str)
                    .block(
                        Block::default()
                            .title("Transactions")
                            .title_alignment(Alignment::Center)
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded),
                    )
                    .style(Style::default().fg(Color::Cyan))
                    .alignment(Alignment::Center)
            },
            rects[0],
        );

        frame.render_widget(
            Paragraph::new(Text::raw("r: Refresh l: Load")).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            ),
            rects[1],
        );
    }
    fn handle_events(&self, app: &RootState, event: Event) -> Result<()> {
        match event {
            Event::Key(key) => match (key.modifiers, key.code) {
                // navigate to fetch page
                (_, KeyCode::Char('r')) => app.send_action(Action::NavigateTo(
                    crate::actions::NaviTarget::Fetch(crate::page::fetch::Fetch::default()),
                )),
                (_, KeyCode::Char('l')) => {
                    app.send_action(Action::Transaction(TransactionAction::LoadTransactions))
                }
                _ => (),
            },
            _ => (),
        };
        Ok(())
    }

    fn update(&mut self, root_state: &RootState, action: Action) {
        if let Action::Transaction(action) = action {
            match action {
                TransactionAction::LoadTransactions => {
                    self.transactions = root_state.manager.fetch_all().unwrap();
                    root_state.send_action(Action::Render);
                }
            }
        }
    }

    fn get_name(&self) -> String {
        "Transactions".to_string()
    }

    fn init(&mut self, app: &mut RootState) {
        app.send_action(Action::Transaction(TransactionAction::LoadTransactions))
    }
}
