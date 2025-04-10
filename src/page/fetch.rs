use chrono::{DateTime, FixedOffset, Local};
use color_eyre::eyre::Context;
use crossterm::event::KeyCode;
use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Style},
    text::Text,
    widgets::{Block, BorderType, Borders, Paragraph},
};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tracing::{info, instrument, warn};

use crate::{
    actions::{ActionSender, LayerManageAction, Layers},
    app::layer_manager::EventHandlingStatus,
    component::input::InputComp,
    libs::{fetcher::MealFetcher, transactions::OFFSET_UTC_PLUS8},
    tui::Event,
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
    UpdateFetchStatus(FetchingState),
    InsertTransaction(Vec<transactions::Transaction>),
}

#[derive(Debug)]
pub struct Fetch {
    fetching_state: FetchingState,
    local_db_cnt: u64,
    fetch_start_date: Option<DateTime<FixedOffset>>,
    current_focus: Focus,

    self_rx: UnboundedReceiver<FetchingAction>,
    self_tx: UnboundedSender<FetchingAction>,

    input: InputComp,
    tx: ActionSender,
    manager: transactions::TransactionManager,

    client: MealFetcher,
}

impl Fetch {
    pub fn new(tx: ActionSender, manager: transactions::TransactionManager) -> Self {
        let (self_tx, self_rx) = mpsc::unbounded_channel::<FetchingAction>();
        Self {
            fetching_state: Default::default(),
            local_db_cnt: Default::default(),
            fetch_start_date: Default::default(),
            current_focus: Default::default(),

            self_rx,
            self_tx,

            input: InputComp::new()
                .title("Custom Start Date (2025-03-02 style input)")
                .auto_submit(true),
            tx,
            manager,

            client: Default::default(),
        }
    }
}

impl Fetch {
    fn get_help_msg(&self) -> HelpMsg {
        if self.input.is_inputting() {
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
    fn handle_events(&mut self, event: &Event) -> EventHandlingStatus {
        let mut status = EventHandlingStatus::default();

        let (input_status, input_result) = self.input.handle_events(event);
        if let Some(result) = input_result {
            self.fetch_start_date = Fetch::parse_user_input(&result)
        }
        if matches!(input_status, EventHandlingStatus::Consumed) {
            return input_status;
        }

        match event {
            Event::Tick => {
                while let Ok(action) = self.self_rx.try_recv() {
                    self.update(action);
                }
            }
            Event::Key(key) => match (key.modifiers, key.code) {
                (_, KeyCode::Char(' ')) => {
                    if let Some(date) = self.fetch_start_date {
                        self.start_fetch(date);
                        status.consumed();
                    }
                }
                (_, KeyCode::Char('j')) | (_, KeyCode::Char('l')) => {
                    self.move_focus(self.current_focus.next());
                    status.consumed();
                }
                (_, KeyCode::Char('k')) | (_, KeyCode::Char('h')) => {
                    self.move_focus(self.current_focus.prev());
                    status.consumed();
                }
                (_, KeyCode::Char('r')) => {
                    self.local_db_cnt = self.manager.fetch_count().unwrap();
                    status.consumed()
                }
                (_, KeyCode::Char('e')) => {
                    self.tx.send(LayerManageAction::Swap(Layers::CookieInput));
                    status.consumed();
                }

                (_, KeyCode::Esc) => {
                    self.tx
                        .send(LayerManageAction::Swap(Layers::Transaction(None)));
                    status.consumed();
                }
                (_, KeyCode::Char('?')) => {
                    self.tx.send(LayerManageAction::Push(
                        Layers::Help(self.get_help_msg()).into_push_config(true),
                    ));
                    status.consumed();
                }
                _ => (),
            },
            _ => (),
        };
        status
    }
}

impl Layer for Fetch {
    fn init(&mut self) {
        self.local_db_cnt = self.manager.fetch_count().unwrap();

        // make sure to load start_fetch_date
        self.move_focus(self.current_focus.clone());
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

    fn fetch<T: Into<MealFetcher>>(
        tx: UnboundedSender<FetchingAction>,
        client: T,
        date: DateTime<FixedOffset>,
    ) {
        let client = client.into();

        let tx2 = tx.clone();
        let update_progress = move |progress: FetchProgress| {
            tx.send(FetchingAction::UpdateFetchStatus(FetchingState::Fetching(
                progress,
            )))
            .context("Updating progress failed because layer was dropped while fetching")
        };

        tokio::task::spawn_blocking(move || {
            let records = match fetcher::fetch(date, client, update_progress)
                .context("Error fetching in Fetch page")
            {
                Ok(data) => data,
                Err(e) => {
                    warn!("Error fetching data: {}", e);
                    return;
                }
            };

            info!("Fetch stopped with {} records", records.len());

            // This may fail if the layer is dropped while fetching
            // but we don't care about the error here
            let _ = tx2.send(FetchingAction::UpdateFetchStatus(FetchingState::Idle));
            let _ = tx2.send(FetchingAction::InsertTransaction(records));
        });
    }

    fn update(&mut self, action: FetchingAction) {
        match action {
            FetchingAction::InsertTransaction(transactions) => {
                self.manager
                    .insert(&transactions)
                    .context("Error when inserting fetched transactions into database")
                    .unwrap();
                self.local_db_cnt = self.manager.fetch_count().unwrap();
            }

            FetchingAction::UpdateFetchStatus(state) => {
                self.fetching_state = state.clone();
            }
        }
    }
    fn move_focus(&mut self, focus: Focus) {
        self.current_focus = focus.clone();

        let get_date_from_now = |days| {
            Local::now()
                .fixed_offset()
                .checked_sub_signed(chrono::Duration::days(days))
                .unwrap()
        };

        self.fetch_start_date = match &self.current_focus {
            Focus::P1Year => Some(get_date_from_now(365)),
            Focus::P1Month => Some(get_date_from_now(30)),
            Focus::P3Months => Some(get_date_from_now(90)),
            Focus::UserInput => None,
        };

        if let Focus::UserInput = &self.current_focus {
            self.input.set_mode(InputMode::Focused);
        } else {
            self.input.set_mode(InputMode::Idle);
        }
    }

    fn start_fetch(&mut self, date: DateTime<FixedOffset>) {
        let tx = self.self_tx.clone();

        match &self.client {
            MealFetcher::Real(c) => {
                if let Ok((account, cookie)) = self.manager.get_account_cookie() {
                    Fetch::fetch(tx, c.clone().account(account).cookie(cookie), date);
                } else {
                    self.tx.send(LayerManageAction::Swap(Layers::CookieInput));
                }
            }
            MealFetcher::Mock(c) => {
                Fetch::fetch(tx, c.clone(), date);
            }
        }
    }
}

#[cfg(test)]
mod test {

    use chrono::TimeZone as _;
    use insta::assert_snapshot;
    use ratatui::backend::TestBackend;
    use tokio::sync::mpsc::{self, UnboundedReceiver};

    use crate::{
        actions::Action,
        libs::transactions::{OFFSET_UTC_PLUS8, TransactionManager},
        tui::Event,
    };

    use super::*;
    fn get_test_objs() -> (UnboundedReceiver<Action>, Fetch) {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut page = Fetch::new(tx.clone().into(), TransactionManager::new(None).unwrap());
        page.init();
        (rx, page)
    }

    #[test]
    fn test_navigation() {
        let (_, mut page) = get_test_objs();
        assert!(matches!(page.current_focus, Focus::P1Year));

        let event_result = [
            ('j', Focus::P3Months),
            ('j', Focus::P1Month),
            ('l', Focus::UserInput),
            ('l', Focus::P1Year),
            ('l', Focus::UserInput),
            ('k', Focus::UserInput),
            ('k', Focus::P1Month),
            ('h', Focus::P3Months),
            ('h', Focus::P1Year),
        ];

        for (key, _result) in event_result.into_iter() {
            page.handle_event_with_status_check(&key.into());
            assert!(matches!(page.current_focus, ref _result))
        }
    }

    #[test]
    fn test_user_input() {
        let (_, mut page) = get_test_objs();
        page.handle_event_with_status_check(&'k'.into());
        assert!(matches!(page.current_focus, Focus::UserInput));
        page.handle_event_with_status_check(&KeyCode::Enter.into());
        let seq = "2025-03-02";
        seq.chars().for_each(|c| {
            page.handle_event_with_status_check(&c.into());
        });
        assert_eq!(
            page.fetch_start_date.unwrap(),
            OFFSET_UTC_PLUS8
                .with_ymd_and_hms(2025, 3, 2, 0, 0, 0)
                .unwrap()
        );
        page.handle_event_with_status_check(&KeyCode::Enter.into());
    }
    #[test]
    fn test_render() {
        let (_, mut page) = get_test_objs();
        let mut terminal = ratatui::Terminal::new(TestBackend::new(120, 20)).unwrap();
        page.handle_event_with_status_check(&'h'.into());
        terminal
            .draw(|f| {
                page.render(f, f.area());
            })
            .unwrap();

        assert_snapshot!(terminal.backend());
    }

    #[tokio::test]
    async fn test_fetch() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<FetchingAction>();

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
                        FetchingAction::UpdateFetchStatus(state) => {
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
                        FetchingAction::InsertTransaction(transactions) => {
                            assert!(!transactions.is_empty(), "Should receive some transactions");
                            received_insert = true;
                        }
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
        let (_, page) = get_test_objs();

        page.manager.update_account("account").unwrap();
        page.manager.update_cookie("cookie").unwrap();
        let mut page = page.client(MealFetcher::Mock(fetcher::MockMealFetcher::default()));

        page.handle_event_with_status_check(&'h'.into());

        let mut seq: Vec<Event> = vec![KeyCode::Enter.into()];
        seq.extend("2024-09-01".chars().map(|c| Event::from(c)));
        seq.push(KeyCode::Enter.into());
        seq.iter().for_each(|e| {
            page.handle_event_with_status_check(&e);
        });

        // start fetching
        page.handle_event_with_status_check(&' '.into());

        let timeout = tokio::time::sleep(std::time::Duration::from_secs(10));
        tokio::pin!(timeout);

        let mut received_insert = false;
        let mut last_progress = 0;

        loop {
            tokio::select! {
                Some(action) = page.self_rx.recv() => {

                    match &action {
                        FetchingAction::InsertTransaction(transactions) => {
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
