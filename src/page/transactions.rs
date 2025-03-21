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
    layout::Alignment,
    style::{Color, Style},
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
        frame.render_widget(
            {
                let fetch_info = match &self.fetching_state {
                    FetchingState::Idle => "Press r to fetch transactions".to_string(),
                    FetchingState::Fetching(info) => format!("Fetching transactions: {}", &info),
                };
                Paragraph::new(format!("{}{}", fetch_info, self.transactions.len()))
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
            area,
        );
    }
    fn handle_events(&mut self, event: Option<Event>) -> Result<Action> {
        if let Some(event) = event {
            match event {
                Event::Key(key) => match (key.modifiers, key.code) {
                    (_, KeyCode::Char('r')) => {
                        Ok(Action::Transaction(TransactionAction::FetchTransactions))
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
