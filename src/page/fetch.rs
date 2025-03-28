use chrono::{DateTime, Local, TimeZone};
use color_eyre::eyre::Context;
use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Flex, Layout},
    style::{Color, Style},
    text::Text,
    widgets::{Block, BorderType, Borders, Paragraph},
};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, info, instrument};

use crate::{
    actions::Action,
    app::RootState,
    component::{Component, input::InputComp},
    libs::fetcher::MealFetcher,
    utils::help_msg::{HelpEntry, HelpMsg},
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

#[derive(Clone, Debug)]
pub enum FetchingAction {
    StartFetching(DateTime<Local>),
    UpdateFetchStatus(FetchingState),
    InsertTransaction(Vec<transactions::Transaction>),

    LoadDbCount,
    MoveFocus(Focus),
}

impl From<FetchingAction> for Action {
    fn from(val: FetchingAction) -> Self {
        Action::Fetching(val)
    }
}

#[derive(Debug, Clone)]
pub struct Fetch {
    fetching_state: FetchingState,
    local_db_cnt: u64,
    fetch_start_date: Option<DateTime<Local>>,
    current_focus: Focus,

    input: InputComp,

    client: MealFetcher,
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
            )
            .set_auto_submit(true),

            client: Default::default(),
        }
    }
}

impl Fetch {
    fn get_help_msg(&self, app: &RootState) -> HelpMsg {
        let mut help: HelpMsg = vec![
            HelpEntry::new_plain("Move focus: hjkl"),
            HelpEntry::new('e', "Edit accnout & cookie"),
            HelpEntry::new('r', "Refresh local db count"),
            HelpEntry::new(KeyCode::Esc, "Back"),
            HelpEntry::new(' ', "Start fetch"),
        ]
        .into();
        if let Focus::UserInput = self.current_focus {
            help.extend(&self.input.get_help_msg(app.input_mode()))
        }
        help
    }

    #[allow(dead_code)]
    fn client<T: Into<MealFetcher>>(self, client: T) -> Self {
        Self {
            client: client.into(),
            ..self
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
                Constraint::Length(3),
            ])
            .split(area);

        let top_areas = &Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
        ])
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
                        "Currently {} records locally stored.\n Press \"Space\" to fetch transactions since {}",
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

        self.get_help_msg(root_state).render(frame, area[3]);
    }

    fn handle_events(
        &self,
        app: &RootState,
        event: crate::tui::Event,
    ) -> color_eyre::eyre::Result<()> {
        if let crate::tui::Event::Key(key) = event {
            if !app.input_mode() {
                match (key.modifiers, key.code) {
                    (_, KeyCode::Char(' ')) => {
                        if let Some(date) = self.fetch_start_date {
                            app.send_action(FetchingAction::StartFetching(date))
                        }
                    }
                    (_, KeyCode::Char('j')) | (_, KeyCode::Char('l')) => {
                        app.send_action(FetchingAction::MoveFocus(self.current_focus.next()))
                    }
                    (_, KeyCode::Char('k')) | (_, KeyCode::Char('h')) => {
                        app.send_action(FetchingAction::MoveFocus(self.current_focus.prev()))
                    }
                    (_, KeyCode::Char('r')) => app.send_action(FetchingAction::LoadDbCount),
                    (_, KeyCode::Char('e')) => {
                        app.send_action(crate::page::cookie_input::CookieInput::new(app))
                    }

                    (_, KeyCode::Esc) => {
                        app.send_action(super::transactions::Transactions::default())
                    }
                    _ => (),
                }
            }
        };
        self.input.handle_events(&event, app)?;
        Ok(())
    }

    fn update(&mut self, app: &crate::RootState, action: crate::actions::Action) {
        if let Action::Fetching(action) = &action {
            match action {
                FetchingAction::StartFetching(date) => {
                    let tx = app.clone_sender();

                    match &self.client {
                        MealFetcher::Real(c) => {
                            if let Ok((account, cookie)) = app.manager.get_account_cookie() {
                                Fetch::fetch(tx, c.clone().account(account).cookie(cookie), *date);
                            } else {
                                app.send_action(crate::page::cookie_input::CookieInput::new(app))
                            }
                        }
                        MealFetcher::Mock(c) => {
                            Fetch::fetch(tx, c.clone(), *date);
                        }
                    }
                }

                FetchingAction::InsertTransaction(transactions) => {
                    app.manager
                        .insert(transactions)
                        .context("Error when inserting fetched transactions into database")
                        .unwrap();
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
        }

        self.input.update(&action, app).unwrap();
    }

    fn get_name(&self) -> String {
        "Fetch".to_string()
    }

    fn init(&mut self, app: &crate::RootState) {
        app.send_action(Action::Fetching(FetchingAction::LoadDbCount));

        // make sure to load start_fetch_date
        app.send_action(FetchingAction::MoveFocus(self.current_focus.clone()));
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
                        debug!("Invalid date input: {}", input);
                        None
                    }
                }
            }
            Err(_) => {
                debug!("Invalid date input: {}", input);
                None
            }
        }
    }

    fn fetch<T: Into<MealFetcher>>(tx: UnboundedSender<Action>, client: T, date: DateTime<Local>) {
        let client = client.into();

        let tx2 = tx.clone();
        let update_progress = move |progress: FetchProgress| {
            tx.send(Action::Fetching(FetchingAction::UpdateFetchStatus(
                FetchingState::Fetching(progress),
            )))
            .unwrap();
            tx.send(Action::Render).unwrap();
        };

        tokio::task::spawn_blocking(move || {
            info!("Start fetching with client {:?}", client);

            let records = fetcher::fetch(date, client, update_progress)
                .context("Error fetching in Fetch page")
                .unwrap();

            info!("Fetch stopped with {} records", records.len());

            tx2.send(Action::Fetching(FetchingAction::UpdateFetchStatus(
                FetchingState::Idle, // 更新状态为 Idle
            )))
            .unwrap();
            tx2.send(Action::Fetching(FetchingAction::InsertTransaction(records)))
                .unwrap()
        });
    }
}

#[cfg(test)]
mod test {

    use insta::assert_snapshot;
    use ratatui::backend::TestBackend;

    use crate::{tui::Event, utils::key_events::KeyEvent};

    use super::*;
    fn get_test_objs() -> (RootState, Fetch) {
        let mut app = RootState::new(None);
        app.manager.init_db().unwrap();
        let mut page = Fetch::default();
        page.init(&app);
        while let Ok(action) = app.try_recv() {
            page.update(&app, action);
        }
        (app, page)
    }

    #[test]
    fn test_navigation() {
        let (mut app, mut page) = get_test_objs();
        assert!(matches!(page.current_focus, Focus::P1Year));
        app.handle_event_and_update(&mut page, Event::Key(KeyEvent::from('j').into()));
        assert!(matches!(page.current_focus, Focus::P3Months));
        app.handle_event_and_update(&mut page, Event::Key(KeyEvent::from('k').into()));
        assert!(matches!(page.current_focus, Focus::P1Year));
        app.handle_event_and_update(&mut page, Event::Key(KeyEvent::from('h').into()));
        assert!(matches!(page.current_focus, Focus::UserInput));
    }

    #[test]
    fn test_user_input() {
        let (mut app, mut page) = get_test_objs();
        app.handle_event_and_update(&mut page, Event::Key(KeyEvent::from('k').into()));
        assert!(matches!(page.current_focus, Focus::UserInput));
        app.handle_event_and_update(&mut page, Event::Key(KeyEvent::from(KeyCode::Enter).into()));
        assert!(app.input_mode());
        let seq = "2025-03-02";
        seq.chars().for_each(|c| {
            app.handle_event_and_update(&mut page, Event::Key(KeyEvent::from(c).into()));
        });
        assert_eq!(
            page.fetch_start_date.unwrap(),
            Local.with_ymd_and_hms(2025, 3, 2, 0, 0, 0).unwrap()
        );
        app.handle_event_and_update(&mut page, Event::Key(KeyEvent::from(KeyCode::Enter).into()));
        assert!(!app.input_mode());
    }
    #[test]
    fn test_render() {
        let (mut app, mut page) = get_test_objs();
        let mut terminal = ratatui::Terminal::new(TestBackend::new(120, 20)).unwrap();
        app.handle_event_and_update(&mut page, Event::Key(KeyEvent::from('k').into()));
        terminal
            .draw(|f| {
                page.render(f, &app);
            })
            .unwrap();

        assert_snapshot!(terminal.backend());
    }

    #[tokio::test]
    #[ignore]
    async fn test_fetch() {
        // FIXME : This test is not working
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Action>();

        let client = MealFetcher::Mock(fetcher::MockMealFetcher::default());
        let date = Local::now()
            .checked_sub_signed(chrono::Duration::days(30))
            .unwrap();

        Fetch::fetch(tx, client, date);

        let timeout = tokio::time::sleep(std::time::Duration::from_secs(10));
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                Some(action) = rx.recv() => {
                    match action {
                        Action::Fetching(FetchingAction::UpdateFetchStatus(state)) => {
                            match state {
                                FetchingState::Fetching(prog) => {
                                    println!("Fetching... Current Page: {}", prog.current_page);
                                    assert_eq!(prog.current_page, 0);
                                }
                                FetchingState::Idle => {
                                    ()
                                }
                            }
                        }
                        Action::Fetching(FetchingAction::InsertTransaction(_)) => {
                            ()
                        }
                        _ => {}
                    }
                }
                _ = &mut timeout => {
                    println!("Timeout reached");
                    break;
                }
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_fetch_progress() {
        // FIXME : This test is not working
        let (mut app, mut page) = get_test_objs();

        app.manager.update_account("abc").unwrap();
        app.manager.update_cookie("123").unwrap();

        page = page.client(fetcher::MealFetcher::default());

        page.handle_events(&app, Event::Key(KeyEvent::from(' ').into()))
            .unwrap();

        let mut cur_count: i32 = -1;
        let mut received_final_idle = false;
        let mut received_insert = false;

        let timeout = tokio::time::sleep(std::time::Duration::from_secs(10));
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                Some(action) = app.recv() => {
                    println!("Action: {:?}", action);
                    app.update(&action).unwrap();
                    println!("app updated");
                    page.update(&app, action.clone());
                    println!("page updated");
                    match action {
                        Action::Fetching(FetchingAction::UpdateFetchStatus(state)) => {
                            match state {
                                FetchingState::Fetching(prog) => {
                                    assert_eq!(prog.current_page as i32, cur_count + 1);
                                    cur_count = cur_count + 1;
                                }
                                FetchingState::Idle => {
                                    received_final_idle = true;
                                }
                            }
                        }
                        Action::Fetching(FetchingAction::InsertTransaction(_)) => {
                            received_insert = true;
                        }
                        _ => {}
                    }

                    // Exit loop when we've received both the Idle state and Insert action
                    if received_final_idle && received_insert {
                        break;
                    }
                }
                _ = &mut timeout => {
                    println!("Timeout reached");
                    break;
                }
            }
        }
        assert_ne!(cur_count, -1, "No action received");
        assert!(matches!(page.fetching_state, FetchingState::Idle)); // Need to unwrap
    }
}
