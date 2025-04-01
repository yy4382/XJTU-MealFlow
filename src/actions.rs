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

impl std::fmt::Display for Layers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Layers::Home => write!(f, "Home"),
            Layers::Fetch => write!(f, "Fetch"),
            Layers::Transaction(_) => write!(f, "Transaction"),
            Layers::CookieInput => write!(f, "CookieInput"),
            Layers::Help(_) => write!(f, "Help"),
        }
    }
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
    PushPage(PushPageConfig),
    SwapPage(Layers),
    PopPage,
}

#[derive(Clone, Debug)]
pub struct PushPageConfig {
    pub layer: Layers,
    /// Whether to render current (not pushed) page when it is no longer to page on top of the stack
    ///
    /// For example, if the pushed page is a help popup, we want to render the current page, so it should be `true`.
    /// If the new page is a full-screen page, we don't want to render the current page (to reduce performance cost), so it should be `false`.
    pub render_self: bool,
}

impl Layers {
    pub fn into_push_config(self, render_self: bool) -> PushPageConfig {
        PushPageConfig {
            layer: self,
            render_self,
        }
    }
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
