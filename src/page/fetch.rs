use chrono::{DateTime, FixedOffset, Local};
use color_eyre::eyre::Context;
use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Style},
    text::Text,
    widgets::{Block, BorderType, Borders, Paragraph},
};
use tracing::{info, instrument};

use crate::{
    actions::{Action, ActionSender, LayerManageAction, Layers},
    component::{Component, input::InputComp},
    libs::{fetcher::MealFetcher, transactions::OFFSET_UTC_PLUS8},
    utils::help_msg::{HelpEntry, HelpMsg},
};
use crate::{
    component::input::InputMode,
    libs::{fetcher, transactions},
};

use super::{EventLoopParticipant, Layer, WidgetExt};

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
    pub oldest_date: Option<DateTime<FixedOffset>>,
}

#[derive(Clone, Debug)]
pub enum FetchingAction {
    StartFetching(DateTime<FixedOffset>),
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
    fetch_start_date: Option<DateTime<FixedOffset>>,
    current_focus: Focus,

    input: InputComp,
    input_mode: bool,
    tx: ActionSender,
    manager: transactions::TransactionManager,

    client: MealFetcher,
}

impl Fetch {
    pub fn new(
        tx: ActionSender,
        manager: transactions::TransactionManager,
        input_mode: bool,
    ) -> Self {
        Self {
            fetching_state: Default::default(),
            local_db_cnt: Default::default(),
            fetch_start_date: Default::default(),
            current_focus: Default::default(),

            input: InputComp::new(rand::random::<u64>(), input_mode, tx.clone())
                .title("Custom Start Date (2025-03-02 style input)")
                .auto_submit(true),
            input_mode,
            tx,
            manager,

            client: Default::default(),
        }
    }
}

impl Fetch {
    fn get_help_msg(&self) -> HelpMsg {
        if self.input_mode {
            return self.input.get_help_msg();
        }

        let mut help: HelpMsg = vec![
            HelpEntry::new_plain("hjkl", "Move focus"),
            HelpEntry::new('e', "Edit account & cookie"),
            HelpEntry::new('?', "Show help"),
            HelpEntry::new('r', "Refresh local db count"),
            HelpEntry::new(KeyCode::Esc, "Back"),
            HelpEntry::new(' ', "Start fetch"),
        ]
        .into();
        if let Focus::UserInput = self.current_focus {
            help.extend(&self.input.get_help_msg())
        }
        help
    }

    pub fn client<T: Into<MealFetcher>>(mut self, client: T) -> Self {
        self.client = client.into();
        self
    }

    #[cfg(test)]
    pub fn get_client(&self) -> MealFetcher {
        self.client.clone()
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

impl WidgetExt for Fetch {
    fn render(&mut self, frame: &mut ratatui::Frame, area: Rect) {
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

        self.input.render(frame, area[1]);

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
            }
        }

        self.get_help_msg().render(frame, area[3]);
    }
}

impl EventLoopParticipant for Fetch {
    fn handle_events(&self, event: crate::tui::Event) -> color_eyre::eyre::Result<()> {
        if let crate::tui::Event::Key(key) = event {
            if !self.input_mode {
                match (key.modifiers, key.code) {
                    (_, KeyCode::Char(' ')) => {
                        if let Some(date) = self.fetch_start_date {
                            self.tx.send(FetchingAction::StartFetching(date))
                        }
                    }
                    (_, KeyCode::Char('j')) | (_, KeyCode::Char('l')) => self
                        .tx
                        .send(FetchingAction::MoveFocus(self.current_focus.next())),
                    (_, KeyCode::Char('k')) | (_, KeyCode::Char('h')) => self
                        .tx
                        .send(FetchingAction::MoveFocus(self.current_focus.prev())),
                    (_, KeyCode::Char('r')) => self.tx.send(FetchingAction::LoadDbCount),
                    (_, KeyCode::Char('e')) => self
                        .tx
                        .send(LayerManageAction::SwapPage(Layers::CookieInput)),

                    (_, KeyCode::Esc) => self
                        .tx
                        .send(LayerManageAction::SwapPage(Layers::Transaction(None))),
                    (_, KeyCode::Char('?')) => {
                        self.tx.send(LayerManageAction::PushPage(
                            Layers::Help(self.get_help_msg()).into_push_config(true),
                        ));
                    }
                    _ => (),
                }
            }
        };
        self.input.handle_events(&event)?;
        Ok(())
    }

    fn update(&mut self, action: crate::actions::Action) {
        if let Action::SwitchInputMode(mode) = &action {
            self.input_mode = *mode;
        }
        if let Action::Fetching(action) = &action {
            match action {
                FetchingAction::StartFetching(date) => {
                    let tx = self.tx.clone();

                    match &self.client {
                        MealFetcher::Real(c) => {
                            if let Ok((account, cookie)) = self.manager.get_account_cookie() {
                                Fetch::fetch(tx, c.clone().account(account).cookie(cookie), *date);
                            } else {
                                self.tx
                                    .send(LayerManageAction::SwapPage(Layers::CookieInput));
                            }
                        }
                        MealFetcher::Mock(c) => {
                            Fetch::fetch(tx, c.clone(), *date);
                        }
                    }
                }

                FetchingAction::InsertTransaction(transactions) => {
                    self.manager
                        .insert(transactions)
                        .context("Error when inserting fetched transactions into database")
                        .unwrap();
                    self.tx.send(Action::Fetching(FetchingAction::LoadDbCount));
                }

                FetchingAction::UpdateFetchStatus(state) => {
                    self.fetching_state = state.clone();
                    self.tx.send(Action::Render);
                }

                FetchingAction::MoveFocus(focus) => {
                    self.current_focus = focus.clone();
                    self.fetch_start_date = match &self.current_focus {
                        Focus::P1Year => Some(
                            Local::now()
                                .fixed_offset()
                                .checked_sub_signed(chrono::Duration::days(365))
                                .unwrap(),
                        ),
                        Focus::P1Month => Some(
                            Local::now()
                                .fixed_offset()
                                .checked_sub_signed(chrono::Duration::days(30))
                                .unwrap(),
                        ),
                        Focus::P3Months => Some(
                            Local::now()
                                .fixed_offset()
                                .checked_sub_signed(chrono::Duration::days(90))
                                .unwrap(),
                        ),
                        Focus::UserInput => None,
                    };

                    if let Focus::UserInput = &self.current_focus {
                        self.tx
                            .send(self.input.get_switch_mode_action(InputMode::Focused));
                    } else {
                        self.tx
                            .send(self.input.get_switch_mode_action(InputMode::Idle));
                    }
                }
                FetchingAction::LoadDbCount => {
                    self.local_db_cnt = self.manager.fetch_count().unwrap();
                    self.tx.send(Action::Render);
                }
            }
        }

        if let Some(input) = self.input.parse_submit_action(&action) {
            self.fetch_start_date = Fetch::parse_user_input(&input);
        }

        self.input.update(&action).unwrap();
    }
}

impl Layer for Fetch {
    fn init(&mut self) {
        self.tx.send(Action::Fetching(FetchingAction::LoadDbCount));

        // make sure to load start_fetch_date
        self.tx
            .send(FetchingAction::MoveFocus(self.current_focus.clone()));
    }
}

impl Fetch {
    #[instrument]
    fn parse_user_input(input: &str) -> Option<DateTime<FixedOffset>> {
        chrono::NaiveDate::parse_from_str(input, "%Y-%m-%d")
            .ok()
            .map(|d| {
                d.and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_local_timezone(OFFSET_UTC_PLUS8)
                    .single()
                    .expect("Failed to convert to local timezone")
            })
    }

    fn fetch<T: Into<MealFetcher>>(tx: ActionSender, client: T, date: DateTime<FixedOffset>) {
        let client = client.into();

        let tx2 = tx.clone();
        let update_progress = move |progress: FetchProgress| {
            tx.send(Action::Fetching(FetchingAction::UpdateFetchStatus(
                FetchingState::Fetching(progress),
            )));
            tx.send(Action::Render);
        };

        tokio::task::spawn_blocking(move || {
            info!("Start fetching with client {:?}", client);

            let records = fetcher::fetch(date, client, update_progress)
                .context("Error fetching in Fetch page")
                .unwrap();

            info!("Fetch stopped with {} records", records.len());

            tx2.send(Action::Fetching(FetchingAction::UpdateFetchStatus(
                FetchingState::Idle, // 更新状态为 Idle
            )));
            tx2.send(Action::Fetching(FetchingAction::InsertTransaction(records)));
        });
    }
}

#[cfg(test)]
mod test {

    use chrono::TimeZone as _;
    use insta::assert_snapshot;
    use ratatui::backend::TestBackend;
    use tokio::sync::mpsc::{self, UnboundedReceiver};

    use crate::{
        libs::transactions::{OFFSET_UTC_PLUS8, TransactionManager},
        tui::Event,
        utils::key_events::KeyEvent,
    };

    use super::*;
    fn get_test_objs() -> (UnboundedReceiver<Action>, Fetch) {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut page = Fetch::new(
            tx.clone().into(),
            TransactionManager::new(None).unwrap(),
            false,
        );
        page.init();
        while let Ok(action) = rx.try_recv() {
            page.update(action);
        }
        (rx, page)
    }

    #[test]
    fn test_navigation() {
        let (mut rx, mut page) = get_test_objs();
        assert!(matches!(page.current_focus, Focus::P1Year));
        page.event_loop_once(&mut rx, 'j'.into());
        assert!(matches!(page.current_focus, Focus::P3Months));
        page.event_loop_once(&mut rx, 'j'.into());
        assert!(matches!(page.current_focus, Focus::P1Month));
        page.event_loop_once(&mut rx, 'l'.into());
        assert!(matches!(page.current_focus, Focus::UserInput));
        page.event_loop_once(&mut rx, 'l'.into());
        assert!(matches!(page.current_focus, Focus::P1Year));
        page.event_loop_once(&mut rx, 'k'.into());
        assert!(matches!(page.current_focus, Focus::UserInput));
        page.event_loop_once(&mut rx, 'k'.into());
        assert!(matches!(page.current_focus, Focus::P1Month));
        page.event_loop_once(&mut rx, 'h'.into());
        assert!(matches!(page.current_focus, Focus::P3Months));
        page.event_loop_once(&mut rx, 'h'.into());
        assert!(matches!(page.current_focus, Focus::P1Year));
    }

    #[test]
    fn test_input_mode_change() {
        let (mut rx, mut page) = get_test_objs();
        assert!(!page.input_mode);
        page.event_loop_once_with_action(&mut rx, Action::SwitchInputMode(true));
        assert!(page.input_mode);
        page.event_loop_once_with_action(&mut rx, Action::SwitchInputMode(false));
        assert!(!page.input_mode);
    }

    #[test]
    fn test_user_input() {
        let (mut rx, mut page) = get_test_objs();
        page.event_loop_once(&mut rx, Event::Key(KeyEvent::from('k').into()));
        assert!(matches!(page.current_focus, Focus::UserInput));
        page.event_loop_once(&mut rx, Event::Key(KeyEvent::from(KeyCode::Enter).into()));
        assert!(page.input_mode);
        let seq = "2025-03-02";
        seq.chars().for_each(|c| {
            page.event_loop_once(&mut rx, Event::Key(KeyEvent::from(c).into()));
        });
        assert_eq!(
            page.fetch_start_date.unwrap(),
            OFFSET_UTC_PLUS8
                .with_ymd_and_hms(2025, 3, 2, 0, 0, 0)
                .unwrap()
        );
        page.event_loop_once(&mut rx, KeyCode::Enter.into());
        assert!(!page.input_mode);
    }
    #[test]
    fn test_render() {
        let (mut rx, mut page) = get_test_objs();
        let mut terminal = ratatui::Terminal::new(TestBackend::new(120, 20)).unwrap();
        page.event_loop_once(&mut rx, 'h'.into());
        terminal
            .draw(|f| {
                page.render(f, f.area());
            })
            .unwrap();

        assert_snapshot!(terminal.backend());
    }

    #[tokio::test]
    async fn test_fetch() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Action>();

        let client = MealFetcher::Mock(fetcher::MockMealFetcher::default());
        let date = OFFSET_UTC_PLUS8
            .with_ymd_and_hms(2025, 03, 1, 0, 0, 0)
            .unwrap();

        Fetch::fetch(tx.into(), client, date);

        let timeout = tokio::time::sleep(std::time::Duration::from_secs(10));
        tokio::pin!(timeout);

        let mut received_fetching = false;
        let mut received_idle = false;
        let mut received_insert = false;
        let mut progress_count = 0;

        loop {
            tokio::select! {
                Some(action) = rx.recv() => {
                    match action {
                        Action::Fetching(FetchingAction::UpdateFetchStatus(state)) => {
                            match state {
                                FetchingState::Fetching(prog) => {
                                    println!("Fetching... Current Page: {}", prog.current_page);
                                    assert_eq!(prog.current_page, progress_count, "Mock fetcher should only process page 0");
                                    received_fetching = true;
                                    progress_count += 1;
                                }
                                FetchingState::Idle => {
                                    received_idle = true;
                                }
                            }
                        }
                        Action::Fetching(FetchingAction::InsertTransaction(transactions)) => {
                            assert!(!transactions.is_empty(), "Should receive some transactions");
                            received_insert = true;
                        }
                        _ => {}
                    }

                    // Exit loop when we've received all expected actions
                    if received_fetching && received_idle && received_insert {
                        break;
                    }
                }
                _ = &mut timeout => {
                    println!("Timeout reached");
                    break;
                }
            }
        }

        // Final assertions
        assert!(received_fetching, "Should have received fetching state");
        assert!(received_idle, "Should have received idle state");
        assert!(
            received_insert,
            "Should have received insert transaction action"
        );
        assert!(
            progress_count > 0,
            "Should have received at least one progress update"
        );
    }

    #[tokio::test]
    async fn test_fetch_progress() {
        let (mut rx, page) = get_test_objs();

        page.manager.update_account("account").unwrap();
        page.manager.update_cookie("cookie").unwrap();
        let mut page = page.client(MealFetcher::Mock(fetcher::MockMealFetcher::default()));

        page.event_loop_once_with_action(
            &mut rx,
            Action::Fetching(FetchingAction::MoveFocus(Focus::UserInput)),
        );
        let mut seq: Vec<Event> = vec![KeyCode::Enter.into()];
        seq.extend("2024-09-01".chars().map(|c| Event::from(c)));
        seq.push(KeyCode::Enter.into());
        seq.iter()
            .for_each(|e| page.event_loop_once(&mut rx, e.clone()));

        // start fetching
        page.handle_events(' '.into()).unwrap();

        let timeout = tokio::time::sleep(std::time::Duration::from_secs(10));
        tokio::pin!(timeout);

        let mut received_insert = false;
        let mut last_progress = 0;

        loop {
            tokio::select! {
                Some(action) = rx.recv() => {

                    match &action {
                        Action::Fetching(FetchingAction::InsertTransaction(transactions)) => {
                            assert!(!transactions.is_empty(), "Should receive some transactions");
                            received_insert = true;
                        }
                        _ => {}
                    }

                    page.update(action);

                    if let FetchingState::Fetching(progress) = &page.fetching_state {
                        if progress.total_entries_fetched >= last_progress {
                            println!("Fetching... Current Page: {}", progress.current_page);
                            last_progress = progress.current_page;
                        } else {
                            panic!("Progress should always increase");
                        }
                    }

                    // Exit loop when we've received all expected actions
                    if received_insert {
                        break;
                    }
                }
                _ = &mut timeout => {
                    println!("Timeout reached");
                    break;
                }
            }
        }

        assert!(
            received_insert,
            "Should have received insert transaction action"
        );
        assert!(
            last_progress > 0,
            "Should have received at least one progress update"
        );
    }
}
