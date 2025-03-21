use crate::RootState;
use crate::actions::Action;
use crate::tui::Event;
use color_eyre::Result;
use ratatui::Frame;

pub mod fetch;
pub mod home;
pub mod transactions;
pub trait Page {
    fn render(&self, frame: &mut Frame);
    fn handle_events(&mut self, event: Option<Event>) -> Result<Action>;
    fn update(&mut self, app: &mut RootState, action: Action);
    fn get_name(&self) -> String;
    fn init(&mut self, _app: &mut RootState) {}
}
