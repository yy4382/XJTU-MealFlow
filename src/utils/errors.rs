//! # 错误处理和 Panic 配置模块
//!
//! 提供全局错误处理机制，配置 panic hook 和错误报告系统。
//! 确保在发生错误时能够正确恢复终端状态并生成有用的错误信息。
//!
//! ## 核心功能
//!
//! - **Color-eyre 集成**: 提供彩色的错误报告和上下文信息
//! - **Panic Hook 配置**: 在程序崩溃时执行清理操作
//! - **终端恢复**: 确保 TUI 应用崩溃时正确恢复终端状态
//! - **环境感知**: 调试和生产环境使用不同的错误处理策略
//!
//! ## 错误处理流程
//!
//! ```text
//! 程序运行
//!     │
//!     ├─ 正常执行 ─────────────────────────→ 程序结束
//!     │
//!     └─ 发生 Panic
//!         │
//!         ├─ 1. 执行 Panic Hook
//!         │   ├─ 恢复终端状态
//!         │   ├─ 生成错误报告
//!         │   └─ 记录日志
//!         │
//!         ├─ 2. 根据环境选择处理方式
//!         │   ├─ Debug: 详细堆栈跟踪
//!         │   └─ Release: 用户友好的错误信息
//!         │
//!         └─ 3. 程序退出
//! ```
//!
//! ## 配置特性
//!
//! ### Color-eyre Hook 配置
//! - 显示项目仓库链接，便于用户报告 Bug
//! - 禁用 span trace 捕获（减少信息冗余）
//! - 隐藏位置信息和环境变量（简化输出）
//!
//! ### Panic Hook 功能
//! - **终端恢复**: 自动退出 TUI 模式，恢复正常终端
//! - **错误记录**: 使用 tracing 记录错误信息到日志
//! - **堆栈跟踪**: 提供详细的调用堆栈信息
//!
//! ## 环境差异化处理
//!
//! ### Debug 模式 (`debug_assertions`)
//! ```text
//! ┌─ Better Panic ─────────────────────────┐
//! │ • 详细的堆栈跟踪信息                    │
//! │ • 显示最近调用的函数                    │
//! │ • 包含行号和文件路径                    │
//! │ • 完整的上下文信息                      │
//! └─────────────────────────────────────────┘
//! ```
//!
//! ### Release 模式 (`not debug_assertions`)
//! ```text
//! ┌─ Human Panic ──────────────────────────┐
//! │ • 用户友好的错误消息                    │
//! │ • 生成崩溃转储文件                      │
//! │ • 简化的错误报告                        │
//! │ • 引导用户报告问题                      │
//! └─────────────────────────────────────────┘
//! ```
//!
//! ## trace_dbg! 宏
//!
//! 提供类似 `std::dbg!` 的调试宏，但输出到 tracing 日志系统而非 stdout。
//!
//! ### 使用方式
//!
//! ```rust
//! use crate::trace_dbg;
//!
//! // 基本用法
//! let value = trace_dbg!(42);
//!
//! // 指定日志级别
//! let value = trace_dbg!(level: tracing::Level::INFO, "hello");
//!
//! // 指定目标模块
//! let value = trace_dbg!(target: "my_module", some_variable);
//!
//! // 同时指定目标和级别
//! let value = trace_dbg!(target: "my_module", level: tracing::Level::WARN, data);
//! ```
//!
//! ## 使用示例
//!
//! ```rust
//! use color_eyre::Result;
//! use crate::utils::errors;
//!
//! fn main() -> Result<()> {
//!     // 应用程序启动时初始化错误处理
//!     errors::init()?;
//!     
//!     // 现在所有的 panic 和错误都会被正确处理
//!     run_application()?;
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## 依赖集成
//!
//! - **color-eyre**: 彩色错误报告和上下文信息
//! - **human-panic**: 生产环境的用户友好错误处理
//! - **better-panic**: 调试环境的详细错误信息
//! - **tracing**: 结构化日志记录
//! - **strip-ansi-escapes**: 清理 ANSI 转义字符

use std::env;

use color_eyre::Result;
use tracing::error;

pub fn init() -> Result<()> {
    let (panic_hook, eyre_hook) = color_eyre::config::HookBuilder::default()
        .panic_section(format!(
            "This is a bug. Consider reporting it at {}",
            env!("CARGO_PKG_REPOSITORY")
        ))
        .capture_span_trace_by_default(false)
        .display_location_section(false)
        .display_env_section(false)
        .into_hooks();
    eyre_hook.install()?;
    std::panic::set_hook(Box::new(move |panic_info| {
        if let Ok(mut t) = crate::tui::Tui::new() {
            if let Err(r) = t.exit() {
                error!("Unable to exit Terminal: {:?}", r);
            }
        }

        #[cfg(not(debug_assertions))]
        {
            use human_panic::{handle_dump, metadata, print_msg};
            let metadata = metadata!();
            let file_path = handle_dump(&metadata, panic_info);
            // prints human-panic message
            print_msg(file_path, &metadata)
                .expect("human-panic: printing error message to console failed");
            eprintln!("{}", panic_hook.panic_report(panic_info)); // prints color-eyre stack trace to stderr
        }
        let msg = format!("{}", panic_hook.panic_report(panic_info));
        error!("Error: {}", strip_ansi_escapes::strip_str(msg));

        #[cfg(debug_assertions)]
        {
            // Better Panic stacktrace that is only enabled when debugging.
            better_panic::Settings::auto()
                .most_recent_first(false)
                .lineno_suffix(true)
                .verbosity(better_panic::Verbosity::Full)
                .create_panic_handler()(panic_info);
        }

        std::process::exit(libc::EXIT_FAILURE);
    }));
    Ok(())
}

/// Similar to the `std::dbg!` macro, but generates `tracing` events rather
/// than printing to stdout.
///
/// By default, the verbosity level for the generated events is `DEBUG`, but
/// this can be customized.
#[macro_export]
macro_rules! trace_dbg {
        (target: $target:expr, level: $level:expr, $ex:expr) => {
            {
                match $ex {
                        value => {
                                tracing::event!(target: $target, $level, ?value, stringify!($ex));
                                value
                        }
                }
            }
        };
        (level: $level:expr, $ex:expr) => {
                trace_dbg!(target: module_path!(), level: $level, $ex)
        };
        (target: $target:expr, $ex:expr) => {
                trace_dbg!(target: $target, level: tracing::Level::DEBUG, $ex)
        };
        ($ex:expr) => {
                trace_dbg!(level: tracing::Level::DEBUG, $ex)
        };
}
