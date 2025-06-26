//! # 可重用UI组件库
//!
//! 该模块包含应用程序中使用的可重用UI组件。
//! 这些组件独立于具体的页面实现，可以在多个地方复用。
//!
//! ## 设计原则
//!
//! - **组件化**: 每个组件都是独立的功能单元
//! - **可重用**: 组件可以在不同页面中重复使用
//! - **可配置**: 组件支持灵活的配置选项
//! - **事件驱动**: 组件通过事件与外部通信
//!
//! ## 组件列表
//!
//! - [`input`]: 输入组件，提供文本输入和编辑功能
//!
//! ## 使用示例
//!
//! ```rust
//! use crate::component::input::{InputComp, InputMode};
//!
//! // 创建输入组件
//! let mut input = InputComp::new()
//!     .title("请输入内容")
//!     .auto_submit(true);
//!
//! // 设置焦点
//! input.set_mode(InputMode::Focused);
//!
//! // 处理事件
//! let (status, output) = input.handle_events(&event);
//! ```

/// 输入组件模块
///
/// 提供文本输入和编辑功能，支持多种输入模式和自定义配置。
pub(crate) mod input;
