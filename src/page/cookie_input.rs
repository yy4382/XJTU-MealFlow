use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Layout};

use crate::component::Component;
use crate::component::input::{InputComp, InputMode};
use crate::utils::help_msg::{HelpEntry, HelpMsg};
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
        let (account, cookie) = app.manager.get_account_cookie().unwrap_or_default();

        Self {
            state: Default::default(),
            cookie_input: InputComp::new(1, Some(cookie), "Cookie", Default::default()),
            account_input: InputComp::new(2, Some(account), "Account", Default::default()),
        }
    }

    pub fn get_help_msg(&self, app: &RootState) -> crate::utils::help_msg::HelpMsg {
        let help_msg: HelpMsg = vec![
            HelpEntry::new_plain("Move focus: hjkl"),
            HelpEntry::new(KeyCode::Esc, "Back"),
        ]
        .into();
        match self.state {
            CookieInputState::Account => {
                help_msg.extend_ret(&self.account_input.get_help_msg(app.input_mode()))
            }
            CookieInputState::Cookie => {
                help_msg.extend_ret(&self.cookie_input.get_help_msg(app.input_mode()))
            }
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

impl From<CookieInputAction> for Action {
    fn from(val: CookieInputAction) -> Self {
        Action::CookieInput(val)
    }
}

impl Page for CookieInput {
    fn render(&self, frame: &mut ratatui::Frame, app: &crate::app::RootState) {
        let chunks = &Layout::default()
            .constraints([Constraint::Fill(1), Constraint::Length(3)])
            .split(frame.area());

        let sub_chunks = &Layout::default()
            .margin(1)
            .constraints([Constraint::Length(5), Constraint::Length(5)])
            .split(chunks[0]);

        self.account_input.draw(frame, &sub_chunks[0], app);
        self.cookie_input.draw(frame, &sub_chunks[1], app);

        self.get_help_msg(app).render(frame, chunks[1]);
    }

    fn handle_events(
        &self,
        app: &RootState,
        event: crate::tui::Event,
    ) -> color_eyre::eyre::Result<()> {
        if let crate::tui::Event::Key(key) = &event {
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
        };
        self.account_input.handle_events(&event, app)?;
        self.cookie_input.handle_events(&event, app)?;
        Ok(())
    }

    fn update(&mut self, app: &crate::app::RootState, action: crate::actions::Action) {
        if let Action::CookieInput(CookieInputAction::ChangeState(next_state)) = &action {
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
    use super::*;
    use crate::app::RootState;
    use crate::tui::Event;
    use crate::utils::key_events::test_utils::{get_char_evt, get_key_evt};

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

    #[test]
    fn test_navigation() {
        let (mut app, mut page) = get_test_objs();
        assert!(matches!(page.state, CookieInputState::Account));
        assert!(matches!(page.account_input.get_mode(), InputMode::Focused));

        app.handle_event_and_update(&mut page, get_key_evt(KeyCode::Char('j')));
        assert!(matches!(page.state, CookieInputState::Cookie));

        app.handle_event_and_update(&mut page, get_key_evt(KeyCode::Char('j')));
        assert!(matches!(page.state, CookieInputState::Account));

        app.handle_event_and_update(&mut page, get_key_evt(KeyCode::Char('k')));
        assert!(matches!(page.state, CookieInputState::Cookie));

        app.handle_event_and_update(&mut page, get_key_evt(KeyCode::Char('k')));
        assert!(matches!(page.state, CookieInputState::Account));
    }

    #[test]
    fn test_account_input() {
        let (mut app, mut page) = get_test_objs();

        app.handle_event_and_update(&mut page, get_key_evt(KeyCode::Enter));
        assert!(app.input_mode());
        app.handle_event_and_update(&mut page, get_char_evt('a'));
        app.handle_event_and_update(&mut page, get_char_evt('j'));
        app.handle_event_and_update(&mut page, get_key_evt(KeyCode::Enter));
        assert_eq!(app.manager.get_account_cookie().unwrap().0, "aj");

        app.handle_event_and_update(&mut page, get_key_evt(KeyCode::Enter));
        app.handle_event_and_update(&mut page, get_key_evt(KeyCode::Left));
        app.handle_event_and_update(&mut page, Event::Paste("kl".into()));
        app.handle_event_and_update(&mut page, get_key_evt(KeyCode::Enter));
        assert_eq!(app.manager.get_account_cookie().unwrap().0, "aklj");
    }

    #[test]
    fn test_cookie_input() {
        let (mut app, mut page) = get_test_objs();

        app.handle_event_and_update(&mut page, get_char_evt('j'));
        app.handle_event_and_update(&mut page, get_key_evt(KeyCode::Enter));
        assert!(app.input_mode());
        app.handle_event_and_update(&mut page, get_char_evt('a'));
        app.handle_event_and_update(&mut page, get_char_evt('j'));
        app.handle_event_and_update(&mut page, get_key_evt(KeyCode::Enter));
        app.handle_event_and_update(&mut page, get_key_evt(KeyCode::Esc));
        assert_eq!(app.manager.get_account_cookie().unwrap().1, "aj");
    }
}
