//! Subcommand dispatch. Each top-level `Cmd` variant is handled by a
//! function in one of the sibling modules:
//!
//! - [`user`] — `user register`, `user login`, `user whoami`
//! - [`keys`] — API key management
//! - [`misc`] — `serve`, `mcp`, `completions`
//!
//! The [`run`] function is the entry point called from `main.rs`. It
//! resolves the [`CliConfig`], builds a [`Client`], and dispatches.

pub mod keys;
pub mod misc;
pub mod user;

use std::process::ExitCode;

use crate::cli::client::Client;
use crate::cli::config::CliConfig;
use crate::cli::output::{fail, CliResult};
use crate::cli::{Cli, Cmd};

/// Entry point. Returns an [`ExitCode`] suitable for `std::process::exit`.
pub async fn run(cli: Cli) -> ExitCode {
    match dispatch(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => fail(e),
    }
}

async fn dispatch(cli: Cli) -> CliResult<()> {
    // serve / mcp are special — they don't need a client.
    match &cli.command {
        Cmd::Serve { .. } => {
            return misc::run_serve(&cli).await;
        }
        Cmd::Mcp => {
            return misc::run_mcp().await;
        }
        Cmd::Completions { shell } => {
            misc::run_completions(*shell);
            return Ok(());
        }
        _ => {}
    }

    // All other commands need a resolved config.
    let config = CliConfig::load(cli.api_url.clone(), cli.api_key.clone())?;
    let client = Client::new(&config)?;

    match cli.command {
        Cmd::Serve { .. } | Cmd::Mcp | Cmd::Completions { .. } => unreachable!(),
        Cmd::User(u) => user::run(client, cli.json, u).await,
        Cmd::Keys(k) => keys::run(client, cli.json, k).await,
    }
}

/// Helper: build a stable path under `/api/v1`.
pub fn api_v1(suffix: &str) -> String {
    if suffix.starts_with('/') {
        format!("/api/v1{suffix}")
    } else {
        format!("/api/v1/{suffix}")
    }
}

/// Trivial helper for handlers that don't return data.
pub fn ok<T>(t: T) -> CliResult<T> {
    Ok(t)
}
