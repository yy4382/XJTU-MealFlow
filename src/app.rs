use std::time::Duration;

use crate::{
    actions::{Action, NaviTarget},
    config::Config,
    libs::{fetcher::MockMealFetcher, transactions::TransactionManager},
    page::{Page, cookie_input::CookieInput, fetch::Fetch, home::Home, transactions::Transactions},
    tui,
};
use color_eyre::eyre::{Context, Result};
use crossterm::event::KeyCode::Char;
use tokio::sync::mpsc;
use tracing::warn;

pub struct RootState {
    should_quit: bool,
    action_tx: tokio::sync::mpsc::UnboundedSender<Action>,
    action_rx: tokio::sync::mpsc::UnboundedReceiver<Action>,
    manager: crate::libs::transactions::TransactionManager,
    input_mode: bool,

    config: Config,
}

impl RootState {
    pub fn new(config: Config) -> Self {
        let (action_tx, action_rx) = mpsc::unbounded_channel();

        let manager = TransactionManager::new(config.config.db_path())
            .with_context(|| {
                format!(
                    "Fail to connect to Database at {}",
                    config
                        .config
                        .db_path()
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or("memory".into())
                )
            })
            .unwrap();

        if let Some(account) = &config.fetch.account {
            manager.update_account(&account).unwrap();
        }
        if let Some(hallticket) = &config.fetch.hallticket {
            manager.update_hallticket(hallticket).unwrap();
        }

        Self {
            should_quit: false,
            action_tx,
            action_rx,
            manager: manager,
            input_mode: false,

            config,
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

            self.event_loop(e)?;

            // application exit
            if self.state.should_quit {
                break;
            }
        }

        self.tui.exit()?;
        Ok(())
    }

    fn event_loop(&mut self, e: tui::Event) -> Result<()> {
        self.handle_event(e.clone())
            .with_context(|| format!("failed to handle event {:?}", e))?;

        while let Ok(action) = self.state.action_rx.try_recv() {
            self.perform_action(action);
        }
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
                        self.page.render(f, f.area());
                    })
                    .unwrap();
            }
            Action::NavigateTo(target) => {
                // debug!("Navigating to {:?}", target);
                match *target {
                    NaviTarget::Home => self.page = Box::new(Home::default()),
                    NaviTarget::Fetch => {
                        let fetch_page = Fetch::new(
                            self.state.action_tx.clone().into(),
                            self.state.manager.clone(),
                            self.state.input_mode,
                        )
                        .client(if self.state.config.fetch.use_mock_data {
                            MockMealFetcher::default()
                                .set_sim_delay(Duration::from_secs(1))
                                .per_page(50)
                        } else {
                            Default::default()
                        });

                        self.page = Box::new(fetch_page);
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

#[cfg(test)]
mod test {
    use clap::Parser;

    use crate::cli::{ClapSource, Cli};

    use super::*;

    fn get_config(mut args: Vec<&str>, append_to_default: bool) -> Config {
        let mut default_args = vec!["test-config"];
        let args = if append_to_default {
            default_args.push("--db-in-mem");
            default_args.append(&mut args);
            default_args
        } else {
            default_args.append(&mut args);
            default_args
        };
        let cli = Cli::parse_from(args);
        crate::config::Config::new(Some(ClapSource::new(&cli))).expect("Failed to load config")
    }
    #[test]
    fn root_state_set_fetch_config() {
        let config = get_config(vec!["--account", "123456", "--hallticket", "543210"], true);

        let root = RootState::new(config);
        let (account, cookie) = root.manager.get_account_cookie().unwrap();
        assert_eq!(account, "123456");
        assert_eq!(cookie, "hallticket=543210");
    }

    #[tokio::test]
    async fn app_navigation() {
        let config = get_config(vec![], true);

        let mut app = App {
            page: Box::new(Home::default()),
            state: RootState::new(config),
            tui: tui::Tui::new().unwrap(),
        };

        // Navigate to Fetch page
        app.event_loop('T'.into()).unwrap();
        assert_eq!(app.page.get_name(), "Transactions");

        app.event_loop('H'.into()).unwrap();
        assert_eq!(app.page.get_name(), "Home");

        app.perform_action(Action::NavigateTo(NaviTarget::Fetch));
        assert_eq!(app.page.get_name(), "Fetch");

        app.perform_action(Action::NavigateTo(NaviTarget::CookieInput));
        assert_eq!(app.page.get_name(), "Cookie Input");
    }

    #[tokio::test]
    async fn app_nav_fetch_mock() {
        let config = get_config(vec!["--use-mock-data"], true);
        let mut app = App {
            page: Box::new(Home::default()),
            state: RootState::new(config),
            tui: tui::Tui::new().unwrap(),
        };

        app.perform_action(Action::NavigateTo(NaviTarget::Fetch));
        assert_eq!(app.page.get_name(), "Fetch");
        // TODO find a way to assert that it is using mock client
    }

    #[tokio::test]
    async fn app_quit() {
        let config = get_config(vec![], true);
        let mut app = App {
            page: Box::new(Home::default()),
            state: RootState::new(config),
            tui: tui::Tui::new().unwrap(),
        };

        app.perform_action(Action::Quit);
        assert_eq!(app.state.should_quit, true);
    }
}
