//! HTTP-proxy MCP mode for remote LineAgent instances.
//!
//! When `LINEAGENT_API_URL` is set, the MCP binary does not open a local
//! SQLite database. Instead it validates `LINEAGENT_API_KEY` against the
//! remote server via `GET /api/v1/auth/whoami`, then proxies every tool call
//! through the REST API.

use std::time::Duration;

use reqwest::{Client, Method};
use serde_json::{json, Value};

use crate::mcp::tools::list_tools;

// ---------------------------------------------------------------------------
// Proxy context
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct ProxyCtx {
    client: Client,
    base_url: String,
    api_key: String,
}

impl ProxyCtx {
    pub fn new(base_url: String, api_key: String) -> anyhow::Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(concat!("lineagent-mcp/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self { client, base_url, api_key })
    }

    fn url(&self, path: &str) -> String {
        let base = self.base_url.trim_end_matches('/');
        if path.starts_with('/') {
            format!("{base}{path}")
        } else {
            format!("{base}/{path}")
        }
    }

    async fn request(&self, method: Method, path: &str, body: Option<Value>) -> anyhow::Result<Value> {
        let url = self.url(path);
        let mut req = self.client.request(method, &url)
            .header("Authorization", format!("Bearer {}", self.api_key));
        if let Some(b) = body {
            req = req.json(&b);
        }
        let resp = req.send().await?;
        let status = resp.status();
        let text = resp.text().await?;
        if status == reqwest::StatusCode::NO_CONTENT || text.is_empty() {
            return Ok(json!(null));
        }
        let v: Value = serde_json::from_str(&text)
            .unwrap_or_else(|_| json!({"raw": text}));
        if !status.is_success() {
            anyhow::bail!("HTTP {}: {}", status, v);
        }
        Ok(v)
    }

    async fn get(&self, path: &str) -> anyhow::Result<Value> {
        self.request(Method::GET, path, None).await
    }
    async fn post(&self, path: &str, body: Value) -> anyhow::Result<Value> {
        self.request(Method::POST, path, Some(body)).await
    }
    async fn patch(&self, path: &str, body: Value) -> anyhow::Result<Value> {
        self.request(Method::PATCH, path, Some(body)).await
    }
    async fn delete(&self, path: &str) -> anyhow::Result<Value> {
        self.request(Method::DELETE, path, None).await
    }

    /// Validate the API key; returns the username on success.
    pub async fn whoami(&self) -> anyhow::Result<String> {
        let v = self.get("/api/v1/auth/whoami").await?;
        v["username"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| anyhow::anyhow!("whoami: unexpected response: {v}"))
    }
}

// ---------------------------------------------------------------------------
// Tool dispatch
// ---------------------------------------------------------------------------

fn arg_str_req(args: &Value, key: &str) -> anyhow::Result<String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| anyhow::anyhow!("missing required arg: {key}"))
}

fn arg_str_opt(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(|v| v.as_str()).map(str::to_string)
}

fn arg_i64_opt(args: &Value, key: &str) -> Option<i64> {
    args.get(key).and_then(|v| v.as_i64())
}

pub async fn dispatch_tool(ctx: &ProxyCtx, name: &str, args: &Value) -> Value {
    match call_tool(ctx, name, args).await {
        Ok(result) => json!({
            "content": [{"type": "text", "text": serde_json::to_string_pretty(&result).unwrap_or_default()}],
            "isError": false
        }),
        Err(e) => json!({
            "content": [{"type": "text", "text": e.to_string()}],
            "isError": true
        }),
    }
}

async fn call_tool(ctx: &ProxyCtx, name: &str, args: &Value) -> anyhow::Result<Value> {
    match name {
        // ── Tickets ─────────────────────────────────────────────────────────
        "create_ticket" => {
            let mut body = json!({
                "project_key": arg_str_req(args, "project_key")?,
                "title":       arg_str_req(args, "title")?,
            });
            for k in &["description", "status", "priority", "assignee", "parent_identifier", "cycle_id"] {
                if let Some(v) = arg_str_opt(args, k) { body[*k] = json!(v); }
            }
            ctx.post("/api/v1/tickets", body).await
        }
        "update_ticket" => {
            let id = arg_str_req(args, "identifier")?;
            let mut body = json!({});
            for k in &["title", "description", "status", "priority", "assignee", "parent_identifier", "cycle_id"] {
                if let Some(v) = arg_str_opt(args, k) { body[*k] = json!(v); }
            }
            ctx.patch(&format!("/api/v1/tickets/{id}"), body).await
        }
        "get_ticket" => {
            let id = arg_str_req(args, "identifier")?;
            ctx.get(&format!("/api/v1/tickets/{id}")).await
        }
        "list_tickets" => {
            let mut qs = Vec::new();
            if let Some(v) = arg_str_opt(args, "project_key") { qs.push(format!("project={v}")); }
            if let Some(v) = arg_str_opt(args, "status")      { qs.push(format!("status={v}")); }
            if let Some(v) = arg_str_opt(args, "priority")    { qs.push(format!("priority={v}")); }
            if let Some(v) = arg_str_opt(args, "assignee")    { qs.push(format!("assignee={v}")); }
            if let Some(v) = arg_str_opt(args, "cycle_id")    { qs.push(format!("cycle_id={v}")); }
            if let Some(v) = arg_str_opt(args, "parent_identifier") { qs.push(format!("parent={v}")); }
            if let Some(v) = arg_i64_opt(args, "limit")       { qs.push(format!("limit={v}")); }
            let path = if qs.is_empty() {
                "/api/v1/tickets".into()
            } else {
                format!("/api/v1/tickets?{}", qs.join("&"))
            };
            ctx.get(&path).await
        }
        "delete_ticket" => {
            let id = arg_str_req(args, "identifier")?;
            ctx.delete(&format!("/api/v1/tickets/{id}")).await
        }
        // ── Comments ────────────────────────────────────────────────────────
        "add_comment" => {
            let ticket_id = arg_str_req(args, "ticket_identifier")?;
            let mut body = json!({ "body": arg_str_req(args, "body")? });
            if let Some(v) = arg_str_opt(args, "author") { body["author"] = json!(v); }
            ctx.post(&format!("/api/v1/tickets/{ticket_id}/comments"), body).await
        }
        "list_comments" => {
            let ticket_id = arg_str_req(args, "ticket_identifier")?;
            ctx.get(&format!("/api/v1/tickets/{ticket_id}/comments")).await
        }
        // ── Relations ───────────────────────────────────────────────────────
        "add_relation" => {
            let body = json!({
                "from_identifier": arg_str_req(args, "from_identifier")?,
                "to_identifier":   arg_str_req(args, "to_identifier")?,
                "relation_type":   arg_str_req(args, "relation_type")?,
            });
            ctx.post("/api/v1/relations", body).await
        }
        "remove_relation" => {
            let id = arg_str_req(args, "relation_id")?;
            ctx.delete(&format!("/api/v1/relations/{id}")).await
        }
        // ── Search ──────────────────────────────────────────────────────────
        "search_tickets" => {
            let raw_q = arg_str_req(args, "query")?;
            let q = url::form_urlencoded::byte_serialize(raw_q.as_bytes()).collect::<String>();
            let mut path = format!("/api/v1/search?q={q}");
            if let Some(limit) = arg_i64_opt(args, "limit") { path.push_str(&format!("&limit={limit}")); }
            ctx.get(&path).await
        }
        // ── Projects ────────────────────────────────────────────────────────
        "create_project" => {
            let mut body = json!({
                "key":  arg_str_req(args, "key")?,
                "name": arg_str_req(args, "name")?,
            });
            if let Some(v) = arg_str_opt(args, "description") { body["description"] = json!(v); }
            ctx.post("/api/v1/projects", body).await
        }
        "get_project" => {
            let key = arg_str_req(args, "key")?;
            ctx.get(&format!("/api/v1/projects/{key}")).await
        }
        "update_project" => {
            let key = arg_str_req(args, "key")?;
            let mut body = json!({});
            if let Some(v) = arg_str_opt(args, "name")        { body["name"] = json!(v); }
            if let Some(v) = arg_str_opt(args, "description") { body["description"] = json!(v); }
            ctx.patch(&format!("/api/v1/projects/{key}"), body).await
        }
        "list_projects" => {
            ctx.get("/api/v1/projects").await
        }
        // ── Cycles ──────────────────────────────────────────────────────────
        "create_cycle" => {
            let project_key = arg_str_req(args, "project_key")?;
            let mut body = json!({ "name": arg_str_req(args, "name")? });
            if let Some(v) = arg_str_opt(args, "starts_at") { body["starts_at"] = json!(v); }
            if let Some(v) = arg_str_opt(args, "ends_at")   { body["ends_at"] = json!(v); }
            ctx.post(&format!("/api/v1/projects/{project_key}/cycles"), body).await
        }
        "update_cycle" => {
            let id = arg_str_req(args, "cycle_id")?;
            let mut body = json!({});
            if let Some(v) = arg_str_opt(args, "name")      { body["name"] = json!(v); }
            if let Some(v) = arg_str_opt(args, "starts_at") { body["starts_at"] = json!(v); }
            if let Some(v) = arg_str_opt(args, "ends_at")   { body["ends_at"] = json!(v); }
            ctx.patch(&format!("/api/v1/cycles/{id}"), body).await
        }
        "list_cycles" => {
            let project_key = arg_str_req(args, "project_key")?;
            ctx.get(&format!("/api/v1/projects/{project_key}/cycles")).await
        }
        // ── Misc ────────────────────────────────────────────────────────────
        "get_index" => ctx.get("/api/v1/index").await,
        "get_log" => {
            let mut qs = Vec::new();
            if let Some(v) = arg_str_opt(args, "since")  { qs.push(format!("since={v}")); }
            if let Some(v) = arg_i64_opt(args, "limit")  { qs.push(format!("limit={v}")); }
            let path = if qs.is_empty() {
                "/api/v1/log".into()
            } else {
                format!("/api/v1/log?{}", qs.join("&"))
            };
            ctx.get(&path).await
        }
        unknown => anyhow::bail!("unknown tool: {unknown}"),
    }
}

// ---------------------------------------------------------------------------
// Stdio loop (proxy mode)
// ---------------------------------------------------------------------------

pub async fn run_stdio_proxy(base_url: String, api_key: String) -> anyhow::Result<()> {
    let ctx = ProxyCtx::new(base_url.clone(), api_key.clone())?;

    let username = ctx.whoami().await
        .map_err(|e| anyhow::anyhow!("LINEAGENT_API_KEY did not resolve to a known user: {e}"))?;

    tracing::info!(username = %username, server = %base_url, "lineagent mcp proxy ready");

    let stdin = tokio::io::stdin();
    let reader = tokio::io::BufReader::new(stdin);
    let mut stdout = tokio::io::stdout();

    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        let line = line.trim().to_string();
        if line.is_empty() { continue; }

        let request: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let err = json!({
                    "jsonrpc": "2.0", "id": null,
                    "error": {"code": -32700, "message": format!("parse error: {e}")}
                });
                let mut out = serde_json::to_string(&err).unwrap();
                out.push('\n');
                stdout.write_all(out.as_bytes()).await?;
                stdout.flush().await?;
                continue;
            }
        };

        let id = request.get("id").cloned().unwrap_or(Value::Null);
        let method = request["method"].as_str().unwrap_or("");

        let response = match method {
            "initialize" => json!({
                "jsonrpc": "2.0", "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": { "tools": {}, "resources": {} },
                    "serverInfo": { "name": "lineagent", "version": env!("CARGO_PKG_VERSION") }
                }
            }),
            "notifications/initialized" | "ping" => continue,
            "tools/list" => json!({
                "jsonrpc": "2.0", "id": id,
                "result": { "tools": list_tools() }
            }),
            "tools/call" => {
                let params = &request["params"];
                let tool_name = params["name"].as_str().unwrap_or("");
                let args = params.get("arguments").unwrap_or(&Value::Null);
                let result = dispatch_tool(&ctx, tool_name, args).await;
                json!({ "jsonrpc": "2.0", "id": id, "result": result })
            }
            "resources/list" => json!({
                "jsonrpc": "2.0", "id": id,
                "result": { "resources": [] }
            }),
            other => json!({
                "jsonrpc": "2.0", "id": id,
                "error": { "code": -32601, "message": format!("method not found: {other}") }
            }),
        };

        let mut out = serde_json::to_string(&response).unwrap();
        out.push('\n');
        stdout.write_all(out.as_bytes()).await?;
        stdout.flush().await?;
    }

    Ok(())
}
