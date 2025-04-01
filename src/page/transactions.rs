use std::cmp::max;

use crate::{
    actions::{Action, ActionSender, LayerManageAction, Layers},
    libs::transactions::{FilterOptions, Transaction, TransactionManager},
    tui::Event,
    utils::help_msg::{HelpEntry, HelpMsg},
};

use super::{EventLoopParticipant, Layer, WidgetExt};
use color_eyre::eyre::{Context, Result};
use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style, Stylize, palette::tailwind},
    text::Text,
    widgets::{
        Cell, Clear, HighlightSpacing, Paragraph, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Table, TableState,
    },
};
use unicode_width::UnicodeWidthStr;

struct TableColors {
    buffer_bg: Color,
    header_bg: Color,
    header_fg: Color,
    row_fg: Color,
    selected_row_style_fg: Color,
    // selected_column_style_fg: Color,
    // selected_cell_style_fg: Color,
    normal_row_color: Color,
    alt_row_color: Color,
    // footer_border_color: Color,
}

const TABLE_COLORS: TableColors = TableColors {
    buffer_bg: Color::Reset,
    header_bg: tailwind::INDIGO.c950,
    header_fg: tailwind::GRAY.c100,
    row_fg: tailwind::GRAY.c200,
    selected_row_style_fg: tailwind::INDIGO.c300,
    // selected_column_style_fg: tailwind::INDIGO.c400,
    // selected_cell_style_fg: tailwind::INDIGO.c600,
    normal_row_color: Color::Reset,
    alt_row_color: tailwind::GRAY.c950,
    // footer_border_color: tailwind::INDIGO.c400,
};

const ITEM_HEIGHT: usize = 3;

#[derive(Clone, Debug)]
pub struct Transactions {
    filter_option: Option<FilterOptions>,
    tx: crate::actions::ActionSender,
    manager: TransactionManager,

    transactions: Vec<crate::libs::transactions::Transaction>,

    table_state: TableState,
    scroll_state: ScrollbarState,
    longest_item_lens: (usize, usize, usize),
}

impl Transactions {
    pub fn new(
        filter_option: Option<FilterOptions>,
        tx: ActionSender,
        manager: TransactionManager,
    ) -> Self {
        let mut t = Self {
            filter_option,
            tx,
            manager,

            transactions: Default::default(),

            table_state: TableState::default(),
            scroll_state: ScrollbarState::default(),
            longest_item_lens: (0, 0, 0),
        };
        t.load_from_db();
        t
    }

    fn get_help_msg(&self) -> HelpMsg {
        let help_msg: HelpMsg = [
            if self.filter_option.is_none() {
                Some(HelpEntry::new('f', "Fetch"))
            } else {
                None
            },
            Some(HelpEntry::new('?', "Show help")),
            Some(HelpEntry::new('l', "Load from local cache")),
        ]
        .into_iter()
        .filter_map(|x| x)
        .collect::<Vec<HelpEntry>>()
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
        frame.render_widget(Clear, area);
        let (main_area, help_area) = match &self.filter_option {
            Some(opt) => {
                let areas = &Layout::vertical([
                    Constraint::Fill(1),
                    Constraint::Length(3),
                    Constraint::Length(3),
                ])
                .split(area);

                frame.render_widget(Paragraph::new(format!("\nFilters: {}\n", opt)), areas[1]);

                (areas[0], areas[2])
            }
            None => {
                let areas =
                    &Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]).split(area);

                (areas[0], areas[1])
            }
        };
        self.render_table(frame, main_area);
        self.render_scrollbar(frame, main_area);

        self.get_help_msg().render(frame, help_area);
    }
}

impl EventLoopParticipant for Transactions {
    fn handle_events(&self, event: Event) -> Result<()> {
        if let Event::Key(key) = event {
            match (key.modifiers, key.code) {
                // navigate to fetch page
                (_, KeyCode::Char('f')) => {
                    if self.filter_option.is_none() {
                        self.tx.send(LayerManageAction::SwapPage(Layers::Fetch))
                    }
                }
                (_, KeyCode::Char('l')) => self.tx.send(TransactionAction::LoadTransactions),
                (_, KeyCode::Char('j')) | (_, KeyCode::Down) => {
                    self.tx.send(TransactionAction::ChangeRowFocus(1))
                }
                (_, KeyCode::Char('k')) | (_, KeyCode::Up) => {
                    self.tx.send(TransactionAction::ChangeRowFocus(-1))
                }
                (_, KeyCode::Char('?')) => {
                    self.tx.send(LayerManageAction::PushPage(
                        Layers::Help(self.get_help_msg()).into_push_config(true),
                    ));
                }
                (_, KeyCode::Enter) => match self.table_state.selected() {
                    // TODO add help info
                    Some(index) => match self.transactions.get(index) {
                        Some(transaction) => {
                            self.tx.send(LayerManageAction::PushPage(
                                Layers::Transaction(Some(
                                    self.filter_option
                                        .clone()
                                        .unwrap_or_default()
                                        .merchant(transaction.merchant.clone()),
                                ))
                                .into_push_config(false),
                            ));
                        }
                        None => {}
                    },
                    None => {}
                },
                // TODO add help info
                (_, KeyCode::Esc) => self.tx.send(LayerManageAction::PopPage),
                _ => (),
            }
        };
        Ok(())
    }

    fn update(&mut self, action: Action) {
        if let Action::Transaction(action) = action {
            match action {
                TransactionAction::LoadTransactions => {
                    self.load_from_db();
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

impl Layer for Transactions {}

impl Transactions {
    fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        let header_style = Style::default()
            .fg(TABLE_COLORS.header_fg)
            .bg(TABLE_COLORS.header_bg);
        let selected_row_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(TABLE_COLORS.selected_row_style_fg);
        // let selected_col_style = Style::default().fg(TABLE_COLORS.selected_column_style_fg);
        // let selected_cell_style = Style::default()
        //     .add_modifier(Modifier::REVERSED)
        //     .fg(TABLE_COLORS.selected_cell_style_fg);

        let header = HEADER_STR
            .into_iter()
            .map(|r| Cell::from(format!("\n{}\n", r)))
            .collect::<Row>()
            .style(header_style)
            .height(3);

        let rows = self.transactions.iter().enumerate().map(|(i, t)| {
            let color = match i % 2 {
                0 => TABLE_COLORS.normal_row_color,
                _ => TABLE_COLORS.alt_row_color,
            };
            Row::new(
                vec![
                    Text::from(format!("\n{}\n", t.amount)).alignment(Alignment::Right),
                    Text::from(format!("\n{}\n", t.time.format("%Y-%m-%d %H:%M"))),
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
        // .column_highlight_style(selected_col_style)
        // .cell_highlight_style(selected_cell_style)
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

    fn load_from_db(&mut self) {
        match &self.filter_option {
            Some(option) => {
                self.transactions = self
                    .manager
                    .fetch_filtered(option)
                    .with_context(|| {
                        format!(
                            "Failed to load transactions from database with filter: {:?}",
                            option
                        )
                    })
                    .unwrap();
            }
            None => {
                self.transactions = self
                    .manager
                    .fetch_all()
                    .context("Failed to load transactions from database")
                    .unwrap();
            }
        }
        self.transactions.sort_by(|a, b| b.time.cmp(&a.time));
        self.scroll_state = self
            .scroll_state
            .content_length(self.transactions.len() * ITEM_HEIGHT);
        self.longest_item_lens = constraint_len_calculator(&self.transactions, HEADER_STR);
    }
}

const HEADER_STR: &[&str] = &["金额", "时间", "商家"];

fn constraint_len_calculator(items: &Vec<Transaction>, header: &[&str]) -> (usize, usize, usize) {
    let data_len = items.iter().fold((0, 0, 0), |acc, item| {
        let amount_len = max(
            acc.0,
            UnicodeWidthStr::width(item.amount.to_string().as_str()),
        );
        let time_len = max(
            acc.1,
            UnicodeWidthStr::width(format!("{}", item.time.format("%Y-%m-%d %H:%M")).as_str()),
        );
        let merchant_len = max(
            acc.2,
            UnicodeWidthStr::width_cjk(item.merchant.to_string().as_str()),
        );
        (amount_len, time_len, merchant_len)
    });
    (
        max(data_len.0, UnicodeWidthStr::width_cjk(header[0])),
        max(data_len.1, UnicodeWidthStr::width_cjk(header[1])),
        max(data_len.2, UnicodeWidthStr::width_cjk(header[2])),
    )
}

#[cfg(test)]
mod test {
    use core::panic;

    use crate::{actions::PushPageConfig, libs::fetcher};

    use super::*;
    use insta::assert_snapshot;
    use ratatui::{Terminal, backend::TestBackend};
    use tokio::sync::mpsc::{self, UnboundedReceiver};

    fn get_test_objs(
        filter_opt: Option<FilterOptions>,
        load_data: u32,
    ) -> (UnboundedReceiver<Action>, Transactions) {
        let (tx, mut rx) = mpsc::unbounded_channel::<Action>();

        let manager = TransactionManager::new(None).unwrap();
        let data = fetcher::test_utils::get_mock_data(load_data);
        manager.insert(&data).unwrap();

        let mut transaction = Transactions::new(filter_opt, tx.into(), manager);
        transaction.init();

        while let Ok(action) = rx.try_recv() {
            transaction.update(action);
        }

        assert!(!transaction.transactions.is_empty());

        (rx, transaction)
    }

    #[test]
    fn table_length() {
        let data = fetcher::test_utils::get_mock_data(5);
        let result = constraint_len_calculator(&data, HEADER_STR);
        println!("data: {:?}", data);
        println!("result: {:?}", result);
        assert_eq!(result.0, 6);
        assert_eq!(result.1, 16);
        assert_eq!(result.2, 16);
    }

    #[test]
    fn table_length_only_header() {
        let data = vec![];
        let result = constraint_len_calculator(&data, HEADER_STR);
        assert_eq!(result.0, 4);
        assert_eq!(result.1, 4);
        assert_eq!(result.2, 4);
    }

    #[test]
    fn order() {
        let (_, transaction) = get_test_objs(None, 50);
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
        let (mut rx, mut transaction) = get_test_objs(None, 50);
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
        let (mut rx, mut transaction) = get_test_objs(None, 50);
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

    #[test]
    fn with_filter() {
        let (mut rx, transaction) =
            get_test_objs(Some(FilterOptions::default().merchant("寿司")), 200);
        assert!(transaction.filter_option.is_some());
        transaction.transactions.iter().for_each(|t| {
            assert!(t.merchant.contains("寿司"));
        });
        transaction.handle_events('f'.into()).unwrap();
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn render_with_filter() {
        let (mut rx, mut transaction) =
            get_test_objs(Some(FilterOptions::default().merchant("寿司")), 200);
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

    #[test]
    fn push_filtered_page() {
        let (mut rx, mut transaction) = get_test_objs(None, 50);
        transaction.event_loop_once(&mut rx, 'j'.into());
        transaction.handle_events(KeyCode::Enter.into()).unwrap();
        while let Ok(action) = rx.try_recv() {
            if let Action::Layer(LayerManageAction::PushPage(PushPageConfig {
                layer: Layers::Transaction(filter),
                render_self: false,
            })) = action
            {
                assert!(filter.is_some());
                Some(assert_eq!(
                    filter.unwrap(),
                    FilterOptions::default().merchant(
                        transaction
                            .transactions
                            .get(transaction.table_state.selected().unwrap())
                            .unwrap()
                            .merchant
                            .clone()
                    )
                ));
            } else {
                panic!("Expected PushPage action");
            }
        }
    }
}
