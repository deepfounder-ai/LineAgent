//! `serve`, `mcp`, `completions`.

use clap::CommandFactory;

use crate::cli::output::{CliError, CliResult};
use crate::cli::{Cli, Cmd};
use crate::config::Config;

// ---------------------------------------------------------------------------
// In-process commands (no client)
// ---------------------------------------------------------------------------

/// `serve` — boot the HTTP API in-process.
pub async fn run_serve(cli: &Cli) -> CliResult<()> {
    let (host, port) = match &cli.command {
        Cmd::Serve { host, port } => (host.clone(), *port),
        _ => (None, None),
    };

    let mut config = Config::from_env().map_err(|e| CliError::Other(format!("config: {e}")))?;
    if let Some(h) = host {
        config.host = h;
    }
    if let Some(p) = port {
        config.port = p;
    }
    std::fs::create_dir_all(&config.data_dir)?;

    let state = crate::storage::init_pool(config)
        .await
        .map_err(|e| CliError::Other(format!("init store: {e}")))?;
    let cfg = state.config.clone();
    crate::api::serve(state, &cfg)
        .await
        .map_err(|e| CliError::Other(format!("server: {e}")))?;
    Ok(())
}

/// `mcp` — run the stdio MCP server.
pub async fn run_mcp() -> CliResult<()> {
    crate::mcp::run_stdio()
        .await
        .map_err(|e| CliError::Other(e.to_string()))
}

/// `completions <shell>` — print a completion script to stdout.
pub fn run_completions(shell: clap_complete::Shell) {
    let mut cmd = Cli::command();
    clap_complete::generate(shell, &mut cmd, "lineagent", &mut std::io::stdout());
}
