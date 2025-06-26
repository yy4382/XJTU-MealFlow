//! # 配置管理模块
//!
//! 该模块负责应用程序的配置管理，支持多种配置源和灵活的配置层次。
//! 使用 [config](https://crates.io/crates/config) 库提供统一的配置接口。
//!
//! ## 配置源优先级
//!
//! 配置值按以下优先级顺序加载（后者覆盖前者）：
//! 1. 默认值
//! 2. 环境变量
//! 3. 配置文件
//! 4. 命令行参数（最高优先级）
//!
//! ## 配置结构
//!
//! ```text
//! Config
//! ├── config: AppConfig          # 应用程序基础配置
//! │   ├── data_dir              # 数据目录
//! │   ├── db_path               # 数据库文件路径
//! │   └── db_in_mem             # 是否使用内存数据库
//! └── fetch: FetchConfig         # 数据获取配置
//!     ├── account               # 校园卡账号
//!     ├── hallticket            # 认证票据
//!     └── use_mock_data         # 是否使用模拟数据
//! ```
//!
//! ## 数据目录
//!
//! 数据目录的确定顺序：
//! 1. 环境变量 `XJTU_MEALFLOW_DATA`
//! 2. 系统标准数据目录（通过 `directories` crate）
//! 3. 当前目录下的 `.data` 文件夹（回退选项）

use std::{env, path::PathBuf};

use color_eyre::{Result, eyre::Context};
use directories::ProjectDirs;
use lazy_static::lazy_static;
use serde::Deserialize;

/// 应用程序基础配置
///
/// 包含应用程序运行所需的核心配置选项，如数据存储路径和数据库设置。
#[derive(Clone, Debug, Deserialize)]
pub struct AppConfig {
    /// 数据目录路径
    ///
    /// 存储数据库文件和其他持久化数据的目录
    #[serde(default)]
    pub data_dir: PathBuf,

    /// 数据库文件路径（相对于data_dir）
    ///
    /// 默认为 "transactions.db"
    #[serde(default)]
    db_path: PathBuf,

    /// 是否使用内存数据库
    ///
    /// 如果为true，所有数据将在程序退出时丢失，适用于测试场景
    #[serde(default)]
    db_in_mem: bool,
}

impl AppConfig {
    /// 获取数据库文件的完整路径
    ///
    /// # 返回值
    ///
    /// - `Some(PathBuf)`: 如果使用文件数据库，返回完整路径
    /// - `None`: 如果使用内存数据库
    ///
    /// # 示例
    ///
    /// ```rust
    /// let config = AppConfig::default();
    /// if let Some(db_path) = config.db_path() {
    ///     println!("Database path: {}", db_path.display());
    /// } else {
    ///     println!("Using in-memory database");
    /// }
    /// ```
    pub fn db_path(&self) -> Option<PathBuf> {
        if self.db_in_mem {
            None
        } else {
            Some(self.data_dir.join(&self.db_path))
        }
    }
}

/// 数据获取配置
///
/// 包含从XJTU服务器获取交易数据所需的配置。
///
/// **重要提示**: 这不应该是获取操作的数据来源，仅用于初始化数据库中的相关配置。
/// 获取操作的真实数据来源应该来自数据库。
#[derive(Clone, Debug, Deserialize, Default)]
pub struct FetchConfig {
    /// 校园卡账号
    ///
    /// 用于登录XJTU校园卡系统获取交易记录
    pub account: Option<String>,

    /// 校园卡认证票据
    ///
    /// 从浏览器Cookie中获取的hallticket值
    pub hallticket: Option<String>,

    /// 是否使用模拟数据
    ///
    /// 启用后将使用预设的测试数据而不是真实服务器数据
    pub use_mock_data: bool,
}

/// 应用程序主配置结构
///
/// 组合了所有配置模块，提供统一的配置接口。
#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    /// 应用程序基础配置
    #[serde(default, flatten)]
    pub config: AppConfig,

    /// 数据获取配置
    #[serde(default)]
    pub fetch: FetchConfig,
}

lazy_static! {
    /// 项目名称常量（大写形式）
    ///
    /// 用于环境变量名称和其他需要项目标识的场景
    pub static ref PROJECT_NAME: String = env!("CARGO_CRATE_NAME").to_uppercase().to_string();
}

impl Config {
    /// 创建新的配置实例
    ///
    /// 从多个配置源加载配置，按优先级顺序合并。
    ///
    /// # 参数
    ///
    /// * `cli_source` - 可选的CLI配置源，具有最高优先级
    ///
    /// # 返回值
    ///
    /// 成功时返回完整的配置实例，失败时返回配置错误
    ///
    /// # 错误
    ///
    /// - 默认值设置失败
    /// - 配置源添加失败
    /// - 配置构建失败
    /// - 反序列化失败
    ///
    /// # 示例
    ///
    /// ```rust
    /// use crate::cli::ClapSource;
    ///
    /// // 使用默认配置
    /// let config = Config::new(None)?;
    ///
    /// // 使用CLI参数配置
    /// let cli_source = ClapSource::new(&cli_args);
    /// let config = Config::new(Some(cli_source))?;
    /// ```
    pub fn new(cli_source: Option<crate::cli::ClapSource>) -> Result<Self> {
        let data_dir = get_data_dir();
        let mut builder = config::Config::builder()
            .set_default("data_dir", data_dir.to_str().unwrap())?
            .set_default("db_path", "transactions.db")?;

        // Add CLI source last (highest priority)
        if let Some(cli_source) = cli_source {
            builder = builder.add_source(cli_source);
        }

        let cfg: Self = builder
            .build()
            .context("Error building config")?
            .try_deserialize()
            .context("Error deserialize config")?;

        Ok(cfg)
    }
}

/// 获取应用程序数据目录
///
/// 按以下优先级确定数据目录：
/// 1. 环境变量 `{PROJECT_NAME}_DATA`
/// 2. 系统标准数据目录（通过 `directories` crate）
/// 3. 当前目录下的 `.data` 文件夹（回退选项）
///
/// # 返回值
///
/// 返回数据目录的路径
///
/// # 示例
///
/// ```rust
/// let data_dir = get_data_dir();
/// println!("Data directory: {}", data_dir.display());
/// ```
pub fn get_data_dir() -> PathBuf {
    let directory = if let Some(s) = env::var(format!("{}_DATA", PROJECT_NAME.clone()))
        .ok()
        .map(PathBuf::from)
        .clone()
    {
        s
    } else if let Some(proj_dirs) = project_directory() {
        proj_dirs.data_local_dir().to_path_buf()
    } else {
        PathBuf::from(".").join(".data")
    };
    directory
}

/// 获取项目目录信息
///
/// 使用 `directories` crate 获取符合操作系统标准的目录路径。
///
/// # 返回值
///
/// - `Some(ProjectDirs)`: 成功获取项目目录信息
/// - `None`: 无法确定项目目录（通常在特殊环境中发生）
fn project_directory() -> Option<ProjectDirs> {
    ProjectDirs::from("dev", "yyang", env!("CARGO_PKG_NAME"))
}

#[cfg(test)]
mod tests {

    use clap::Parser;
    use tempfile::tempdir_in;

    use crate::cli::{ClapSource, Cli};

    use super::*;

    #[test]
    fn data_dir_from_env() {
        let temp_data = tempdir_in(".").unwrap();

        temp_env::with_vars(
            [(
                format!("{}_DATA", PROJECT_NAME.clone()).as_str(),
                Some(temp_data.path().to_str().unwrap()),
            )],
            || {
                let config = Config::new(None).unwrap();
                println!("{:?}", config);
                assert_eq!(config.config.data_dir, temp_data.path());

                assert_eq!(
                    config.config.db_path(),
                    Some(config.config.data_dir.join("transactions.db"))
                );
            },
        );
    }

    #[test]
    fn data_dir_from_cli() {
        let args = crate::cli::Cli::parse_from(&["test-config", "--data-dir", ".cli-data"]);
        let config = Config::new(Some(ClapSource::new(&args))).expect("Failed to load config");

        assert_eq!(config.config.data_dir, PathBuf::from(".cli-data"));
    }

    #[test]
    fn account_from_dir() {
        let args = Cli::parse_from(&["test-config", "--account", "123456"]);
        let config = Config::new(Some(ClapSource::new(&args))).expect("Failed to load config");

        assert_eq!(config.fetch.account.unwrap(), "123456");
    }

    #[test]
    fn hallticket_from_dir() {
        let args = Cli::parse_from(&["test-config", "--hallticket", "123456"]);
        let config = Config::new(Some(ClapSource::new(&args))).expect("Failed to load config");

        assert_eq!(config.fetch.hallticket.unwrap(), "123456");
    }

    #[test]
    fn db_in_men_path() {
        let args = Cli::parse_from(&["test-config", "--db-in-mem"]);
        let config = Config::new(Some(ClapSource::new(&args))).expect("Failed to load config");

        assert_eq!(config.config.db_path(), None);
    }

    #[test]
    fn use_mock_data() {
        let args = Cli::parse_from(&["test-config", "--use-mock-data"]);
        let config = Config::new(Some(ClapSource::new(&args))).expect("Failed to load config");

        assert_eq!(config.fetch.use_mock_data, true);

        let args = Cli::parse_from(&["test-config"]);
        let config = Config::new(Some(ClapSource::new(&args))).expect("Failed to load config");

        assert_eq!(config.fetch.use_mock_data, false);
    }
}
