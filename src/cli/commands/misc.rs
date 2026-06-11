//! `serve`, `mcp`, `completions`, `search`, `index`, `log`.

use clap::CommandFactory;

use crate::cli::client::Client;
use crate::cli::config::CliConfig;
use crate::cli::output::{print_json, CliError, CliResult};
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

// ---------------------------------------------------------------------------
// HTTP-backed commands
// ---------------------------------------------------------------------------

/// `search QUERY [--limit N]`
pub async fn run_search(
    query: &str,
    limit: Option<i64>,
    cfg: &CliConfig,
) -> CliResult<()> {
    let client = Client::new(cfg)?;
    // Simple percent-encoding for query string values.
    let encoded = url_encode(query);
    let mut path = format!("/api/v1/search?q={encoded}");
    if let Some(l) = limit {
        path.push_str(&format!("&limit={l}"));
    }
    let results: serde_json::Value = client.get(&path).await?;
    print_json(&results)?;
    Ok(())
}

/// `index` — dump the search index.
pub async fn run_index(cfg: &CliConfig) -> CliResult<()> {
    let client = Client::new(cfg)?;
    let result: serde_json::Value = client.get("/api/v1/index").await?;
    print_json(&result)?;
    Ok(())
}

/// `log [--since <ts>] [--limit N]`
pub async fn run_log(
    since: Option<&str>,
    limit: Option<i64>,
    cfg: &CliConfig,
) -> CliResult<()> {
    let client = Client::new(cfg)?;
    let mut params: Vec<String> = Vec::new();
    if let Some(s) = since {
        params.push(format!("since={}", url_encode(s)));
    }
    if let Some(l) = limit {
        params.push(format!("limit={l}"));
    }
    let path = if params.is_empty() {
        "/api/v1/log".to_string()
    } else {
        format!("/api/v1/log?{}", params.join("&"))
    };
    let result: serde_json::Value = client.get(&path).await?;
    print_json(&result)?;
    Ok(())
}

/// Minimal percent-encoder for query-string values (encodes everything except
/// unreserved chars: ALPHA / DIGIT / "-" / "." / "_" / "~").
fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'.'
            | b'_'
            | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
