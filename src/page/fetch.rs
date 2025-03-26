use chrono::{DateTime, Local, TimeZone};
use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Flex, Layout},
    style::{Color, Style},
    text::Text,
    widgets::{Block, BorderType, Borders, Paragraph},
};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{info, instrument};

use crate::{
    actions::Action,
    app::RootState,
    component::{Component, input::InputComp},
};
use crate::{
    component::input::InputMode,
    libs::{fetcher, transactions},
};

use super::Page;

#[derive(Clone, Default, Debug)]
pub enum FetchingState {
    #[default]
    Idle,
    Fetching(FetchProgress),
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
    InsertTransaction(Vec<transactions::Transaction>),

    LoadDbCount,
    MoveFocus(Focus),
}

impl Into<Action> for FetchingAction {
    fn into(self) -> Action {
        Action::Fetching(self)
    }
}

#[derive(Debug, Clone)]
pub struct Fetch {
    fetching_state: FetchingState,
    local_db_cnt: u64,
    fetch_start_date: Option<DateTime<Local>>,
    current_focus: Focus,

    input: InputComp,
}

impl Default for Fetch {
    fn default() -> Self {
        Self {
            fetching_state: Default::default(),
            local_db_cnt: Default::default(),
            fetch_start_date: Default::default(),
            current_focus: Default::default(),

            input: InputComp::new(
                rand::random::<u64>(),
                None::<String>,
                "Custom Start Date (2025-03-02 style input)",
                Default::default(),
            ),
        }
    }
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
    fn render(&self, frame: &mut ratatui::Frame, root_state: &crate::RootState) {
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

        self.input.draw(frame, &area[1], root_state);

        // 修改这里：显示获取结果
        match &self.fetching_state {
            FetchingState::Idle => {
                frame.render_widget(
                    Text::raw(format!(
                        "Currently {} records locally stored.\n Press \"Enter\" to fetch transactions since {}",
                        self.local_db_cnt,
                        self.fetch_start_date.map_or("N/A".to_string(), |date| date.format("%Y-%m-%d").to_string()),
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
        &self,
        app: &RootState,
        event: crate::tui::Event,
    ) -> color_eyre::eyre::Result<()> {
        match event {
            crate::tui::Event::Key(key) => {
                if !app.input_mode {
                    match (key.modifiers, key.code) {
                        (_, KeyCode::Enter) => match self.fetch_start_date {
                            Some(date) => app.send_action(FetchingAction::StartFetching(date)),
                            None => (),
                        },
                        (_, KeyCode::Char('j')) => {
                            app.send_action(FetchingAction::MoveFocus(self.current_focus.next()))
                        }
                        (_, KeyCode::Char('k')) => {
                            app.send_action(FetchingAction::MoveFocus(self.current_focus.prev()))
                        }
                        (_, KeyCode::Char('l')) => app.send_action(FetchingAction::LoadDbCount),
                        (_, KeyCode::Char('e')) => {
                            app.send_action(crate::page::cookie_input::CookieInput::new(app))
                        }

                        (_, KeyCode::Esc) => {
                            app.send_action(super::transactions::Transactions::default())
                        }
                        _ => (),
                    }
                }
            }
            _ => (),
        };
        self.input.handle_events(&event, app)?;
        Ok(())
    }

    fn update(&mut self, app: &crate::RootState, action: crate::actions::Action) {
        if let Action::Fetching(action) = &action {
            match action {
                FetchingAction::StartFetching(date) => {
                    let tx = app.clone_sender();

                    if let Ok((account, cookie)) = app.manager.get_account_cookie() {
                        tokio::spawn(Fetch::fetch(tx, cookie, account, *date));
                    } else {
                        app.send_action(crate::page::cookie_input::CookieInput::new(app))
                    }
                }

                FetchingAction::InsertTransaction(transactions) => {
                    app.manager.insert(transactions).unwrap();
                    app.send_action(Action::Fetching(FetchingAction::LoadDbCount));
                }

                FetchingAction::UpdateFetchStatus(state) => {
                    self.fetching_state = state.clone();
                    app.send_action(Action::Render);
                }

                FetchingAction::MoveFocus(focus) => {
                    self.current_focus = focus.clone();
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
                        Focus::UserInput => None,
                    };

                    if let Focus::UserInput = &self.current_focus {
                        app.send_action(self.input.get_switch_mode_action(InputMode::Focused));
                    } else {
                        app.send_action(self.input.get_switch_mode_action(InputMode::Idle));
                    }
                }
                FetchingAction::LoadDbCount => {
                    self.local_db_cnt = app.manager.fetch_count().unwrap();
                    app.send_action(Action::Render);
                }
            }
        }

        if let Some(input) = self.input.parse_submit_action(&action) {
            self.fetch_start_date = Fetch::parse_user_input(&input);
            if self.fetch_start_date.is_some() {
                app.send_action(Action::SwitchInputMode(false));
            } else {
                app.send_action(Action::None);
            }
        }

        self.input.update(&action, app).unwrap();
    }

    fn get_name(&self) -> String {
        "Fetch".to_string()
    }

    fn init(&mut self, app: &mut crate::RootState) {
        app.send_action(Action::Fetching(FetchingAction::LoadDbCount))
    }
}

impl Fetch {
    #[instrument]
    fn parse_user_input(input: &str) -> Option<DateTime<Local>> {
        match chrono::NaiveDate::parse_from_str(input.trim(), "%Y-%m-%d") {
            Ok(dt) => {
                match chrono::Local::now()
                    .timezone()
                    .from_local_datetime(&dt.and_hms_opt(0, 0, 0).unwrap())
                {
                    chrono::LocalResult::Single(t) => Some(t),
                    _ => {
                        info!("Invalid date input: {}", input);
                        None
                    }
                }
            }
            Err(_) => {
                info!("Invalid date input: {}", input);
                None
            }
        }
    }

    async fn fetch(
        tx: UnboundedSender<Action>,
        cookie: String,
        account: String,
        date: DateTime<Local>,
    ) {
        let tx2 = tx.clone();
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
        let records = fetcher::fetch_transactions(
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
            .unwrap()
    }
}
