use std::{env, path::PathBuf};

use color_eyre::{Result, eyre::Context};
use directories::ProjectDirs;
use lazy_static::lazy_static;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub data_dir: PathBuf,
    #[serde(default)]
    db_path: PathBuf,

    /// Use an in-memory database, which means all data will lost when the program exits
    #[serde(default)]
    db_in_mem: bool,
}

impl AppConfig {
    /// Returns the path to the database file
    /// If using a memory database, returns None
    pub fn db_path(&self) -> Option<PathBuf> {
        if self.db_in_mem {
            None
        } else {
            Some(self.data_dir.join(&self.db_path))
        }
    }
}

/// Configuration for fetching transactions from XJTU server
///
/// This SHOULD NOT be the source of truth for fetching. This is only for initializing
/// related configurations in database.
///
/// The source of truth for fetching is from Database.
#[derive(Clone, Debug, Deserialize, Default)]
pub struct FetchConfig {
    pub account: Option<String>,
    pub hallticket: Option<String>,
    pub use_mock_data: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    #[serde(default, flatten)]
    pub config: AppConfig,
    #[serde(default)]
    pub fetch: FetchConfig,
}

lazy_static! {
    pub static ref PROJECT_NAME: String = env!("CARGO_CRATE_NAME").to_uppercase().to_string();
}

impl Config {
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
