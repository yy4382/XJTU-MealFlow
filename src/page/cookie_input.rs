use crossterm::event::KeyCode;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::layout::{Constraint, Layout};

use crate::actions::ActionSender;
use crate::actions::LayerManageAction;
use crate::actions::Layers;
use crate::app::layer_manager::EventHandlingStatus;
use crate::component::input::{InputComp, InputMode};
use crate::libs::transactions::TransactionManager;
use crate::utils::help_msg::{HelpEntry, HelpMsg};

use super::{EventLoopParticipant, Layer, WidgetExt};

#[derive(Clone, Debug)]
pub struct CookieInput {
    state: Focus,
    manager: TransactionManager,

    tx: ActionSender,

    cookie_input: InputComp,
    account_input: InputComp,
}

impl CookieInput {
    pub fn new(action_tx: ActionSender, manager: TransactionManager) -> Self {
        let (account, cookie) = manager.get_account_cookie_may_empty().unwrap_or_default();
        Self {
            state: Default::default(),
            manager,
            cookie_input: InputComp::new().init_text(cookie).title("Cookie"),
            account_input: InputComp::new().init_text(account).title("Account"),
            tx: action_tx,
        }
    }

    pub fn get_help_msg(&self) -> crate::utils::help_msg::HelpMsg {
        let help_msg: HelpMsg = vec![
            HelpEntry::new_plain("hjkl", "Move focus"),
            HelpEntry::new('?', "Help"),
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

impl WidgetExt for CookieInput {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = &Layout::default()
            .constraints([Constraint::Fill(1), Constraint::Length(3)])
            .split(area);

        let sub_chunks = &Layout::default()
            .margin(1)
            .constraints([Constraint::Length(5), Constraint::Length(5)])
            .split(chunks[0]);

        self.account_input.render(frame, sub_chunks[0]);
        self.cookie_input.render(frame, sub_chunks[1]);

        self.get_help_msg().render(frame, chunks[1]);
    }
}

impl EventLoopParticipant for CookieInput {
    fn handle_events(&mut self, event: &crate::tui::Event) -> EventHandlingStatus {
        let mut status = EventHandlingStatus::default();

        let (account_state, account_result) = self.account_input.handle_events(event);
        if let Some(result) = account_result {
            self.manager.update_account(&result).unwrap();
        }
        if matches!(account_state, EventHandlingStatus::Consumed) {
            return account_state;
        }

        let (cookie_state, cookie_result) = self.cookie_input.handle_events(event);
        if let Some(result) = cookie_result {
            self.manager.update_cookie(&result).unwrap();
        }
        if matches!(cookie_state, EventHandlingStatus::Consumed) {
            return cookie_state;
        }

        if let crate::tui::Event::Key(key) = &event {
            match (key.modifiers, key.code) {
                (_, KeyCode::Char('k')) => {
                    self.change_focus(self.state.prev());
                    status.consumed();
                }
                (_, KeyCode::Char('j')) => {
                    self.change_focus(self.state.next());
                    status.consumed();
                }
                (_, KeyCode::Esc) => self.tx.send(LayerManageAction::Swap(Layers::Fetch)),
                (_, KeyCode::Char('?')) => {
                    self.tx.send(LayerManageAction::Push(
                        Layers::Help(self.get_help_msg()).into_push_config(true),
                    ));
                }
                _ => (),
            }
        };
        status
    }
}

impl Layer for CookieInput {
    fn init(&mut self) {
        self.account_input.set_mode(InputMode::Focused);
    }
}

impl CookieInput {
    fn change_focus(&mut self, focus: Focus) {
        self.state = focus.clone();

        self.account_input
            .set_mode(if matches!(self.state, Focus::Account) {
                InputMode::Focused
            } else {
                InputMode::Idle
            });

        self.cookie_input
            .set_mode(if matches!(self.state, Focus::Cookie) {
                InputMode::Focused
            } else {
                InputMode::Idle
            });
    }
}

#[cfg(test)]
mod test {

    use insta::assert_snapshot;
    use ratatui::backend::TestBackend;
    use tokio::sync::mpsc::{self, UnboundedReceiver};

    use super::*;
    use crate::{actions::Action, tui::Event};

    fn get_test_objs() -> (UnboundedReceiver<Action>, CookieInput) {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut page = CookieInput::new(tx.clone().into(), TransactionManager::new(None).unwrap());
        page.init();
        (rx, page)
    }

    #[test]
    fn test_navigation() {
        let (_, mut page) = get_test_objs();
        assert!(matches!(page.state, Focus::Account));
        assert!(matches!(page.account_input.mode, InputMode::Focused));

        page.handle_event_with_status_check(&'j'.into());
        assert!(matches!(page.state, Focus::Cookie));

        page.handle_event_with_status_check(&'j'.into());
        assert!(matches!(page.state, Focus::Account));

        page.handle_event_with_status_check(&'k'.into());
        assert!(matches!(page.state, Focus::Cookie));

        page.handle_event_with_status_check(&'k'.into());
        assert!(matches!(page.state, Focus::Account));
    }

    #[test]
    fn test_account_input() {
        let (_, mut page) = get_test_objs();

        page.handle_event_with_status_check(&KeyCode::Enter.into());
        assert!(page.account_input.is_inputting());
        page.handle_event_with_status_check(&'a'.into());
        page.handle_event_with_status_check(&'j'.into());
        page.handle_event_with_status_check(&KeyCode::Enter.into());
        assert_eq!(page.manager.get_account_cookie_may_empty().unwrap().0, "aj");

        page.handle_event_with_status_check(&KeyCode::Enter.into());
        page.handle_event_with_status_check(&KeyCode::Left.into());
        page.handle_event_with_status_check(&Event::Paste("kl".into()));
        page.handle_event_with_status_check(&KeyCode::Enter.into());
        assert_eq!(
            page.manager.get_account_cookie_may_empty().unwrap().0,
            // cSpell:ignore aklj
            "aklj"
        );
    }

    #[test]
    fn test_cookie_input() {
        let (_, mut page) = get_test_objs();

        page.handle_event_with_status_check(&'j'.into());
        page.handle_event_with_status_check(&KeyCode::Enter.into());
        assert!(page.cookie_input.is_inputting());
        page.handle_event_with_status_check(&'a'.into());
        page.handle_event_with_status_check(&'j'.into());
        page.handle_event_with_status_check(&KeyCode::Enter.into());
        assert_eq!(page.manager.get_account_cookie_may_empty().unwrap().1, "aj");
    }

    #[test]
    fn test_cookie_input_render() {
        let (_, mut page) = get_test_objs();
        let mut terminal = ratatui::Terminal::new(TestBackend::new(80, 20)).unwrap();

        terminal
            .draw(|f| {
                page.render(f, f.area());
            })
            .unwrap();

        assert_snapshot!(terminal.backend())
    }
}
