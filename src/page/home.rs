//! # 主页面模块
//!
//! 提供应用程序的主页面功能，展示 XJTU MealFlow 的欢迎界面和主要导航选项。
//!
//! ## 功能特性
//!
//! - **响应式布局**: 根据终端宽度自动调整 ASCII 艺术字显示
//! - **键盘导航**: 支持单键快捷键进行页面导航
//! - **帮助系统**: 集成帮助提示和快捷键说明
//!
//! ## ASCII 艺术字显示规则
//!
//! ```text
//! 终端宽度 >= 100 字符  →  显示完整 XJTU MealFlow 艺术字
//! 终端宽度 >= 60 字符   →  显示简化 MealFlow 艺术字  
//! 终端宽度 < 60 字符    →  显示纯文本 "XJTU MealFlow"
//! ```
//!
//! ## 键盘快捷键
//!
//! | 按键 | 功能 | 目标页面 |
//! |------|------|----------|
//! | `T` | 交易记录 | Transactions |
//! | `a` | 数据分析 | Analysis |
//! | `?` | 帮助信息 | Help Popup |
//! | `q` | 退出应用 | - |
//!
//! ## 页面布局
//!
//! ```text
//! ┌─────────────────────────────────────┐
//! │                                     │
//! │         ASCII 艺术字                │
//! │     (垂直居中显示)                  │
//! │                                     │
//! ├─────────────────────────────────────┤
//! │ 快捷键帮助信息 (固定高度 3 行)     │
//! └─────────────────────────────────────┘
//! ```
//!
//! ## 使用示例
//!
//! ```rust
//! use crate::page::home::Home;
//! use crate::actions::ActionSender;
//!
//! // 创建主页面实例
//! let home = Home {
//!     tx: action_sender,
//! };
//!
//! // 页面会自动处理渲染和事件
//! ```
//!
//! ## 测试支持
//!
//! 模块包含完整的测试覆盖：
//! - 不同尺寸终端的渲染测试
//! - 键盘事件处理测试
//! - 页面导航功能测试

use std::vec;

use crate::{
    actions::{Action, ActionSender, LayerManageAction, Layers},
    app::layer_manager::EventHandlingStatus,
    tui::Event,
    utils::help_msg::{HelpEntry, HelpMsg},
};

use super::{EventLoopParticipant, Layer, WidgetExt};
use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Flex, Layout},
    style::{Color, Style},
    widgets::Paragraph,
};

/// 主页面结构体
///
/// 负责渲染应用程序的主页面，包括 ASCII 艺术字显示和导航功能。
/// 实现了响应式布局，能够根据终端大小自动调整显示内容。
///
/// ## 字段说明
///
/// - `tx`: Action 发送器，用于处理用户交互和页面导航
#[derive(Clone, Debug)]
pub struct Home {
    /// Action 消息发送器
    ///
    /// 用于发送用户操作产生的 Action 到应用程序的消息处理系统
    pub tx: ActionSender,
}

impl Home {
    /// 获取主页面的帮助信息
    ///
    /// 返回当前页面可用的快捷键列表和功能说明。
    /// 这些帮助信息会显示在页面底部的帮助区域。
    ///
    /// # 返回值
    ///
    /// 返回 `HelpMsg` 结构，包含所有可用的快捷键和对应功能描述
    fn get_help_msg(&self) -> HelpMsg {
        let help_msg: HelpMsg = vec![
            HelpEntry::new('T', "Go to transactions page"),
            HelpEntry::new('q', "Quit"),
            HelpEntry::new('?', "Show help"),
        ]
        .into();
        help_msg
    }
}

#[cfg(test)]
impl Default for Home {
    fn default() -> Self {
        let (tx, _) = tokio::sync::mpsc::unbounded_channel();
        Self {
            tx: ActionSender(tx),
        }
    }
}

impl WidgetExt for Home {
    fn render(&mut self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let ascii_art = if area.width >= 100 {
            include_str!("../../data/xjtu-mealflow.txt")
        } else if area.width >= 60 {
            include_str!("../../data/mealflow.txt")
        } else {
            "XJTU MealFlow"
        };

        let area = &Layout::default()
            .constraints([Constraint::Fill(1), Constraint::Length(3)])
            .split(area);

        let height = ascii_art.lines().count() as u16;
        let [v_align_area] = &Layout::vertical([Constraint::Length(height + 1)])
            .flex(Flex::Center)
            .areas(area[0]);

        frame.render_widget(
            Paragraph::new(ascii_art)
                .style(Style::default().fg(Color::Cyan))
                .alignment(Alignment::Center),
            *v_align_area,
        );

        self.get_help_msg().render(frame, area[1]);
    }
}

impl EventLoopParticipant for Home {
    fn handle_events(&mut self, _event: &crate::tui::Event) -> EventHandlingStatus {
        let mut status = EventHandlingStatus::default();
        if let Event::Key(key) = _event {
            match key.code {
                KeyCode::Char('?') => {
                    self.tx.send(LayerManageAction::Push(
                        Layers::Help(self.get_help_msg()).into_push_config(true),
                    ));
                    status.consumed();
                }
                KeyCode::Char('a') => {
                    // TODO add help msg for this
                    self.tx.send(LayerManageAction::Push(
                        Layers::Analysis.into_push_config(false),
                    ));
                    status.consumed();
                }
                KeyCode::Char('T') => {
                    self.tx.send(LayerManageAction::Push(
                        Layers::Transaction(None).into_push_config(false),
                    ));
                    status.consumed();
                }
                KeyCode::Char('q') => {
                    self.tx.send(Action::Quit);
                    status.consumed();
                }
                _ => {}
            }
        }
        status
    }
}

impl Layer for Home {}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;
    use ratatui::{Terminal, backend::TestBackend};
    use tokio::sync::mpsc;

    use crate::actions::Action;

    use super::*;

    fn get_test_page() -> Home {
        let mut home = Home {
            tx: ActionSender(tokio::sync::mpsc::unbounded_channel().0),
        };
        home.init();
        home
    }

    #[test]
    fn test_render() {
        let mut page = get_test_page();
        let mut terminal = Terminal::new(TestBackend::new(80, 25)).unwrap();
        terminal
            .draw(|frame| page.render(frame, frame.area()))
            .unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_render_large() {
        let mut page = get_test_page();
        let mut terminal = Terminal::new(TestBackend::new(100, 25)).unwrap();
        terminal
            .draw(|frame| page.render(frame, frame.area()))
            .unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_render_small() {
        let mut page = get_test_page();
        let mut terminal = Terminal::new(TestBackend::new(40, 25)).unwrap();
        terminal
            .draw(|frame| page.render(frame, frame.area()))
            .unwrap();
        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn test_events() {
        let (tx, mut _rx) = mpsc::unbounded_channel::<Action>();
        let mut home = Home { tx: tx.into() };
        home.handle_event_with_status_check(&'?'.into());
        let mut should_receive_layer_opt = false;
        while let Ok(action) = _rx.try_recv() {
            if let Action::Layer(LayerManageAction::Push(act)) = action {
                assert!(matches!(act.layer, Layers::Help(_)));
                should_receive_layer_opt = true;
            }
        }
        assert!(should_receive_layer_opt);
    }
    #[test]
    fn test_event_nav_to_analysis() {
        let (tx, mut rx) = mpsc::unbounded_channel::<Action>();
        let mut home = Home { tx: tx.into() };
        home.handle_event_with_status_check(&'a'.into());
        let mut should_receive_layer_opt = false;
        while let Ok(action) = rx.try_recv() {
            if let Action::Layer(LayerManageAction::Push(act)) = action {
                assert!(matches!(act.layer, Layers::Analysis));
                should_receive_layer_opt = true;
            }
        }
        assert!(should_receive_layer_opt);
    }
    #[test]
    fn test_event_nav_to_transactions() {
        let (tx, mut rx) = mpsc::unbounded_channel::<Action>();
        let mut home = Home { tx: tx.into() };
        home.handle_event_with_status_check(&'T'.into());
        let mut should_receive_layer_opt = false;
        while let Ok(action) = rx.try_recv() {
            if let Action::Layer(LayerManageAction::Push(act)) = action {
                assert!(matches!(act.layer, Layers::Transaction(_)));
                should_receive_layer_opt = true;
            }
        }
        assert!(should_receive_layer_opt);
    }
}
