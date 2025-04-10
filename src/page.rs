//! Page module handles different UI pages and their behaviors.
//!
//! This module provides traits and implementations for managing different pages
//! in the TUI application, including their rendering, event handling, and state management.

use crate::app::layer_manager::EventHandlingStatus;
use crate::tui::Event;
use downcast_rs::{DowncastSync, impl_downcast};
use ratatui::Frame;
use ratatui::layout::Rect;

pub(crate) mod analysis;
pub(crate) mod cookie_input;
pub(crate) mod fetch;
pub(crate) mod help_popup;
pub(crate) mod home;
pub(crate) mod transactions;

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
    #[must_use]
    fn handle_events(&mut self, event: &Event) -> EventHandlingStatus;

    #[cfg(test)]
    /// Handle the event and check the returned status as [`EventHandlingStatus::Consumed`].
    fn handle_event_with_status_check(&mut self, event: &Event) {
        let status = self.handle_events(event);
        assert!(matches!(status, EventHandlingStatus::Consumed));
    }
}
