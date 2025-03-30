use std::cmp::max;

use crate::{
    actions::{Action, ActionSender, NaviTarget},
    libs::transactions::{Transaction, TransactionManager},
    tui::Event,
    utils::help_msg::{HelpEntry, HelpMsg},
};

use super::{EventLoopParticipant, Page, WidgetExt};
use color_eyre::eyre::Result;
use crossterm::event::KeyCode;
use lazy_static::lazy_static;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style, Stylize, palette::tailwind},
    text::Text,
    widgets::{
        Cell, HighlightSpacing, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table,
        TableState,
    },
};
use unicode_width::UnicodeWidthStr;

struct TableColors {
    buffer_bg: Color,
    header_bg: Color,
    header_fg: Color,
    row_fg: Color,
    selected_row_style_fg: Color,
    selected_column_style_fg: Color,
    selected_cell_style_fg: Color,
    normal_row_color: Color,
    alt_row_color: Color,
    // footer_border_color: Color,
}

impl Default for TableColors {
    fn default() -> Self {
        Self {
            buffer_bg: tailwind::SLATE.c950,
            header_bg: tailwind::INDIGO.c900,
            header_fg: tailwind::SLATE.c200,
            row_fg: tailwind::INDIGO.c200,
            selected_row_style_fg: tailwind::INDIGO.c400,
            selected_column_style_fg: tailwind::INDIGO.c400,
            selected_cell_style_fg: tailwind::INDIGO.c600,
            normal_row_color: tailwind::SLATE.c950,
            alt_row_color: tailwind::SLATE.c900,
            // footer_border_color: tailwind::INDIGO.c400,
        }
    }
}

lazy_static! {
    static ref TABLE_COLORS: TableColors = TableColors::default();
}

const ITEM_HEIGHT: usize = 3;

#[derive(Clone, Debug)]
pub struct Transactions {
    transactions: Vec<crate::libs::transactions::Transaction>,

    tx: crate::actions::ActionSender,
    manager: TransactionManager,

    table_state: TableState,
    scroll_state: ScrollbarState,
    longest_item_lens: (usize, usize, usize),
}

impl Transactions {
    pub fn new(tx: ActionSender, manager: TransactionManager) -> Self {
        Self {
            transactions: Default::default(),
            tx,
            manager,

            table_state: TableState::default(),
            scroll_state: ScrollbarState::default(),
            longest_item_lens: (0, 0, 0),
        }
    }

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
    ChangeRowFocus(isize),
}

impl From<TransactionAction> for Action {
    fn from(val: TransactionAction) -> Self {
        Action::Transaction(val)
    }
}

impl WidgetExt for Transactions {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let vertical = &Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]);
        let rects = vertical.split(area);

        self.render_table(frame, rects[0]);
        self.render_scrollbar(frame, rects[0]);

        self.get_help_msg().render(frame, rects[1]);
    }
}

impl EventLoopParticipant for Transactions {
    fn handle_events(&self, event: Event) -> Result<()> {
        if let Event::Key(key) = event {
            match (key.modifiers, key.code) {
                // navigate to fetch page
                (_, KeyCode::Char('f')) => self.tx.send(Action::NavigateTo(NaviTarget::Fetch)),
                (_, KeyCode::Char('l')) => self.tx.send(TransactionAction::LoadTransactions),
                (_, KeyCode::Char('j')) => self.tx.send(TransactionAction::ChangeRowFocus(1)),
                (_, KeyCode::Char('k')) => self.tx.send(TransactionAction::ChangeRowFocus(-1)),
                _ => (),
            }
        };
        Ok(())
    }

    fn update(&mut self, action: Action) {
        if let Action::Transaction(action) = action {
            match action {
                TransactionAction::LoadTransactions => {
                    self.transactions = self.manager.fetch_all().unwrap();
                    self.transactions.sort_by(|a, b| b.time.cmp(&a.time));
                    self.scroll_state = self
                        .scroll_state
                        .content_length(self.transactions.len() * ITEM_HEIGHT);
                    self.longest_item_lens = constraint_len_calculator(&self.transactions);
                }
                TransactionAction::ChangeRowFocus(index) => {
                    let cur_index = self.table_state.selected().unwrap_or(0);
                    let max = self.transactions.len();
                    if max == 0 {
                        return;
                    }
                    // add index to current index, wrap around from 0  to max
                    let new_index = if index < 0 {
                        (cur_index as isize + index + max as isize) as usize % max
                    } else {
                        (cur_index as isize + index) as usize % max
                    };
                    self.table_state.select(Some(new_index));
                    self.scroll_state = self.scroll_state.position(new_index * ITEM_HEIGHT);
                }
            }
        }
    }
}

impl Page for Transactions {
    fn get_name(&self) -> String {
        "Transactions".to_string()
    }

    fn init(&mut self) {
        self.tx.send(TransactionAction::LoadTransactions)
    }
}

impl Transactions {
    fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        let header_style = Style::default()
            .fg(TABLE_COLORS.header_fg)
            .bg(TABLE_COLORS.header_bg);
        let selected_row_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(TABLE_COLORS.selected_row_style_fg);
        let selected_col_style = Style::default().fg(TABLE_COLORS.selected_column_style_fg);
        let selected_cell_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(TABLE_COLORS.selected_cell_style_fg);

        let header = ["金额", "时间", "商家"]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .style(header_style)
            .height(1);

        let rows = self.transactions.iter().enumerate().map(|(i, t)| {
            let color = match i % 2 {
                0 => TABLE_COLORS.normal_row_color,
                _ => TABLE_COLORS.alt_row_color,
            };
            Row::new(
                vec![
                    Text::from(format!("\n{}\n", t.amount)).alignment(Alignment::Right),
                    Text::from(format!("\n{}\n", t.time)),
                    Text::from(format!("\n{}\n", t.merchant)),
                ]
                .into_iter(),
            )
            .style(Style::new().fg(TABLE_COLORS.row_fg).bg(color))
            .height(ITEM_HEIGHT as u16)
        });
        let bar = " █ ";

        let t = Table::new(
            rows,
            [
                Constraint::Length((self.longest_item_lens.0 + 2).try_into().unwrap()),
                Constraint::Min((self.longest_item_lens.1 + 2).try_into().unwrap()),
                Constraint::Min((self.longest_item_lens.2 + 2).try_into().unwrap()),
            ],
        )
        .header(header)
        .row_highlight_style(selected_row_style)
        .column_highlight_style(selected_col_style)
        .cell_highlight_style(selected_cell_style)
        .highlight_symbol(Text::from(vec!["".into(), bar.into(), "".into()]))
        .bg(TABLE_COLORS.buffer_bg)
        .highlight_spacing(HighlightSpacing::Always)
        .column_spacing(4);

        frame.render_stateful_widget(t, area, &mut self.table_state);
    }

    fn render_scrollbar(&mut self, frame: &mut Frame, area: Rect) {
        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            area.inner(Margin {
                vertical: 1,
                horizontal: 1,
            }),
            &mut self.scroll_state,
        );
    }
}

fn constraint_len_calculator(items: &Vec<Transaction>) -> (usize, usize, usize) {
    items.iter().fold((0, 0, 0), |acc, item| {
        let amount_len = max(
            acc.0,
            UnicodeWidthStr::width(item.amount.to_string().as_str()),
        );
        let time_len = max(
            acc.1,
            UnicodeWidthStr::width(item.time.to_string().as_str()),
        );
        let merchant_len = max(
            acc.2,
            UnicodeWidthStr::width_cjk(item.merchant.to_string().as_str()),
        );
        (amount_len, time_len, merchant_len)
    })
}

#[cfg(test)]
mod test {
    use crate::libs::fetcher;

    use super::*;
    use insta::assert_snapshot;
    use ratatui::{Terminal, backend::TestBackend};
    use tokio::sync::mpsc::{self, UnboundedReceiver};

    fn get_test_objs() -> (UnboundedReceiver<Action>, Transactions) {
        let (tx, mut rx) = mpsc::unbounded_channel::<Action>();

        let manager = TransactionManager::new(None).unwrap();
        let data = fetcher::test_utils::get_mock_data(50);
        manager.insert(&data).unwrap();

        let mut transaction = Transactions::new(tx.into(), manager);
        transaction.init();

        while let Ok(action) = rx.try_recv() {
            transaction.update(action);
        }

        assert!(!transaction.transactions.is_empty());

        (rx, transaction)
    }

    #[test]
    fn order() {
        let (_, transaction) = get_test_objs();
        transaction
            .transactions
            .iter()
            .enumerate()
            .for_each(|(i, t)| {
                if i == 0 {
                    return;
                }
                assert!(t.time <= transaction.transactions[i - 1].time);
            });
    }

    #[test]
    fn navigation() {
        let (mut rx, mut transaction) = get_test_objs();
        assert_eq!(transaction.table_state.selected(), None);

        transaction.event_loop_once(&mut rx, 'j'.into());
        assert_eq!(transaction.table_state.selected(), Some(1));

        transaction.event_loop_once(&mut rx, 'k'.into());
        assert_eq!(transaction.table_state.selected(), Some(0));

        transaction.event_loop_once(&mut rx, 'k'.into());
        assert_eq!(
            transaction.table_state.selected(),
            Some(transaction.transactions.len() - 1)
        );
    }

    #[test]
    fn render() {
        let (mut rx, mut transaction) = get_test_objs();
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();

        terminal
            .draw(|frame| transaction.render(frame, frame.area()))
            .unwrap();
        assert_snapshot!(terminal.backend());

        transaction.event_loop_once(&mut rx, 'k'.into());
        terminal
            .draw(|frame| transaction.render(frame, frame.area()))
            .unwrap();
        assert_snapshot!(terminal.backend());
    }
}
