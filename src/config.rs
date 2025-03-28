use std::{env, path::PathBuf};

use color_eyre::{Result, eyre::Context};
use directories::ProjectDirs;
use lazy_static::lazy_static;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub data_dir: PathBuf,
    #[serde(default)]
    db_path: PathBuf,
}

impl AppConfig {
    pub fn db_path(&self) -> PathBuf {
        // concat the data_dir and db_path
        self.data_dir.join(&self.db_path)
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Config {
    #[serde(default, flatten)]
    pub config: AppConfig,
}

lazy_static! {
    pub static ref PROJECT_NAME: String = env!("CARGO_CRATE_NAME").to_uppercase().to_string();
    pub static ref DATA_FOLDER: Option<PathBuf> =
        env::var(format!("{}_DATA", PROJECT_NAME.clone()))
            .ok()
            .map(PathBuf::from);
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
    let directory = if let Some(s) = DATA_FOLDER.clone() {
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

    use crate::cli::ClapSource;

    use super::*;

    #[test]
    fn test_default_config() {
        let temp_data = tempdir_in(".").unwrap();

        temp_env::with_vars(
            [(
                format!("{}_DATA", PROJECT_NAME.clone()).as_str(),
                Some(temp_data.path().to_str().unwrap()),
            )],
            || {
                let config = Config::new(None).unwrap();
                println!("{:?}", config);
                assert!(config.config.data_dir.exists());

                assert_eq!(
                    config.config.db_path(),
                    config.config.data_dir.join("transactions.db")
                );
            },
        );
    }

    #[test]
    fn test_config_with_cli() {
        color_eyre::install().unwrap();
        let args = crate::cli::Cli::parse_from(&["test-config", "--data-dir", ".cli-data"]);
        let config = Config::new(Some(ClapSource::new(&args))).expect("Failed to load config");

        assert_eq!(config.config.data_dir, PathBuf::from(".cli-data"));
    }
}
