use color_eyre::eyre::Context;

use crate::{
    component::input::InputAction,
    libs::transactions::FilterOptions,
    page::{
        cookie_input::CookieInputAction, fetch::FetchingAction, help_popup::HelpPopupAction,
        transactions::TransactionAction,
    },
    utils::help_msg::HelpMsg,
};

#[derive(Clone, Debug)]
pub enum Action {
    Tick,
    Layer(LayerManageAction),
    SwitchInputMode(bool),

    Transaction(TransactionAction),
    Fetching(FetchingAction),
    CookieInput(CookieInputAction),
    HelpPopup(HelpPopupAction),

    Comp((CompAction, u64)),

    Quit,
    Render,

    #[cfg(test)]
    TestPage(crate::component::input::test::TestInputPageAction),
}
#[derive(Clone, Debug)]
pub enum Layers {
    #[allow(dead_code)]
    Home,
    Fetch,
    Transaction(Option<FilterOptions>),
    CookieInput,
    Help(HelpMsg),
}

#[derive(Clone, Debug)]
pub enum CompAction {
    Input(InputAction),
    #[allow(dead_code)]
    Placeholder,
}

#[derive(Clone, Debug)]
/// These actions should only be sent by page at the top of the stack
/// and should only be handled by the root app.
pub enum LayerManageAction {
    PushPage(Layers),
    SwapPage(Layers),
    PopPage,
}

impl From<LayerManageAction> for Action {
    fn from(value: LayerManageAction) -> Self {
        Action::Layer(value)
    }
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
