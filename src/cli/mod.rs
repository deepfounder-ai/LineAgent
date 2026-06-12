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
    /// Project management.
    #[command(subcommand)]
    Project(ProjectCmd),
    /// Ticket management.
    #[command(subcommand)]
    Ticket(TicketCmd),
    /// Comment management.
    #[command(subcommand)]
    Comment(CommentCmd),
    /// Relation management.
    #[command(subcommand)]
    Relation(RelationCmd),
    /// Cycle management.
    #[command(subcommand)]
    Cycle(CycleCmd),
    /// Full-text search across tickets.
    Search {
        /// Query string.
        query: String,
        /// Maximum number of results.
        #[arg(long)]
        limit: Option<i64>,
    },
    /// Dump the search index.
    Index,
    /// Show the audit / event log.
    Log {
        /// Only events at or after this timestamp (RFC 3339).
        #[arg(long)]
        since: Option<String>,
        /// Maximum number of events to return.
        #[arg(long)]
        limit: Option<i64>,
    },
    /// Import data from external sources.
    #[command(subcommand)]
    Import(ImportCmd),
}

// ---------------------------------------------------------------------------
// Import
// ---------------------------------------------------------------------------

#[derive(Debug, Subcommand)]
pub enum ImportCmd {
    /// Import all issues from a Linear workspace into LineAgent.
    Linear {
        /// Linear personal API key (overrides LINEAGENT_LINEAR_API_KEY).
        #[arg(long)]
        linear_key: Option<String>,
        /// Only import these team keys (repeatable). Imports all teams if omitted.
        #[arg(long = "team")]
        teams: Vec<String>,
        /// Dry-run: fetch from Linear and print what would be created, but don't write anything.
        #[arg(long)]
        dry_run: bool,
    },
}

// ---------------------------------------------------------------------------
// User
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Keys
// ---------------------------------------------------------------------------

#[derive(Debug, Subcommand)]
pub enum KeysCmd {
    List,
    Create { name: String },
    Revoke { id: String },
}

// ---------------------------------------------------------------------------
// Projects
// ---------------------------------------------------------------------------

#[derive(Debug, Subcommand)]
pub enum ProjectCmd {
    /// List all projects.
    List,
    /// Get a single project by key.
    Get {
        /// Project key (e.g. LIN).
        key: String,
    },
    /// Create a new project.
    Create {
        /// Short uppercase key (e.g. LIN).
        key: String,
        /// Human-readable name.
        #[arg(long)]
        name: String,
        /// Optional description.
        #[arg(long)]
        description: Option<String>,
    },
    /// Update an existing project.
    Update {
        /// Project key.
        key: String,
        /// New name.
        #[arg(long)]
        name: Option<String>,
        /// New description.
        #[arg(long)]
        description: Option<String>,
    },
}

// ---------------------------------------------------------------------------
// Tickets
// ---------------------------------------------------------------------------

#[derive(Debug, Subcommand)]
pub enum TicketCmd {
    /// List tickets, with optional filters.
    List {
        /// Filter by project key.
        #[arg(long)]
        project: Option<String>,
        /// Filter by status.
        #[arg(long)]
        status: Option<String>,
        /// Filter by priority.
        #[arg(long)]
        priority: Option<String>,
        /// Filter by assignee.
        #[arg(long)]
        assignee: Option<String>,
        /// Maximum number of tickets to return.
        #[arg(long)]
        limit: Option<i64>,
    },
    /// Get a single ticket by ID.
    Get {
        /// Ticket ID (e.g. LIN-1).
        id: String,
    },
    /// Create a new ticket.
    Create {
        /// Project key.
        #[arg(long)]
        project: String,
        /// Ticket title.
        #[arg(long)]
        title: String,
        /// Optional description.
        #[arg(long)]
        description: Option<String>,
        /// Status (e.g. open, in_progress, closed).
        #[arg(long)]
        status: Option<String>,
        /// Priority (e.g. low, medium, high, urgent).
        #[arg(long)]
        priority: Option<String>,
        /// Assignee username or ID.
        #[arg(long)]
        assignee: Option<String>,
    },
    /// Update a ticket.
    Update {
        /// Ticket ID.
        id: String,
        /// New title.
        #[arg(long)]
        title: Option<String>,
        /// New status.
        #[arg(long)]
        status: Option<String>,
        /// New priority.
        #[arg(long)]
        priority: Option<String>,
        /// New description.
        #[arg(long)]
        description: Option<String>,
        /// New assignee username or ID.
        #[arg(long)]
        assignee: Option<String>,
        /// New parent ticket identifier.
        #[arg(long, name = "parent")]
        parent_identifier: Option<String>,
    },
    /// Delete a ticket.
    Delete {
        /// Ticket ID.
        id: String,
    },
}

// ---------------------------------------------------------------------------
// Comments
// ---------------------------------------------------------------------------

#[derive(Debug, Subcommand)]
pub enum CommentCmd {
    /// List comments on a ticket.
    List {
        /// Ticket ID.
        ticket_id: String,
    },
    /// Add a comment to a ticket.
    Add {
        /// Ticket ID.
        ticket_id: String,
        /// Comment body.
        #[arg(long)]
        body: String,
        /// Author username or ID (optional).
        #[arg(long)]
        author: Option<String>,
    },
}

// ---------------------------------------------------------------------------
// Relations
// ---------------------------------------------------------------------------

#[derive(Debug, Subcommand)]
pub enum RelationCmd {
    /// List relations for a ticket.
    List {
        /// Ticket ID.
        ticket_id: String,
    },
    /// Add a relation between two tickets.
    Add {
        /// Source ticket ID.
        #[arg(long)]
        from: String,
        /// Target ticket ID.
        #[arg(long)]
        to: String,
        /// Relation type (e.g. blocks, relates_to, duplicates).
        #[arg(long, name = "type")]
        rtype: String,
    },
    /// Remove a relation by its ID.
    Remove {
        /// Relation ID.
        relation_id: String,
    },
}

// ---------------------------------------------------------------------------
// Cycles
// ---------------------------------------------------------------------------

#[derive(Debug, Subcommand)]
pub enum CycleCmd {
    /// List cycles for a project.
    List {
        /// Project key.
        #[arg(long)]
        project: Option<String>,
    },
    /// Create a new cycle.
    Create {
        /// Project key.
        #[arg(long)]
        project: String,
        /// Cycle name.
        #[arg(long)]
        name: String,
        /// Start date/time (RFC 3339).
        #[arg(long)]
        starts_at: Option<String>,
        /// End date/time (RFC 3339).
        #[arg(long)]
        ends_at: Option<String>,
    },
    /// Update an existing cycle.
    Update {
        /// Cycle ID.
        cycle_id: String,
        /// New name.
        #[arg(long)]
        name: Option<String>,
        /// New start date/time.
        #[arg(long)]
        starts_at: Option<String>,
        /// New end date/time.
        #[arg(long)]
        ends_at: Option<String>,
    },
}
