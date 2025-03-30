use crate::actions::Action;
use crate::tui::Event;
use color_eyre::Result;
use ratatui::Frame;

pub mod cookie_input;
pub mod fetch;
pub mod home;
pub mod transactions;

#[cfg(test)]
use tokio::sync::mpsc::UnboundedReceiver;

pub trait Page: Send + Sync {
    /// Render the page
    fn render(&mut self, frame: &mut Frame);

    /// Convert Events to Actions
    fn handle_events(&self, event: Event) -> Result<()>;

    /// Perform Actions and update the state of the page
    fn update(&mut self, action: Action);

    /// Get the name of the page
    fn get_name(&self) -> String;

    /// Initialize the page
    fn init(&mut self) {}

    #[cfg(test)]
    fn event_loop_once(&mut self, rx: &mut UnboundedReceiver<Action>, event: Event) {
        self.handle_events(event).unwrap();
        while let Ok(action) = rx.try_recv() {
            self.update(action);
        }
    }

    #[cfg(test)]
    fn event_loop_once_with_action(&mut self, rx: &mut UnboundedReceiver<Action>, action: Action) {
        self.update(action);
        while let Ok(action) = rx.try_recv() {
            self.update(action);
        }
    }
}
