use crate::{
    RootState,
    actions::{Action, FetchingState, TransactionAction},
    fetcher,
    tui::Event,
};

use super::Page;
use chrono::DateTime;
use color_eyre::eyre::Result;
use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Flex, Layout},
    style::{Color, Style},
    text::Text,
    widgets::{Block, BorderType, Borders, Paragraph},
};

#[derive(Default)]
pub struct Transactions {
    transactions: Vec<crate::transactions::Transaction>,
    fetching_state: FetchingState,
}

impl Page for Transactions {
    fn render(&self, frame: &mut Frame) {
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

        let bottom_rects = &Layout::horizontal([Constraint::Fill(2), Constraint::Fill(1)])
            .flex(Flex::SpaceBetween)
            .split(rects[1]);

        frame.render_widget(
            Paragraph::new(Text::raw("r: Refresh l: Load")).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            ),
            bottom_rects[0],
        );

        frame.render_widget(
            {
                let fetch_info = match &self.fetching_state {
                    FetchingState::Idle => "Fetch State: Idle".to_string(),
                    FetchingState::Fetching(info) => format!("Fetch State: {}", &info),
                };
                Paragraph::new(Text::raw(fetch_info))
                    .alignment(Alignment::Right)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded),
                    )
            },
            bottom_rects[1],
        );
    }
    fn handle_events(&mut self, event: Option<Event>) -> Result<Action> {
        if let Some(event) = event {
            match event {
                Event::Key(key) => match (key.modifiers, key.code) {
                    (_, KeyCode::Char('r')) => {
                        Ok(Action::Transaction(TransactionAction::FetchTransactions))
                    }
                    (_, KeyCode::Char('l')) => {
                        Ok(Action::Transaction(TransactionAction::LoadTransactions))
                    }
                    _ => Ok(Action::None),
                },
                _ => Ok(Action::None),
            }
        } else {
            Ok(Action::None)
        }
    }

    fn update(&mut self, root_state: &mut RootState, action: Action) {
        if let Action::Transaction(action) = action {
            match action {
                // A Fetch will 1. set fetch state 2. fetch (async) 3.set fetch state to idle
                //  4. Insert to db 5. load db
                TransactionAction::FetchTransactions => {
                    let tx = root_state.action_tx.clone();
                    tokio::spawn(async move {
                        tx.send(Action::Transaction(TransactionAction::UpdateFetchStatus(
                            FetchingState::Fetching("fetching".to_string()),
                        )))
                        .unwrap();
                        let cookie = std::env::var("COOKIE").unwrap();
                        let end_timestamp =
                        // FIXME: This is a hardcoded timestamp
                            DateTime::parse_from_rfc3339("2025-03-15T00:00:00+08:00")
                                .unwrap()
                                .timestamp();
                        let records = fetcher::fetch_transactions(cookie.as_str(), end_timestamp)
                            .await
                            .unwrap();
                        assert!(!records.is_empty());
                        tx.send(Action::Transaction(TransactionAction::UpdateFetchStatus(
                            FetchingState::Idle,
                        )))
                        .unwrap();
                        tx.send(Action::Transaction(TransactionAction::InsertTransaction(
                            records,
                        )))
                        .unwrap();
                    });
                }

                TransactionAction::UpdateFetchStatus(state) => {
                    self.fetching_state = state;
                    root_state.action_tx.send(Action::Render).unwrap();
                }

                TransactionAction::InsertTransaction(transactions) => {
                    root_state.manager.insert(&transactions).unwrap();
                    root_state
                        .action_tx
                        .send(Action::Transaction(TransactionAction::LoadTransactions))
                        .unwrap();
                }

                TransactionAction::LoadTransactions => {
                    self.transactions = root_state.manager.fetch_all().unwrap();
                    root_state.action_tx.send(Action::Render).unwrap();
                }
            }
        }
    }

    fn get_name(&self) -> String {
        "Transactions".to_string()
    }
}
