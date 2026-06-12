//! In-process tests for the MCP tool surface.
//!
//! Each test sets up a temporary in-memory SQLite store, creates a test user,
//! constructs an `AuthedContext`, and calls `list_tools()` / `call_tool()`
//! directly — no process spawning required.

use std::sync::Arc;

use lineagent::config::Config;
use lineagent::mcp::tools::{self, TextContent};
use lineagent::mcp::{AuthedContext, McpContext};
use lineagent::storage::pool::init_pool;
use lineagent::storage::AppState;
use serde_json::json;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Test harness
// ---------------------------------------------------------------------------

async fn setup() -> (AppState, AuthedContext) {
    let mut cfg = Config::for_test(std::path::PathBuf::from("/tmp"));
    cfg.db_url = "sqlite::memory:".to_string();
    let state = init_pool(cfg).await.expect("init_pool failed");

    // Insert a bare-bones user row so FK constraints are satisfied.
    let user_id = Uuid::now_v7().to_string();
    sqlx::query(
        "INSERT INTO users (id, username, password_hash, created_at) \
         VALUES (?1, 'testuser', 'hash', datetime('now'))",
    )
    .bind(&user_id)
    .execute(&state.db)
    .await
    .expect("insert user");

    let authed = AuthedContext {
        ctx: Arc::new(McpContext::new(state.clone())),
        user_id,
        api_key_id: "test-key-id".to_string(),
    };

    (state, authed)
}

// Decode the first TextContent item as JSON Value.
fn decode_json(content: &[TextContent]) -> serde_json::Value {
    assert!(!content.is_empty(), "expected at least one TextContent");
    serde_json::from_str(&content[0].text).expect("TextContent is not valid JSON")
}

// ---------------------------------------------------------------------------
// 1. list_tools — exactly 19 tools
// ---------------------------------------------------------------------------

#[test]
fn list_tools_returns_19_tools() {
    let tools = tools::list_tools();
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();

    let expected = [
        "create_ticket",
        "update_ticket",
        "get_ticket",
        "list_tickets",
        "delete_ticket",
        "add_comment",
        "list_comments",
        "add_relation",
        "remove_relation",
        "search_tickets",
        "create_project",
        "get_project",
        "update_project",
        "list_projects",
        "create_cycle",
        "update_cycle",
        "list_cycles",
        "get_log",
        "get_index",
    ];

    assert_eq!(
        tools.len(),
        expected.len(),
        "expected {} tools, got {}. Names: {:?}",
        expected.len(),
        tools.len(),
        names,
    );

    for name in &expected {
        assert!(
            names.contains(name),
            "tool '{name}' missing from list_tools(). Got: {names:?}",
        );
    }
}

// ---------------------------------------------------------------------------
// 2. create_project
// ---------------------------------------------------------------------------

#[tokio::test]
async fn call_create_project() {
    let (_state, ctx) = setup().await;

    let result = tools::call_tool(
        "create_project",
        json!({ "key": "LIN", "name": "LineAgent" }),
        &ctx,
    )
    .await
    .expect("create_project failed");

    let v = decode_json(&result);
    assert_eq!(v["key"].as_str().unwrap(), "LIN");
    assert_eq!(v["name"].as_str().unwrap(), "LineAgent");
}

// ---------------------------------------------------------------------------
// 3. create_ticket → identifier "LIN-1"
// ---------------------------------------------------------------------------

#[tokio::test]
async fn call_create_ticket_returns_lin1() {
    let (_state, ctx) = setup().await;

    // First create the project.
    tools::call_tool(
        "create_project",
        json!({ "key": "LIN", "name": "LineAgent" }),
        &ctx,
    )
    .await
    .expect("create_project failed");

    let result = tools::call_tool(
        "create_ticket",
        json!({ "project_key": "LIN", "title": "First ticket" }),
        &ctx,
    )
    .await
    .expect("create_ticket failed");

    let v = decode_json(&result);
    assert_eq!(v["identifier"].as_str().unwrap(), "LIN-1");
    assert_eq!(v["title"].as_str().unwrap(), "First ticket");
}

// ---------------------------------------------------------------------------
// 4. get_ticket returns ticket JSON
// ---------------------------------------------------------------------------

#[tokio::test]
async fn call_get_ticket() {
    let (_state, ctx) = setup().await;

    tools::call_tool(
        "create_project",
        json!({ "key": "LIN", "name": "LineAgent" }),
        &ctx,
    )
    .await
    .expect("create_project");

    tools::call_tool(
        "create_ticket",
        json!({ "project_key": "LIN", "title": "Fetch me" }),
        &ctx,
    )
    .await
    .expect("create_ticket");

    let result = tools::call_tool("get_ticket", json!({ "identifier": "LIN-1" }), &ctx)
        .await
        .expect("get_ticket");

    let v = decode_json(&result);
    assert_eq!(v["identifier"].as_str().unwrap(), "LIN-1");
    // TicketView also includes comments and relations arrays.
    assert!(v["comments"].is_array(), "expected comments array");
    assert!(v["relations"].is_array(), "expected relations array");
}

// ---------------------------------------------------------------------------
// 5. add_comment
// ---------------------------------------------------------------------------

#[tokio::test]
async fn call_add_comment() {
    let (_state, ctx) = setup().await;

    tools::call_tool(
        "create_project",
        json!({ "key": "LIN", "name": "LineAgent" }),
        &ctx,
    )
    .await
    .expect("create_project");

    tools::call_tool(
        "create_ticket",
        json!({ "project_key": "LIN", "title": "Commented ticket" }),
        &ctx,
    )
    .await
    .expect("create_ticket");

    let result = tools::call_tool(
        "add_comment",
        json!({ "ticket_identifier": "LIN-1", "body": "Hello world", "author": "agent" }),
        &ctx,
    )
    .await
    .expect("add_comment");

    let v = decode_json(&result);
    assert_eq!(v["body"].as_str().unwrap(), "Hello world");
    assert_eq!(v["author"].as_str().unwrap(), "agent");
}

// ---------------------------------------------------------------------------
// 6. search_tickets returns results
// ---------------------------------------------------------------------------

#[tokio::test]
async fn call_search_tickets() {
    let (_state, ctx) = setup().await;

    tools::call_tool(
        "create_project",
        json!({ "key": "LIN", "name": "LineAgent" }),
        &ctx,
    )
    .await
    .expect("create_project");

    tools::call_tool(
        "create_ticket",
        json!({ "project_key": "LIN", "title": "unique-search-term-xyz" }),
        &ctx,
    )
    .await
    .expect("create_ticket");

    let result = tools::call_tool(
        "search_tickets",
        json!({ "query": "unique-search-term-xyz" }),
        &ctx,
    )
    .await
    .expect("search_tickets");

    let v = decode_json(&result);
    let hits = v.as_array().expect("expected array");
    assert!(!hits.is_empty(), "expected at least one search hit");
    assert_eq!(hits[0]["identifier"].as_str().unwrap(), "LIN-1");
}

// ---------------------------------------------------------------------------
// 7. Unknown tool name → error
// ---------------------------------------------------------------------------

#[tokio::test]
async fn call_unknown_tool_returns_error() {
    let (_state, ctx) = setup().await;

    let result = tools::call_tool("no_such_tool", json!({}), &ctx).await;
    assert!(result.is_err(), "expected Err for unknown tool");
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("no_such_tool"),
        "error message should name the unknown tool: {msg}"
    );
}
