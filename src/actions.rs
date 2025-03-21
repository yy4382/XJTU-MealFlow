use crate::page::{Page, fetch::FetchingAction, transactions::TransactionAction};

pub enum Action {
    Tick,
    NavigateTo(Box<dyn Page>),
    SwitchInputMode(bool),

    Transaction(TransactionAction),
    Fetching(FetchingAction),

    Quit,
    Render,
    None,
}
