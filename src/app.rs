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
//!   is controlled by frame rate parameter
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

use crate::{
    actions::Action,
    config::Config,
    libs::transactions::TransactionManager,
    page::home::Home,
    tui::{self, TuiEnum},
};
use color_eyre::eyre::{Context, Result};
use crossterm::event::KeyCode::Char;
use layer_manager::LayerManager;
use tokio::sync::mpsc;
use tracing::warn;

mod layer_manager;

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
            manager.update_account(account).unwrap();
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
    layer_manager: LayerManager,
    state: RootState,
    tui: tui::TuiEnum,
}

impl App {
    pub fn new(state: RootState, tui: TuiEnum) -> Self {
        Self {
            layer_manager: LayerManager::new(Box::new(Home {
                tx: state.action_tx.clone().into(),
            })),
            state,
            tui,
        }
    }
}

impl App {
    #[cfg(not(tarpaulin_include))]
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
        match event {
            tui::Event::Tick => self.send_action(Action::Tick),
            tui::Event::Render => self.send_action(Action::Render),

            // TODO impl these events
            tui::Event::Error => self.send_action(Action::Quit),
            tui::Event::FocusGained => (),
            tui::Event::FocusLost => (),
            tui::Event::Init => (),
            tui::Event::Resize(_, _) => self.send_action(Action::Render),

            tui::Event::Key(key) => match key.code {
                Char('q') => self.send_action(Action::Quit),
                _ => self.layer_manager.handle_event(event).unwrap(),
            },

            _ => self.layer_manager.handle_event(event).unwrap(),
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
            Action::Render => self.tui.draw(|f| self.layer_manager.render(f)).unwrap(),
            Action::Layer(layer_action) => self
                .layer_manager
                .handle_layer_action(layer_action, &self.state),
            _ => {}
        }
        self.state.update(&action).unwrap();
        self.layer_manager.handle_action(action);
    }
}

#[cfg(test)]
pub(super) mod test {
    use clap::Parser;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use insta::assert_snapshot;

    use crate::{
        actions::{LayerManageAction, Layers},
        cli::{ClapSource, Cli},
        libs::fetcher::MealFetcher,
        page::{
            cookie_input::CookieInput, fetch::Fetch, help_popup::HelpPopup,
            transactions::Transactions,
        },
        tui::Event,
        utils::help_msg::HelpEntry,
    };

    use super::*;

    pub fn get_config(mut args: Vec<&str>, append_to_default: bool) -> Config {
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

    pub fn get_app() -> App {
        let config = get_config(vec!["--use-mock-data"], true);
        let state = RootState::new(config);
        let app = App::new(state, tui::TestTui::new().into());
        app
    }

    #[tokio::test]
    async fn app_navigation() {
        let mut app = get_app();

        // Navigate to Fetch page
        app.event_loop('T'.into()).unwrap();
        assert!(app.layer_manager.last().unwrap().is::<Transactions>());

        app.perform_action(Action::Layer(LayerManageAction::Swap(Layers::Fetch)));
        assert!(app.layer_manager.last().unwrap().is::<Fetch>());

        app.perform_action(Action::Layer(LayerManageAction::Swap(Layers::CookieInput)));
        assert!(app.layer_manager.last().unwrap().is::<CookieInput>());
        app.perform_action(Action::Layer(LayerManageAction::Swap(Layers::Help(
            vec![HelpEntry::new('?', "Help")].into(),
        ))));
        assert!(app.layer_manager.last().unwrap().is::<HelpPopup>());
    }

    #[tokio::test]
    async fn app_nav_fetch_mock() {
        let mut app = get_app();

        app.perform_action(Action::Layer(LayerManageAction::Swap(Layers::Fetch)));
        assert!(app.layer_manager.last().unwrap().is::<Fetch>());
        let fetch = app
            .layer_manager
            .last()
            .unwrap()
            .downcast_ref::<Fetch>()
            .unwrap();
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

        app.perform_action(Action::Layer(LayerManageAction::Pop));
        assert!(app.layer_manager.last().unwrap().is::<Home>());
    }

    #[tokio::test]
    async fn app_push_layer() {
        let mut app = get_app();

        app.perform_action(Action::Layer(LayerManageAction::Push(
            Layers::Transaction(None).into_push_config(false),
        )));
        assert_eq!(app.layer_manager.len(), 2);
        assert!(app.layer_manager.first().unwrap().is::<Home>());
        assert!(app.layer_manager.last().unwrap().is::<Transactions>());
        app.perform_action(Action::Layer(LayerManageAction::Pop));
        assert_eq!(app.layer_manager.len(), 1);
        assert!(app.layer_manager.first().unwrap().is::<Home>());
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
