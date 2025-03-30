use clap::{Parser, Subcommand};
use color_eyre::Result;
use config::Source;

use crate::config::get_data_dir;

#[derive(Parser, Debug)]
#[command(author, version = version(), about = "How much did you eat at XJTU?")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Tick rate, i.e. number of ticks per second
    #[arg(short, long, value_name = "FLOAT", default_value_t = 4.0)]
    pub tick_rate: f64,

    /// Frame rate, i.e. number of frames per second
    #[arg(short, long, value_name = "FLOAT", default_value_t = 60.0)]
    pub frame_rate: f64,

    /// Path to the data directory
    #[arg(short, long, value_name = "PATH")]
    pub data_dir: Option<String>,

    /// Use an in-memory database, which means all data will lost when the program exits [default: false]
    #[arg(long, default_value_t = false)]
    pub db_in_mem: bool,

    /// Account for fetching transactions
    ///
    /// Get it on https://card.xjtu.edu.cn
    #[arg(long, value_name = "STRING")]
    pub account: Option<String>,

    /// hallticket for fetching transactions
    ///
    /// Get it on https://card.xjtu.edu.cn
    #[arg(long, value_name = "STRING")]
    pub hallticket: Option<String>,

    /// Use mock data when fetching transactions
    ///
    /// Note that you still need to set account or hallticket, but they can be fake placeholders
    #[arg(long, default_value_t = false)]
    pub use_mock_data: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Clean the local database
    ///
    /// Clean up the database used for caching transactions and other data.
    ///
    /// If you have set a custom data path, you will need to set the same path
    /// if you want to clear the database when running this command.
    ///
    /// This command is helpful when you switches between different accounts or
    /// using mock data.
    ClearDb,
}

const VERSION_MESSAGE: &str = concat!(env!("CARGO_PKG_VERSION"));

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

#[derive(Debug, Clone)]
pub(crate) struct ClapSource {
    data_dir: Option<String>,
    db_in_men: bool,
    account: Option<String>,
    hallticket: Option<String>,
    use_mock_data: bool,
}

impl ClapSource {
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
