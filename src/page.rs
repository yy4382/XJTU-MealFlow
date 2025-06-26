//! # 用户界面页面系统
//!
//! 该模块定义了应用程序的页面架构和用户界面组件。
//! 采用基于trait的设计，提供灵活的页面管理和事件处理机制。
//!
//! ## 核心概念
//!
//! - **Layer**: 页面/层级抽象，代表一个独立的UI界面
//! - **WidgetExt**: 可渲染组件，定义了渲染接口
//! - **EventLoopParticipant**: 事件循环参与者，处理用户输入
//!
//! ## 页面架构
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │              Layer Trait                │
//! │  ┌─────────────┐  ┌─────────────────────┐ │
//! │  │ WidgetExt   │  │ EventLoopParticipant│ │
//! │  │ 渲染接口     │  │    事件处理          │ │
//! │  └─────────────┘  └─────────────────────┘ │
//! └─────────────────────────────────────────┘
//!              │
//!              ▼
//!      具体页面实现
//! ┌─────────┬─────────┬─────────┬─────────┐
//! │  Home   │Analysis │ Fetch   │Settings │
//! │ 主页     │ 分析页   │ 获取页   │ 设置页   │
//! └─────────┴─────────┴─────────┴─────────┘
//! ```
//!
//! ## 页面列表
//!
//! - analysis: 数据分析页面，提供多维度数据分析
//! - cookie_input: Cookie输入页面，用于配置认证信息
//! - fetch: 数据获取页面，从服务器获取交易记录
//! - help_popup: 帮助弹窗，显示快捷键和操作指南
//! - home: 主页面，应用程序的起始页面
//! - transactions: 交易记录页面，显示和管理交易数据
//!
//! ## 事件处理流程
//!
//! 1. 用户输入事件被TUI捕获
//! 2. 事件传递给当前活跃的Layer
//! 3. Layer通过EventLoopParticipant处理事件
//! 4. 事件处理可能产生Action
//! 5. Action被发送到应用程序进行状态更新
//! 6. 状态更新触发重新渲染

use crate::app::layer_manager::EventHandlingStatus;
use crate::tui::Event;
use downcast_rs::{DowncastSync, impl_downcast};
use ratatui::Frame;
use ratatui::layout::Rect;

/// 数据分析页面模块
///
/// 提供多维度的交易数据分析功能，包括时间分析、商家分析、类别分析等。
pub(crate) mod analysis;

/// Cookie输入页面模块
///
/// 用于配置XJTU校园卡系统的认证信息，包括账号和Cookie设置。
pub(crate) mod cookie_input;

/// 数据获取页面模块
///
/// 负责从XJTU服务器获取交易记录，支持进度显示和错误处理。
pub(crate) mod fetch;

/// 帮助弹窗模块
///
/// 显示应用程序的快捷键说明和操作指南。
pub(crate) mod help_popup;

/// 主页面模块
///
/// 应用程序的起始页面，提供主要功能的导航入口。
pub(crate) mod home;

/// 交易记录页面模块
///
/// 显示交易记录列表，支持筛选、排序和详细查看。
pub(crate) mod transactions;

/// 页面层级抽象
///
/// 代表应用程序中的一个UI层级或页面。每个Layer都是一个独立的功能单元，
/// 具有自己的渲染逻辑和事件处理机制。
///
/// ## 实现要求
///
/// 实现Layer的类型必须同时实现：
/// - [`WidgetExt`]: 提供渲染能力
/// - [`EventLoopParticipant`]: 提供事件处理能力
/// - [`DowncastSync`]: 支持类型转换（用于层级管理）
///
/// ## 生命周期
///
/// 1. **创建**: Layer在需要时被创建
/// 2. **初始化**: 调用 `init()` 方法进行初始化设置
/// 3. **运行**: 持续接收事件并渲染界面
/// 4. **销毁**: 当不再需要时自动销毁
///
/// ## 示例
///
/// ```rust
/// use crate::page::{Layer, WidgetExt, EventLoopParticipant};
///
/// struct MyPage {
///     // 页面状态
/// }
///
/// impl Layer for MyPage {
///     fn init(&mut self) {
///         // 初始化逻辑
///     }
/// }
///
/// impl WidgetExt for MyPage {
///     fn render(&mut self, frame: &mut Frame, area: Rect) {
///         // 渲染逻辑
///     }
/// }
///
/// impl EventLoopParticipant for MyPage {
///     fn handle_events(&mut self, event: &Event) -> EventHandlingStatus {
///         // 事件处理逻辑
///     }
/// }
/// ```
pub trait Layer: WidgetExt + EventLoopParticipant + DowncastSync {
    /// 初始化页面
    ///
    /// 在页面首次显示前调用，用于执行必要的初始化操作。
    /// 默认实现为空，子类可以根据需要重写。
    ///
    /// # 用途
    ///
    /// - 加载初始数据
    /// - 设置初始状态
    /// - 建立必要的连接
    /// - 注册事件监听器
    fn init(&mut self) {}
}
impl_downcast!(sync Layer);

/// 可渲染组件扩展trait
///
/// 为组件提供渲染到终端屏幕的能力。所有需要显示在界面上的组件
/// 都必须实现这个trait。
///
/// ## 渲染概念
///
/// - **Frame**: Ratatui提供的渲染帧，代表当前的渲染上下文
/// - **Area**: 组件可用的屏幕区域，定义了组件的位置和大小
/// - **Widget**: Ratatui的基础渲染单元
///
/// ## 实现指南
///
/// ```rust
/// impl WidgetExt for MyComponent {
///     fn render(&mut self, frame: &mut Frame, area: Rect) {
///         // 创建要渲染的widget
///         let widget = ratatui::widgets::Paragraph::new("Hello, World!");
///         
///         // 渲染到指定区域
///         frame.render_widget(widget, area);
///     }
/// }
/// ```
pub(crate) trait WidgetExt {
    /// 渲染组件到指定区域
    ///
    /// # 参数
    ///
    /// * `frame` - 当前渲染帧，用于绘制UI元素
    /// * `area` - 组件可用的屏幕区域
    ///
    /// # 注意事项
    ///
    /// - 不应该渲染超出给定area范围的内容
    /// - 应该处理area为空或过小的情况
    /// - 渲染操作应该是幂等的（多次调用结果相同）
    fn render(&mut self, frame: &mut Frame, area: Rect);
}

/// 事件循环参与者trait
///
/// 为组件提供处理用户输入事件的能力。实现此trait的组件可以：
/// - 响应键盘输入
/// - 响应鼠标操作
/// - 响应系统事件
/// - 控制事件传播
///
/// ## 事件处理机制
///
/// 事件处理采用链式传播模式：
/// 1. 事件首先传递给顶层Layer
/// 2. 如果Layer消费了事件，传播停止
/// 3. 如果Layer没有消费事件，继续传播给下层
///
/// ## 事件类型
///
/// - **Key**: 键盘按键事件
/// - **Mouse**: 鼠标操作事件
/// - **Paste**: 文本粘贴事件
/// - **Resize**: 终端大小变化事件
/// - **System**: 系统级事件（焦点变化等）
pub(crate) trait EventLoopParticipant {
    /// 处理事件
    ///
    /// # 参数
    ///
    /// * `event` - 要处理的事件
    ///
    /// # 返回值
    ///
    /// 返回事件处理状态：
    /// - `EventHandlingStatus::Consumed`: 事件已被消费，停止传播
    /// - `EventHandlingStatus::ShouldPropagate`: 事件未被消费，继续传播
    ///
    /// # 实现指南
    ///
    /// ```rust
    /// fn handle_events(&mut self, event: &Event) -> EventHandlingStatus {
    ///     match event {
    ///         Event::Key(key) => {
    ///             match key.code {
    ///                 KeyCode::Enter => {
    ///                     // 处理回车键
    ///                     self.on_enter();
    ///                     EventHandlingStatus::Consumed
    ///                 }
    ///                 _ => EventHandlingStatus::ShouldPropagate
    ///             }
    ///         }
    ///         _ => EventHandlingStatus::ShouldPropagate
    ///     }
    /// }
    /// ```
    #[must_use]
    fn handle_events(&mut self, event: &Event) -> EventHandlingStatus;

    /// 处理事件并检查返回状态（测试辅助方法）
    ///
    /// 这是一个测试辅助方法，用于验证事件是否被正确消费。
    /// 在测试中可以使用此方法来确保事件处理的正确性。
    ///
    /// # 参数
    ///
    /// * `event` - 要处理的事件
    ///
    /// # Panics
    ///
    /// 如果事件没有被消费（返回状态不是Consumed），将会panic
    #[cfg(test)]
    fn handle_event_with_status_check(&mut self, event: &Event) {
        let status = self.handle_events(event);
        assert!(matches!(status, EventHandlingStatus::Consumed));
    }
}
