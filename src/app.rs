//! # 应用程序核心模块
//!
//! 该模块包含应用程序的核心逻辑，包括：
//! - 应用程序状态管理 (`RootState`)
//! - 主应用程序结构 (`App`)
//! - 事件循环和动作处理
//! - 层级管理系统
//!
//! ## 架构概述
//!
//! 应用程序采用分层架构：
//! ```text
//! ┌─────────────────┐
//! │      App        │  <- 主应用程序，处理事件循环
//! ├─────────────────┤
//! │  LayerManager   │  <- 管理页面堆栈
//! ├─────────────────┤
//! │   RootState     │  <- 全局状态和配置
//! ├─────────────────┤
//! │      TUI        │  <- 终端用户界面
//! └─────────────────┘
//! ```
//!
//! ## 事件流程
//!
//! 1. TUI接收用户输入事件
//! 2. App将事件转换为动作并发送
//! 3. LayerManager处理层级管理动作
//! 4. 各个页面处理特定的业务动作
//! 5. 状态更新触发界面重新渲染

use crate::{
    actions::Action,
    config::Config,
    libs::transactions::TransactionManager,
    page::home::Home,
    tui::{self, TuiEnum},
};
use color_eyre::eyre::{Context, Result};
use layer_manager::LayerManager;
use tokio::sync::mpsc;
use tracing::warn;

pub(crate) mod layer_manager;

/// 应用程序的根状态
///
/// 包含应用程序运行所需的所有全局状态和资源：
/// - 应用程序生命周期标志
/// - 动作通信通道
/// - 数据库管理器
/// - 配置信息
///
/// 这个结构体在应用程序启动时创建，并在整个生命周期中保持。
pub(crate) struct RootState {
    /// 应用程序是否应该退出的标志
    should_quit: bool,
    /// 发送动作的通道端
    action_tx: tokio::sync::mpsc::UnboundedSender<Action>,
    /// 接收动作的通道端
    action_rx: tokio::sync::mpsc::UnboundedReceiver<Action>,

    /// 交易管理器，用于与数据库交互
    manager: crate::libs::transactions::TransactionManager,

    /// 应用程序配置
    config: Config,
}

impl RootState {
    /// 创建新的根状态实例
    ///
    /// # 参数
    ///
    /// * `config` - 应用程序配置
    ///
    /// # 返回值
    ///
    /// 返回初始化完成的 `RootState` 实例
    ///
    /// # Panics
    ///
    /// 如果数据库连接失败将会 panic
    pub fn new(config: Config) -> Self {
        let (action_tx, action_rx) = mpsc::unbounded_channel();

        let manager = TransactionManager::new(config.config.db_path())
            .with_context(|| {
                format!(
                    "Fail to connect to Database at {}",
                    config
                        .config
                        .db_path()
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or("memory".into())
                )
            })
            .unwrap();

        if let Some(account) = &config.fetch.account {
            manager.update_account(account).unwrap();
        }
        if let Some(hallticket) = &config.fetch.hallticket {
            manager.update_hallticket(hallticket).unwrap();
        }

        Self {
            should_quit: false,
            action_tx,
            action_rx,
            manager,
            config,
        }
    }

    /// 发送动作到动作处理系统
    ///
    /// # 参数
    ///
    /// * `action` - 要发送的动作
    pub fn send_action<T: Into<Action>>(&self, action: T) {
        let result = self.action_tx.send(action.into());
        if let Err(e) = result {
            warn!("Failed to send action: {:?}", e);
        }
    }

    /// 根据动作更新应用程序状态
    ///
    /// # 参数
    ///
    /// * `action` - 要处理的动作
    pub(crate) fn update(&mut self, action: &Action) {
        // match action {
        //     Action::Quit => {
        //         self.should_quit = true;
        //     }
        //     _ => {}
        // }
        if let Action::Quit = action {
            self.should_quit = true;
        }
    }
}

/// 主应用程序结构
///
/// 这是应用程序的核心，负责：
/// - 管理页面层级堆栈
/// - 处理事件循环
/// - 协调TUI和业务逻辑
///
/// # 生命周期
///
/// 1. 创建时初始化所有子系统
/// 2. `run()` 方法启动主事件循环
/// 3. 处理用户输入直到退出条件满足
pub(super) struct App {
    /// 页面层级管理器
    layer_manager: LayerManager,
    /// 应用程序根状态
    state: RootState,
    /// 终端用户界面
    tui: tui::TuiEnum,
}

impl App {
    /// 创建新的应用程序实例
    ///
    /// # 参数
    ///
    /// * `state` - 根状态实例
    /// * `tui` - TUI实例
    ///
    /// # 返回值
    ///
    /// 返回配置好的 `App` 实例，默认显示主页
    pub fn new(state: RootState, tui: TuiEnum) -> Self {
        Self {
            layer_manager: LayerManager::new(Box::new(Home {
                tx: state.action_tx.clone().into(),
            })),
            state,
            tui,
        }
    }
}

impl App {
    /// 应用程序主事件循环
    ///
    /// 这是应用程序的核心运行方法，包含：
    /// 1. 初始化TUI
    /// 2. 持续处理事件直到退出
    /// 3. 清理TUI资源
    ///
    /// # 返回值
    ///
    /// 成功时返回 `Ok(())`，失败时返回相应错误
    #[cfg(not(tarpaulin_include))]
    pub async fn run(&mut self) -> Result<()> {
        self.tui.enter()?;

        loop {
            let e = self.tui.next().await?;

            self.event_loop(e)?;

            // application exit
            if self.state.should_quit {
                break;
            }
        }

        self.tui.exit()?;
        Ok(())
    }

    /// 单次事件循环处理
    ///
    /// 处理一个事件并执行所有待处理的动作，直到动作队列为空。
    ///
    /// # 参数
    ///
    /// * `e` - 要处理的事件
    ///
    /// # 返回值
    ///
    /// 成功时返回 `Ok(())`，失败时返回相应错误
    fn event_loop(&mut self, e: tui::Event) -> Result<()> {
        self.handle_event(e);

        while let Ok(action) = self.state.action_rx.try_recv() {
            self.perform_action(action);
            if self.state.should_quit {
                break;
            }
        }
        Ok(())
    }

    /// 发送动作到动作处理系统
    ///
    /// # 参数
    ///
    /// * `action` - 要发送的动作
    pub fn send_action<T: Into<Action>>(&self, action: T) {
        self.state.send_action(action);
    }

    /// 将TUI事件转换为动作并发送
    ///
    /// 这个函数负责：
    /// - 处理应用程序级别的事件（如退出、调整大小等）
    /// - 将其他事件委托给当前页面处理
    ///
    /// # 参数
    ///
    /// * `event` - 要处理的TUI事件
    fn handle_event(&mut self, event: tui::Event) {
        match event {
            tui::Event::Render => self.send_action(Action::Render),

            // TODO impl these events
            tui::Event::Error => self.send_action(Action::Quit),
            tui::Event::FocusGained => (),
            tui::Event::FocusLost => (),
            tui::Event::Init => (),
            tui::Event::Resize(_, _) => self.send_action(Action::Render),

            _ => self.layer_manager.handle_event(event),
        };
    }

    /// 执行动作
    ///
    /// 这是应用程序状态改变的唯一入口点。
    /// 负责处理所有类型的动作并更新相应的状态。
    ///
    /// # 参数
    ///
    /// * `action` - 要执行的动作
    fn perform_action(&mut self, action: Action) {
        match &action {
            Action::Render => self.tui.draw(|f| self.layer_manager.render(f)).unwrap(),
            Action::Layer(layer_action) => self
                .layer_manager
                .handle_layer_action(layer_action, &self.state),
            _ => {}
        }
        self.state.update(&action);
    }
}

#[cfg(test)]
pub(super) mod test {
    use clap::Parser;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use insta::assert_snapshot;

    use crate::{
        actions::{LayerManageAction, Layers},
        cli::{ClapSource, Cli},
        libs::fetcher::MealFetcher,
        page::{
            cookie_input::CookieInput, fetch::Fetch, help_popup::HelpPopup,
            transactions::Transactions,
        },
        tui::Event,
        utils::help_msg::HelpEntry,
    };

    use super::*;

    pub fn get_config(mut args: Vec<&str>, append_to_default: bool) -> Config {
        let mut default_args = vec!["test-config"];
        let args = if append_to_default {
            default_args.push("--db-in-mem");
            default_args.append(&mut args);
            default_args
        } else {
            default_args.append(&mut args);
            default_args
        };
        let cli = Cli::parse_from(args);
        crate::config::Config::new(Some(ClapSource::new(&cli))).expect("Failed to load config")
    }
    #[test]
    fn root_state_set_fetch_config() {
        let config = get_config(vec!["--account", "123456", "--hallticket", "543210"], true);

        let root = RootState::new(config);
        let (account, cookie) = root.manager.get_account_cookie().unwrap();
        assert_eq!(account, "123456");
        assert_eq!(cookie, "hallticket=543210");
    }

    pub fn get_app() -> App {
        let config = get_config(vec!["--use-mock-data"], true);
        let state = RootState::new(config);
        let app = App::new(state, tui::TestTui::new().into());
        app
    }

    #[tokio::test]
    async fn app_navigation() {
        let mut app = get_app();

        // Navigate to Fetch page
        app.event_loop('T'.into()).unwrap();
        assert!(app.layer_manager.last().unwrap().is::<Transactions>());

        app.perform_action(Action::Layer(LayerManageAction::Swap(Layers::Fetch)));
        assert!(app.layer_manager.last().unwrap().is::<Fetch>());

        app.perform_action(Action::Layer(LayerManageAction::Swap(Layers::CookieInput)));
        assert!(app.layer_manager.last().unwrap().is::<CookieInput>());
        app.perform_action(Action::Layer(LayerManageAction::Swap(Layers::Help(
            vec![HelpEntry::new('?', "Help")].into(),
        ))));
        assert!(app.layer_manager.last().unwrap().is::<HelpPopup>());
    }

    #[tokio::test]
    async fn app_nav_fetch_mock() {
        let mut app = get_app();

        app.perform_action(Action::Layer(LayerManageAction::Swap(Layers::Fetch)));
        assert!(app.layer_manager.last().unwrap().is::<Fetch>());
        let fetch = app
            .layer_manager
            .last()
            .unwrap()
            .downcast_ref::<Fetch>()
            .unwrap();
        assert!(matches!(fetch.get_client(), MealFetcher::Mock(_)));
    }

    #[tokio::test]
    async fn app_quit() {
        let mut app = get_app();

        app.perform_action(Action::Quit);
        assert_eq!(app.state.should_quit, true);
    }

    #[tokio::test]
    async fn app_quit_due_to_last_layer_pop() {
        let mut app = get_app();

        app.perform_action(Action::Layer(LayerManageAction::Pop));
        assert!(app.layer_manager.last().unwrap().is::<Home>());
    }

    #[tokio::test]
    async fn app_push_layer() {
        let mut app = get_app();

        app.perform_action(Action::Layer(LayerManageAction::Push(
            Layers::Transaction(None).into_push_config(false),
        )));
        assert_eq!(app.layer_manager.len(), 2);
        assert!(app.layer_manager.first().unwrap().is::<Home>());
        assert!(app.layer_manager.last().unwrap().is::<Transactions>());
        app.perform_action(Action::Layer(LayerManageAction::Pop));
        assert_eq!(app.layer_manager.len(), 1);
        assert!(app.layer_manager.first().unwrap().is::<Home>());
    }

    #[tokio::test]
    async fn app_render() {
        let mut app = get_app();

        app.perform_action(Action::Render);
        assert_snapshot!(app.tui.backend());
    }

    #[tokio::test]
    async fn app_stacked_render() {
        let mut app = get_app();

        app.event_loop(Event::Key(KeyEvent::new(
            KeyCode::Char('?'),
            KeyModifiers::NONE,
        )))
        .unwrap();

        app.perform_action(Action::Render);
        assert_snapshot!(app.tui.backend());
    }
}
