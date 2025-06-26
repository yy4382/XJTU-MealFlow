//! # 认证信息输入页面模块
//!
//! 提供用户账号和 Cookie 信息的输入界面，用于配置 XJTU 校园卡系统的认证信息。
//! 支持数据持久化存储和自动格式化处理。
//!
//! ## 功能特性
//!
//! - **双输入框设计**: 分别输入学号和 Hallticket Cookie
//! - **焦点切换**: 键盘导航支持在输入框间切换
//! - **数据持久化**: 自动保存输入信息到本地数据库
//! - **格式化处理**: 自动处理 Cookie 格式，支持有无 `hallticket=` 前缀
//! - **数据回显**: 从数据库加载已保存的认证信息
//!
//! ## 页面布局
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │ ┌─ Account ──────────────────────────────────────────┐ │
//! │ │ [用户学号输入框]                                   │ │
//! │ │                                                    │ │
//! │ └────────────────────────────────────────────────────┘ │
//! │                                                         │
//! │ ┌─ Hallticket ───────────────────────────────────────┐ │
//! │ │ [Cookie 输入框]                                    │ │
//! │ │                                                    │ │
//! │ └────────────────────────────────────────────────────┘ │
//! ├─────────────────────────────────────────────────────────┤
//! │ 快捷键帮助信息                                          │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! ## 认证信息获取方法
//!
//! ### 1. 登录 XJTU 校园卡系统
//!
//! 访问 `http://card.xjtu.edu.cn` 并使用统一身份认证登录
//!
//! ### 2. 获取 Cookie 信息
//!
//! 在浏览器开发者工具中：
//! 1. 打开 Network 标签页
//! 2. 访问任意校园卡功能页面
//! 3. 找到请求头中的 `Cookie` 字段
//! 4. 复制 `hallticket=xxx` 部分的值
//!
//! ### 3. 获取学号信息
//!
//! 学号通常显示在校园卡系统的用户信息页面
//!
//! ## Cookie 格式处理
//!
//! 模块支持多种 Cookie 输入格式：
//!
//! ```text
//! 输入格式1: "abc123def456"          → 存储为: "hallticket=abc123def456"
//! 输入格式2: "hallticket=abc123def456" → 存储为: "hallticket=abc123def456"
//! ```
//!
//! ## 焦点状态管理
//!
//! ```rust
//! enum Focus {
//!     Account,    // 学号输入框获得焦点
//!     Cookie,     // Cookie 输入框获得焦点
//! }
//! ```
//!
//! ## 键盘快捷键
//!
//! | 按键 | 功能 |
//! |------|------|
//! | `j`/`k` | 在输入框间切换焦点 |
//! | `Esc` | 返回数据获取页面 |
//! | `?` | 显示帮助信息 |
//!
//! ## 数据存储
//!
//! 输入的认证信息自动保存到 SQLite 数据库的 `cookies` 表：
//!
//! ```sql
//! CREATE TABLE cookies (
//!     account TEXT PRIMARY KEY,    -- 学号
//!     cookie TEXT NOT NULL        -- 完整的 Cookie 字符串
//! );
//! ```
//!
//! ## 使用流程
//!
//! 1. 用户打开认证信息输入页面
//! 2. 系统自动加载已保存的认证信息（如果存在）
//! 3. 用户可以修改学号和 Cookie 信息
//! 4. 输入完成后自动保存到数据库
//! 5. 返回数据获取页面继续操作
//!
//! ## 错误处理
//!
//! - Cookie 格式自动标准化，无需用户手动添加前缀
//! - 数据库操作失败时会显示错误信息
//! - 空输入会被忽略，不会覆盖已有数据
//!
//! ## 安全注意事项
//!
//! - Cookie 信息包含敏感的会话数据
//! - 建议定期更新 Cookie（会话过期时）
//! - 本地存储使用明文，注意保护数据库文件

use crossterm::event::KeyCode;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::layout::{Constraint, Layout};

use crate::actions::ActionSender;
use crate::actions::LayerManageAction;
use crate::actions::Layers;
use crate::app::layer_manager::EventHandlingStatus;
use crate::component::input::{InputComp, InputMode};
use crate::libs::transactions::TransactionManager;
use crate::utils::help_msg::{HelpEntry, HelpMsg};

use super::{EventLoopParticipant, Layer, WidgetExt};

#[derive(Clone, Debug)]
pub struct CookieInput {
    state: Focus,
    manager: TransactionManager,

    tx: ActionSender,

    cookie_input: InputComp,
    account_input: InputComp,
}

impl CookieInput {
    pub fn new(action_tx: ActionSender, manager: TransactionManager) -> Self {
        let (account, mut cookie) = manager.get_account_cookie_may_empty().unwrap_or_default();
        if cookie.starts_with("hallticket=") {
            cookie.replace_range(..11, "");
        }
        Self {
            state: Default::default(),
            manager,
            cookie_input: InputComp::new().init_text(cookie).title("Hallticket"),
            account_input: InputComp::new().init_text(account).title("Account"),
            tx: action_tx,
        }
    }

    pub fn get_help_msg(&self) -> crate::utils::help_msg::HelpMsg {
        let help_msg: HelpMsg = vec![
            HelpEntry::new_plain("hjkl", "Move focus"),
            HelpEntry::new('?', "Help"),
            HelpEntry::new(KeyCode::Esc, "Back"),
        ]
        .into();
        match self.state {
            Focus::Account => help_msg.extend_ret(&self.account_input.get_help_msg()),
            Focus::Cookie => help_msg.extend_ret(&self.cookie_input.get_help_msg()),
        }
    }
}

#[derive(Default, Clone, Debug)]
pub enum Focus {
    #[default]
    Account,
    Cookie,
}

impl Focus {
    fn next(&self) -> Focus {
        match self {
            Focus::Account => Focus::Cookie,
            Focus::Cookie => Focus::Account,
        }
    }
    fn prev(&self) -> Focus {
        match self {
            Focus::Account => Focus::Cookie,
            Focus::Cookie => Focus::Account,
        }
    }
}

impl WidgetExt for CookieInput {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = &Layout::default()
            .constraints([Constraint::Fill(1), Constraint::Length(3)])
            .split(area);

        let sub_chunks = &Layout::default()
            .margin(1)
            .constraints([Constraint::Length(5), Constraint::Length(5)])
            .split(chunks[0]);

        self.account_input.render(frame, sub_chunks[0]);
        self.cookie_input.render(frame, sub_chunks[1]);

        self.get_help_msg().render(frame, chunks[1]);
    }
}

impl EventLoopParticipant for CookieInput {
    fn handle_events(&mut self, event: &crate::tui::Event) -> EventHandlingStatus {
        let mut status = EventHandlingStatus::default();

        let (account_state, account_result) = self.account_input.handle_events(event);
        if let Some(result) = account_result {
            self.manager.update_account(&result).unwrap();
        }
        if matches!(account_state, EventHandlingStatus::Consumed) {
            return account_state;
        }

        let (cookie_state, cookie_result) = self.cookie_input.handle_events(event);
        if let Some(result) = cookie_result {
            if !result.is_empty() {
                if result.starts_with("hallticket=") {
                    self.manager.update_cookie(&result).unwrap();
                } else {
                    self.manager
                        .update_cookie(&format!("hallticket={}", result))
                        .unwrap();
                }
            }
        }
        if matches!(cookie_state, EventHandlingStatus::Consumed) {
            return cookie_state;
        }

        if let crate::tui::Event::Key(key) = &event {
            match (key.modifiers, key.code) {
                (_, KeyCode::Char('k')) => {
                    self.change_focus(self.state.prev());
                    status.consumed();
                }
                (_, KeyCode::Char('j')) => {
                    self.change_focus(self.state.next());
                    status.consumed();
                }
                (_, KeyCode::Esc) => self.tx.send(LayerManageAction::Swap(Layers::Fetch)),
                (_, KeyCode::Char('?')) => {
                    self.tx.send(LayerManageAction::Push(
                        Layers::Help(self.get_help_msg()).into_push_config(true),
                    ));
                }
                _ => (),
            }
        };
        status
    }
}

impl Layer for CookieInput {
    fn init(&mut self) {
        self.account_input.set_mode(InputMode::Focused);
    }
}

impl CookieInput {
    fn change_focus(&mut self, focus: Focus) {
        self.state = focus.clone();

        self.account_input
            .set_mode(if matches!(self.state, Focus::Account) {
                InputMode::Focused
            } else {
                InputMode::Idle
            });

        self.cookie_input
            .set_mode(if matches!(self.state, Focus::Cookie) {
                InputMode::Focused
            } else {
                InputMode::Idle
            });
    }
}

#[cfg(test)]
mod test {

    use insta::assert_snapshot;
    use ratatui::backend::TestBackend;
    use tokio::sync::mpsc::{self, UnboundedReceiver};

    use super::*;
    use crate::{actions::Action, tui::Event};

    fn get_test_objs() -> (UnboundedReceiver<Action>, CookieInput) {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut page = CookieInput::new(tx.clone().into(), TransactionManager::new(None).unwrap());
        page.init();
        (rx, page)
    }

    #[test]
    fn test_navigation() {
        let (_, mut page) = get_test_objs();
        assert!(matches!(page.state, Focus::Account));
        assert!(matches!(page.account_input.mode, InputMode::Focused));

        page.handle_event_with_status_check(&'j'.into());
        assert!(matches!(page.state, Focus::Cookie));

        page.handle_event_with_status_check(&'j'.into());
        assert!(matches!(page.state, Focus::Account));

        page.handle_event_with_status_check(&'k'.into());
        assert!(matches!(page.state, Focus::Cookie));

        page.handle_event_with_status_check(&'k'.into());
        assert!(matches!(page.state, Focus::Account));
    }

    #[test]
    fn test_account_input() {
        let (_, mut page) = get_test_objs();

        page.handle_event_with_status_check(&KeyCode::Enter.into());
        assert!(page.account_input.is_inputting());
        page.handle_event_with_status_check(&'a'.into());
        page.handle_event_with_status_check(&'j'.into());
        page.handle_event_with_status_check(&KeyCode::Enter.into());
        assert_eq!(page.manager.get_account_cookie_may_empty().unwrap().0, "aj");

        page.handle_event_with_status_check(&KeyCode::Enter.into());
        page.handle_event_with_status_check(&KeyCode::Left.into());
        page.handle_event_with_status_check(&Event::Paste("kl".into()));
        page.handle_event_with_status_check(&KeyCode::Enter.into());
        assert_eq!(
            page.manager.get_account_cookie_may_empty().unwrap().0,
            // cSpell:ignore aklj
            "aklj"
        );
    }

    #[test]
    fn test_cookie_input() {
        let (_, mut page) = get_test_objs();

        page.handle_event_with_status_check(&'j'.into());
        page.handle_event_with_status_check(&KeyCode::Enter.into());
        assert!(page.cookie_input.is_inputting());
        page.handle_event_with_status_check(&'a'.into());
        page.handle_event_with_status_check(&'j'.into());
        page.handle_event_with_status_check(&KeyCode::Enter.into());
        assert_eq!(
            page.manager.get_account_cookie_may_empty().unwrap().1,
            "hallticket=aj"
        );
    }

    #[test]
    fn test_cookie_get_strip() {
        let manager = TransactionManager::new(None).unwrap();
        manager.update_cookie("hallticket=abc").unwrap();
        let (tx, _) = mpsc::unbounded_channel();
        let mut page = CookieInput::new(tx.clone().into(), manager);
        page.init();
        assert_eq!(page.cookie_input.get_text(), "abc");
    }

    #[test]
    fn test_cookie_input_render() {
        let (_, mut page) = get_test_objs();
        let mut terminal = ratatui::Terminal::new(TestBackend::new(80, 20)).unwrap();

        terminal
            .draw(|f| {
                page.render(f, f.area());
            })
            .unwrap();

        assert_snapshot!(terminal.backend())
    }
}
