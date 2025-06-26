//! # 终端用户界面模块
//!
//! 该模块提供基于终端的用户界面功能，使用 [ratatui](https://ratatui.rs/)
//! 和 [crossterm](https://crates.io/crates/crossterm) 库构建富文本终端界面。
//!
//! ## 主要功能
//!
//! - **事件处理**: 键盘、鼠标、系统事件的统一处理
//! - **异步架构**: 基于tokio的异步事件循环
//! - **跨平台支持**: 通过crossterm支持多种终端
//! - **测试支持**: 提供测试用的TUI实现
//!
//! ## 架构设计
//!
//! ```text
//! ┌─────────────────┐    ┌─────────────────┐
//! │   TuiEnum       │───▶│  Event Stream   │
//! │ (统一接口)       │    │  (异步事件)      │
//! └─────────────────┘    └─────────────────┘
//!          │                       │
//!          ▼                       ▼
//! ┌─────────────────┐    ┌─────────────────┐
//! │   Crossterm     │    │   TestBackend   │
//! │ (生产环境)       │    │  (测试环境)      │
//! └─────────────────┘    └─────────────────┘
//! ```
//!
//! ## 事件类型
//!
//! - **键盘事件**: 按键按下、释放
//! - **鼠标事件**: 点击、移动、滚轮
//! - **系统事件**: 窗口大小变化、焦点变化
//! - **应用事件**: 渲染、退出等内部事件
//!
//! ## 使用示例
//!
//! ```rust
//! use crate::tui::{Tui, Event};
//!
//! let mut tui = Tui::new()?;
//! tui.enter()?;
//!
//! loop {
//!     let event = tui.next().await?;
//!     match event {
//!         Event::Key(key) => {
//!             // 处理键盘事件
//!         }
//!         Event::Render => {
//!             // 渲染界面
//!         }
//!         _ => {}
//!     }
//! }
//!
//! tui.exit()?;
//! ```

use std::{
    io::{Stderr, stderr},
    ops::{Deref, DerefMut},
    time::Duration,
};

use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use futures::{FutureExt, StreamExt};
use ratatui::{
    Frame,
    backend::{CrosstermBackend, TestBackend},
    crossterm::{
        cursor,
        event::{
            DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
            Event as CrosstermEvent, KeyEvent, KeyEventKind, MouseEvent,
        },
        terminal::{EnterAlternateScreen, LeaveAlternateScreen},
    },
};
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

/// TUI事件枚举
///
/// 定义了应用程序中所有可能的事件类型，包括用户输入事件和系统事件。
/// 这个枚举统一了不同来源的事件，提供一致的事件处理接口。
#[derive(Clone, Debug)]
pub enum Event {
    /// 应用程序初始化事件
    ///
    /// 在TUI启动时发送，用于触发初始化逻辑
    Init,

    /// 错误事件
    ///
    /// 当系统发生错误时发送，通常导致应用程序退出
    Error,

    /// Tick事件
    ///
    /// 定时发送的事件，用于驱动动画和定期更新
    Tick,

    /// 渲染事件
    ///
    /// 触发界面重新渲染
    Render,

    /// 焦点获得事件
    ///
    /// 当终端窗口获得焦点时发送
    FocusGained,

    /// 焦点失去事件
    ///
    /// 当终端窗口失去焦点时发送
    FocusLost,

    /// 文本粘贴事件
    ///
    /// 当用户粘贴文本时发送，包含粘贴的文本内容
    Paste(String),

    /// 键盘按键事件
    ///
    /// 当用户按下键盘按键时发送
    Key(KeyEvent),

    /// 鼠标事件
    ///
    /// 当用户进行鼠标操作时发送（当前未使用）
    #[allow(dead_code)]
    Mouse(MouseEvent),

    /// 终端大小调整事件
    ///
    /// 当终端窗口大小变化时发送，包含新的宽度和高度
    #[allow(dead_code)]
    Resize(u16, u16),
}

impl From<KeyCode> for Event {
    fn from(value: KeyCode) -> Self {
        Event::Key(KeyEvent::new(value, KeyModifiers::NONE))
    }
}
impl From<char> for Event {
    fn from(value: char) -> Self {
        Event::Key(KeyEvent::new(KeyCode::Char(value), KeyModifiers::NONE))
    }
}

/// TUI统一接口枚举
///
/// 为不同的TUI后端提供统一的接口，支持生产环境和测试环境的切换。
/// 这种设计允许在不同环境下使用相同的代码逻辑。
pub enum TuiEnum {
    /// Crossterm后端（生产环境）
    ///
    /// 使用真实的终端进行交互，支持完整的TUI功能
    Crossterm(Tui),

    /// 测试后端（测试环境）
    ///
    /// 使用内存中的虚拟终端，用于单元测试和集成测试
    Test(TestTui),
}

impl From<Tui> for TuiEnum {
    fn from(tui: Tui) -> Self {
        TuiEnum::Crossterm(tui)
    }
}
impl From<TestTui> for TuiEnum {
    fn from(tui: TestTui) -> Self {
        TuiEnum::Test(tui)
    }
}
impl TuiEnum {
    /// 进入TUI模式
    ///
    /// 初始化终端设置，切换到备用屏幕缓冲区
    pub fn enter(&mut self) -> Result<()> {
        match self {
            TuiEnum::Crossterm(tui) => tui.enter(),
            TuiEnum::Test(_) => Ok(()),
        }
    }

    /// 退出TUI模式
    ///
    /// 恢复终端设置，切换回主屏幕缓冲区
    pub fn exit(&mut self) -> Result<()> {
        match self {
            TuiEnum::Crossterm(tui) => tui.exit(),
            TuiEnum::Test(_) => Ok(()),
        }
    }

    /// 获取下一个事件
    ///
    /// 异步等待下一个事件的到达
    pub async fn next(&mut self) -> Result<Event> {
        match self {
            TuiEnum::Crossterm(tui) => tui.next().await,
            TuiEnum::Test(_) => Ok(Event::Tick),
        }
    }

    /// 绘制界面
    ///
    /// 使用提供的闭包绘制当前帧
    pub fn draw(&mut self, f: impl FnOnce(&mut Frame)) -> Result<()> {
        match self {
            TuiEnum::Crossterm(tui) => tui.draw(f).map(|_| ()).map_err(Into::into),
            TuiEnum::Test(tui) => tui.draw(f).map(|_| ()).map_err(Into::into),
        }
    }
}

/// 生产环境TUI实现
///
/// 基于Crossterm的真实终端用户界面实现，提供完整的TUI功能。
/// 包含异步事件循环、终端管理和资源清理。
pub struct Tui {
    /// Ratatui终端实例
    pub terminal: ratatui::Terminal<CrosstermBackend<Stderr>>,
    /// 异步任务句柄
    pub task: JoinHandle<()>,
    /// 取消令牌，用于停止事件循环
    pub cancellation_token: CancellationToken,
    /// 事件接收器
    pub event_rx: UnboundedReceiver<Event>,
    /// 事件发送器
    pub event_tx: UnboundedSender<Event>,
    /// 帧率（每秒帧数）
    pub frame_rate: f64,
    /// Tick率（每秒tick数）
    pub tick_rate: f64,
    /// 是否启用鼠标支持
    pub mouse: bool,
    /// 是否启用粘贴支持
    pub paste: bool,
}

impl Tui {
    /// 创建新的TUI实例
    ///
    /// 使用默认配置创建TUI，包括标准的帧率和tick率设置。
    ///
    /// # 返回值
    ///
    /// 成功时返回配置好的TUI实例，失败时返回错误
    pub fn new() -> Result<Self> {
        let tick_rate = 4.0;
        let frame_rate = 60.0;
        let terminal = ratatui::Terminal::new(CrosstermBackend::new(stderr()))?;
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let cancellation_token = CancellationToken::new();
        let task = tokio::spawn(async {});
        let mouse = false;
        let paste = false;
        Ok(Self {
            terminal,
            task,
            cancellation_token,
            event_rx,
            event_tx,
            frame_rate,
            tick_rate,
            mouse,
            paste,
        })
    }

    /// 设置tick率
    ///
    /// # 参数
    ///
    /// * `tick_rate` - 每秒tick数，影响动画和定时更新的频率
    pub fn tick_rate(mut self, tick_rate: f64) -> Self {
        self.tick_rate = tick_rate;
        self
    }

    /// 设置帧率
    ///
    /// # 参数
    ///
    /// * `frame_rate` - 每秒帧数，影响渲染的流畅度
    pub fn frame_rate(mut self, frame_rate: f64) -> Self {
        self.frame_rate = frame_rate;
        self
    }

    /// 启用或禁用鼠标支持
    ///
    /// # 参数
    ///
    /// * `mouse` - 是否启用鼠标事件捕获
    #[allow(dead_code)]
    pub fn mouse(mut self, mouse: bool) -> Self {
        self.mouse = mouse;
        self
    }

    /// 启用或禁用粘贴支持
    ///
    /// # 参数
    ///
    /// * `paste` - 是否启用括号粘贴模式
    #[allow(dead_code)]
    pub fn paste(mut self, paste: bool) -> Self {
        self.paste = paste;
        self
    }

    /// 启动异步事件循环
    ///
    /// 创建后台任务来处理终端事件，包括键盘输入、鼠标操作和定时事件。
    /// 事件通过内部通道传递给主应用程序。
    pub fn start(&mut self) {
        let tick_delay = std::time::Duration::from_secs_f64(1.0 / self.tick_rate);
        let render_delay = std::time::Duration::from_secs_f64(1.0 / self.frame_rate);
        self.cancel();
        self.cancellation_token = CancellationToken::new();
        let _cancellation_token = self.cancellation_token.clone();
        let _event_tx = self.event_tx.clone();
        self.task = tokio::spawn(async move {
            let mut reader = crossterm::event::EventStream::new();
            let mut tick_interval = tokio::time::interval(tick_delay);
            let mut render_interval = tokio::time::interval(render_delay);
            _event_tx.send(Event::Init).unwrap();
            loop {
                let tick_delay = tick_interval.tick();
                let render_delay = render_interval.tick();
                let crossterm_event = reader.next().fuse();
                tokio::select! {
                  _ = _cancellation_token.cancelled() => {
                    break;
                  }
                  maybe_event = crossterm_event => {
                    match maybe_event {
                      Some(Ok(evt)) => {
                        match evt {
                          CrosstermEvent::Key(key) => {
                            if key.kind == KeyEventKind::Press {
                              _event_tx.send(Event::Key(key)).unwrap();
                            }
                          },
                          CrosstermEvent::Mouse(mouse) => {
                            _event_tx.send(Event::Mouse(mouse)).unwrap();
                          },
                          CrosstermEvent::Resize(x, y) => {
                            _event_tx.send(Event::Resize(x, y)).unwrap();
                          },
                          CrosstermEvent::FocusLost => {
                            _event_tx.send(Event::FocusLost).unwrap();
                          },
                          CrosstermEvent::FocusGained => {
                            _event_tx.send(Event::FocusGained).unwrap();
                          },
                          CrosstermEvent::Paste(s) => {
                            _event_tx.send(Event::Paste(s)).unwrap();
                          }
                        }
                      }
                      Some(Err(_)) => {
                        _event_tx.send(Event::Error).unwrap();
                      }
                      None => {},
                    }
                  },
                  _ = tick_delay => {
                      _event_tx.send(Event::Tick).unwrap();
                  },
                  _ = render_delay => {
                      _event_tx.send(Event::Render).unwrap();
                  },
                }
            }
        });
    }

    /// 停止事件循环
    ///
    /// 发送取消信号并等待后台任务完成，包含超时机制避免无限等待。
    pub fn stop(&self) -> Result<()> {
        self.cancel();
        let mut counter = 0;
        while !self.task.is_finished() {
            std::thread::sleep(Duration::from_millis(1));
            counter += 1;
            if counter > 50 {
                self.task.abort();
            }
            if counter > 100 {
                tracing::error!("Failed to abort task in 100 milliseconds for unknown reason");
                break;
            }
        }
        Ok(())
    }

    /// 进入TUI模式
    ///
    /// 设置终端为原始模式，启用备用屏幕缓冲区，并启动事件循环。
    pub fn enter(&mut self) -> Result<()> {
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(std::io::stderr(), EnterAlternateScreen, cursor::Hide)?;
        if self.mouse {
            crossterm::execute!(std::io::stderr(), EnableMouseCapture)?;
        }
        if self.paste {
            crossterm::execute!(std::io::stderr(), EnableBracketedPaste)?;
        }
        self.start();
        Ok(())
    }

    /// 退出TUI模式
    ///
    /// 停止事件循环，恢复终端设置，禁用备用屏幕缓冲区。
    pub fn exit(&mut self) -> Result<()> {
        self.stop()?;
        if crossterm::terminal::is_raw_mode_enabled()? {
            self.flush()?;
            if self.paste {
                crossterm::execute!(std::io::stderr(), DisableBracketedPaste)?;
            }
            if self.mouse {
                crossterm::execute!(std::io::stderr(), DisableMouseCapture)?;
            }
            crossterm::execute!(std::io::stderr(), LeaveAlternateScreen, cursor::Show)?;
            crossterm::terminal::disable_raw_mode()?;
        }
        Ok(())
    }

    /// 取消事件循环
    ///
    /// 发送取消信号给后台任务
    pub fn cancel(&self) {
        self.cancellation_token.cancel();
    }

    /// 暂停TUI
    ///
    /// 退出TUI模式并发送SIGTSTP信号暂停进程（仅Unix系统）
    #[allow(dead_code)]
    pub fn suspend(&mut self) -> Result<()> {
        self.exit()?;
        #[cfg(not(windows))]
        signal_hook::low_level::raise(signal_hook::consts::signal::SIGTSTP)?;
        Ok(())
    }

    /// 恢复TUI
    ///
    /// 重新进入TUI模式
    #[allow(dead_code)]
    pub fn resume(&mut self) -> Result<()> {
        self.enter()?;
        Ok(())
    }

    /// 获取下一个事件
    ///
    /// 异步等待下一个事件从事件循环中到达
    pub async fn next(&mut self) -> Result<Event> {
        self.event_rx
            .recv()
            .await
            .ok_or(color_eyre::eyre::eyre!("Unable to get event"))
    }
}

impl Deref for Tui {
    type Target = ratatui::Terminal<CrosstermBackend<Stderr>>;

    fn deref(&self) -> &Self::Target {
        &self.terminal
    }
}

impl DerefMut for Tui {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.terminal
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        self.exit().unwrap();
    }
}

/// 测试环境TUI实现
///
/// 基于内存的虚拟终端，用于单元测试和集成测试。
/// 不需要真实的终端环境，可以在CI/CD环境中运行。
pub struct TestTui {
    /// 测试用的虚拟终端
    pub terminal: ratatui::Terminal<TestBackend>,
}

impl TestTui {
    /// 创建新的测试TUI实例
    ///
    /// 使用固定大小的虚拟终端（80x25字符）
    #[cfg(test)]
    pub fn new() -> Self {
        let terminal = ratatui::Terminal::new(TestBackend::new(80, 25)).unwrap();
        Self { terminal }
    }
}

impl Deref for TestTui {
    type Target = ratatui::Terminal<TestBackend>;

    fn deref(&self) -> &Self::Target {
        &self.terminal
    }
}

impl DerefMut for TestTui {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.terminal
    }
}

#[cfg(test)]
impl TuiEnum {
    /// 获取测试后端的引用（仅测试环境）
    ///
    /// 用于测试中访问虚拟终端的内容进行断言
    pub fn backend(&self) -> &TestBackend {
        match self {
            TuiEnum::Crossterm(_) => panic!("Not a test backend"),
            TuiEnum::Test(tui) => tui.backend(),
        }
    }
}
