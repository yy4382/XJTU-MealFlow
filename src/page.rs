use crate::RootState;
use crate::actions::Action;
use crate::tui::Event;
use color_eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::Frame;

pub mod fetch;
pub mod home;
pub mod transactions;
pub trait Page {
    fn render(&self, frame: &mut Frame, app: &RootState);
    fn handle_events(&mut self, event: Option<Event>) -> Result<Action>;
    fn handle_input_mode_events(&mut self, _event: KeyEvent) -> Result<Action> {
        Ok(Action::None)
    }
    fn update(&mut self, app: &mut RootState, action: Action);
    fn get_name(&self) -> String;
    fn init(&mut self, _app: &mut RootState) {}
}
