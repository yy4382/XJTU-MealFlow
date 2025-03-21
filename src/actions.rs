#[derive(Clone)]
pub enum Action {
    Tick,

    Transaction(TransactionAction),

    Quit,
    Render,
    None,
}

#[derive(Clone)]
pub enum TransactionAction {
    FetchTransactions,
    UpdateFetchStatus(FetchingState),
    InsertTransaction(Vec<crate::transactions::Transaction>),
    LoadTransactions,
}

#[derive(Clone, Default)]
pub enum FetchingState {
    #[default]
    Idle,
    Fetching(String),
}
