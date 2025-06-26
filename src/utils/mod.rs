//! # 工具模块集合
//!
//! 提供应用程序所需的各种工具函数和辅助模块，包括错误处理、帮助系统、
//! 键盘事件处理、日志记录和商家分类等功能。
//!
//! ## 模块组织
//!
//! ```text
//! utils/
//! ├── errors.rs         - 错误处理和 Panic Hook 配置
//! ├── help_msg.rs       - 帮助信息显示系统
//! ├── key_events.rs     - 键盘事件处理工具
//! ├── logging.rs        - 日志记录配置
//! ├── merchant_class.rs - 商家分类和识别
//! └── mod.rs           - 模块导出声明
//! ```
//!
//! ## 功能模块说明
//!
//! ### 错误处理 (`errors`)
//! 配置全局错误处理机制，包括：
//! - Color-eyre 错误报告
//! - Panic Hook 设置
//! - 终端恢复逻辑
//! - 调试和生产环境的不同处理策略
//!
//! ### 帮助系统 (`help_msg`)
//! 提供统一的帮助信息显示功能：
//! - 快捷键说明格式化
//! - 帮助信息组合和扩展
//! - TUI 帮助面板渲染
//!
//! ### 键盘事件 (`key_events`)
//! 键盘输入的标准化处理：
//! - 按键事件封装
//! - 修饰键处理
//! - 字符串表示转换
//!
//! ### 日志记录 (`logging`)
//! 应用程序日志配置：
//! - Tracing 日志系统初始化
//! - 日志级别和格式配置
//! - 文件和终端输出管理
//!
//! ### 商家分类 (`merchant_class`)
//! 商家名称的分类和识别：
//! - 商家类型识别算法
//! - 分类规则定义
//! - 统计分析支持
//!
//! ## 编译条件
//!
//! 部分模块使用条件编译：
//! - `#[cfg(not(tarpaulin_include))]`: 在代码覆盖率测试时排除某些模块
//!
//! ## 使用示例
//!
//! ```rust
//! // 初始化错误处理
//! use crate::utils::errors;
//! errors::init()?;
//!
//! // 使用帮助系统
//! use crate::utils::help_msg::{HelpEntry, HelpMsg};
//! let help = HelpMsg::from(vec![
//!     HelpEntry::new('q', "Quit"),
//!     HelpEntry::new('?', "Help"),
//! ]);
//!
//! // 商家分类
//! use crate::utils::merchant_class::classify_merchant;
//! let category = classify_merchant("梧桐苑餐厅");
//! ```

#[cfg(not(tarpaulin_include))]
pub(crate) mod errors;
pub(crate) mod help_msg;
pub(crate) mod key_events;
#[cfg(not(tarpaulin_include))]
pub(crate) mod logging;
pub(crate) mod merchant_class;
