//! # 输入组件模块
//!
//! 提供可重用的文本输入组件，支持多种输入模式、按键配置和状态管理。
//! 组件集成了 TUI 渲染、事件处理和帮助系统。
//!
//! ## 核心特性
//!
//! - **状态驱动**: 基于状态机的输入模式管理
//! - **可配置按键**: 支持自定义进入、提交和退出按键
//! - **自动提交**: 可配置实时提交或手动提交模式
//! - **帮助集成**: 自动生成上下文相关的帮助信息
//! - **事件处理**: 完整的键盘和粘贴事件支持
//!
//! ## 输入状态机
//!
//! ```text
//! ┌─────────┐    Focus     ┌─────────┐    Enter     ┌─────────────┐
//! │  Idle   │─────────────→│ Focused │─────────────→│ Inputting   │
//! └─────────┘              └─────────┘              └─────────────┘
//!                               ↑                          │
//!                               │           Exit           │
//!                               │         ←─────────────── │
//!                               │           Submit         │
//!                               └─────────────────────────────┘
//! ```
//!
//! ### 状态说明
//!
//! - **Idle**: 初始状态，组件不响应输入
//! - **Focused**: 获得焦点，显示边框高亮，等待用户按 Enter 开始输入
//! - **Inputting**: 输入模式，接收文本输入和控制命令
//!
//! ## 输入模式
//!
//! ### 手动提交模式（默认）
//! ```rust
//! let input = InputComp::new()
//!     .title("Enter your name")
//!     .auto_submit(false);  // 手动提交模式
//! ```
//!
//! 行为特点：
//! - 用户输入文本后需要按 Enter 确认提交
//! - 支持 Esc 键取消输入，恢复原始值
//! - 适用于重要输入，需要用户明确确认
//!
//! ### 自动提交模式
//! ```rust
//! let input = InputComp::new()
//!     .title("Search")
//!     .auto_submit(true);   // 自动提交模式
//! ```
//!
//! 行为特点：
//! - 每次按键都会触发提交事件
//! - 适用于搜索框、实时筛选等场景
//! - 只有 Enter 键可以退出输入模式
//!
//! ## 按键配置
//!
//! 支持自定义按键绑定：
//!
//! ```rust
//! use crossterm::event::KeyCode;
//! use crate::utils::key_events::KeyEvent;
//!
//! let input = InputComp::new()
//!     .enter_keys(vec![KeyCode::Enter.into(), KeyCode::Tab.into()])
//!     .submit_keys(vec![KeyCode::Enter.into()])
//!     .exit_keys(vec![KeyCode::Esc.into(), KeyCode::F10.into()]);
//! ```
//!
//! ### 默认按键绑定
//!
//! | 功能 | 按键 | 说明 |
//! |------|------|------|
//! | 进入输入 | Enter | 从 Focused 进入 Inputting |
//! | 提交输入 | Enter | 完成输入并返回结果 |
//! | 取消输入 | Esc | 放弃输入并恢复原值 |
//!
//! ## 事件处理
//!
//! 组件返回二元组 `(EventHandlingStatus, Option<String>)`：
//!
//! ```rust
//! let (status, result) = input.handle_events(&event);
//!
//! match status {
//!     EventHandlingStatus::Consumed => {
//!         // 事件被组件消费，停止传播
//!     }
//!     EventHandlingStatus::Propagate => {
//!         // 事件未被处理，继续传播
//!     }
//! }
//!
//! if let Some(text) = result {
//!     // 用户完成了输入，处理结果
//!     println!("User input: {}", text);
//! }
//! ```
//!
//! ## 渲染样式
//!
//! 组件根据当前状态自动调整视觉样式：
//!
//! ```text
//! Idle:       ┌─ Title ─────────────┐
//!             │                     │
//!             └─────────────────────┘
//!
//! Focused:    ┌─ Title ─────────────┐  (青色边框)
//!             │ [cursor]            │
//!             └─────────────────────┘
//!
//! Inputting:  ┌─ Title ─────────────┐  (绿色边框)
//!             │ Hello World[cursor] │
//!             └─────────────────────┘
//! ```
//!
//! ## 使用示例
//!
//! ### 基本输入框
//! ```rust
//! use crate::component::input::{InputComp, InputMode};
//!
//! // 创建输入组件
//! let mut input = InputComp::new()
//!     .title("Enter username")
//!     .init_text("default_value");
//!
//! // 设置焦点
//! input.set_mode(InputMode::Focused);
//!
//! // 在事件循环中处理输入
//! let (status, result) = input.handle_events(&event);
//! if let Some(username) = result {
//!     println!("Username: {}", username);
//! }
//! ```
//!
//! ### 搜索框组件
//! ```rust
//! let mut search_box = InputComp::new()
//!     .title("Search transactions")
//!     .auto_submit(true)
//!     .init_text("");
//!
//! // 每次按键都会触发搜索
//! let (_, query) = search_box.handle_events(&event);
//! if let Some(search_term) = query {
//!     perform_search(&search_term);
//! }
//! ```
//!
//! ## 集成帮助系统
//!
//! 组件自动生成上下文相关的帮助信息：
//!
//! ```rust
//! // 根据当前状态获取帮助信息
//! let help = input.get_help_msg();
//!
//! // 与页面帮助信息合并
//! let mut page_help = get_page_help();
//! page_help.extend(&help);
//! ```
//!
//! ## 高级特性
//!
//! ### 粘贴支持
//! 组件支持文本粘贴事件，在自动提交模式下会立即触发提交。
//!
//! ### 状态查询
//! ```rust
//! if input.is_inputting() {
//!     // 当前正在输入状态
//! }
//!
//! let current_text = input.get_text();
//! ```
//!
//! ### 文本重置
//! 在取消输入时，组件会自动恢复到初始文本值。

use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
};
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::{
    app::layer_manager::EventHandlingStatus,
    page::WidgetExt,
    tui::Event,
    utils::{
        help_msg::{HelpEntry, HelpMsg},
        key_events::KeyEvent,
    },
};

#[derive(Clone, Debug)]
/// A input Component

pub(crate) struct InputComp {
    input: Input,
    pub mode: InputMode,

    title: Option<String>,

    auto_submit: bool,
    control_keys: InputCompCtrlKeys,
}

#[derive(Default, Clone, Debug)]
pub(crate) enum InputMode {
    #[default]
    Idle,
    Focused,
    Inputting,
}

#[derive(Clone, Debug)]
pub(crate) struct InputCompCtrlKeys {
    enter_keys: Vec<KeyEvent>,
    submit_keys: Vec<KeyEvent>,
    exit_keys: Vec<KeyEvent>,
}

impl Default for InputCompCtrlKeys {
    fn default() -> Self {
        Self {
            enter_keys: vec![KeyCode::Enter.into()],
            submit_keys: vec![KeyCode::Enter.into()],
            exit_keys: vec![KeyCode::Esc.into()],
        }
    }
}

impl InputComp {
    pub fn new() -> Self {
        Self {
            input: Input::default(),
            mode: InputMode::default(),
            title: Default::default(),
            auto_submit: false,
            control_keys: Default::default(),
        }
    }

    pub fn init_text<T: Into<String>>(self, text: T) -> Self {
        Self {
            input: Input::new(text.into()),
            ..self
        }
    }

    pub fn title<T: Into<String>>(self, title: T) -> Self {
        Self {
            title: Some(title.into()),
            ..self
        }
    }

    #[allow(dead_code)]
    pub fn enter_keys(mut self, enter_keys: Vec<KeyEvent>) -> Self {
        self.control_keys.enter_keys = enter_keys;
        self
    }
    #[allow(dead_code)]
    pub fn submit_keys(mut self, submit_keys: Vec<KeyEvent>) -> Self {
        self.control_keys.submit_keys = submit_keys;
        self
    }
    #[allow(dead_code)]
    pub fn exit_keys(mut self, exit_keys: Vec<KeyEvent>) -> Self {
        self.control_keys.exit_keys = exit_keys;
        self
    }

    pub fn auto_submit(self, b: bool) -> Self {
        Self {
            auto_submit: b,
            ..self
        }
    }

    pub fn get_help_msg(&self) -> HelpMsg {
        let mut msg = HelpMsg::default();
        match self.mode {
            InputMode::Idle => {}
            InputMode::Focused => {
                msg.push(HelpEntry::new(
                    self.control_keys.enter_keys[0].clone(),
                    "Start input",
                ));
            }
            InputMode::Inputting => {
                if self.auto_submit {
                    msg.push(HelpEntry::new(
                        self.control_keys.submit_keys[0].clone(),
                        "quit input",
                    ));
                } else {
                    msg.push(HelpEntry::new(
                        self.control_keys.exit_keys[0].clone(),
                        "quit input",
                    ));
                    msg.push(HelpEntry::new(
                        self.control_keys.submit_keys[0].clone(),
                        "submit input",
                    ));
                }
            }
        };
        msg
    }
}

impl InputComp {
    #[must_use]
    pub fn handle_events(
        &mut self,
        event: &crate::tui::Event,
    ) -> (EventHandlingStatus, Option<String>) {
        let mut status = EventHandlingStatus::default();
        let mut output_string: Option<String> = None;
        match self.mode {
            InputMode::Idle => (),
            InputMode::Focused => {
                if let Event::Key(key) = event {
                    if self.control_keys.enter_keys.contains(&(*key).into()) {
                        self.mode = InputMode::Inputting;
                        status.consumed();
                    }
                }
            }
            InputMode::Inputting => match event {
                Event::Key(key) => {
                    if self.control_keys.submit_keys.contains(&(*key).into()) {
                        output_string = Some(self.input.value().to_string());
                        self.mode = InputMode::Focused;
                        status.consumed();
                    } else if self.control_keys.exit_keys.contains(&(*key).into()) {
                        self.input.reset();
                        self.mode = InputMode::Focused;
                        status.consumed();
                    } else {
                        self.input.handle_event(&crossterm::event::Event::Key(*key));
                        if self.auto_submit {
                            output_string = Some(self.input.value().to_string());
                        }
                        status.consumed();
                    }
                }
                Event::Paste(s) => {
                    s.chars().for_each(|c| {
                        self.input.handle(tui_input::InputRequest::InsertChar(c));
                    });
                    if self.auto_submit {
                        output_string = Some(self.input.value().to_string());
                    }
                    status.consumed();
                }
                _ => (),
            },
        };
        (status, output_string)
    }

    pub fn set_mode(&mut self, mode: InputMode) {
        self.mode = mode;
    }
    pub fn is_inputting(&self) -> bool {
        matches!(self.mode, InputMode::Inputting)
    }
}

impl WidgetExt for InputComp {
    fn render(&mut self, frame: &mut Frame, area: ratatui::prelude::Rect) {
        let width = area.width.max(3) - 3;
        let scroll = self.input.visual_scroll(width as usize);
        let style = match self.mode {
            InputMode::Focused => Color::Cyan.into(),
            InputMode::Inputting => Color::Yellow.into(),
            InputMode::Idle => Style::default(),
        };

        let input_widget = Paragraph::new(self.input.value())
            .style(style)
            .scroll((0, scroll as u16))
            .block(
                match &self.title {
                    Some(title) => Block::default().title(title.as_str()),
                    None => Block::default(),
                }
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
            );
        frame.render_widget(input_widget, area);

        if matches!(self.mode, InputMode::Inputting) {
            // Ratatui hides the cursor unless it's explicitly set. Position the  cursor past the
            // end of the input text and one line down from the border to the input line
            let x = self.input.visual_cursor().max(scroll) - scroll + 1;
            frame.set_cursor_position((area.x + x as u16, area.y + 1))
        }
    }
}
#[cfg(test)]
impl InputComp {
    pub fn get_text(&self) -> String {
        self.input.value().to_string()
    }
}

#[cfg(test)]
pub mod test {
    use insta::assert_snapshot;
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    fn get_input(auto_submit: bool) -> InputComp {
        InputComp::new()
            .auto_submit(auto_submit)
            .title("Input Test")
    }

    impl InputComp {
        fn handle_seq(&mut self, seq: Vec<Event>) -> Option<String> {
            let mut output = None;
            seq.into_iter().for_each(|e| {
                let status = self.handle_events(&e);
                assert!(matches!(status.0, EventHandlingStatus::Consumed));
                if let Some(s) = status.1 {
                    output = Some(s);
                }
            });
            output
        }
    }

    #[test]
    fn input_mode_change() {
        let mut input = get_input(false);
        assert!(matches!(input.mode, InputMode::Idle));
        input.set_mode(InputMode::Focused);
        let (s, _) = input.handle_events(&KeyCode::Enter.into());
        assert!(matches!(s, EventHandlingStatus::Consumed));
        assert!(matches!(input.mode, InputMode::Inputting));
        let (s, _) = input.handle_events(&KeyCode::Enter.into());
        assert!(matches!(s, EventHandlingStatus::Consumed));
        assert!(matches!(input.mode, InputMode::Focused));
    }

    #[test]
    fn test_input() {
        let mut input = get_input(false);
        input.set_mode(InputMode::Focused);

        let seq: Vec<Event> = vec![
            KeyCode::Enter.into(),
            'a'.into(),
            'b'.into(),
            KeyCode::Enter.into(),
        ];
        let output = input.handle_seq(seq.to_vec());

        assert_eq!(output, Some("ab".to_string()));

        let seq = [
            KeyCode::Enter.into(),
            KeyCode::Left.into(),
            'c'.into(),
            KeyCode::Enter.into(),
        ];
        let output = input.handle_seq(seq.to_vec());

        assert_eq!(output, Some("acb".to_string()));
    }

    #[test]
    fn test_input_auto_submit() {
        let mut input = get_input(true);
        input.set_mode(InputMode::Focused);

        let seq: Vec<Event> = vec![KeyCode::Enter.into(), 'a'.into(), 'b'.into()];
        let output = input.handle_seq(seq.to_vec());

        assert_eq!(output, Some("ab".to_string()));

        let seq = [KeyCode::Left.into(), 'c'.into()];
        let output = input.handle_seq(seq.to_vec());

        assert_eq!(output, Some("acb".to_string()));
    }

    #[test]
    fn test_input_paste() {
        let mut input = get_input(false);
        input.set_mode(InputMode::Focused);

        let seq = [
            KeyCode::Enter.into(),
            'a'.into(),
            'b'.into(),
            KeyCode::Enter.into(),
        ];

        assert_eq!(input.handle_seq(seq.to_vec()), Some("ab".to_string()));

        let seq = [
            KeyCode::Enter.into(),
            KeyCode::Left.into(),
            Event::Paste("ccc".into()),
            KeyCode::Enter.into(),
        ];
        // cSpell:ignore acccb
        assert_eq!(input.handle_seq(seq.to_vec()), Some("acccb".to_string()));
    }

    #[test]
    fn test_input_paste_auto_commit() {
        let mut input = get_input(true);
        input.set_mode(InputMode::Focused);

        let seq = [KeyCode::Enter.into(), Event::Paste("ccc".into())];

        assert_eq!(input.handle_seq(seq.to_vec()), Some("ccc".to_string()));
    }

    fn get_buffer_color(t: &Terminal<TestBackend>) -> Color {
        let cell = t
            .backend()
            .buffer()
            .content()
            .iter()
            .find(|&c| c.symbol() == "I")
            .unwrap();

        cell.fg
    }

    #[test]
    fn test_render() {
        let mut input = get_input(false);

        let mut terminal = Terminal::new(TestBackend::new(40, 10)).unwrap();
        terminal
            .draw(|frame| input.render(frame, frame.area()))
            .unwrap();
        assert_snapshot!(terminal.backend());
        assert_eq!(get_buffer_color(&terminal), Color::Reset);

        input.set_mode(InputMode::Focused);
        terminal.draw(|f| input.render(f, f.area())).unwrap();
        assert_eq!(get_buffer_color(&terminal), Color::Cyan);

        let seq = [KeyCode::Enter.into(), 'a'.into(), 'b'.into()];
        input.handle_seq(seq.to_vec());

        terminal.draw(|f| input.render(f, f.area())).unwrap();
        assert_eq!(get_buffer_color(&terminal), Color::Yellow);

        let _ = input.handle_events(&KeyCode::Enter.into());
        terminal.draw(|f| input.render(f, f.area())).unwrap();
        assert_snapshot!(terminal.backend());
        assert_eq!(get_buffer_color(&terminal), Color::Cyan);
    }

    #[test]
    fn test_help_msg() {
        let mut input = get_input(false);
        assert_eq!(input.get_help_msg().to_string(), "");

        input.set_mode(InputMode::Focused);
        assert_eq!(input.get_help_msg().to_string(), "Start input: enter");
        input.set_mode(InputMode::Inputting);
        assert_eq!(
            input.get_help_msg().to_string(),
            "quit input: esc | submit input: enter"
        );
    }

    #[test]
    fn test_help_msg_auto_commit() {
        let mut input = get_input(true);
        assert_eq!(input.get_help_msg().to_string(), "");

        input.set_mode(InputMode::Focused);
        assert_eq!(input.get_help_msg().to_string(), "Start input: enter");
        input.set_mode(InputMode::Inputting);
        assert_eq!(input.get_help_msg().to_string(), "quit input: enter");
    }

    #[test]
    fn test_key_configurations() {
        let mut input = get_input(false);
        input.set_mode(InputMode::Focused);

        // Test custom enter keys
        let custom_enter = vec![KeyCode::Char('e').into()];
        input = input.enter_keys(custom_enter.clone());
        let (s, _) = input.handle_events(&KeyCode::Enter.into());
        assert!(matches!(s, EventHandlingStatus::ShouldPropagate));
        let (s, _) = input.handle_events(&KeyCode::Char('e').into());
        assert!(matches!(s, EventHandlingStatus::Consumed));
        assert!(matches!(input.mode, InputMode::Inputting));

        // Test custom submit keys
        let custom_submit = vec![KeyCode::Char('s').into()];
        input = input.submit_keys(custom_submit.clone());
        let (s, output) = input.handle_events(&KeyCode::Char('s').into());
        assert!(matches!(s, EventHandlingStatus::Consumed));
        assert!(output.is_some());
        assert!(matches!(input.mode, InputMode::Focused));

        // Test custom exit keys
        input.set_mode(InputMode::Inputting);
        let custom_exit = vec![KeyCode::Char('q').into()];
        input = input.exit_keys(custom_exit.clone());
        let (s, _) = input.handle_events(&KeyCode::Esc.into());
        assert!(matches!(s, EventHandlingStatus::Consumed));
        let (s, _) = input.handle_events(&KeyCode::Char('q').into());
        assert!(matches!(s, EventHandlingStatus::Consumed));
        assert!(matches!(input.mode, InputMode::Focused));
    }
}
