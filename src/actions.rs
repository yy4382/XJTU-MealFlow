//! # 应用程序动作系统
//!
//! 该模块定义了应用程序中的所有动作类型和相关的枚举。
//! 动作系统是应用程序状态管理的核心，采用类似Redux的模式。
//!
//! ## 核心概念
//!
//! - **Action**: 应用程序中所有可能的动作
//! - **LayerManageAction**: 页面层级管理相关的动作
//! - **Layers**: 应用程序中的所有页面/层级类型
//! - **ActionSender**: 动作发送器，提供线程安全的动作传递
//!
//! ## 架构说明
//!
//! 应用程序使用基于动作的架构模式：
//! 1. UI组件生成动作(Action)
//! 2. 动作通过ActionSender发送到中央处理器
//! 3. 应用程序根据动作类型更新状态
//! 4. 状态变化触发UI重新渲染

use color_eyre::eyre::Context;

use crate::{libs::transactions::FilterOptions, utils::help_msg::HelpMsg};

/// 应用程序中的所有动作类型
///
/// 这是应用程序动作系统的根枚举，包含所有可能的动作类型。
/// 每个动作都会导致应用程序状态的变化。
#[derive(Clone, Debug)]
pub enum Action {
    /// 层级管理动作，用于页面导航和层级操作
    Layer(LayerManageAction),

    /// 退出应用程序
    Quit,
    /// 触发界面重新渲染
    Render,
}

/// 应用程序中的所有页面/层级类型
///
/// 定义了应用程序中所有可能的页面和对话框类型。
/// 每个层级代表一个独立的UI界面或功能模块。
#[derive(Clone, Debug)]
pub enum Layers {
    /// 主页面（当前未使用）
    #[allow(dead_code)]
    Home,
    /// 数据获取页面
    Fetch,
    /// 交易记录列表页面，可选的过滤选项
    Transaction(Option<FilterOptions>),
    /// Cookie输入页面，用于配置认证信息
    CookieInput,
    /// 帮助对话框，显示指定的帮助信息
    Help(HelpMsg),
    /// 数据分析页面
    Analysis,
}

impl std::fmt::Display for Layers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Layers::Home => write!(f, "Home"),
            Layers::Fetch => write!(f, "Fetch"),
            Layers::Transaction(_) => write!(f, "Transaction"),
            Layers::CookieInput => write!(f, "CookieInput"),
            Layers::Help(_) => write!(f, "Help"),
            Layers::Analysis => write!(f, "Analysis"),
        }
    }
}

/// 层级管理相关的动作
///
/// 这些动作用于控制应用程序的页面导航和层级堆栈管理。
/// 只有位于堆栈顶部的页面才能发送这些动作，并且只有根应用程序才能处理它们。
#[derive(Clone, Debug)]
pub enum LayerManageAction {
    /// 推入新页面到层级堆栈
    Push(PushPageConfig),
    /// 替换当前页面
    Swap(Layers),
    /// 弹出当前页面
    Pop,
}

/// 推入页面的配置
///
/// 定义了推入新页面时的行为配置。
#[derive(Clone, Debug)]
pub struct PushPageConfig {
    /// 要推入的页面类型
    pub layer: Layers,
    /// 是否在新页面位于堆栈顶部时继续渲染当前页面
    ///
    /// 例如：
    /// - 如果推入的是帮助弹窗，应该设置为 `true`，这样可以看到下层页面
    /// - 如果推入的是全屏页面，应该设置为 `false`，以减少性能开销
    pub render_self: bool,
}

impl Layers {
    /// 将层级类型转换为推入配置
    ///
    /// # 参数
    ///
    /// * `render_self` - 是否渲染下层页面
    ///
    /// # 返回值
    ///
    /// 返回配置好的 `PushPageConfig` 实例
    pub fn into_push_config(self, render_self: bool) -> PushPageConfig {
        PushPageConfig {
            layer: self,
            render_self,
        }
    }
}

impl From<LayerManageAction> for Action {
    fn from(value: LayerManageAction) -> Self {
        Action::Layer(value)
    }
}

/// 动作发送器
///
/// 提供线程安全的动作传递机制。所有UI组件都应该使用这个发送器来发送动作，
/// 而不是直接操作应用程序状态。
#[derive(Clone, Debug)]
pub struct ActionSender(pub tokio::sync::mpsc::UnboundedSender<Action>);

impl ActionSender {
    /// 发送动作到应用程序
    ///
    /// # 参数
    ///
    /// * `action` - 要发送的动作，可以是任何实现了 `Into<Action>` 的类型
    ///
    /// # Panics
    ///
    /// 如果接收端已关闭或掉线，将会 panic。这通常表示应用程序已经停止运行。
    pub fn send<T: Into<Action>>(&self, action: T) {
        self.0.send(action.into()).with_context(||"Action Receiver is dropped or closed, which should not happen if app is still running.").unwrap();
    }
}

impl From<tokio::sync::mpsc::UnboundedSender<Action>> for ActionSender {
    fn from(value: tokio::sync::mpsc::UnboundedSender<Action>) -> Self {
        ActionSender(value)
    }
}
