use crate::{
    RootState,
    actions::Action,
    tui::Event,
    utils::help_msg::{HelpEntry, HelpMsg},
};

use super::Page;
use color_eyre::eyre::Result;
use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
};

#[derive(Default, Clone, Debug)]
pub struct Transactions {
    transactions: Vec<crate::libs::transactions::Transaction>,
}

impl Transactions {
    fn get_help_msg(&self) -> HelpMsg {
        let help_msg: HelpMsg = vec![
            HelpEntry::new_plain("Move focus: hjkl"),
            HelpEntry::new('f', "Fetch"),
            HelpEntry::new('l', "Load from local cache"),
        ]
        .into();
        help_msg
    }
}

#[derive(Clone, Debug)]
pub enum TransactionAction {
    LoadTransactions,
}

impl From<TransactionAction> for Action {
    fn from(val: TransactionAction) -> Self {
        Action::Transaction(val)
    }
}

impl Page for Transactions {
    fn render(&self, frame: &mut Frame, _app: &RootState) {
        let area = frame.area();

        let vertical = &Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]);
        let rects = vertical.split(area);

        frame.render_widget(
            {
                let transactions_str = self
                    .transactions
                    .iter()
                    .fold(String::new(), |acc, x| acc + &format!("{:?}\n", x));
                Paragraph::new(transactions_str)
                    .block(
                        Block::default()
                            .title("Transactions")
                            .title_alignment(Alignment::Center)
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded),
                    )
                    .style(Style::default().fg(Color::Cyan))
                    .alignment(Alignment::Center)
            },
            rects[0],
        );

        self.get_help_msg().render(frame, rects[1]);
    }
    fn handle_events(&self, app: &RootState, event: Event) -> Result<()> {
        if let Event::Key(key) = event {
            match (key.modifiers, key.code) {
                // navigate to fetch page
                (_, KeyCode::Char('f')) => app.send_action(crate::page::fetch::Fetch::default()),
                (_, KeyCode::Char('l')) => app.send_action(TransactionAction::LoadTransactions),
                _ => (),
            }
        };
        Ok(())
    }

    fn update(&mut self, root_state: &RootState, action: Action) {
        if let Action::Transaction(action) = action {
            match action {
                TransactionAction::LoadTransactions => {
                    self.transactions = root_state.manager.fetch_all().unwrap();
                    root_state.send_action(Action::Render);
                }
            }
        }
    }

    fn get_name(&self) -> String {
        "Transactions".to_string()
    }

    fn init(&mut self, app: &RootState) {
        app.send_action(TransactionAction::LoadTransactions)
    }
}
