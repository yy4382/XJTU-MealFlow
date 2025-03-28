use color_eyre::eyre::Context;

use crate::{
    component::input::InputAction,
    page::{
        cookie_input::CookieInputAction, fetch::FetchingAction, transactions::TransactionAction,
    },
};

#[derive(Clone, Debug)]
pub enum Action {
    Tick,
    NavigateTo(NaviTarget),
    SwitchInputMode(bool),

    Transaction(TransactionAction),
    Fetching(FetchingAction),
    CookieInput(CookieInputAction),

    Comp((CompAction, u64)),

    Quit,
    Render,

    #[cfg(test)]
    TestPage(crate::component::input::test::TestInputPageAction),
}
#[derive(Clone, Debug)]
pub enum NaviTarget {
    Home,
    Fetch,
    Transaction,
    CookieInput,
}

#[derive(Clone, Debug)]
pub enum CompAction {
    Input(InputAction),
    #[allow(dead_code)]
    Placeholder,
}

#[derive(Clone, Debug)]
pub struct ActionSender(pub tokio::sync::mpsc::UnboundedSender<Action>);

impl ActionSender {
    pub fn send<T: Into<Action>>(&self, action: T) {
        self.0.send(action.into()).with_context(||"Action Receiver is dropped or closed, which should not happen if app is still running.").unwrap();
    }
}
impl From<tokio::sync::mpsc::UnboundedSender<Action>> for ActionSender {
    fn from(value: tokio::sync::mpsc::UnboundedSender<Action>) -> Self {
        ActionSender(value)
    }
}
