use crate::RootState;
use crate::actions::Action;
use crate::tui::Event;
use color_eyre::Result;
use ratatui::Frame;

pub mod cookie_input;
pub mod fetch;
pub mod home;
pub mod transactions;
pub trait Page: Send + Sync {
    /// Render the page
    fn render(&self, frame: &mut Frame, app: &RootState);

    /// Convert Events to Actions
    fn handle_events(&self, app: &RootState, event: Event) -> Result<()>;

    /// Perform Actions and update the state of the page
    fn update(&mut self, app: &RootState, action: Action);

    /// Get the name of the page
    fn get_name(&self) -> String;

    /// Initialize the page
    fn init(&mut self, _app: &mut RootState) {}
}
