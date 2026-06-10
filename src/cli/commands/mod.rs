//! Subcommand dispatch. Each top-level `Cmd` variant is handled by a
//! function in one of the sibling modules:
//!
//! - [`user`] — `user register`, `user login`, `user whoami`
//! - [`keys`] — API key management
//! - [`misc`] — `serve`, `mcp`, `completions`, `search`, `index`, `log`
//! - [`projects`] — project CRUD
//! - [`tickets`] — ticket CRUD
//! - [`comments`] — comment list / add
//! - [`relations`] — relation list / add / remove
//! - [`cycles`] — cycle list / create / update
//!
//! The [`run`] function is the entry point called from `main.rs`. It
//! resolves the [`CliConfig`], builds a [`Client`], and dispatches.

pub mod comments;
pub mod cycles;
pub mod keys;
pub mod misc;
pub mod projects;
pub mod relations;
pub mod tickets;
pub mod user;

use std::process::ExitCode;

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
    // serve / mcp / completions are special — they don't need a client.
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
    let json = cli.json;

    match cli.command {
        Cmd::Serve { .. } | Cmd::Mcp | Cmd::Completions { .. } => unreachable!(),
        Cmd::User(u) => {
            use crate::cli::client::Client;
            let client = Client::new(&config)?;
            user::run(client, json, u).await
        }
        Cmd::Keys(k) => {
            use crate::cli::client::Client;
            let client = Client::new(&config)?;
            keys::run(client, json, k).await
        }
        Cmd::Project(p) => projects::run(&p, &config, json).await,
        Cmd::Ticket(t) => tickets::run(&t, &config, json).await,
        Cmd::Comment(c) => comments::run(&c, &config, json).await,
        Cmd::Relation(r) => relations::run(&r, &config, json).await,
        Cmd::Cycle(c) => cycles::run(&c, &config, json).await,
        Cmd::Search { query, limit } => {
            misc::run_search(&query, limit, &config).await
        }
        Cmd::Index => misc::run_index(&config).await,
        Cmd::Log { since, limit } => {
            misc::run_log(since.as_deref(), limit, &config).await
        }
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
