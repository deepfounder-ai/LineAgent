//! Environment-driven configuration.
//!
//! All values are loaded once at process start. The `LINEAGENT_*` env vars take
//! precedence over compiled defaults. Tests construct [`Config::for_test`]
//! directly with a temporary directory.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Top-level service configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// HTTP host to bind to.
    #[serde(default = "default_host")]
    pub host: String,

    /// HTTP port to listen on.
    #[serde(default = "default_port")]
    pub port: u16,

    /// Directory for the SQLite database and on-disk user data.
    pub data_dir: PathBuf,

    /// SQLx connection string. If empty, defaults to
    /// `sqlite://<data_dir>/lineagent.db?mode=rwc`.
    #[serde(default)]
    pub db_url: String,

    /// Log filter (overridden by `RUST_LOG` if set in the environment).
    #[serde(default = "default_log_filter")]
    pub log_filter: String,

    /// If set, registration requires this secret in the request body.
    /// Leave unset to allow open registration.
    #[serde(default)]
    pub registration_secret: Option<String>,

    /// Slack bot token (`xoxb-…`). If set, ticket events are posted to Slack.
    #[serde(default)]
    pub slack_token: Option<String>,

    /// Slack channel to post ticket events to (e.g. `#lineagent`).
    #[serde(default)]
    pub slack_channel: Option<String>,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_log_filter() -> String {
    "lineagent=info,tower_http=info,axum=info".to_string()
}

impl Config {
    /// Build a config from environment variables. Falls back to `./data` for
    /// `data_dir` and `8080` for `port` if nothing is set.
    pub fn from_env() -> Result<Self, ConfigError> {
        let data_dir: PathBuf = std::env::var("LINEAGENT_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./data"));

        let port: u16 = match std::env::var("LINEAGENT_PORT") {
            Ok(s) => s.parse().map_err(|e: std::num::ParseIntError| {
                ConfigError::ParseInt(format!("LINEAGENT_PORT: {e}"))
            })?,
            Err(_) => default_port(),
        };

        let host = std::env::var("LINEAGENT_HOST").unwrap_or_else(|_| default_host());
        let db_url = std::env::var("LINEAGENT_DB_URL").unwrap_or_default();
        let log_filter = std::env::var("LINEAGENT_LOG")
            .or_else(|_| std::env::var("RUST_LOG"))
            .unwrap_or_else(|_| default_log_filter());

        let registration_secret = std::env::var("LINEAGENT_SECRET").ok();
        let slack_token = std::env::var("LINEAGENT_SLACK_TOKEN").ok();
        let slack_channel = std::env::var("LINEAGENT_SLACK_CHANNEL").ok();

        Ok(Self {
            host,
            port,
            data_dir,
            db_url,
            log_filter,
            registration_secret,
            slack_token,
            slack_channel,
        })
    }

    /// Build a deterministic config for tests. The data dir is the caller's
    /// responsibility — pass a `tempfile::TempDir` path so cleanup is easy.
    pub fn for_test(data_dir: PathBuf) -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 0,
            data_dir,
            db_url: String::new(),
            log_filter: "warn".to_string(),
            registration_secret: None,
            slack_token: None,
            slack_channel: None,
        }
    }

    /// Resolve the final SQLite connection URL, taking `db_url` overrides
    /// into account.
    pub fn resolved_db_url(&self) -> String {
        if self.db_url.is_empty() {
            let path = self.data_dir.join("lineagent.db");
            format!("sqlite://{}?mode=rwc", path.display())
        } else {
            self.db_url.clone()
        }
    }
}

/// Configuration errors.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("invalid integer in env: {0}")]
    ParseInt(String),
}
