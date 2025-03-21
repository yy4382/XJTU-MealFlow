use crate::page::{
    fetch::{Fetch, FetchingAction},
    home::Home,
    transactions::{TransactionAction, Transactions},
};

#[derive(Clone)]
pub enum Action {
    Tick,
    NavigateTo(NavigateTarget),

    Transaction(TransactionAction),
    Fetching(FetchingAction),

    Quit,
    Render,
    None,
}

#[derive(Clone)]
pub enum NavigateTarget {
    Transaction(Transactions),
    Fetch(Fetch),
    Home(Home),
}
