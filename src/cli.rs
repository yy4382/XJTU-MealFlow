use clap::Parser;
use color_eyre::Result;
use config::Source;

use crate::config::get_data_dir;

#[derive(Parser, Debug)]
#[command(author, version = version(), about = "How much did you eat at XJTU?")]
pub struct Cli {
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
    #[arg(long, value_name = "BOOL", default_value = "false")]
    pub db_in_mem: bool,
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

#[derive(Debug)]
pub(crate) struct ClapSource {
    pub data_dir: Option<String>,
}

impl ClapSource {
    pub fn new(cli: &Cli) -> Self {
        Self {
            data_dir: cli.data_dir.clone(),
        }
    }
}

impl Source for ClapSource {
    fn clone_into_box(&self) -> Box<dyn Source + Send + Sync> {
        Box::new(Self {
            data_dir: self.data_dir.clone(),
        })
    }

    fn collect(&self) -> Result<config::Map<String, config::Value>, config::ConfigError> {
        let mut map = config::Map::new();
        if self.data_dir.is_some() {
            map.insert(
                "data_dir".to_string(),
                config::Value::new(None, self.data_dir.clone()),
            );
        }
        Ok(map)
    }
}
