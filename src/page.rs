//! Page module handles different UI pages and their behaviors.
//!
//! This module provides traits and implementations for managing different pages
//! in the TUI application, including their rendering, event handling, and state management.

use crate::actions::Action;
use crate::tui::Event;
use color_eyre::Result;
use downcast_rs::{DowncastSync, impl_downcast};
use ratatui::Frame;
use ratatui::layout::Rect;

pub mod cookie_input;
pub mod fetch;
pub mod help_popup;
pub mod home;
pub mod transactions;

#[cfg(test)]
use tokio::sync::mpsc::UnboundedReceiver;

/// A trait that represents a UI layer/page in the application.
///
/// Implements core functionality for pages including initialization,
/// rendering, and event handling. Pages must be both Send and Sync safe.
///
/// # Type Requirements
/// - Must implement [`WidgetExt`]
/// - Must implement [`EventLoopParticipant`]
/// - Must implement [`DowncastSync`]
pub trait Layer: WidgetExt + EventLoopParticipant + DowncastSync {
    /// Initialize the page
    fn init(&mut self) {}
}
impl_downcast!(sync Layer);

/// Extension trait for widgets that can be rendered to the screen.
///
/// Provides the interface for rendering UI components to a specific area
/// of the terminal frame.
pub(crate) trait WidgetExt {
    fn render(&mut self, frame: &mut Frame, area: Rect);
}

/// Trait for components that participate in the event loop.
///
/// Provides methods for handling events and updating state based on actions.
/// Also includes test utilities for simulating event loop iterations.
///
/// # Test Features
/// When compiled with test configuration, provides additional methods:
/// - `event_loop_once`: Processes a single event and subsequent actions
/// - `event_loop_once_with_action`: Processes a single action and subsequent actions
pub(crate) trait EventLoopParticipant {
    /// Handle events
    fn handle_events(&self, event: Event) -> Result<()>;

    /// Perform Actions and update the state of the page
    fn update(&mut self, action: Action);

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
