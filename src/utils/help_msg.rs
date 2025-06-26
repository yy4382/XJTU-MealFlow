//! # 帮助信息显示系统
//!
//! 提供统一的帮助信息管理和显示功能，用于在 TUI 界面中展示快捷键说明和操作指南。
//! 支持灵活的帮助条目组合、格式化和渲染。
//!
//! ## 设计理念
//!
//! 帮助系统采用组合式设计，允许不同页面和组件定义自己的帮助条目，
//! 然后通过统一的接口进行组合和显示。这样既保证了一致性，
//! 又提供了足够的灵活性。
//!
//! ## 核心组件
//!
//! ```text
//! HelpEntry (帮助条目)
//!     ├─ key: 按键或操作名称
//!     └─ desc: 功能描述
//!         │
//!         ↓
//! HelpMsg (帮助消息集合)
//!     ├─ 包含多个 HelpEntry
//!     ├─ 支持条目组合和扩展
//!     └─ 提供统一的渲染接口
//! ```
//!
//! ## 帮助条目类型
//!
//! ### 键盘按键类型
//! ```rust
//! // 单个字符按键
//! HelpEntry::new('q', "Quit application");
//!
//! // 功能键
//! HelpEntry::new(KeyCode::Esc, "Go back");
//! HelpEntry::new(KeyCode::Enter, "Confirm");
//! ```
//!
//! ### 纯文本类型
//! ```rust
//! // 组合按键说明
//! HelpEntry::new_plain("hjkl", "Move cursor");
//! HelpEntry::new_plain("Ctrl+C", "Copy");
//! ```
//!
//! ## 显示格式
//!
//! 帮助信息在 TUI 中以统一格式显示：
//!
//! ```text
//! ┌─ Help ────────────────────────────────────────────┐
//! │ Quit: q | Go back: Esc | Move cursor: hjkl        │
//! └────────────────────────────────────────────────────┘
//! ```
//!
//! 格式规则：
//! - 每个条目: `描述: 按键`
//! - 条目间分隔: ` | `
//! - 自动换行和布局适应
//!
//! ## 使用模式
//!
//! ### 基本使用
//! ```rust
//! use crate::utils::help_msg::{HelpEntry, HelpMsg};
//!
//! // 创建帮助条目
//! let help: HelpMsg = vec![
//!     HelpEntry::new('q', "Quit"),
//!     HelpEntry::new('?', "Help"),
//!     HelpEntry::new_plain("hjkl", "Navigate"),
//! ].into();
//!
//! // 在 TUI 中渲染
//! help.render(frame, help_area);
//! ```
//!
//! ### 条目组合
//! ```rust
//! // 基础帮助信息
//! let mut base_help = HelpMsg::from(vec![
//!     HelpEntry::new('q', "Quit"),
//! ]);
//!
//! // 页面特定帮助
//! let page_help = HelpMsg::from(vec![
//!     HelpEntry::new('s', "Save"),
//!     HelpEntry::new('l', "Load"),
//! ]);
//!
//! // 组合帮助信息
//! base_help.extend(&page_help);
//! // 或者使用链式调用
//! let combined = base_help.extend_ret(&page_help);
//! ```
//!
//! ### 动态帮助更新
//! ```rust
//! impl SomePage {
//!     fn get_help_msg(&self) -> HelpMsg {
//!         let mut help = self.base_help();
//!         
//!         // 根据当前状态添加特定帮助
//!         if self.is_editing() {
//!             help.extend(&self.editing_help());
//!         }
//!         
//!         help
//!     }
//! }
//! ```
//!
//! ## 渲染特性
//!
//! - **自适应布局**: 根据终端宽度自动调整显示
//! - **边框装饰**: 使用圆角边框和内边距
//! - **统一样式**: 所有页面使用相同的帮助显示样式
//! - **实时更新**: 支持帮助信息的动态更新和重渲染
//!
//! ## 集成示例
//!
//! ```rust
//! // 在页面中集成帮助系统
//! impl WidgetExt for MyPage {
//!     fn render(&mut self, frame: &mut Frame, area: Rect) {
//!         let areas = Layout::default()
//!             .constraints([
//!                 Constraint::Fill(1),        // 主内容区域
//!                 Constraint::Length(3),      // 帮助信息区域
//!             ])
//!             .split(area);
//!
//!         // 渲染主内容
//!         self.render_content(frame, areas[0]);
//!         
//!         // 渲染帮助信息
//!         self.get_help_msg().render(frame, areas[1]);
//!     }
//! }
//! ```
//!
//! ## 扩展性
//!
//! 系统设计考虑了未来扩展的需求：
//! - 可以添加长描述字段用于详细帮助弹窗
//! - 支持按键分组和分类显示
//! - 可以集成键盘快捷键配置系统
//! - 支持多语言帮助信息

use std::ops::{Deref, DerefMut};

use ratatui::widgets::{Block, BorderType, Borders, Padding};

use super::key_events::KeyEvent;

#[derive(Debug, Clone)]

enum HelpKeyEvent {
    Key(KeyEvent),
    Plain(String),
}
// TODO add a long_desc field to HelpEntry to show in popup

#[derive(Debug, Clone)]
pub(crate) struct HelpEntry {
    key: HelpKeyEvent,
    desc: String,
}

impl HelpEntry {
    pub(crate) fn new<T: Into<String>, K: Into<KeyEvent>>(event: K, desc: T) -> Self {
        Self {
            key: HelpKeyEvent::Key(event.into()),
            desc: desc.into(),
        }
    }
    pub(crate) fn new_plain<T: Into<String>>(event: T, desc: T) -> Self {
        Self {
            key: HelpKeyEvent::Plain(event.into()),
            desc: desc.into(),
        }
    }

    pub(crate) fn key(&self) -> String {
        match &self.key {
            HelpKeyEvent::Key(key) => key.to_string(),
            HelpKeyEvent::Plain(key) => key.clone(),
        }
    }

    pub(crate) fn desc(&self) -> &str {
        &self.desc
    }
}

impl std::fmt::Display for HelpEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.desc(), self.key())
    }
}

impl From<HelpEntry> for String {
    fn from(val: HelpEntry) -> Self {
        format!("{}", val)
    }
}

#[derive(Default, Clone, Debug)]
pub(crate) struct HelpMsg {
    slices: Vec<HelpEntry>,
}

impl From<Vec<HelpEntry>> for HelpMsg {
    fn from(slices: Vec<HelpEntry>) -> Self {
        Self { slices }
    }
}

impl HelpMsg {
    pub(crate) fn extend(&mut self, other: &HelpMsg) {
        self.slices.extend(other.slices.clone());
    }

    pub(crate) fn extend_ret(mut self, other: &HelpMsg) -> Self {
        self.slices.extend(other.slices.clone());
        self
    }
    pub(crate) fn push(&mut self, entry: HelpEntry) {
        self.slices.push(entry);
    }

    pub(crate) fn render(&mut self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        let help_msg: String = self.to_string();
        let paragraph = ratatui::widgets::Paragraph::new(help_msg).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .padding(Padding::horizontal(1)),
        );
        frame.render_widget(paragraph, area);
    }
}

impl Deref for HelpMsg {
    type Target = Vec<HelpEntry>;

    fn deref(&self) -> &Self::Target {
        &self.slices
    }
}

impl DerefMut for HelpMsg {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.slices
    }
}

impl std::fmt::Display for HelpMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.slices
                .iter()
                .map(|s| s.clone().into())
                .collect::<Vec<String>>()
                .join(" | ")
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_help_entry_key() {
        let entry = HelpEntry::new('c', "Create a new transaction");
        assert_eq!(entry.key(), "c");
        assert_eq!(entry.desc(), "Create a new transaction");
        assert_eq!(entry.to_string(), "Create a new transaction: c");
    }
    #[test]
    fn test_help_entry_plain() {
        let entry = HelpEntry::new_plain("hjkl", "Move cursor");
        assert_eq!(entry.key(), "hjkl");
        assert_eq!(entry.desc(), "Move cursor");
        assert_eq!(entry.to_string(), "Move cursor: hjkl");
    }
}
