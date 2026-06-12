use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::core::comment::CommentService;
use crate::core::cycle::CycleService;
use crate::core::index::IndexService;
use crate::core::project::ProjectService;
use crate::core::relation::RelationService;
use crate::core::search::SearchService;
use crate::core::ticket::{CreateTicket, TicketService, TicketServiceFilter, UpdateTicket};
use crate::error::{AppError, Result};
use crate::mcp::AuthedContext;
use crate::storage::event_repo;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextContent {
    #[serde(rename = "type")]
    pub kind: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

impl TextContent {
    pub fn text(s: impl Into<String>) -> Self {
        Self {
            kind: "text".into(),
            text: s.into(),
            mime_type: None,
        }
    }

    pub fn json(v: impl serde::Serialize) -> Self {
        Self::text(serde_json::to_string_pretty(&serde_json::to_value(v).unwrap()).unwrap())
    }
}

// ---------------------------------------------------------------------------
// Helper arg extractors
// ---------------------------------------------------------------------------

fn arg_str_req(args: &Value, key: &str) -> Result<String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| AppError::Validation(format!("missing required arg: {key}")))
}

fn arg_str_opt(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(|v| v.as_str()).map(str::to_string)
}

fn arg_i64_opt(args: &Value, key: &str) -> Option<i64> {
    args.get(key).and_then(|v| v.as_i64())
}

// ---------------------------------------------------------------------------
// Tool definition helper
// ---------------------------------------------------------------------------

fn tool(name: &str, description: &str, input_schema: Value) -> Tool {
    Tool {
        name: name.to_string(),
        description: description.to_string(),
        input_schema,
    }
}

// ---------------------------------------------------------------------------
// list_tools — 19 tool definitions
// ---------------------------------------------------------------------------

pub fn list_tools() -> Vec<Tool> {
    vec![
        // ── Tickets ────────────────────────────────────────────────────────
        tool(
            "create_ticket",
            "Create a new ticket in a project.",
            json!({
                "type": "object",
                "properties": {
                    "project_key":         { "type": "string", "description": "Project key (e.g. LIN)." },
                    "title":               { "type": "string", "description": "Ticket title." },
                    "description":         { "type": "string", "description": "Ticket description (markdown)." },
                    "status":              { "type": "string", "enum": ["backlog","todo","in_progress","review","done","cancelled"], "description": "Initial status. Defaults to backlog." },
                    "priority":            { "type": "string", "enum": ["critical","high","medium","low"], "description": "Priority. Defaults to medium." },
                    "assignee":            { "type": "string", "description": "Username or identifier of the assignee." },
                    "parent_identifier":   { "type": "string", "description": "Identifier of the parent ticket (e.g. LIN-1)." },
                    "cycle_id":            { "type": "string", "description": "Cycle id to assign the ticket to." }
                },
                "required": ["project_key", "title"],
                "additionalProperties": false
            }),
        ),
        tool(
            "update_ticket",
            "Update an existing ticket.",
            json!({
                "type": "object",
                "properties": {
                    "identifier":          { "type": "string", "description": "Ticket identifier (e.g. LIN-1)." },
                    "title":               { "type": "string" },
                    "description":         { "type": "string" },
                    "status":              { "type": "string", "enum": ["backlog","todo","in_progress","review","done","cancelled"] },
                    "priority":            { "type": "string", "enum": ["critical","high","medium","low"] },
                    "assignee":            { "type": "string" },
                    "parent_identifier":   { "type": "string", "description": "Identifier of the parent ticket." },
                    "cycle_id":            { "type": "string" }
                },
                "required": ["identifier"],
                "additionalProperties": false
            }),
        ),
        tool(
            "get_ticket",
            "Get a single ticket with its comments and relations.",
            json!({
                "type": "object",
                "properties": {
                    "identifier": { "type": "string", "description": "Ticket identifier (e.g. LIN-1)." }
                },
                "required": ["identifier"],
                "additionalProperties": false
            }),
        ),
        tool(
            "list_tickets",
            "List tickets with optional filters.",
            json!({
                "type": "object",
                "properties": {
                    "project_key":       { "type": "string", "description": "Filter by project key." },
                    "status":            { "type": "string", "enum": ["backlog","todo","in_progress","review","done","cancelled"] },
                    "priority":          { "type": "string", "enum": ["critical","high","medium","low"] },
                    "assignee":          { "type": "string" },
                    "cycle_id":          { "type": "string" },
                    "parent_identifier": { "type": "string" },
                    "limit":             { "type": "integer", "minimum": 1, "maximum": 1000, "description": "Max results (default 20)." }
                },
                "additionalProperties": false
            }),
        ),
        tool(
            "delete_ticket",
            "Delete a ticket. Idempotent — no error if already absent.",
            json!({
                "type": "object",
                "properties": {
                    "identifier": { "type": "string", "description": "Ticket identifier (e.g. LIN-1)." }
                },
                "required": ["identifier"],
                "additionalProperties": false
            }),
        ),
        // ── Comments ───────────────────────────────────────────────────────
        tool(
            "add_comment",
            "Add a comment to a ticket.",
            json!({
                "type": "object",
                "properties": {
                    "ticket_identifier": { "type": "string", "description": "Ticket identifier (e.g. LIN-1)." },
                    "body":              { "type": "string", "description": "Comment body (markdown)." },
                    "author":            { "type": "string", "description": "Author name or id." }
                },
                "required": ["ticket_identifier", "body"],
                "additionalProperties": false
            }),
        ),
        tool(
            "list_comments",
            "List all comments for a ticket.",
            json!({
                "type": "object",
                "properties": {
                    "ticket_identifier": { "type": "string", "description": "Ticket identifier (e.g. LIN-1)." }
                },
                "required": ["ticket_identifier"],
                "additionalProperties": false
            }),
        ),
        // ── Relations ──────────────────────────────────────────────────────
        tool(
            "add_relation",
            "Add a directed relation between two tickets.",
            json!({
                "type": "object",
                "properties": {
                    "from_identifier": { "type": "string", "description": "Source ticket identifier." },
                    "to_identifier":   { "type": "string", "description": "Target ticket identifier." },
                    "relation_type":   { "type": "string", "enum": ["blocks","duplicates","relates_to"], "description": "Relation type." }
                },
                "required": ["from_identifier", "to_identifier", "relation_type"],
                "additionalProperties": false
            }),
        ),
        tool(
            "remove_relation",
            "Remove a relation by its id.",
            json!({
                "type": "object",
                "properties": {
                    "relation_id": { "type": "string", "description": "Relation id." }
                },
                "required": ["relation_id"],
                "additionalProperties": false
            }),
        ),
        // ── Search ─────────────────────────────────────────────────────────
        tool(
            "search_tickets",
            "Full-text search over tickets (FTS5 BM25).",
            json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search query." },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 1000, "description": "Max results (default 20)." }
                },
                "required": ["query"],
                "additionalProperties": false
            }),
        ),
        // ── Projects ───────────────────────────────────────────────────────
        tool(
            "create_project",
            "Create a new project.",
            json!({
                "type": "object",
                "properties": {
                    "key":         { "type": "string", "description": "Short uppercase key (e.g. LIN). Auto-uppercased." },
                    "name":        { "type": "string", "description": "Human-readable project name." },
                    "description": { "type": "string", "description": "Optional project description." }
                },
                "required": ["key", "name"],
                "additionalProperties": false
            }),
        ),
        tool(
            "get_project",
            "Get a project by key.",
            json!({
                "type": "object",
                "properties": {
                    "key": { "type": "string", "description": "Project key (e.g. LIN)." }
                },
                "required": ["key"],
                "additionalProperties": false
            }),
        ),
        tool(
            "update_project",
            "Update a project's name and/or description.",
            json!({
                "type": "object",
                "properties": {
                    "key":         { "type": "string", "description": "Project key (e.g. LIN)." },
                    "name":        { "type": "string" },
                    "description": { "type": "string" }
                },
                "required": ["key"],
                "additionalProperties": false
            }),
        ),
        tool(
            "list_projects",
            "List all projects for the current user.",
            json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        ),
        // ── Cycles ─────────────────────────────────────────────────────────
        tool(
            "create_cycle",
            "Create a new cycle (sprint) in a project.",
            json!({
                "type": "object",
                "properties": {
                    "project_key": { "type": "string", "description": "Project key (e.g. LIN)." },
                    "name":        { "type": "string", "description": "Cycle name." },
                    "starts_at":   { "type": "string", "description": "ISO 8601 start date/time." },
                    "ends_at":     { "type": "string", "description": "ISO 8601 end date/time." }
                },
                "required": ["project_key", "name"],
                "additionalProperties": false
            }),
        ),
        tool(
            "update_cycle",
            "Update a cycle's mutable fields.",
            json!({
                "type": "object",
                "properties": {
                    "cycle_id":  { "type": "string", "description": "Cycle id." },
                    "name":      { "type": "string" },
                    "starts_at": { "type": "string", "description": "ISO 8601 start date/time." },
                    "ends_at":   { "type": "string", "description": "ISO 8601 end date/time." }
                },
                "required": ["cycle_id"],
                "additionalProperties": false
            }),
        ),
        tool(
            "list_cycles",
            "List all cycles for a project.",
            json!({
                "type": "object",
                "properties": {
                    "project_key": { "type": "string", "description": "Project key (e.g. LIN)." }
                },
                "required": ["project_key"],
                "additionalProperties": false
            }),
        ),
        // ── Misc ───────────────────────────────────────────────────────────
        tool(
            "get_log",
            "Return recent audit-log events. Optional since (RFC3339) and limit.",
            json!({
                "type": "object",
                "properties": {
                    "since": { "type": "string", "description": "RFC3339 timestamp; only events after this are returned." },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 1000, "description": "Max results (default 100)." }
                },
                "additionalProperties": false
            }),
        ),
        tool(
            "get_index",
            "Return an index of all projects with ticket status counts.",
            json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        ),
    ]
}

// ---------------------------------------------------------------------------
// call_tool — dispatch
// ---------------------------------------------------------------------------

pub async fn call_tool(name: &str, args: Value, ctx: &AuthedContext) -> Result<Vec<TextContent>> {
    let user_id = &ctx.user_id;
    let state = ctx.ctx.state.clone();

    match name {
        // ── Tickets ────────────────────────────────────────────────────────
        "create_ticket" => {
            let svc = TicketService::new(state);
            let input = CreateTicket {
                project_key: arg_str_req(&args, "project_key")?,
                title: arg_str_req(&args, "title")?,
                description: arg_str_opt(&args, "description"),
                status: arg_str_opt(&args, "status"),
                priority: arg_str_opt(&args, "priority"),
                assignee: arg_str_opt(&args, "assignee"),
                parent_identifier: arg_str_opt(&args, "parent_identifier"),
                cycle_id: arg_str_opt(&args, "cycle_id"),
            };
            let ticket = svc.create(user_id, input).await?;
            Ok(vec![TextContent::json(ticket)])
        }

        "update_ticket" => {
            let identifier = arg_str_req(&args, "identifier")?;
            let svc = TicketService::new(state);
            let patch = UpdateTicket {
                title: arg_str_opt(&args, "title"),
                description: arg_str_opt(&args, "description"),
                status: arg_str_opt(&args, "status"),
                priority: arg_str_opt(&args, "priority"),
                assignee: arg_str_opt(&args, "assignee"),
                parent_identifier: arg_str_opt(&args, "parent_identifier"),
                cycle_id: arg_str_opt(&args, "cycle_id"),
            };
            let ticket = svc.update(user_id, &identifier, patch).await?;
            Ok(vec![TextContent::json(ticket)])
        }

        "get_ticket" => {
            let identifier = arg_str_req(&args, "identifier")?;
            let svc = TicketService::new(state);
            let view = svc.get(user_id, &identifier).await?;
            Ok(vec![TextContent::json(view)])
        }

        "list_tickets" => {
            let svc = TicketService::new(state);
            let filter = TicketServiceFilter {
                project_key: arg_str_opt(&args, "project_key"),
                status: arg_str_opt(&args, "status"),
                priority: arg_str_opt(&args, "priority"),
                assignee: arg_str_opt(&args, "assignee"),
                cycle_id: arg_str_opt(&args, "cycle_id"),
                parent_identifier: arg_str_opt(&args, "parent_identifier"),
                limit: arg_i64_opt(&args, "limit"),
            };
            let tickets = svc.list(user_id, filter).await?;
            Ok(vec![TextContent::json(tickets)])
        }

        "delete_ticket" => {
            let identifier = arg_str_req(&args, "identifier")?;
            let svc = TicketService::new(state);
            svc.delete(user_id, &identifier).await?;
            Ok(vec![TextContent::json(
                json!({ "ok": true, "identifier": identifier }),
            )])
        }

        // ── Comments ───────────────────────────────────────────────────────
        "add_comment" => {
            let ticket_identifier = arg_str_req(&args, "ticket_identifier")?;
            let body = arg_str_req(&args, "body")?;
            let author = arg_str_opt(&args, "author");
            let svc = CommentService::new(state);
            let comment = svc
                .add(user_id, &ticket_identifier, author.as_deref(), &body)
                .await?;
            Ok(vec![TextContent::json(comment)])
        }

        "list_comments" => {
            let ticket_identifier = arg_str_req(&args, "ticket_identifier")?;
            let svc = CommentService::new(state);
            let comments = svc.list(user_id, &ticket_identifier).await?;
            Ok(vec![TextContent::json(comments)])
        }

        // ── Relations ──────────────────────────────────────────────────────
        "add_relation" => {
            let from_identifier = arg_str_req(&args, "from_identifier")?;
            let to_identifier = arg_str_req(&args, "to_identifier")?;
            let relation_type = arg_str_req(&args, "relation_type")?;
            let svc = RelationService::new(state);
            let relation = svc
                .add(user_id, &from_identifier, &to_identifier, &relation_type)
                .await?;
            Ok(vec![TextContent::json(relation)])
        }

        "remove_relation" => {
            let relation_id = arg_str_req(&args, "relation_id")?;
            let svc = RelationService::new(state);
            svc.remove(user_id, &relation_id).await?;
            Ok(vec![TextContent::json(
                json!({ "ok": true, "relation_id": relation_id }),
            )])
        }

        // ── Search ─────────────────────────────────────────────────────────
        "search_tickets" => {
            let query = arg_str_req(&args, "query")?;
            let limit = arg_i64_opt(&args, "limit");
            let svc = SearchService::new(state);
            let hits = svc.search(user_id, &query, limit).await?;
            Ok(vec![TextContent::json(hits)])
        }

        // ── Projects ───────────────────────────────────────────────────────
        "create_project" => {
            let key = arg_str_req(&args, "key")?;
            let name = arg_str_req(&args, "name")?;
            let description = arg_str_opt(&args, "description");
            let svc = ProjectService::new(state);
            let project = svc
                .create(user_id, &key, &name, description.as_deref())
                .await?;
            Ok(vec![TextContent::json(project)])
        }

        "get_project" => {
            let key = arg_str_req(&args, "key")?;
            let svc = ProjectService::new(state);
            let project = svc.get(user_id, &key).await?;
            Ok(vec![TextContent::json(project)])
        }

        "update_project" => {
            let key = arg_str_req(&args, "key")?;
            let name = arg_str_opt(&args, "name");
            let description = arg_str_opt(&args, "description");
            let svc = ProjectService::new(state);
            let project = svc
                .update(user_id, &key, name.as_deref(), description.as_deref())
                .await?;
            Ok(vec![TextContent::json(project)])
        }

        "list_projects" => {
            let svc = ProjectService::new(state);
            let projects = svc.list(user_id).await?;
            Ok(vec![TextContent::json(projects)])
        }

        // ── Cycles ─────────────────────────────────────────────────────────
        "create_cycle" => {
            let project_key = arg_str_req(&args, "project_key")?;
            let name = arg_str_req(&args, "name")?;
            let starts_at = arg_str_opt(&args, "starts_at");
            let ends_at = arg_str_opt(&args, "ends_at");
            let svc = CycleService::new(state);
            let cycle = svc
                .create(
                    user_id,
                    &project_key,
                    &name,
                    starts_at.as_deref(),
                    ends_at.as_deref(),
                )
                .await?;
            Ok(vec![TextContent::json(cycle)])
        }

        "update_cycle" => {
            let cycle_id = arg_str_req(&args, "cycle_id")?;
            let name = arg_str_opt(&args, "name");
            let starts_at = arg_str_opt(&args, "starts_at");
            let ends_at = arg_str_opt(&args, "ends_at");
            let svc = CycleService::new(state);
            let cycle = svc
                .update(
                    user_id,
                    &cycle_id,
                    name.as_deref(),
                    starts_at.as_deref(),
                    ends_at.as_deref(),
                )
                .await?;
            Ok(vec![TextContent::json(cycle)])
        }

        "list_cycles" => {
            let project_key = arg_str_req(&args, "project_key")?;
            let svc = CycleService::new(state);
            let cycles = svc.list(user_id, &project_key).await?;
            Ok(vec![TextContent::json(cycles)])
        }

        // ── Misc ───────────────────────────────────────────────────────────
        "get_log" => {
            let since = arg_str_opt(&args, "since");
            let limit = arg_i64_opt(&args, "limit");

            let since_dt = match since {
                Some(ref s) => Some(
                    chrono::DateTime::parse_from_rfc3339(s)
                        .map(|d| d.with_timezone(&chrono::Utc))
                        .map_err(|e| AppError::Validation(format!("invalid since: {e}")))?,
                ),
                None => None,
            };

            let filter = event_repo::EventFilter {
                since: since_dt,
                limit,
            };
            let events = event_repo::list_for_user(&state.db, user_id, &filter).await?;
            Ok(vec![TextContent::json(events)])
        }

        "get_index" => {
            let svc = IndexService::new(state);
            let index = svc.build(user_id).await?;
            Ok(vec![TextContent::json(index)])
        }

        // ── Unknown ────────────────────────────────────────────────────────
        other => Err(AppError::Validation(format!("unknown tool '{other}'"))),
    }
}
