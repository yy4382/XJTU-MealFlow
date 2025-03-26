use crate::{
    component::input::InputAction,
    page::{
        cookie_input::{CookieInput, CookieInputAction},
        fetch::{Fetch, FetchingAction},
        home::Home,
        transactions::{TransactionAction, Transactions},
    },
};

#[derive(Clone, Debug)]
pub enum Action {
    Tick,
    NavigateTo(Box<NaviTarget>),
    SwitchInputMode(bool),

    Transaction(TransactionAction),
    Fetching(FetchingAction),
    CookieInput(CookieInputAction),

    Comp((CompAction, u64)),

    Quit,
    Render,
}
#[derive(Clone, Debug)]
pub enum NaviTarget {
    Home(Home),
    Fetch(Fetch),
    Transaction(Transactions),
    CookieInput(CookieInput),
}

#[derive(Clone, Debug)]
pub enum CompAction {
    Input(InputAction),
    #[allow(dead_code)]
    Placeholder,
}

impl From<Home> for Action {
    fn from(value: Home) -> Self {
        Action::NavigateTo(Box::new(NaviTarget::Home(value)))
    }
}

impl From<Fetch> for Action {
    fn from(value: Fetch) -> Self {
        Action::NavigateTo(Box::new(NaviTarget::Fetch(value)))
    }
}

impl From<Transactions> for Action {
    fn from(value: Transactions) -> Self {
        Action::NavigateTo(Box::new(NaviTarget::Transaction(value)))
    }
}

impl From<CookieInput> for Action {
    fn from(value: CookieInput) -> Self {
        Action::NavigateTo(Box::new(NaviTarget::CookieInput(value)))
    }
}
