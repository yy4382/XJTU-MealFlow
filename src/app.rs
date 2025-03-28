use crate::{
    actions::{Action, NaviTarget},
    libs::transactions::TransactionManager,
    page::{Page, cookie_input::CookieInput, fetch::Fetch, home::Home, transactions::Transactions},
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
    manager: crate::libs::transactions::TransactionManager,
    input_mode: bool,
}

impl RootState {
    pub fn new(db_path: Option<PathBuf>) -> Self {
        let (action_tx, action_rx) = mpsc::unbounded_channel();

        Self {
            should_quit: false,
            action_tx,
            action_rx,
            manager: TransactionManager::new(db_path.clone())
                .with_context(|| {
                    format!(
                        "Fail to connect to Database at {}",
                        db_path
                            .as_ref()
                            .map(|p| p.display().to_string())
                            .unwrap_or("memory".into())
                    )
                })
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

    pub(crate) fn update(&mut self, action: &Action) -> Result<()> {
        match action {
            Action::Tick => {
                // Make sure all children's input modes are up to date
                self.send_action(Action::SwitchInputMode(self.input_mode));
            }
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

        loop {
            let e = self.tui.next().await?;

            self.handle_event(e.clone())
                .with_context(|| format!("failed to handle event {:?}", e))?;

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
                            self.send_action(Action::NavigateTo(NaviTarget::Home))
                        }
                    }
                    Char('T') => {
                        // check if the current page is not Transactions
                        if self.page.get_name() != "Transactions" {
                            self.send_action(Action::NavigateTo(NaviTarget::Transaction))
                        }
                    }
                    Char('q') => self.send_action(Action::Quit),
                    _ => self.page.handle_events(event).unwrap(),
                }
            }

            _ => self.page.handle_events(event).unwrap(),
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
                        self.page.render(f);
                    })
                    .unwrap();
            }
            Action::NavigateTo(target) => {
                // debug!("Navigating to {:?}", target);
                match *target {
                    NaviTarget::Home => self.page = Box::new(Home::default()),
                    NaviTarget::Fetch => {
                        self.page = Box::new(Fetch::new(
                            self.state.action_tx.clone().into(),
                            self.state.manager.clone(),
                            self.state.input_mode,
                        ))
                    }
                    NaviTarget::Transaction => {
                        self.page = Box::new(Transactions::new(
                            self.state.action_tx.clone().into(),
                            self.state.manager.clone(),
                        ))
                    }
                    NaviTarget::CookieInput => {
                        self.page = Box::new(CookieInput::new(
                            self.state.action_tx.clone().into(),
                            self.state.manager.clone(),
                            self.state.input_mode,
                        ))
                    }
                }
                self.page.init();
            }

            _ => {}
        }
        self.state.update(&action).unwrap();
        self.page.update(action);
    }
}
