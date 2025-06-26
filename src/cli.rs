//! # 命令行接口模块
//!
//! 该模块负责处理应用程序的命令行参数解析和配置。
//! 使用 [clap](https://crates.io/crates/clap) 库提供强大的CLI功能。
//!
//! ## 主要功能
//!
//! - **子命令支持**: 提供多种运行模式（TUI、Web、导出等）
//! - **配置选项**: 支持数据库、认证信息等配置
//! - **版本信息**: 自动生成详细的版本和构建信息
//! - **配置源**: 提供配置文件系统的接口
//!
//! ## 命令结构
//!
//! ```text
//! xjtu_mealflow [全局选项] [子命令]
//! ├── clear-db          # 清理数据库
//! ├── web              # 启动Web服务器
//! └── export-csv       # 导出CSV文件
//!     ├── --output     # 输出文件路径
//!     ├── --merchant   # 按商家筛选
//!     ├── --min-amount # 最小金额
//!     ├── --max-amount # 最大金额
//!     ├── --time-start # 开始日期
//!     └── --time-end   # 结束日期
//! ```
//!
//! ## 配置集成
//!
//! CLI参数通过 `ClapSource` 集成到应用程序的配置系统中，
//! 具有最高的配置优先级。

use clap::{Parser, Subcommand};
use color_eyre::Result;
use config::Source;

use crate::config::get_data_dir;

/// XJTU MealFlow 命令行接口
///
/// 西安交通大学校园卡消费记录管理工具的主命令行接口。
/// 支持多种运行模式和丰富的配置选项。
///
/// # 示例
///
/// ```bash
/// # 启动TUI模式
/// xjtu_mealflow
///
/// # 启动Web服务器
/// xjtu_mealflow web
///
/// # 导出CSV文件
/// xjtu_mealflow export-csv --output my_data.csv
///
/// # 使用自定义配置
/// xjtu_mealflow --account 123456 --hallticket abc123
/// ```
#[derive(Parser, Debug)]
#[command(author, version = version(), about = "How much did you eat at XJTU?")]
pub struct Cli {
    /// 子命令
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Tick频率，即每秒tick数
    ///
    /// 控制应用程序的更新频率，影响动画和响应性
    #[arg(short, long, value_name = "FLOAT", default_value_t = 2.0)]
    pub tick_rate: f64,

    /// 帧率，即每秒帧数
    ///
    /// 控制界面渲染频率，影响视觉流畅度
    #[arg(short, long, value_name = "FLOAT", default_value_t = 30.0)]
    pub frame_rate: f64,

    /// 数据目录路径
    ///
    /// 指定存储数据库和其他持久化文件的目录
    #[arg(short, long, value_name = "PATH")]
    pub data_dir: Option<String>,

    /// 使用内存数据库
    ///
    /// 启用后所有数据将在程序退出时丢失，适用于测试场景
    #[arg(long, default_value_t = false)]
    pub db_in_mem: bool,

    /// 校园卡账号
    ///
    /// 用于获取交易记录。可以在 <https://card.xjtu.edu.cn> 获取
    #[arg(long, value_name = "STRING")]
    pub account: Option<String>,

    /// 校园卡认证票据
    ///
    /// 用于获取交易记录。可以在 <https://card.xjtu.edu.cn> 获取
    #[arg(long, value_name = "STRING")]
    pub hallticket: Option<String>,

    /// 使用模拟数据
    ///
    /// 启用后将使用预设的模拟数据而不是真实的服务器数据。
    /// 注意：仍需要设置account或hallticket，但可以是假的占位符
    #[arg(long, default_value_t = false)]
    pub use_mock_data: bool,
}

/// 应用程序子命令
///
/// 定义了应用程序支持的所有子命令和相关参数。
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// 清理本地数据库
    ///
    /// 清除用于缓存交易记录和其他数据的数据库。
    ///
    /// 如果设置了自定义数据路径，运行此命令时需要设置相同的路径。
    ///
    /// 此命令在以下情况下很有用：
    /// - 切换不同的账户时
    /// - 在模拟数据和真实数据之间切换时
    /// - 数据库损坏需要重建时
    ClearDb,

    /// 启动Web服务器模式
    ///
    /// 启动HTTP服务器，提供Web界面和REST API。
    /// 默认监听在 http://localhost:8080
    Web,

    /// 导出交易记录为CSV文件
    ///
    /// 支持多种筛选条件和输出格式。
    ExportCsv {
        /// 输出CSV文件路径
        ///
        /// 如果不指定，默认为 "transactions_export.csv"
        #[arg(short, long, value_name = "FILE_PATH")]
        output: Option<String>,

        /// 按商家名称筛选
        ///
        /// 只导出指定商家的交易记录
        #[arg(short, long, value_name = "MERCHANT_NAME")]
        merchant: Option<String>,

        /// 最小交易金额筛选（正数）
        ///
        /// 只导出消费金额大于等于此值的记录。
        /// 注意：数据库中存储的是负数，此参数会自动转换
        #[arg(long, value_name = "FLOAT")]
        min_amount: Option<f64>,

        /// 最大交易金额筛选（正数）
        ///
        /// 只导出消费金额小于等于此值的记录。
        /// 注意：数据库中存储的是负数，此参数会自动转换
        #[arg(long, value_name = "FLOAT")]
        max_amount: Option<f64>,

        /// 开始日期筛选（包含）
        ///
        /// 格式：YYYY-MM-DD
        /// 示例：2023-01-01
        #[arg(long, value_name = "DATE")]
        time_start: Option<String>,

        /// 结束日期筛选（不包含）
        ///
        /// 格式：YYYY-MM-DD
        /// 示例：2023-12-31
        #[arg(long, value_name = "DATE")]
        time_end: Option<String>,
    },
}

const VERSION_MESSAGE: &str = concat!(env!("CARGO_PKG_VERSION"));

/// 生成详细的版本信息字符串
///
/// 包含以下信息：
/// - 版本号
/// - 作者信息  
/// - 数据目录路径
///
/// # 返回值
///
/// 返回格式化的版本信息字符串
pub fn version() -> String {
    let author = clap::crate_authors!();

    // let current_exe_path = PathBuf::from(clap::crate_name!()).display().to_string();
    let data_dir_path = get_data_dir().display().to_string();

    format!(
        "\
{VERSION_MESSAGE}

Authors: {author}

Data directory: {data_dir_path}"
    )
}

/// Clap配置源
///
/// 实现 `config::Source` trait，用于将命令行参数集成到配置系统中。
/// 提供最高优先级的配置值。
#[derive(Debug, Clone)]
pub(crate) struct ClapSource {
    data_dir: Option<String>,
    db_in_men: bool,
    account: Option<String>,
    hallticket: Option<String>,
    use_mock_data: bool,
}

impl ClapSource {
    /// 从CLI参数创建配置源
    ///
    /// # 参数
    ///
    /// * `cli` - 解析后的CLI参数
    ///
    /// # 返回值
    ///
    /// 返回新的 `ClapSource` 实例
    pub fn new(cli: &Cli) -> Self {
        Self {
            data_dir: cli.data_dir.clone(),
            db_in_men: cli.db_in_mem,
            account: cli.account.clone(),
            hallticket: cli.hallticket.clone(),
            use_mock_data: cli.use_mock_data,
        }
    }
}

impl Source for ClapSource {
    fn clone_into_box(&self) -> Box<dyn Source + Send + Sync> {
        Box::new(self.clone())
    }

    /// 收集配置值
    ///
    /// 将CLI参数转换为配置系统可识别的键值对。
    ///
    /// # 返回值
    ///
    /// 成功时返回配置映射，失败时返回配置错误
    fn collect(&self) -> Result<config::Map<String, config::Value>, config::ConfigError> {
        let mut map = config::Map::new();
        if self.data_dir.is_some() {
            map.insert(
                "data_dir".to_string(),
                config::Value::new(None, self.data_dir.clone()),
            );
        }

        map.insert(
            "db_in_mem".to_string(),
            config::Value::new(None, self.db_in_men),
        );

        if self.account.is_some() {
            map.insert(
                "fetch.account".to_string(),
                config::Value::new(None, self.account.clone()),
            );
        }
        if self.hallticket.is_some() {
            map.insert(
                "fetch.hallticket".to_string(),
                config::Value::new(None, self.hallticket.clone()),
            );
        }

        map.insert(
            "fetch.use_mock_data".to_string(),
            config::Value::new(None, self.use_mock_data),
        );
        Ok(map)
    }
}
