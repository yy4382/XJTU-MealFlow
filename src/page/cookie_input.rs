use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Layout};

use crate::actions::Action;
use crate::actions::ActionSender;
use crate::actions::NaviTarget;
use crate::component::Component;
use crate::component::input::{InputComp, InputMode};
use crate::libs::transactions::TransactionManager;
use crate::utils::help_msg::{HelpEntry, HelpMsg};

use super::Page;

#[derive(Clone, Debug)]
pub struct CookieInput {
    state: Focus,
    manager: TransactionManager,

    input_mode: bool,
    tx: ActionSender,

    cookie_input: InputComp,
    account_input: InputComp,
}

impl CookieInput {
    pub fn new(action_tx: ActionSender, manager: TransactionManager, input_mode: bool) -> Self {
        let (account, cookie) = manager.get_account_cookie().unwrap_or_default();
        Self {
            state: Default::default(),
            manager,
            input_mode,
            cookie_input: InputComp::new(1, input_mode, action_tx.clone())
                .init_text(cookie)
                .title("Cookie"),
            account_input: InputComp::new(2, input_mode, action_tx.clone())
                .init_text(account)
                .title("Account"),
            tx: action_tx,
        }
    }

    pub fn get_help_msg(&self) -> crate::utils::help_msg::HelpMsg {
        let help_msg: HelpMsg = vec![
            HelpEntry::new_plain("Move focus: hjkl"),
            HelpEntry::new(KeyCode::Esc, "Back"),
        ]
        .into();
        match self.state {
            Focus::Account => help_msg.extend_ret(&self.account_input.get_help_msg()),
            Focus::Cookie => help_msg.extend_ret(&self.cookie_input.get_help_msg()),
        }
    }
}

#[derive(Default, Clone, Debug)]
pub enum Focus {
    #[default]
    Account,
    Cookie,
}

impl Focus {
    fn next(&self) -> Focus {
        match self {
            Focus::Account => Focus::Cookie,
            Focus::Cookie => Focus::Account,
        }
    }
    fn prev(&self) -> Focus {
        match self {
            Focus::Account => Focus::Cookie,
            Focus::Cookie => Focus::Account,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum CookieInputAction {
    ChangeState(Focus),
}

impl From<CookieInputAction> for Action {
    fn from(val: CookieInputAction) -> Self {
        Action::CookieInput(val)
    }
}

impl Page for CookieInput {
    fn render(&self, frame: &mut ratatui::Frame) {
        let chunks = &Layout::default()
            .constraints([Constraint::Fill(1), Constraint::Length(3)])
            .split(frame.area());

        let sub_chunks = &Layout::default()
            .margin(1)
            .constraints([Constraint::Length(5), Constraint::Length(5)])
            .split(chunks[0]);

        self.account_input.draw(frame, &sub_chunks[0]);
        self.cookie_input.draw(frame, &sub_chunks[1]);

        self.get_help_msg().render(frame, chunks[1]);
    }

    fn handle_events(&self, event: crate::tui::Event) -> color_eyre::eyre::Result<()> {
        if let crate::tui::Event::Key(key) = &event {
            if !self.input_mode {
                match (key.modifiers, key.code) {
                    (_, KeyCode::Char('k')) => self
                        .tx
                        .send(CookieInputAction::ChangeState(self.state.prev())),
                    (_, KeyCode::Char('j')) => self
                        .tx
                        .send(CookieInputAction::ChangeState(self.state.next())),
                    (_, KeyCode::Esc) => self.tx.send(Action::NavigateTo(NaviTarget::Fetch)),
                    _ => (),
                }
            }
        };
        self.account_input.handle_events(&event)?;
        self.cookie_input.handle_events(&event)?;
        Ok(())
    }

    fn update(&mut self, action: crate::actions::Action) {
        if let Action::SwitchInputMode(mode) = &action {
            self.input_mode = *mode;
        }

        if let Action::CookieInput(CookieInputAction::ChangeState(next_state)) = &action {
            self.state = next_state.clone();

            self.tx.send(self.account_input.get_switch_mode_action(
                if matches!(self.state, Focus::Account) {
                    InputMode::Focused
                } else {
                    InputMode::Idle
                },
            ));

            self.tx.send(self.cookie_input.get_switch_mode_action(
                if matches!(self.state, Focus::Cookie) {
                    InputMode::Focused
                } else {
                    InputMode::Idle
                },
            ));
        }

        if let Some(string) = self.account_input.parse_submit_action(&action) {
            self.manager.update_account(&string).unwrap();
        };
        if let Some(string) = self.cookie_input.parse_submit_action(&action) {
            self.manager.update_cookie(&string).unwrap();
        }

        self.account_input.update(&action).unwrap();
        self.cookie_input.update(&action).unwrap();
    }

    fn get_name(&self) -> String {
        "Cookie Input".to_string()
    }
    fn init(&mut self) {
        self.tx.send(
            self.account_input
                .get_switch_mode_action(InputMode::Focused),
        );
    }
}

#[cfg(test)]
mod test {

    use insta::assert_snapshot;
    use ratatui::backend::TestBackend;
    use tokio::sync::mpsc::{self, UnboundedReceiver};

    use super::*;
    use crate::tui::Event;
    use crate::utils::key_events::test_utils::{get_char_evt, get_key_evt};

    fn get_test_objs() -> (UnboundedReceiver<Action>, CookieInput) {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut page = CookieInput::new(
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
        assert!(matches!(page.state, Focus::Account));
        assert!(matches!(page.account_input.get_mode(), InputMode::Focused));

        page.event_loop_once(&mut rx, get_key_evt(KeyCode::Char('j')));
        assert!(matches!(page.state, Focus::Cookie));

        page.event_loop_once(&mut rx, get_key_evt(KeyCode::Char('j')));
        assert!(matches!(page.state, Focus::Account));

        page.event_loop_once(&mut rx, get_key_evt(KeyCode::Char('k')));
        assert!(matches!(page.state, Focus::Cookie));

        page.event_loop_once(&mut rx, get_key_evt(KeyCode::Char('k')));
        assert!(matches!(page.state, Focus::Account));
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
    fn test_account_input() {
        let (mut rx, mut page) = get_test_objs();

        page.event_loop_once(&mut rx, get_key_evt(KeyCode::Enter));
        assert!(page.input_mode);
        page.event_loop_once(&mut rx, get_char_evt('a'));
        page.event_loop_once(&mut rx, get_char_evt('j'));
        page.event_loop_once(&mut rx, get_key_evt(KeyCode::Enter));
        assert_eq!(page.manager.get_account_cookie_may_empty().unwrap().0, "aj");

        page.event_loop_once(&mut rx, get_key_evt(KeyCode::Enter));
        page.event_loop_once(&mut rx, get_key_evt(KeyCode::Left));
        page.event_loop_once(&mut rx, Event::Paste("kl".into()));
        page.event_loop_once(&mut rx, get_key_evt(KeyCode::Enter));
        assert_eq!(
            page.manager.get_account_cookie_may_empty().unwrap().0,
            "aklj"
        );
    }

    #[test]
    fn test_cookie_input() {
        let (mut rx, mut page) = get_test_objs();

        page.event_loop_once(&mut rx, get_char_evt('j'));
        page.event_loop_once(&mut rx, get_key_evt(KeyCode::Enter));
        assert!(page.input_mode);
        page.event_loop_once(&mut rx, get_char_evt('a'));
        page.event_loop_once(&mut rx, get_char_evt('j'));
        page.event_loop_once(&mut rx, get_key_evt(KeyCode::Enter));
        page.event_loop_once(&mut rx, get_key_evt(KeyCode::Esc));
        assert_eq!(page.manager.get_account_cookie_may_empty().unwrap().1, "aj");
    }

    #[test]
    fn test_cookie_input_render() {
        let (_, page) = get_test_objs();
        let mut terminal = ratatui::Terminal::new(TestBackend::new(80, 20)).unwrap();

        terminal
            .draw(|f| {
                page.render(f);
            })
            .unwrap();

        assert_snapshot!(terminal.backend())
    }
}
