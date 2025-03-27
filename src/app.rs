use crate::{
    actions::{Action, NaviTarget},
    libs::transactions::TransactionManager,
    page::{self, Page},
    tui,
};
use color_eyre::eyre::{Context, Result};
use crossterm::event::KeyCode::Char;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tracing::warn;

pub struct RootState {
    should_quit: bool,
    action_tx: tokio::sync::mpsc::UnboundedSender<Action>,
    action_rx: tokio::sync::mpsc::UnboundedReceiver<Action>,
    pub manager: crate::libs::transactions::TransactionManager,
    input_mode: bool,
    // pub config: Config,
}

impl RootState {
    pub fn new(db_path: Option<PathBuf>) -> Self {
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        Self {
            should_quit: false,
            action_tx,
            action_rx,
            manager: TransactionManager::new(db_path)
                .context("Error when creating local cache db manager")
                .unwrap(),
            input_mode: false,
        }
    }
    pub fn send_action<T: Into<Action>>(&self, action: T) {
        let result = self.action_tx.send(action.into());
        if let Err(e) = result {
            warn!("Failed to send action: {:?}", e);
        }
    }
    pub fn clone_sender(&self) -> tokio::sync::mpsc::UnboundedSender<Action> {
        self.action_tx.clone()
    }
    pub fn input_mode(&self) -> bool {
        self.input_mode
    }

    pub(crate) fn update(&mut self, action: &Action) -> Result<()> {
        match action {
            Action::Quit => {
                self.should_quit = true;
            }
            Action::SwitchInputMode(mode) => {
                self.input_mode = *mode;
            }
            _ => {}
        }
        Ok(())
    }
}

pub(super) struct App {
    pub page: Box<dyn Page>,
    pub state: RootState,
    pub tui: tui::Tui,
}

impl App {
    pub async fn run(&mut self) -> Result<()> {
        self.tui.enter()?;
        self.state.manager.init_db()?;

        loop {
            let e = self.tui.next().await?;

            self.handle_event(e)
                .with_context(|| format!("failed to handle event"))?;

            while let Ok(action) = self.state.action_rx.try_recv() {
                self.perform_action(action);
            }

            // application exit
            if self.state.should_quit {
                break;
            }
        }

        self.tui.exit()?;
        Ok(())
    }

    pub fn send_action<T: Into<Action>>(&self, action: T) {
        self.state.send_action(action);
    }

    /// Convert a [`tui::Event`] to an Action and send to channel
    ///
    /// This function is responsible for converting a [`tui::Event`] to an Action.
    ///
    /// It handles some application-wide events like quitting the application
    /// and switching between input modes,
    /// and delegates the handling of page-specific events (remaining events, currently only key events)
    /// to the current page.
    fn handle_event(&self, event: tui::Event) -> Result<()> {
        match event {
            tui::Event::Tick => self.send_action(Action::Tick),
            tui::Event::Render => self.send_action(Action::Render),

            // TODO impl these events
            tui::Event::Error => self.send_action(Action::Quit),
            tui::Event::FocusGained => (),
            tui::Event::FocusLost => (),
            tui::Event::Init => (),
            // tui::Event::Closed => action_tx.send(Action::Quit)?,
            // tui::Event::Quit => action_tx.send(Action::Quit)?,
            tui::Event::Resize(_, _) => self.send_action(Action::Render),

            tui::Event::Key(key) => {
                match key.code {
                    Char('H') => {
                        // check if the current page is not Home
                        if self.page.get_name() != "Home" {
                            self.send_action(page::home::Home::default())
                        }
                    }
                    Char('T') => {
                        // check if the current page is not Transactions
                        if self.page.get_name() != "Transactions" {
                            self.send_action(page::transactions::Transactions::default())
                        }
                    }
                    Char('q') => self.send_action(Action::Quit),
                    _ => self.page.handle_events(&self.state, event).unwrap(),
                }
            }

            _ => self.page.handle_events(&self.state, event).unwrap(),
        };
        Ok(())
    }

    /// Perform an action
    ///
    /// This function is responsible for performing an action (Changing the state of the application).
    ///
    /// This SHOULD be the only place where the state of the application is changed.
    ///
    /// It handles some application-wide actions like quitting the application
    /// and switching between pages,
    /// and delegates the handling of page-specific actions to the current page.
    fn perform_action(&mut self, action: Action) {
        match &action {
            Action::Render => {
                self.tui
                    .draw(|f| {
                        self.page.render(f, &self.state);
                    })
                    .unwrap();
            }
            Action::NavigateTo(target) => {
                // debug!("Navigating to {:?}", target);
                match *target.clone() {
                    NaviTarget::Home(h) => self.page = Box::new(h),
                    NaviTarget::Fetch(fetch) => self.page = Box::new(fetch),
                    NaviTarget::Transaction(transactions) => self.page = Box::new(transactions),
                    NaviTarget::CookieInput(cookie_input) => self.page = Box::new(cookie_input),
                }
                self.page.init(&self.state);
            }

            _ => {}
        }
        self.state.update(&action).unwrap();
        self.page.update(&self.state, action);
    }
}

#[cfg(test)]
impl RootState {
    pub fn try_recv(&mut self) -> Result<Action> {
        Ok(self.action_rx.try_recv()?)
    }

    pub fn handle_event_and_update(&mut self, page: &mut dyn Page, evt: tui::Event) {
        page.handle_events(self, evt).unwrap();
        while let Ok(action) = self.try_recv() {
            self.update(&action).unwrap();
            page.update(self, action);
        }
    }
}
