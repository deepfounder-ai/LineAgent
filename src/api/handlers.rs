//! HTTP handlers. Each handler is a thin adapter: parse the request,
//! call an `auth` service scoped by the authenticated user, and
//! shape the response. Domain errors flow back as [`AppError`] and are
//! mapped to [`ApiError`] by the `?` operator.

use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

use crate::api::dto::*;
use crate::auth::middleware::AuthContext;
use crate::auth::UserService;
use crate::core::comment::CommentService;
use crate::core::cycle::CycleService;
use crate::core::index::IndexService;
use crate::core::project::ProjectService;
use crate::core::relation::RelationService;
use crate::core::search::SearchService;
use crate::core::ticket::{CreateTicket, TicketService, TicketServiceFilter, UpdateTicket};
use crate::error::{ApiError, AppError};
use crate::storage::{event_repo, user_repo, AppState};

type ApiResult<T> = Result<T, ApiError>;

// ---------------------------------------------------------------------------
// Health
// ---------------------------------------------------------------------------

/// Root landing page — HTML description of the lineagent service.
pub async fn root(State(state): State<AppState>) -> Response {
    let host = if state.config.host == "0.0.0.0" || state.config.host.is_empty() {
        "127.0.0.1".to_string()
    } else {
        state.config.host.clone()
    };
    let base = format!("http://{host}:{}", state.config.port);
    let html = LANDING_HTML.replace("__BASE__", &base);
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html,
    )
        .into_response()
}

const LANDING_HTML: &str = r###"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>lineagent</title>
<style>
  :root { color-scheme: light dark; }
  body { font: 15px/1.6 system-ui, -apple-system, sans-serif; max-width: 760px;
         margin: 3rem auto; padding: 0 1.2rem; }
  h1 { margin-bottom: .2rem; }
  .tag { color: #888; margin-top: 0; }
  code, pre { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }
  pre { background: rgba(127,127,127,.12); padding: .8rem 1rem; border-radius: 8px;
        overflow-x: auto; }
  code { background: rgba(127,127,127,.15); padding: .1rem .35rem; border-radius: 4px; }
  pre code { background: none; padding: 0; }
  h2 { margin-top: 2rem; border-bottom: 1px solid rgba(127,127,127,.25); padding-bottom: .3rem; }
  table { border-collapse: collapse; width: 100%; }
  td, th { text-align: left; padding: .35rem .6rem; border-bottom: 1px solid rgba(127,127,127,.18); }
  .muted { color: #888; font-size: .9em; }
</style>
</head>
<body>
<h1>lineagent</h1>
<p class="tag">Issue tracker for AI agents — REST, MCP, and CLI over SQLite.</p>

<h2>Endpoints</h2>
<table>
<tr><th>Path</th><th>Purpose</th></tr>
<tr><td><code>GET /healthz</code></td><td>Liveness probe (no auth).</td></tr>
<tr><td><code>POST /api/v1/auth/register</code></td><td>Create a user, get first API key.</td></tr>
<tr><td><code>POST /api/v1/auth/login</code></td><td>Exchange username+password for a fresh key.</td></tr>
<tr><td><code>GET /api/v1/auth/whoami</code></td><td>Inspect current principal.</td></tr>
<tr><td><code>GET|POST /api/v1/auth/keys</code></td><td>List / create API keys.</td></tr>
<tr><td><code>DELETE /api/v1/auth/keys/:id</code></td><td>Revoke an API key.</td></tr>
</table>

<h2>Connect over REST</h2>
<pre><code># 1. register
curl -sS -X POST __BASE__/api/v1/auth/register \
  -H 'Content-Type: application/json' \
  -d '{"username":"alice","password":"correct horse battery staple"}'
# -> {"user_id":"…","api_key":"lineagent_…", …}

# 2. use the key
export KEY=lineagent_…
curl -sS __BASE__/api/v1/auth/whoami -H "Authorization: Bearer $KEY"</code></pre>

<h2>Connect over MCP</h2>
<pre><code>{
  "mcpServers": {
    "lineagent": {
      "command": "lineagent",
      "args": ["mcp"],
      "env": {
        "LINEAGENT_API_URL": "__BASE__",
        "LINEAGENT_API_KEY": "lineagent_…"
      }
    }
  }
}</code></pre>

<p class="muted">See docs/ for REST, CLI, and MCP references.</p>
</body>
</html>
"###;

pub async fn healthz() -> Response {
    (
        StatusCode::OK,
        Json(HealthResponse {
            status: "ok",
            version: crate::VERSION,
            build_rev: crate::BUILD_REV,
        }),
    )
        .into_response()
}

// ---------------------------------------------------------------------------
// Auth
// ---------------------------------------------------------------------------

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> ApiResult<Response> {
    validate_api_username(&req.username).map_err(unprocessable)?;
    validate_api_password(&req.password).map_err(unprocessable)?;

    let svc = UserService::new(state);
    let (user, key) = svc.register(&req.username, &req.password).await?;
    let body = AuthResponse::new(user.id, user.username, key.id, key.plaintext);
    Ok((StatusCode::CREATED, Json(body)).into_response())
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> ApiResult<Response> {
    let svc = UserService::new(state);
    let (user, key) = svc.login(&req.username, &req.password).await?;
    let body = AuthResponse::new(user.id, user.username, key.id, key.plaintext);
    Ok((StatusCode::OK, Json(body)).into_response())
}

pub async fn whoami(State(state): State<AppState>, ctx: AuthContext) -> ApiResult<Response> {
    let user = user_repo::get_by_id(&state.db, &ctx.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("user".into()))?;
    Ok(Json(json!({
        "user_id": user.id,
        "username": user.username,
        "api_key_id": ctx.api_key_id,
    }))
    .into_response())
}

pub async fn list_keys(State(state): State<AppState>, ctx: AuthContext) -> ApiResult<Response> {
    let svc = UserService::new(state);
    let rows = svc.list_api_keys(&ctx.user_id).await?;
    let keys: Vec<KeyView> = rows.into_iter().map(KeyView::from).collect();
    Ok(Json(json!({ "keys": keys })).into_response())
}

pub async fn create_key(
    State(state): State<AppState>,
    ctx: AuthContext,
    Json(req): Json<CreateKeyRequest>,
) -> ApiResult<Response> {
    validate_key_name(&req.name).map_err(unprocessable)?;
    let svc = UserService::new(state);
    let key = svc.create_api_key(&ctx.user_id, &req.name).await?;
    let view = CreatedKeyView {
        view: KeyView {
            id: key.id,
            name: key.name,
            created_at: chrono::Utc::now(),
            last_used_at: None,
        },
        api_key: key.plaintext,
    };
    Ok((StatusCode::CREATED, Json(view)).into_response())
}

pub async fn revoke_key(
    State(state): State<AppState>,
    ctx: AuthContext,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let svc = UserService::new(state);
    // Idempotent: a missing id still returns 204 so callers cannot probe.
    svc.revoke_api_key(&ctx.user_id, &id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

// ---------------------------------------------------------------------------
// Projects
// ---------------------------------------------------------------------------

pub async fn list_projects(
    State(state): State<AppState>,
    ctx: AuthContext,
) -> ApiResult<Response> {
    let svc = ProjectService::new(state);
    let projects = svc.list(&ctx.user_id).await?;
    Ok(Json(projects).into_response())
}

pub async fn create_project(
    State(state): State<AppState>,
    ctx: AuthContext,
    Json(req): Json<CreateProjectReq>,
) -> ApiResult<Response> {
    let svc = ProjectService::new(state);
    let project = svc
        .create(&ctx.user_id, &req.key, &req.name, req.description.as_deref())
        .await?;
    Ok((StatusCode::CREATED, Json(project)).into_response())
}

pub async fn get_project(
    State(state): State<AppState>,
    ctx: AuthContext,
    Path(key): Path<String>,
) -> ApiResult<Response> {
    let svc = ProjectService::new(state);
    let project = svc.get(&ctx.user_id, &key).await?;
    Ok(Json(project).into_response())
}

pub async fn update_project(
    State(state): State<AppState>,
    ctx: AuthContext,
    Path(key): Path<String>,
    Json(req): Json<UpdateProjectReq>,
) -> ApiResult<Response> {
    let svc = ProjectService::new(state);
    let project = svc
        .update(&ctx.user_id, &key, req.name.as_deref(), req.description.as_deref())
        .await?;
    Ok(Json(project).into_response())
}

// ---------------------------------------------------------------------------
// Tickets
// ---------------------------------------------------------------------------

pub async fn list_tickets(
    State(state): State<AppState>,
    ctx: AuthContext,
    Query(q): Query<ListTicketsQuery>,
) -> ApiResult<Response> {
    let svc = TicketService::new(state);
    let filter = TicketServiceFilter {
        project_key: q.project,
        status: q.status,
        priority: q.priority,
        assignee: q.assignee,
        cycle_id: q.cycle_id,
        parent_identifier: q.parent,
        limit: q.limit,
    };
    let tickets = svc.list(&ctx.user_id, filter).await?;
    Ok(Json(tickets).into_response())
}

pub async fn create_ticket(
    State(state): State<AppState>,
    ctx: AuthContext,
    Json(req): Json<CreateTicketReq>,
) -> ApiResult<Response> {
    let svc = TicketService::new(state);
    let input = CreateTicket {
        project_key: req.project_key,
        title: req.title,
        description: req.description,
        status: req.status,
        priority: req.priority,
        assignee: req.assignee,
        parent_identifier: req.parent_identifier,
        cycle_id: req.cycle_id,
    };
    let ticket = svc.create(&ctx.user_id, input).await?;
    Ok((StatusCode::CREATED, Json(ticket)).into_response())
}

pub async fn get_ticket(
    State(state): State<AppState>,
    ctx: AuthContext,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let svc = TicketService::new(state);
    let view = svc.get(&ctx.user_id, &id).await?;
    Ok(Json(view).into_response())
}

pub async fn update_ticket(
    State(state): State<AppState>,
    ctx: AuthContext,
    Path(id): Path<String>,
    Json(req): Json<UpdateTicketReq>,
) -> ApiResult<Response> {
    let svc = TicketService::new(state);
    let patch = UpdateTicket {
        title: req.title,
        description: req.description,
        status: req.status,
        priority: req.priority,
        assignee: req.assignee,
        parent_identifier: req.parent_identifier,
        cycle_id: req.cycle_id,
    };
    let ticket = svc.update(&ctx.user_id, &id, patch).await?;
    Ok(Json(ticket).into_response())
}

pub async fn delete_ticket(
    State(state): State<AppState>,
    ctx: AuthContext,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let svc = TicketService::new(state);
    svc.delete(&ctx.user_id, &id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

// ---------------------------------------------------------------------------
// Comments
// ---------------------------------------------------------------------------

pub async fn list_comments(
    State(state): State<AppState>,
    ctx: AuthContext,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let svc = CommentService::new(state);
    let comments = svc.list(&ctx.user_id, &id).await?;
    Ok(Json(comments).into_response())
}

pub async fn add_comment(
    State(state): State<AppState>,
    ctx: AuthContext,
    Path(id): Path<String>,
    Json(req): Json<AddCommentReq>,
) -> ApiResult<Response> {
    let svc = CommentService::new(state);
    let comment = svc
        .add(&ctx.user_id, &id, req.author.as_deref(), &req.body)
        .await?;
    Ok((StatusCode::CREATED, Json(comment)).into_response())
}

// ---------------------------------------------------------------------------
// Relations
// ---------------------------------------------------------------------------

pub async fn list_relations(
    State(state): State<AppState>,
    ctx: AuthContext,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    // Resolve ticket identifier → ticket view (which includes relations)
    let svc = TicketService::new(state);
    let view = svc.get(&ctx.user_id, &id).await?;
    Ok(Json(view.relations).into_response())
}

pub async fn add_relation(
    State(state): State<AppState>,
    ctx: AuthContext,
    Json(req): Json<AddRelationReq>,
) -> ApiResult<Response> {
    let svc = RelationService::new(state);
    let relation = svc
        .add(
            &ctx.user_id,
            &req.from_identifier,
            &req.to_identifier,
            &req.relation_type,
        )
        .await?;
    Ok((StatusCode::CREATED, Json(relation)).into_response())
}

pub async fn remove_relation(
    State(state): State<AppState>,
    ctx: AuthContext,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let svc = RelationService::new(state);
    svc.remove(&ctx.user_id, &id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

// ---------------------------------------------------------------------------
// Cycles
// ---------------------------------------------------------------------------

pub async fn list_cycles(
    State(state): State<AppState>,
    ctx: AuthContext,
    Path(key): Path<String>,
) -> ApiResult<Response> {
    let svc = CycleService::new(state);
    let cycles = svc.list(&ctx.user_id, &key).await?;
    Ok(Json(cycles).into_response())
}

pub async fn create_cycle(
    State(state): State<AppState>,
    ctx: AuthContext,
    Path(key): Path<String>,
    Json(req): Json<CreateCycleReq>,
) -> ApiResult<Response> {
    let svc = CycleService::new(state);
    let cycle = svc
        .create(
            &ctx.user_id,
            &key,
            &req.name,
            req.starts_at.as_deref(),
            req.ends_at.as_deref(),
        )
        .await?;
    Ok((StatusCode::CREATED, Json(cycle)).into_response())
}

pub async fn update_cycle(
    State(state): State<AppState>,
    ctx: AuthContext,
    Path(id): Path<String>,
    Json(req): Json<UpdateCycleReq>,
) -> ApiResult<Response> {
    let svc = CycleService::new(state);
    let cycle = svc
        .update(
            &ctx.user_id,
            &id,
            req.name.as_deref(),
            req.starts_at.as_deref(),
            req.ends_at.as_deref(),
        )
        .await?;
    Ok(Json(cycle).into_response())
}

// ---------------------------------------------------------------------------
// Search / Index / Log
// ---------------------------------------------------------------------------

pub async fn search_tickets(
    State(state): State<AppState>,
    ctx: AuthContext,
    Query(q): Query<SearchQuery>,
) -> ApiResult<Response> {
    let svc = SearchService::new(state);
    let hits = svc.search(&ctx.user_id, &q.q, q.limit).await?;
    Ok(Json(json!({ "count": hits.len(), "hits": hits })).into_response())
}

pub async fn get_index(
    State(state): State<AppState>,
    ctx: AuthContext,
) -> ApiResult<Response> {
    let svc = IndexService::new(state);
    let index = svc.build(&ctx.user_id).await?;
    Ok(Json(index).into_response())
}

pub async fn get_log(
    State(state): State<AppState>,
    ctx: AuthContext,
    Query(q): Query<LogQuery>,
) -> ApiResult<Response> {
    let since = q
        .since
        .as_deref()
        .map(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .map(|d| d.with_timezone(&chrono::Utc))
                .map_err(|e| AppError::BadRequest(format!("invalid since datetime: {e}")))
        })
        .transpose()?;

    let filter = event_repo::EventFilter {
        since,
        limit: q.limit,
    };
    let events = event_repo::list_for_user(&state.db, &ctx.user_id, &filter).await?;
    Ok(Json(json!({ "items": events })).into_response())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn unprocessable(msg: String) -> AppError {
    AppError::Unprocessable(msg)
}
