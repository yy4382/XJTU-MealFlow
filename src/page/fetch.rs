use chrono::{DateTime, Local};
use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Flex, Layout},
    style::{Color, Style},
    text::Text,
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::{actions::Action, tui::Event};

use super::Page;

#[derive(Clone, Default, Debug)]
pub enum FetchingState {
    #[default]
    Idle,
    Fetching(FetchProgress),
    // Completed(Vec<crate::transactions::Transaction>), // 新增状态，表示获取完成
}

#[derive(Clone, Default, Debug)]
pub struct FetchProgress {
    pub current_page: u32,
    pub total_entries_fetched: u32,
    pub oldest_date: Option<DateTime<Local>>,
}

#[derive(Clone)]
pub enum FetchingAction {
    StartFetching(DateTime<Local>),
    UpdateFetchStatus(FetchingState),
    InsertTransaction(Vec<crate::transactions::Transaction>),

    LoadDbCount,
    MoveFocus(Focus),
}

#[derive(Default, Debug, Clone)]
pub struct Fetch {
    fetching_state: FetchingState,
    local_db_cnt: u64,
    fetch_start_date: Option<DateTime<Local>>,
    current_focus: Focus,
}

#[derive(Clone, Default, Debug)]
pub enum Focus {
    #[default]
    P1Year,
    P3Months,
    P1Month,
    UserInput,
}

impl Focus {
    fn next(&self) -> Self {
        match self {
            Focus::P1Year => Focus::P3Months,
            Focus::P3Months => Focus::P1Month,
            Focus::P1Month => Focus::UserInput,
            Focus::UserInput => Focus::P1Year,
        }
    }

    fn prev(&self) -> Self {
        match self {
            Focus::P1Year => Focus::UserInput,
            Focus::P3Months => Focus::P1Year,
            Focus::P1Month => Focus::P3Months,
            Focus::UserInput => Focus::P1Month,
        }
    }
}

impl Page for Fetch {
    fn render(&self, frame: &mut ratatui::Frame) {
        let area = frame.area();
        let area = &Layout::default()
            .constraints([
                Constraint::Length(3),
                Constraint::Length(4),
                Constraint::Fill(1),
            ])
            .margin(1)
            .split(area);

        let top_areas = &Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
        ])
        .horizontal_margin(1)
        .flex(Flex::SpaceAround)
        .split(area[0]);

        let render_button = |focused: bool, text: String| {
            Paragraph::new(text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded),
                )
                .centered()
                .style(Style::default().fg(if focused { Color::Cyan } else { Color::Reset }))
        };

        frame.render_widget(
            render_button(
                matches!(self.current_focus, Focus::P1Year),
                "Past 1 year".to_string(),
            ),
            top_areas[0],
        );

        frame.render_widget(
            render_button(
                matches!(self.current_focus, Focus::P3Months),
                "Past 3 months".to_string(),
            ),
            top_areas[1],
        );

        frame.render_widget(
            render_button(
                matches!(self.current_focus, Focus::P1Month),
                "Past 1 month".to_string(),
            ),
            top_areas[2],
        );

        frame.render_widget(
            Text::raw("UserInputPlaceholder").style(Style::default().fg(
                if let Focus::UserInput = self.current_focus {
                    Color::Cyan
                } else {
                    Color::Reset
                },
            )),
            area[1],
        );

        // 修改这里：显示获取结果
        match &self.fetching_state {
            FetchingState::Idle => {
                frame.render_widget(
                    Text::raw(format!(
                        "Currently {} records locally stored. Fetch more?",
                        self.local_db_cnt
                    ))
                    .style(Style::default().fg(Color::Gray))
                    .centered(),
                    area[2],
                );
            }
            FetchingState::Fetching(progress) => {
                let progress_text = format!(
                    "Fetching...\nCurrent Page: {}\nTotal Entries Fetched: {}\nOldest Date: {}",
                    progress.current_page,
                    progress.total_entries_fetched,
                    progress
                        .oldest_date
                        .map_or("N/A".to_string(), |date| date.to_string())
                );
                frame.render_widget(
                    Text::raw(progress_text)
                        .centered()
                        .style(Style::default().fg(Color::Gray)),
                    area[2],
                );
            } // FetchingState::Completed(transactions) => {
              //     // 显示获取到的交易数据
              //     let transaction_text = transactions
              //         .iter()
              //         .map(|t| format!("{:?}", t)) // 假设 Transaction 实现了 Debug
              //         .collect::<Vec<String>>()
              //         .join("\n");
              //     frame.render_widget(Text::raw(transaction_text), area[2]);
              // }
        }
    }

    fn handle_events(
        &mut self,
        event: Option<crate::tui::Event>,
    ) -> color_eyre::eyre::Result<crate::actions::Action> {
        if let Some(event) = event {
            match event {
                Event::Key(key) => match (key.modifiers, key.code) {
                    (_, KeyCode::Enter) => match self.fetch_start_date {
                        Some(date) => Ok(Action::Fetching(FetchingAction::StartFetching(date))),
                        None => Ok(Action::None),
                    },
                    (_, KeyCode::Char('j')) => Ok(Action::Fetching(FetchingAction::MoveFocus(
                        self.current_focus.next(),
                    ))),
                    (_, KeyCode::Char('k')) => Ok(Action::Fetching(FetchingAction::MoveFocus(
                        self.current_focus.prev(),
                    ))),
                    (_, KeyCode::Char('l')) => Ok(Action::Fetching(FetchingAction::LoadDbCount)),
                    _ => Ok(Action::None),
                },
                _ => Ok(Action::None),
            }
        } else {
            Ok(Action::None)
        }
    }

    fn update(&mut self, app: &mut crate::RootState, action: crate::actions::Action) {
        if let Action::Fetching(action) = action {
            match action {
                FetchingAction::StartFetching(date) => {
                    let tx = app.action_tx.clone();
                    let tx2 = app.action_tx.clone();
                    tokio::spawn(async move {
                        let update_progress = move |progress: FetchProgress| {
                            tx.send(Action::Fetching(FetchingAction::UpdateFetchStatus(
                                FetchingState::Fetching(progress),
                            )))
                            .unwrap();
                            tx.send(Action::Render).unwrap();
                        };
                        update_progress(FetchProgress {
                            current_page: 1,
                            total_entries_fetched: 0,
                            oldest_date: None,
                        });
                        let cookie = std::env::var("COOKIE").unwrap();
                        let account = std::env::var("ACCOUNT").unwrap();
                        let records = crate::fetcher::fetch_transactions(
                            &cookie,
                            &account,
                            date.timestamp(),
                            Some(Box::new(update_progress)),
                        )
                        .await
                        .unwrap();
                        assert!(!records.is_empty());
                        tx2.send(Action::Fetching(FetchingAction::UpdateFetchStatus(
                            FetchingState::Idle, // 更新状态为 Idle
                        )))
                        .unwrap();
                        tx2.send(Action::Fetching(FetchingAction::InsertTransaction(records)))
                    });
                }

                FetchingAction::InsertTransaction(transactions) => {
                    app.manager.insert(&transactions).unwrap();
                    app.action_tx
                        .send(Action::Fetching(FetchingAction::LoadDbCount))
                        .unwrap();
                }

                FetchingAction::UpdateFetchStatus(state) => {
                    self.fetching_state = state;
                    app.action_tx.send(Action::Render).unwrap();
                }

                FetchingAction::MoveFocus(focus) => {
                    self.current_focus = focus;
                    self.fetch_start_date = match &self.current_focus {
                        Focus::P1Year => Some(
                            Local::now()
                                .checked_sub_signed(chrono::Duration::days(365))
                                .unwrap(),
                        ),
                        Focus::P1Month => Some(
                            Local::now()
                                .checked_sub_signed(chrono::Duration::days(30))
                                .unwrap(),
                        ),
                        Focus::P3Months => Some(
                            Local::now()
                                .checked_sub_signed(chrono::Duration::days(90))
                                .unwrap(),
                        ),
                        Focus::UserInput => {
                            // FIXME
                            Some(
                                Local::now()
                                    .checked_sub_signed(chrono::Duration::days(1))
                                    .unwrap(),
                            )
                        }
                    };
                    app.action_tx.send(Action::Render).unwrap();
                }
                FetchingAction::LoadDbCount => {
                    self.local_db_cnt = app.manager.fetch_count().unwrap();
                    app.action_tx.send(Action::Render).unwrap();
                }
            }
        }
    }

    fn get_name(&self) -> String {
        "Fetch".to_string()
    }

    fn init(&mut self, _app: &mut crate::RootState) {
        _app.action_tx
            .send(Action::Fetching(FetchingAction::LoadDbCount))
            .unwrap();
    }
}
