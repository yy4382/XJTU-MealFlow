//! Application state management and main event loop.
//!
//! This module contains the core application logic, including:
//! - Root state management ([`RootState`])
//! - Main application structure ([`App`])
//! - Event handling and action dispatch
//! - Page/Layer management
//!
//! # Architecture
//!
//! The application follows an event-driven architecture with the following components:
//!
//! - Events are received from the TUI
//! - Events are converted to Actions
//! - Actions are processed to update the application state
//! - Rerender of the UI is triggered not by state changes, but by time-sequential events which
//! is controlled by frame rate parameter
//!
//! # State Management
//!
//! The application state is split into two main parts:
//!
//! - [`RootState`]: Handles global application state like quit flags and input modes
//! - Page-specific state: Each page manages its own internal state
//!
//! # Action Flow
//!
//! 1. Events are received in the main event loop
//! 2. Events are converted to Actions via [`App::handle_event`]
//! 3. Actions are processed by [`App::perform_action`]
//! 4. State is updated accordingly
//!
//! # Layer Management
//!
//! The application uses a stack-based approach for managing UI layers:
//!
//! - Pages can be pushed onto the stack ([`LayerManageAction::PushPage`])
//! - Pages can be popped off the stack ([`LayerManageAction::PopPage`])
//! - The current page can be swapped ([`LayerManageAction::SwapPage`])
//!
//! When the last page is popped, the application exits.

use std::time::Duration;

use crate::{
    actions::{Action, LayerManageAction, Layers},
    config::Config,
    libs::{fetcher::MockMealFetcher, transactions::TransactionManager},
    page::{
        Layer, cookie_input::CookieInput, fetch::Fetch, help_popup::HelpPopup, home::Home,
        transactions::Transactions,
    },
    tui,
};
use color_eyre::eyre::{Context, Result};
use crossterm::event::KeyCode::Char;
use tokio::sync::mpsc;
use tracing::warn;

pub(crate) struct RootState {
    /// Flag to indicate if the application should quit
    should_quit: bool,
    /// Channel for sending actions
    action_tx: tokio::sync::mpsc::UnboundedSender<Action>,
    /// Channel for receiving actions
    action_rx: tokio::sync::mpsc::UnboundedReceiver<Action>,

    /// Transaction manager for interacting with the database
    manager: crate::libs::transactions::TransactionManager,

    /// Flag to indicate if the application is in input mode
    input_mode: bool,

    /// Configuration for the application
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
            manager,
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

    pub fn clone_tx(&self) -> tokio::sync::mpsc::UnboundedSender<Action> {
        self.action_tx.clone()
    }

    /// Update the state based on the action
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
    pub page: Vec<Box<dyn Layer>>,
    pub state: RootState,
    pub tui: tui::TuiEnum,
}

impl App {
    /// The main event loop for the application
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

    /// The event loop for the application
    ///
    /// Takes in a [`tui::Event`] and handles it, updating the application state
    fn event_loop(&mut self, e: tui::Event) -> Result<()> {
        self.handle_event(e.clone())
            .with_context(|| format!("failed to handle event {:?}", e))?;

        while let Ok(action) = self.state.action_rx.try_recv() {
            self.perform_action(action);
            if self.state.should_quit {
                break;
            }
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
        let last_page = self.page.last().expect("Page stack is empty");

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

            tui::Event::Key(key) => match key.code {
                Char('q') => self.send_action(Action::Quit),
                _ => last_page.handle_events(event).unwrap(),
            },

            _ => last_page.handle_events(event).unwrap(),
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
                        self.page
                            .iter_mut()
                            .for_each(|page| page.render(f, f.area()));
                    })
                    .unwrap();
            }
            Action::Layer(layer_action) => match layer_action {
                LayerManageAction::SwapPage(target) => {
                    self.page.pop();
                    self.page
                        .push(self.get_layer(target).expect("Failed to get layer"));
                }
                LayerManageAction::PushPage(target) => {
                    if let Some(page) = self.get_layer(target) {
                        self.page.push(page);
                    } else {
                        warn!("Failed to get layer");
                    }
                }
                LayerManageAction::PopPage => {
                    if self.page.len() > 1 {
                        self.page.pop();
                    } else {
                        self.state.should_quit = true;
                    }
                }
            },

            _ => {}
        }
        self.state.update(&action).unwrap();
        self.page
            .last_mut()
            .expect("Page stack is empty")
            .update(action);
    }

    /// Get the page from Layers enum
    ///
    /// Page is already initialized
    fn get_layer(&self, layer: &Layers) -> Option<Box<dyn Layer>> {
        let mut page = match layer.clone() {
            Layers::Home => Box::new(Home {
                tx: self.state.action_tx.clone().into(),
            }) as Box<dyn Layer>,
            Layers::Transaction => Box::new(Transactions::new(
                self.state.action_tx.clone().into(),
                self.state.manager.clone(),
            )),
            Layers::Fetch => Box::new(
                Fetch::new(
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
                }),
            ),
            Layers::CookieInput => Box::new(CookieInput::new(
                self.state.action_tx.clone().into(),
                self.state.manager.clone(),
                self.state.input_mode,
            )),
            Layers::Help(help_msg) => {
                let help = HelpPopup::new(self.state.action_tx.clone().into(), help_msg.clone());
                match help {
                    Some(help) => Box::new(help) as Box<dyn Layer>,
                    None => {
                        warn!("Help message is empty");
                        return None;
                    }
                }
            }
        };
        page.init();
        Some(page)
    }
}

#[cfg(test)]
mod test {
    use clap::Parser;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use insta::assert_snapshot;

    use crate::{
        cli::{ClapSource, Cli},
        libs::fetcher::MealFetcher,
        tui::Event,
        utils::help_msg::HelpEntry,
    };

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

    fn get_app() -> App {
        let config = get_config(vec!["--use-mock-data"], true);
        let state = RootState::new(config);
        let app = App {
            page: vec![Box::new(Home {
                tx: state.action_tx.clone().into(),
            })],
            state: state,
            tui: tui::TestTui::new().into(),
        };
        app
    }

    #[tokio::test]
    async fn app_navigation() {
        let mut app = get_app();

        // Navigate to Fetch page
        app.event_loop('T'.into()).unwrap();
        assert!(app.page.last().unwrap().is::<Transactions>());

        app.perform_action(Action::Layer(LayerManageAction::SwapPage(Layers::Fetch)));
        assert!(app.page.last().unwrap().is::<Fetch>());

        app.perform_action(Action::Layer(LayerManageAction::SwapPage(
            Layers::CookieInput,
        )));
        assert!(app.page.last().unwrap().is::<CookieInput>());
        app.perform_action(Action::Layer(LayerManageAction::SwapPage(Layers::Help(
            vec![HelpEntry::new('?', "Help")].into(),
        ))));
        assert!(app.page.last().unwrap().is::<HelpPopup>());
    }

    #[tokio::test]
    async fn app_nav_fetch_mock() {
        let mut app = get_app();

        app.perform_action(Action::Layer(LayerManageAction::SwapPage(Layers::Fetch)));
        assert!(app.page.last().unwrap().is::<Fetch>());
        let fetch = app.page.last().unwrap().downcast_ref::<Fetch>().unwrap();
        assert!(matches!(fetch.get_client(), MealFetcher::Mock(_)));
    }

    #[tokio::test]
    async fn app_quit() {
        let mut app = get_app();

        app.perform_action(Action::Quit);
        assert_eq!(app.state.should_quit, true);
    }

    #[tokio::test]
    async fn app_quit_due_to_last_layer_pop() {
        let mut app = get_app();

        app.perform_action(Action::Layer(LayerManageAction::PopPage));
        assert_eq!(app.state.should_quit, true);
    }

    #[tokio::test]
    async fn app_push_layer() {
        let mut app = get_app();

        app.perform_action(Action::Layer(LayerManageAction::PushPage(
            Layers::Transaction,
        )));
        assert_eq!(app.page.len(), 2);
        assert!(app.page.first().unwrap().is::<Home>());
        assert!(app.page.last().unwrap().is::<Transactions>());
        app.perform_action(Action::Layer(LayerManageAction::PopPage));
        assert_eq!(app.page.len(), 1);
        assert!(app.page.first().unwrap().is::<Home>());
    }

    #[tokio::test]
    async fn app_render() {
        let mut app = get_app();

        app.perform_action(Action::Render);
        assert_snapshot!(app.tui.backend());
    }

    #[tokio::test]
    async fn app_stacked_render() {
        let mut app = get_app();

        app.event_loop(Event::Key(KeyEvent::new(
            KeyCode::Char('?'),
            KeyModifiers::NONE,
        )))
        .unwrap();

        app.perform_action(Action::Render);
        assert_snapshot!(app.tui.backend());
    }
}
