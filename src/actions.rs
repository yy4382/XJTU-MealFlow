use crate::{
    component::input::InputAction,
    page::{
        cookie_input::{CookieInput, CookieInputAction},
        fetch::{Fetch, FetchingAction},
        home::Home,
        transactions::{TransactionAction, Transactions},
    },
};

#[derive(Clone)]
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
    None,
}
#[derive(Clone)]
pub enum NaviTarget {
    Home(Home),
    Fetch(Fetch),
    Transaction(Transactions),
    CookieInput(CookieInput),
}

#[derive(Clone)]
pub enum CompAction {
    Input(InputAction),
    #[allow(dead_code)]
    Placeholder,
}
