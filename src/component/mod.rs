//! Component module defines the core trait for UI components in the application.
//! 
//! A Component is an *interactive* UI element that has its own actions but lives within a [`crate::page::Layer`].
//!
//! This module provides the base Component trait that all UI components must implement,
//! enabling consistent event handling and state updates across the application.

pub(crate) mod input;

use crate::{actions::Action, page::WidgetExt};
use color_eyre::eyre::Result;

/// A trait that defines the core functionality required for UI components.
///
/// Components are interactive UI elements that can:
/// - Handle input events
/// - Update their state based on actions
/// - Be uniquely identified
///
/// # Type Parameters
///
/// The trait requires implementing [`WidgetExt`] for basic widget functionality.
///
/// # Examples
///
/// ```rust
/// struct MyComponent {
///     id: u64,
///     // ... other fields
/// }
///
/// impl Component for MyComponent {
///     fn get_id(&self) -> u64 {
///         self.id
///     }
///     
///     fn handle_events(&self, event: &Event) -> Result<()> {
///         // Handle events...
///         Ok(())
///     }
///     
///     fn update(&mut self, action: &Action) -> Result<()> {
///         // Update component state...
///         Ok(())
///     }
/// }
/// ```
pub(crate) trait Component: WidgetExt {
    #[allow(dead_code)]
    fn get_id(&self) -> u64;

    fn handle_events(&self, event: &crate::tui::Event) -> Result<()>;

    fn update(&mut self, action: &Action) -> Result<()>;
}
