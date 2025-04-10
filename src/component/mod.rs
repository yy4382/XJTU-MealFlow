//! Component module defines the core trait for UI components in the application.
//!
//! A Component is an *interactive* UI element that has its own actions but lives within a [`crate::page::Layer`].
//!
//! This module provides the base Component trait that all UI components must implement,
//! enabling consistent event handling and state updates across the application.

pub(crate) mod input;
