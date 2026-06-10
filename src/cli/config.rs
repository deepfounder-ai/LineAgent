//! CLI configuration resolution.
//!
//! Precedence (highest to lowest):
//!
//! 1. Explicit `--api-url` / `--api-key` flags
//! 2. `LINEAGENT_API_URL` / `LINEAGENT_API_KEY` env vars
//! 3. `~/.config/lineagent/config.toml` (optional)
//! 4. Compiled defaults (`http://127.0.0.1:8080`)
//!
//! The config file is **optional** — a missing or unreadable file is
//! treated as "no override" rather than a hard error, so the CLI is
//! usable out of the box.
//!
//! ## Credentials
//!
//! On a successful `user register` / `user login`, the CLI persists the
//! freshly minted API key to `~/.config/lineagent/credentials.toml` with
//! `0600` permissions. Subsequent invocations re-read it automatically
//! when no other source of truth is set.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::cli::output::{CliError, CliResult};

const DEFAULT_API_URL: &str = "http://127.0.0.1:8080";

/// Resolved CLI configuration.
#[derive(Debug, Clone)]
pub struct CliConfig {
    pub api_url: String,
    pub api_key: Option<String>,
    pub credentials_path: PathBuf,
    pub config_path: Option<PathBuf>,
}

/// On-disk shape of `~/.config/lineagent/config.toml`. All fields are
/// optional — the resolver fills the gaps with env vars or defaults.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileConfig {
    #[serde(default)]
    pub api_url: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
}

/// On-disk shape of `~/.config/lineagent/credentials.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Credentials {
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    /// When the credentials file was last written.
    #[serde(default)]
    pub updated_at: Option<String>,
}

impl CliConfig {
    /// Resolve config from flags + env + config file.
    pub fn load(api_url: Option<String>, api_key: Option<String>) -> CliResult<Self> {
        let config_path = std::env::var("LINEAGENT_CONFIG")
            .ok()
            .map(PathBuf::from)
            .or_else(default_config_path);

        // Read the config file. Missing file is fine.
        let file_cfg = config_path
            .as_deref()
            .and_then(read_file_config)
            .unwrap_or_default();

        let api_url = api_url
            .or_else(|| std::env::var("LINEAGENT_API_URL").ok())
            .or(file_cfg.api_url.clone())
            .unwrap_or_else(|| DEFAULT_API_URL.to_string());

        let api_key = api_key
            .or_else(|| std::env::var("LINEAGENT_API_KEY").ok())
            .or_else(|| file_cfg.api_key.clone())
            .or_else(|| read_credentials().and_then(|c| c.api_key));

        Ok(Self {
            api_url,
            api_key,
            credentials_path: default_credentials_path(),
            config_path,
        })
    }
}

impl Credentials {
    /// Persist this credentials struct to `~/.config/lineagent/credentials.toml`
    /// with `0600` permissions. The parent directory is created if missing.
    pub fn save_to_default(&self) -> CliResult<()> {
        let path = default_credentials_path();
        save_credentials_to(&path, self)
    }
}

/// `~/.config/lineagent/credentials.toml`.
pub fn default_credentials_path() -> PathBuf {
    config_dir().join("credentials.toml")
}

/// `~/.config/lineagent/config.toml`.
pub fn default_config_path() -> Option<PathBuf> {
    Some(config_dir().join("config.toml"))
}

fn config_dir() -> PathBuf {
    // XDG_CONFIG_HOME wins; otherwise ~/.config (XDG default).
    let base = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .ok()
                .map(|h| PathBuf::from(h).join(".config"))
        })
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("lineagent")
}

fn read_file_config(path: &Path) -> Option<FileConfig> {
    let body = fs::read_to_string(path).ok()?;
    // Tolerate empty files (treated as no config).
    if body.trim().is_empty() {
        return None;
    }
    match toml::from_str::<FileConfig>(&body) {
        Ok(c) => Some(c),
        Err(e) => {
            eprintln!("lineagent: warning: could not parse {}: {e}", path.display());
            None
        }
    }
}

/// Read `~/.config/lineagent/credentials.toml` if it exists.
pub fn read_credentials() -> Option<Credentials> {
    let path = default_credentials_path();
    let body = fs::read_to_string(&path).ok()?;
    if body.trim().is_empty() {
        return None;
    }
    match toml::from_str::<Credentials>(&body) {
        Ok(c) => Some(c),
        Err(e) => {
            eprintln!("lineagent: warning: could not parse {}: {e}", path.display());
            None
        }
    }
}

/// Persist credentials. Atomic-ish: write to a sibling temp file, set
/// mode 0600, rename.
pub fn save_credentials_to(path: &Path, creds: &Credentials) -> CliResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(CliError::Io)?;
    }
    let body = toml::to_string(creds)
        .map_err(|e| CliError::Other(format!("serialize credentials: {e}")))?;
    let tmp = path.with_extension("toml.tmp");
    {
        let mut f = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&tmp)
            .map_err(CliError::Io)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            let mut opts = std::fs::OpenOptions::new();
            opts.create(true).write(true).truncate(true).mode(0o600);
            f = opts.open(&tmp).map_err(CliError::Io)?;
        }
        f.write_all(body.as_bytes()).map_err(CliError::Io)?;
        f.sync_all().map_err(CliError::Io)?;
    }
    fs::rename(&tmp, path).map_err(CliError::Io)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_dir_is_under_lineagent() {
        let d = config_dir();
        assert!(d.ends_with("lineagent"), "got {d:?}");
    }

    #[test]
    fn default_url_is_loopback() {
        assert!(DEFAULT_API_URL.contains("127.0.0.1"));
    }

    #[test]
    fn read_empty_file_config_is_none() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("config.toml");
        std::fs::write(&p, "").unwrap();
        assert!(read_file_config(&p).is_none());
    }

    #[test]
    fn read_file_config_parses() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("config.toml");
        std::fs::write(&p, "api_url = \"http://example:9000\"\n").unwrap();
        let c = read_file_config(&p).unwrap();
        assert_eq!(c.api_url.as_deref(), Some("http://example:9000"));
    }

    #[test]
    fn credentials_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("credentials.toml");
        let c = Credentials {
            api_key: Some("lineagent_xxx".into()),
            username: Some("alice".into()),
            updated_at: Some("2026-06-10T00:00:00Z".into()),
        };
        save_credentials_to(&p, &c).unwrap();
        let body = std::fs::read_to_string(&p).unwrap();
        let back: Credentials = toml::from_str(&body).unwrap();
        assert_eq!(back.api_key.as_deref(), Some("lineagent_xxx"));
        assert_eq!(back.username.as_deref(), Some("alice"));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let meta = std::fs::metadata(&p).unwrap();
            let mode = meta.permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "credentials file must be 0600, got {mode:o}");
        }
    }
}
