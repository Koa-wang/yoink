use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub max_entries: usize,
    pub history_days: i64,
    pub ignored_patterns: Vec<String>,
    pub theme: String,
    pub preview_length: usize,
    pub daemon: DaemonConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DaemonConfig {
    pub poll_interval_ms: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            history_days: 30,
            ignored_patterns: Vec::new(),
            theme: "dark".to_string(),
            preview_length: 80,
            daemon: DaemonConfig::default(),
        }
    }
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self { poll_interval_ms: 500 }
    }
}

impl Config {
    /// Load config from disk, creating a default file if none exists.
    pub fn load() -> Result<Config> {
        let path = config_path();
        if path.exists() {
            let raw = std::fs::read_to_string(&path)?;
            let mut cfg: Config = toml::from_str(&raw)
                .map_err(|e| anyhow::anyhow!("invalid config at {}: {e}", path.display()))?;
            cfg.sanitize();
            Ok(cfg)
        } else {
            let cfg = Config::default();
            cfg.save()?;
            Ok(cfg)
        }
    }

    pub fn save(&self) -> Result<()> {
        std::fs::create_dir_all(config_dir())?;
        std::fs::write(config_path(), self.to_toml())?;
        Ok(())
    }

    pub fn to_toml(&self) -> String {
        toml::to_string_pretty(self).unwrap_or_default()
    }

    /// Clamp invalid values to sane defaults.
    pub fn sanitize(&mut self) {
        if self.max_entries == 0 {
            self.max_entries = 1000;
        }
        if self.preview_length == 0 {
            self.preview_length = 80;
        }
        if self.daemon.poll_interval_ms == 0 {
            self.daemon.poll_interval_ms = 500;
        }
        if self.history_days < 0 {
            self.history_days = 30;
        }
        if self.theme.is_empty() {
            self.theme = "dark".to_string();
        }
    }

    pub fn ignored_regexes(&self) -> Vec<regex::Regex> {
        self.ignored_patterns
            .iter()
            .filter_map(|p| regex::Regex::new(p).ok())
            .collect()
    }
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .join("yoink")
}

pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .join("yoink")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

pub fn db_path() -> PathBuf {
    data_dir().join("yoink.db")
}
