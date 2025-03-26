use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Layout};

use crate::component::Component;
use crate::component::input::{InputComp, InputMode};
use crate::{actions::Action, app::RootState};

use super::Page;

#[derive(Clone, Debug)]
pub struct CookieInput {
    state: CookieInputState,

    cookie_input: InputComp,
    account_input: InputComp,
}

impl CookieInput {
    pub fn new(app: &RootState) -> Self {
        let mut comp_ids = vec![rand::random::<u64>()];
        loop {
            let rand2 = rand::random::<u64>();
            if !comp_ids.contains(&rand2) {
                comp_ids.push(rand2);
                break;
            }
        }

        let (account, cookie) = app.manager.get_account_cookie().unwrap_or_default();

        Self {
            state: Default::default(),
            cookie_input: InputComp::new(comp_ids[0], Some(cookie), "Cookie", Default::default()),
            account_input: InputComp::new(
                comp_ids[1],
                Some(account),
                "Account",
                Default::default(),
            ),
        }
    }
}

#[derive(Default, Clone, Debug)]
pub(crate) enum CookieInputState {
    #[default]
    Account,
    Cookie,
}

impl CookieInputState {
    fn next(&self) -> CookieInputState {
        match self {
            CookieInputState::Account => CookieInputState::Cookie,
            CookieInputState::Cookie => CookieInputState::Account,
        }
    }
    fn prev(&self) -> CookieInputState {
        match self {
            CookieInputState::Account => CookieInputState::Cookie,
            CookieInputState::Cookie => CookieInputState::Account,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum CookieInputAction {
    ChangeState(CookieInputState),
}

impl Into<Action> for CookieInputAction {
    fn into(self) -> Action {
        Action::CookieInput(self)
    }
}

impl Page for CookieInput {
    fn render(&self, frame: &mut ratatui::Frame, app: &crate::app::RootState) {
        // TODO add keybindings guide
        let chunks = &Layout::default()
            .margin(1)
            .constraints([Constraint::Length(5), Constraint::Length(5)])
            .split(frame.area());

        self.account_input.draw(frame, &chunks[0], app);
        self.cookie_input.draw(frame, &chunks[1], app);
    }

    fn handle_events(
        &self,
        app: &RootState,
        event: crate::tui::Event,
    ) -> color_eyre::eyre::Result<()> {
        match &event {
            crate::tui::Event::Key(key) => {
                if !app.input_mode() {
                    match (key.modifiers, key.code) {
                        (_, KeyCode::Char('k')) => {
                            app.send_action(CookieInputAction::ChangeState(self.state.prev()))
                        }
                        (_, KeyCode::Char('j')) => {
                            app.send_action(CookieInputAction::ChangeState(self.state.next()))
                        }
                        (_, KeyCode::Esc) => app.send_action(crate::page::fetch::Fetch::default()),
                        _ => (),
                    }
                }
            }
            _ => (),
        };
        self.account_input.handle_events(&event, app)?;
        self.cookie_input.handle_events(&event, app)?;
        Ok(())
    }

    fn update(&mut self, app: &crate::app::RootState, action: crate::actions::Action) {
        match &action {
            Action::CookieInput(CookieInputAction::ChangeState(next_state)) => {
                self.state = next_state.clone();

                app.send_action(self.account_input.get_switch_mode_action(
                    if matches!(self.state, CookieInputState::Account) {
                        InputMode::Focused
                    } else {
                        InputMode::Idle
                    },
                ));

                app.send_action(self.cookie_input.get_switch_mode_action(
                    if matches!(self.state, CookieInputState::Cookie) {
                        InputMode::Focused
                    } else {
                        InputMode::Idle
                    },
                ));
            }
            _ => {}
        }

        if let Some(string) = self.account_input.parse_submit_action(&action) {
            app.manager.update_account(&string).unwrap();
        };
        if let Some(string) = self.cookie_input.parse_submit_action(&action) {
            app.manager.update_cookie(&string).unwrap();
        }

        self.account_input.update(&action, app).unwrap();
        self.cookie_input.update(&action, app).unwrap();
    }

    fn get_name(&self) -> String {
        "Cookie Input".to_string()
    }

    fn init(&mut self, app: &crate::app::RootState) {
        app.send_action(
            self.account_input
                .get_switch_mode_action(InputMode::Focused),
        );
    }
}

#[cfg(test)]
mod test {
    use crossterm::event::{KeyEvent, KeyModifiers};

    use crate::tui::Event;

    use super::*;
    fn get_test_objs() -> (RootState, CookieInput) {
        let mut app = RootState::new(None);
        app.manager.init_db().unwrap();
        let mut page = CookieInput::new(&app);
        page.init(&app);
        while let Ok(action) = app.try_recv() {
            page.update(&app, action);
        }
        (app, page)
    }

    fn get_key_evt(key: KeyCode) -> Event {
        Event::Key(KeyEvent::new(key, KeyModifiers::NONE))
    }
    fn get_char_evt(key: char) -> Event {
        Event::Key(KeyEvent::new(KeyCode::Char(key), KeyModifiers::NONE))
    }

    fn handle_event_and_update(app: &mut RootState, page: &mut CookieInput, evt: Event) {
        page.handle_events(&app, evt).unwrap();
        while let Ok(action) = app.try_recv() {
            app.update(&action).unwrap();
            page.update(&app, action);
        }
    }

    #[test]
    fn test_navigation() {
        let (mut app, mut page) = get_test_objs();
        assert!(matches!(page.state, CookieInputState::Account));
        assert!(matches!(page.account_input.get_mode(), InputMode::Focused));

        handle_event_and_update(&mut app, &mut page, get_key_evt(KeyCode::Char('j')));
        assert!(matches!(page.state, CookieInputState::Cookie));

        handle_event_and_update(&mut app, &mut page, get_key_evt(KeyCode::Char('j')));
        assert!(matches!(page.state, CookieInputState::Account));

        handle_event_and_update(&mut app, &mut page, get_key_evt(KeyCode::Char('k')));
        assert!(matches!(page.state, CookieInputState::Cookie));

        handle_event_and_update(&mut app, &mut page, get_key_evt(KeyCode::Char('k')));
        assert!(matches!(page.state, CookieInputState::Account));
    }

    #[test]
    fn test_account_input() {
        let (mut app, mut page) = get_test_objs();

        handle_event_and_update(&mut app, &mut page, get_key_evt(KeyCode::Enter));
        assert!(app.input_mode());
        handle_event_and_update(&mut app, &mut page, get_char_evt('a'));
        handle_event_and_update(&mut app, &mut page, get_char_evt('j'));
        handle_event_and_update(&mut app, &mut page, get_key_evt(KeyCode::Enter));
        assert_eq!(app.manager.get_account_cookie().unwrap().0, "aj");

        handle_event_and_update(&mut app, &mut page, get_key_evt(KeyCode::Enter));
        handle_event_and_update(&mut app, &mut page, get_key_evt(KeyCode::Left));
        handle_event_and_update(&mut app, &mut page, Event::Paste("kl".into()));
        handle_event_and_update(&mut app, &mut page, get_key_evt(KeyCode::Enter));
        assert_eq!(app.manager.get_account_cookie().unwrap().0, "aklj");
    }

    #[test]
    fn test_cookie_input() {
        let (mut app, mut page) = get_test_objs();

        handle_event_and_update(&mut app, &mut page, get_char_evt('j'));
        handle_event_and_update(&mut app, &mut page, get_key_evt(KeyCode::Enter));
        assert!(app.input_mode());
        handle_event_and_update(&mut app, &mut page, get_char_evt('a'));
        handle_event_and_update(&mut app, &mut page, get_char_evt('j'));
        handle_event_and_update(&mut app, &mut page, get_key_evt(KeyCode::Enter));
        assert_eq!(app.manager.get_account_cookie().unwrap().1, "aj");
    }
}
