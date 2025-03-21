use crate::RootState;
use crate::actions::Action;
use crate::tui::Event;
use color_eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::Frame;

pub mod fetch;
pub mod home;
pub mod transactions;
pub trait Page: Send + Sync {
    /// Render the page
    fn render(&self, frame: &mut Frame, app: &RootState);

    /// Convert Events to Actions (not input mode events)
    fn handle_events(&self, event: Option<Event>) -> Result<Action>;

    /// Convert input mode events to Actions
    fn handle_input_mode_events(&self, _event: KeyEvent) -> Result<Action> {
        Ok(Action::None)
    }

    /// Perform Actions and update the state of the page
    fn update(&mut self, app: &mut RootState, action: Action);

    /// Get the name of the page
    fn get_name(&self) -> String;

    /// Initialize the page
    fn init(&mut self, _app: &mut RootState) {}
}
