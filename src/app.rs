use crate::config::Config;
use crate::{
    actions::Action,
    page::{self, Page},
    tui,
};
use color_eyre::eyre::Result;
use crossterm::event::KeyCode::{self, Char};

pub struct RootState {
    pub should_quit: bool,
    pub action_tx: tokio::sync::mpsc::UnboundedSender<Action>,
    pub action_rx: tokio::sync::mpsc::UnboundedReceiver<Action>,
    pub manager: crate::libs::transactions::TransactionManager,
    pub input_mode: bool,

    pub config: Config,
}
pub struct App {
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

            self.state.action_tx.send(self.event2action(e))?;

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

    /// Convert a [`tui::Event`] to an Action
    ///
    /// This function is responsible for converting a [`tui::Event`] to an Action.
    ///
    /// It handles some application-wide events like quitting the application
    /// and switching between input modes,
    /// and delegates the handling of page-specific events (remaining events, currently only key events)
    /// to the current page.
    fn event2action(&self, event: tui::Event) -> Action {
        match event {
            tui::Event::Tick => Action::Tick,
            tui::Event::Render => Action::Render,

            // TODO impl these events
            tui::Event::Error => Action::Quit,
            tui::Event::FocusGained => Action::None,
            tui::Event::FocusLost => Action::None,
            tui::Event::Init => Action::None,
            // tui::Event::Closed => action_tx.send(Action::Quit)?,
            // tui::Event::Quit => action_tx.send(Action::Quit)?,
            tui::Event::Paste(_) => Action::None,
            tui::Event::Resize(_, _) => Action::Render,
            tui::Event::Mouse(_) => Action::None,

            tui::Event::Key(key) => {
                // if a input widget is focused, handle the key event in the input widget
                if self.state.input_mode {
                    match key.code {
                        KeyCode::Esc => Action::SwitchInputMode(false),
                        _ => self.page.handle_input_mode_events(key).unwrap(),
                    }
                } else {
                    match key.code {
                        Char('H') => {
                            // check if the current page is not Home
                            if self.page.get_name() != "Home" {
                                Action::NavigateTo(Box::new(page::home::Home::default()))
                            } else {
                                Action::None
                            }
                        }
                        Char('T') => {
                            // check if the current page is not Transactions
                            if self.page.get_name() != "Transactions" {
                                Action::NavigateTo(Box::new(
                                    page::transactions::Transactions::default(),
                                ))
                            } else {
                                Action::None
                            }
                        }
                        Char('q') => Action::Quit,
                        _ => self.page.handle_events(Some(event)).unwrap(),
                    }
                }
            }
        }
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
        match action {
            Action::Quit => {
                self.state.should_quit = true;
            }
            Action::None => {}
            Action::Render => {
                self.tui
                    .draw(|f| {
                        self.page.render(f, &self.state);
                    })
                    .unwrap();
            }
            Action::NavigateTo(target) => {
                self.page = target;
                self.page.init(&mut self.state);
            }
            Action::SwitchInputMode(mode) => {
                self.state.input_mode = mode;
            }
            _ => {
                self.page.update(&mut self.state, action);
            }
        }
    }
}
