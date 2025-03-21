mod actions;
mod errors;
mod fetcher;
mod page;
mod transactions;
mod tui;

use actions::Action;
use color_eyre::eyre::Result;
use dotenv::dotenv;
use page::Page;
use ratatui::crossterm::event::KeyCode::Char;
use tokio::sync::mpsc::{self};
use transactions::TransactionManager;

pub struct RootState {
    should_quit: bool,
    action_tx: mpsc::UnboundedSender<Action>,
    manager: TransactionManager,
}
pub struct App {
    page: Box<dyn Page>,
    state: RootState,
}

async fn run() -> Result<()> {
    let (action_tx, mut action_rx) = mpsc::unbounded_channel(); // new

    // ratatui terminal
    let mut tui = tui::Tui::new()?.tick_rate(1.0).frame_rate(30.0);
    tui.enter()?;

    let root_state = RootState {
        should_quit: false,
        action_tx: action_tx.clone(),
        manager: TransactionManager::new().unwrap(),
    };

    root_state.manager.init_db()?;

    // application state
    let mut app = App {
        state: root_state,
        page: Box::new(page::home::Home::default()),
    };

    loop {
        let e = tui.next().await?;
        match e {
            // tui::Event::Quit => action_tx.send(Action::Quit)?,
            tui::Event::Tick => action_tx.send(Action::Tick)?,
            tui::Event::Render => action_tx.send(Action::Render)?,
            // TODO handle resize
            tui::Event::Resize(_, _) => action_tx.send(Action::Render)?,
            // TODO handle close
            // tui::Event::Closed => action_tx.send(Action::Quit)?,
            tui::Event::Key(key) => match key.code {
                Char('H') => {
                    // check if the current page is not Home
                    if app.page.get_name() != "Home" {
                        action_tx.send(Action::NavigateTo(actions::NavigateTarget::Home(
                            page::home::Home::default(),
                        )))?;
                    }
                }
                Char('T') => {
                    // check if the current page is not Transactions
                    if app.page.get_name() != "Transactions" {
                        action_tx.send(Action::NavigateTo(
                            actions::NavigateTarget::Transaction(
                                page::transactions::Transactions::default(),
                            ),
                        ))?;
                    }
                }
                Char('q') => action_tx.send(Action::Quit)?,
                _ => {
                    let action: Action = app.page.handle_events(Some(e)).unwrap();

                    action_tx.send(action.clone())?;
                }
            },
            _ => {}
        };

        while let Ok(action) = action_rx.try_recv() {
            match action {
                Action::Quit => {
                    app.state.should_quit = true;
                }
                Action::None => {}
                Action::Render => {
                    tui.draw(|f| {
                        app.page.render(f);
                    })?;
                }
                Action::NavigateTo(target) => match target {
                    actions::NavigateTarget::Home(page) => {
                        app.page = Box::new(page);
                        app.page.init(&mut app.state);
                    }
                    actions::NavigateTarget::Transaction(page) => {
                        app.page = Box::new(page);
                        app.page.init(&mut app.state);
                    }
                    actions::NavigateTarget::Fetch(page) => {
                        app.page = Box::new(page);
                        app.page.init(&mut app.state);
                    }
                },
                _ => {
                    app.page.update(&mut app.state, action.clone());
                }
            }
        }

        // application exit
        if app.state.should_quit {
            break;
        }
    }

    tui.exit()?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    errors::init()?;

    let result = run().await;

    result?;

    Ok(())
}
