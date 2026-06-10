//! CLI surface (clap derive). The CLI is a thin HTTP client over the REST
//! API; `serve`, `mcp`, and `completions` are the only subcommands that run
//! in-process.

pub mod client;
pub mod commands;
pub mod config;
pub mod output;

use clap::{Parser, Subcommand};
use clap_complete::Shell;

#[derive(Debug, Parser)]
#[command(
    name = "lineagent",
    version,
    about = "Issue tracker for AI agents",
    long_about = None,
)]
pub struct Cli {
    /// Base URL of the lineagent server (overrides LINEAGENT_API_URL).
    #[arg(long, global = true)]
    pub api_url: Option<String>,

    /// API key (overrides LINEAGENT_API_KEY).
    #[arg(long, global = true)]
    pub api_key: Option<String>,

    /// Emit raw JSON instead of human-readable output.
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Cmd,
}

#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Run the HTTP API server.
    Serve {
        /// Override host (defaults to LINEAGENT_HOST or 0.0.0.0).
        #[arg(long)]
        host: Option<String>,
        /// Override port (defaults to LINEAGENT_PORT or 8080).
        #[arg(long)]
        port: Option<u16>,
    },
    /// Run the MCP server on stdio.
    Mcp,
    /// Generate shell completions.
    Completions {
        /// Target shell.
        shell: Shell,
    },
    /// User management.
    #[command(subcommand)]
    User(UserCmd),
    /// API key management.
    #[command(subcommand)]
    Keys(KeysCmd),
}

#[derive(Debug, Subcommand)]
pub enum UserCmd {
    /// Register a new user; prints the initial API key once.
    Register {
        username: String,
        #[arg(long)]
        password: Option<String>,
        /// Read the password from stdin (trailing newline trimmed).
        #[arg(long)]
        password_stdin: bool,
    },
    /// Exchange username + password for a fresh API key.
    Login {
        username: String,
        #[arg(long)]
        password: Option<String>,
        #[arg(long)]
        password_stdin: bool,
    },
    /// Print the user_id + username of the configured API key.
    Whoami,
}

#[derive(Debug, Subcommand)]
pub enum KeysCmd {
    List,
    Create { name: String },
    Revoke { id: String },
}
