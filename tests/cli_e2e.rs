//! End-to-end CLI tests.
//!
//! Each test boots the full Axum stack on an ephemeral port, registers a user,
//! then calls the CLI command handler functions directly with a `CliConfig`
//! pointing at the test server. No subprocess is spawned.

use lineagent::cli::commands::{comments, cycles, misc, projects, relations, tickets};
use lineagent::cli::config::CliConfig;
use lineagent::cli::{CommentCmd, CycleCmd, ProjectCmd, RelationCmd, TicketCmd};
use lineagent::config::Config;
use serde_json::Value;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Test harness
// ---------------------------------------------------------------------------

struct TestServer {
    base: String,
    http: reqwest::Client,
    _dir: tempfile::TempDir,
}

impl TestServer {
    async fn start() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg = Config::for_test(dir.path().to_path_buf());
        let state = lineagent::storage::init_pool(cfg).await.expect("init pool");
        let app = lineagent::api::router(state);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        TestServer {
            base: format!("http://{addr}"),
            http: reqwest::Client::new(),
            _dir: dir,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }

    /// Register a fresh user and return its bearer API key.
    async fn register(&self, username: &str) -> String {
        let resp = self
            .http
            .post(self.url("/api/v1/auth/register"))
            .json(&serde_json::json!({ "username": username, "password": "hunter2hunter2" }))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 201, "register should 201");
        let body: Value = resp.json().await.unwrap();
        body["api_key"].as_str().unwrap().to_string()
    }

    /// Build a `CliConfig` for this server with the given API key.
    fn cli_config(&self, api_key: &str) -> CliConfig {
        CliConfig {
            api_url: self.base.clone(),
            api_key: Some(api_key.to_string()),
            credentials_path: PathBuf::from("/tmp/test-credentials.toml"),
            config_path: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Helper: capture stdout from a handler
// ---------------------------------------------------------------------------

/// We can't easily capture stdout in unit style, so we just assert the calls
/// succeed (return Ok). JSON output correctness is verified via the API tests.
macro_rules! ok {
    ($e:expr) => {
        $e.await.unwrap_or_else(|e| panic!("command failed: {e}"))
    };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn cli_project_crud() {
    let s = TestServer::start().await;
    let key = s.register("alice").await;
    let cfg = s.cli_config(&key);

    // Create project
    ok!(projects::run(
        &ProjectCmd::Create {
            key: "LIN".to_string(),
            name: "Linear Clone".to_string(),
            description: Some("Test project".to_string()),
        },
        &cfg,
        false
    ));

    // List projects
    ok!(projects::run(&ProjectCmd::List, &cfg, false));

    // Get project
    ok!(projects::run(
        &ProjectCmd::Get {
            key: "LIN".to_string(),
        },
        &cfg,
        true
    ));

    // Update project
    ok!(projects::run(
        &ProjectCmd::Update {
            key: "LIN".to_string(),
            name: Some("Linear Clone Updated".to_string()),
            description: None,
        },
        &cfg,
        false
    ));
}

#[tokio::test]
async fn cli_ticket_crud() {
    let s = TestServer::start().await;
    let key = s.register("bob").await;
    let cfg = s.cli_config(&key);

    // Need a project first
    ok!(projects::run(
        &ProjectCmd::Create {
            key: "LIN".to_string(),
            name: "Linear Clone".to_string(),
            description: None,
        },
        &cfg,
        false
    ));

    // Create ticket → should become LIN-1
    ok!(tickets::run(
        &TicketCmd::Create {
            project: "LIN".to_string(),
            title: "First issue".to_string(),
            description: Some("desc".to_string()),
            status: None,
            priority: None,
            assignee: None,
        },
        &cfg,
        false
    ));

    // List tickets
    ok!(tickets::run(
        &TicketCmd::List {
            project: Some("LIN".to_string()),
            status: None,
            priority: None,
            assignee: None,
            limit: None,
        },
        &cfg,
        false
    ));

    // Get ticket
    ok!(tickets::run(
        &TicketCmd::Get {
            id: "LIN-1".to_string(),
        },
        &cfg,
        true
    ));

    // Update ticket
    ok!(tickets::run(
        &TicketCmd::Update {
            id: "LIN-1".to_string(),
            title: None,
            status: Some("done".to_string()),
            priority: None,
            description: None,
            assignee: None,
            parent_identifier: None,
        },
        &cfg,
        false
    ));

    // Delete ticket
    ok!(tickets::run(
        &TicketCmd::Delete {
            id: "LIN-1".to_string(),
        },
        &cfg,
        false
    ));
}

#[tokio::test]
async fn cli_comment_flow() {
    let s = TestServer::start().await;
    let key = s.register("carol").await;
    let cfg = s.cli_config(&key);

    // Setup
    ok!(projects::run(
        &ProjectCmd::Create {
            key: "LIN".to_string(),
            name: "Linear Clone".to_string(),
            description: None,
        },
        &cfg,
        false
    ));
    ok!(tickets::run(
        &TicketCmd::Create {
            project: "LIN".to_string(),
            title: "Issue with comments".to_string(),
            description: None,
            status: None,
            priority: None,
            assignee: None,
        },
        &cfg,
        false
    ));

    // Add comment
    ok!(comments::run(
        &CommentCmd::Add {
            ticket_id: "LIN-1".to_string(),
            body: "Great issue!".to_string(),
            author: Some("carol".to_string()),
        },
        &cfg,
        false
    ));

    // List comments
    ok!(comments::run(
        &CommentCmd::List {
            ticket_id: "LIN-1".to_string(),
        },
        &cfg,
        true
    ));
}

#[tokio::test]
async fn cli_relation_flow() {
    let s = TestServer::start().await;
    let key = s.register("dave").await;
    let cfg = s.cli_config(&key);

    // Setup
    ok!(projects::run(
        &ProjectCmd::Create {
            key: "LIN".to_string(),
            name: "Linear Clone".to_string(),
            description: None,
        },
        &cfg,
        false
    ));
    ok!(tickets::run(
        &TicketCmd::Create {
            project: "LIN".to_string(),
            title: "Ticket A".to_string(),
            description: None,
            status: None,
            priority: None,
            assignee: None,
        },
        &cfg,
        false
    ));
    ok!(tickets::run(
        &TicketCmd::Create {
            project: "LIN".to_string(),
            title: "Ticket B".to_string(),
            description: None,
            status: None,
            priority: None,
            assignee: None,
        },
        &cfg,
        false
    ));

    // Add relation
    ok!(relations::run(
        &RelationCmd::Add {
            from: "LIN-1".to_string(),
            to: "LIN-2".to_string(),
            rtype: "blocks".to_string(),
        },
        &cfg,
        false
    ));

    // List relations
    ok!(relations::run(
        &RelationCmd::List {
            ticket_id: "LIN-1".to_string(),
        },
        &cfg,
        true
    ));
}

#[tokio::test]
async fn cli_cycle_flow() {
    let s = TestServer::start().await;
    let key = s.register("eve").await;
    let cfg = s.cli_config(&key);

    // Setup project
    ok!(projects::run(
        &ProjectCmd::Create {
            key: "LIN".to_string(),
            name: "Linear Clone".to_string(),
            description: None,
        },
        &cfg,
        false
    ));

    // Create cycle
    ok!(cycles::run(
        &CycleCmd::Create {
            project: "LIN".to_string(),
            name: "Sprint 1".to_string(),
            starts_at: Some("2026-06-01T00:00:00Z".to_string()),
            ends_at: Some("2026-06-14T00:00:00Z".to_string()),
        },
        &cfg,
        false
    ));

    // List cycles
    ok!(cycles::run(
        &CycleCmd::List {
            project: Some("LIN".to_string()),
        },
        &cfg,
        true
    ));
}

#[tokio::test]
async fn cli_search_and_index() {
    let s = TestServer::start().await;
    let key = s.register("frank").await;
    let cfg = s.cli_config(&key);

    // Setup
    ok!(projects::run(
        &ProjectCmd::Create {
            key: "LIN".to_string(),
            name: "Linear Clone".to_string(),
            description: None,
        },
        &cfg,
        false
    ));
    ok!(tickets::run(
        &TicketCmd::Create {
            project: "LIN".to_string(),
            title: "Searchable ticket".to_string(),
            description: Some("unique-term-xyz".to_string()),
            status: None,
            priority: None,
            assignee: None,
        },
        &cfg,
        false
    ));

    // Search
    ok!(misc::run_search("Searchable", Some(10), &cfg));

    // Index
    ok!(misc::run_index(&cfg));

    // Log
    ok!(misc::run_log(None, Some(10), &cfg));
}

#[tokio::test]
async fn cli_project_list_json_mode() {
    let s = TestServer::start().await;
    let key = s.register("grace").await;
    let cfg = s.cli_config(&key);

    ok!(projects::run(
        &ProjectCmd::Create {
            key: "A".to_string(),
            name: "Alpha".to_string(),
            description: None,
        },
        &cfg,
        false
    ));
    ok!(projects::run(
        &ProjectCmd::Create {
            key: "B".to_string(),
            name: "Beta".to_string(),
            description: None,
        },
        &cfg,
        false
    ));

    // JSON mode for list
    ok!(projects::run(&ProjectCmd::List, &cfg, true));
}
