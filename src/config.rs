#![allow(dead_code)] // Remove this once you start using the code

use std::{env, path::PathBuf};

use color_eyre::Result;
use directories::ProjectDirs;
use lazy_static::lazy_static;
use serde::Deserialize;
use tracing::error;

#[derive(Clone, Debug, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub data_dir: PathBuf,
    #[serde(default)]
    pub config_dir: PathBuf,
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
    pub static ref CONFIG_FOLDER: Option<PathBuf> =
        env::var(format!("{}_CONFIG", PROJECT_NAME.clone()))
            .ok()
            .map(PathBuf::from);
}

impl Config {
    pub fn new() -> Result<Self, config::ConfigError> {
        let data_dir = get_data_dir();
        let config_dir = get_config_dir();
        let mut builder = config::Config::builder()
            .set_default("data_dir", data_dir.to_str().unwrap())?
            .set_default("config_dir", config_dir.to_str().unwrap())?
            .set_default("db_path", "transactions.db")?;

        let config_files = [
            ("config.json5", config::FileFormat::Json5),
            ("config.json", config::FileFormat::Json),
            ("config.yaml", config::FileFormat::Yaml),
            ("config.toml", config::FileFormat::Toml),
            ("config.ini", config::FileFormat::Ini),
        ];
        let mut found_config = false;
        for (file, format) in &config_files {
            let source = config::File::from(config_dir.join(file))
                .format(*format)
                .required(false);
            builder = builder.add_source(source);
            if config_dir.join(file).exists() {
                found_config = true
            }
        }
        if !found_config {
            error!("No configuration file found. Application may not behave as expected");
        }

        let cfg: Self = builder.build()?.try_deserialize()?;

        // cfg.fetch.cookie = env::var("COOKIE").unwrap_or(cfg.fetch.cookie);
        // cfg.fetch.account = env::var("ACCOUNT").unwrap_or(cfg.fetch.account);

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

pub fn get_config_dir() -> PathBuf {
    let directory = if let Some(s) = CONFIG_FOLDER.clone() {
        s
    } else if let Some(proj_dirs) = project_directory() {
        proj_dirs.config_local_dir().to_path_buf()
    } else {
        PathBuf::from(".").join(".config")
    };
    directory
}

fn project_directory() -> Option<ProjectDirs> {
    ProjectDirs::from("dev", "yyang", env!("CARGO_PKG_NAME"))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir_in;

    use super::*;

    #[test]
    fn test_default_config() {
        let temp_data = tempdir_in(".").unwrap();
        let temp_config = tempdir_in(".").unwrap();

        // write a config.yaml file to temp_config
        let config_file = temp_config.path().join("config.yaml");

        temp_env::with_vars(
            [
                (
                    format!("{}_DATA", PROJECT_NAME.clone()).as_str(),
                    Some(temp_data.path().to_str().unwrap()),
                ),
                (
                    format!("{}_CONFIG", PROJECT_NAME.clone()).as_str(),
                    Some(temp_config.path().to_str().unwrap()),
                ),
            ],
            || {
                fs::write(
                    &config_file,
                    include_str!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/test/mock-data/config.yaml"
                    )),
                )
                .unwrap();

                let config = Config::new().unwrap();
                println!("{:?}", config);
                assert!(config.config.data_dir.exists());
                assert!(config.config.config_dir.exists());

                assert_eq!(
                    config.config.db_path(),
                    config.config.data_dir.join("transactions.db")
                );
            },
        );
    }
}
