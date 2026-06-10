//! End-to-end HTTP tests against the real Axum router.
//!
//! Each test spins up the full stack (router + `require_auth` middleware +
//! a temp SQLite store) on an ephemeral port and drives it with `reqwest`.

use lineagent::config::Config;
use serde_json::{json, Value};

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
            .json(&json!({ "username": username, "password": "hunter2hunter2" }))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 201, "register should 201");
        let body: Value = resp.json().await.unwrap();
        body["api_key"].as_str().unwrap().to_string()
    }

    fn get(&self, path: &str, key: &str) -> reqwest::RequestBuilder {
        self.http.get(self.url(path)).bearer_auth(key)
    }

    fn post_json(&self, path: &str, key: &str, body: &Value) -> reqwest::RequestBuilder {
        self.http.post(self.url(path)).bearer_auth(key).json(body)
    }

    fn patch_json(&self, path: &str, key: &str, body: &Value) -> reqwest::RequestBuilder {
        self.http.patch(self.url(path)).bearer_auth(key).json(body)
    }

    fn delete(&self, path: &str, key: &str) -> reqwest::RequestBuilder {
        self.http.delete(self.url(path)).bearer_auth(key)
    }
}

// ---------------------------------------------------------------------------
// 1. Health check — no auth required
// ---------------------------------------------------------------------------

#[tokio::test]
async fn healthz_is_public() {
    let s = TestServer::start().await;
    let resp = s.http.get(s.url("/healthz")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
}

// ---------------------------------------------------------------------------
// 2–10. Happy-path smoke test
// ---------------------------------------------------------------------------

#[tokio::test]
async fn full_happy_path() {
    let s = TestServer::start().await;

    // 2. Register user → get API key
    let key = s.register("alice").await;
    assert!(key.starts_with("lineagent_"), "key prefix: {key}");

    // 3. Create LIN project
    let project: Value = s
        .post_json(
            "/api/v1/projects",
            &key,
            &json!({ "key": "LIN", "name": "Linear Clone" }),
        )
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(project["key"], "LIN");

    // 4. Create ticket → expect identifier LIN-1
    let resp = s
        .post_json(
            "/api/v1/tickets",
            &key,
            &json!({ "project_key": "LIN", "title": "First issue" }),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let ticket: Value = resp.json().await.unwrap();
    assert_eq!(ticket["identifier"], "LIN-1", "got: {ticket}");

    // 5. GET ticket — returns view with empty comments and relations
    let view: Value = s
        .get("/api/v1/tickets/LIN-1", &key)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(view["identifier"], "LIN-1");
    assert!(view["comments"].as_array().unwrap().is_empty());
    assert!(view["relations"].as_array().unwrap().is_empty());

    // 6. PATCH ticket → status done → 200
    let updated: Value = s
        .patch_json(
            "/api/v1/tickets/LIN-1",
            &key,
            &json!({ "status": "done" }),
        )
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(updated["status"], "done");

    // 7. Add comment
    let resp = s
        .post_json(
            "/api/v1/tickets/LIN-1/comments",
            &key,
            &json!({ "body": "Nice work!", "author": "alice" }),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let comment: Value = resp.json().await.unwrap();
    assert_eq!(comment["body"], "Nice work!");

    // Verify the comment appears in the ticket view
    let view2: Value = s
        .get("/api/v1/tickets/LIN-1", &key)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(view2["comments"].as_array().unwrap().len(), 1);

    // 8. Search tickets
    let search: Value = s
        .get("/api/v1/search?q=First", &key)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(
        search["count"].as_i64().unwrap() >= 1,
        "search returned: {search}"
    );

    // 9. Index — returns project list with counts
    let index: Value = s
        .get("/api/v1/index", &key)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let projects = index.as_array().expect("index should be an array");
    assert!(!projects.is_empty(), "expected at least one project in index");
    assert_eq!(projects[0]["key"], "LIN");
    // total count should be 1 (the ticket we created)
    assert_eq!(projects[0]["counts"]["total"], 1);
}

// ---------------------------------------------------------------------------
// 10. Unauthenticated request → 401
// ---------------------------------------------------------------------------

#[tokio::test]
async fn unauthenticated_returns_401() {
    let s = TestServer::start().await;
    let resp = s
        .http
        .get(s.url("/api/v1/projects"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
    assert!(resp.headers().contains_key("www-authenticate"));
    let body: Value = resp.json().await.unwrap();
    assert!(body["error"]["code"].is_string(), "got {body}");
}

// ---------------------------------------------------------------------------
// Extra: relations, cycles, log
// ---------------------------------------------------------------------------

#[tokio::test]
async fn relations_and_cycles() {
    let s = TestServer::start().await;
    let key = s.register("bob").await;

    // Setup project + two tickets
    s.post_json(
        "/api/v1/projects",
        &key,
        &json!({ "key": "REL", "name": "Relations Test" }),
    )
    .send()
    .await
    .unwrap();

    s.post_json(
        "/api/v1/tickets",
        &key,
        &json!({ "project_key": "REL", "title": "Ticket A" }),
    )
    .send()
    .await
    .unwrap();

    s.post_json(
        "/api/v1/tickets",
        &key,
        &json!({ "project_key": "REL", "title": "Ticket B" }),
    )
    .send()
    .await
    .unwrap();

    // Add a relation
    let rel: Value = s
        .post_json(
            "/api/v1/relations",
            &key,
            &json!({
                "from_identifier": "REL-1",
                "to_identifier": "REL-2",
                "relation_type": "blocks"
            }),
        )
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(rel["from_identifier"], "REL-1");
    assert_eq!(rel["relation_type"], "blocks");

    // List relations for ticket
    let relations: Value = s
        .get("/api/v1/tickets/REL-1/relations", &key)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(relations.as_array().unwrap().len(), 1);

    // Create and list cycles
    let cycle: Value = s
        .post_json(
            "/api/v1/projects/REL/cycles",
            &key,
            &json!({ "name": "Sprint 1" }),
        )
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(cycle["name"], "Sprint 1");

    let cycles: Value = s
        .get("/api/v1/projects/REL/cycles", &key)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(cycles.as_array().unwrap().len(), 1);
}

// ---------------------------------------------------------------------------
// Uncovered routes: get_project, update_project, list_tickets with filters,
// delete_ticket, remove_relation, update_cycle
// ---------------------------------------------------------------------------

#[tokio::test]
async fn uncovered_routes() {
    let s = TestServer::start().await;
    let key = s.register("dave").await;

    // 1. Create project "LIN"
    let project: Value = s
        .post_json(
            "/api/v1/projects",
            &key,
            &json!({ "key": "LIN", "name": "Linear Clone" }),
        )
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(project["key"], "LIN");

    // 2. GET /api/v1/projects/LIN → 200, key="LIN"
    let resp = s.get("/api/v1/projects/LIN", &key).send().await.unwrap();
    assert_eq!(resp.status(), 200, "get_project should 200");
    let got: Value = resp.json().await.unwrap();
    assert_eq!(got["key"], "LIN", "got: {got}");

    // 3. PATCH /api/v1/projects/LIN → update name → 200
    let resp = s
        .patch_json(
            "/api/v1/projects/LIN",
            &key,
            &json!({ "name": "Updated" }),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "update_project should 200");
    let updated: Value = resp.json().await.unwrap();
    assert_eq!(updated["name"], "Updated", "got: {updated}");

    // 4. Create ticket LIN-1 (backlog status)
    let resp = s
        .post_json(
            "/api/v1/tickets",
            &key,
            &json!({ "project_key": "LIN", "title": "First issue", "status": "backlog" }),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let t1: Value = resp.json().await.unwrap();
    assert_eq!(t1["identifier"], "LIN-1", "got: {t1}");

    // 5. Create ticket LIN-2 (needed for relation)
    let resp = s
        .post_json(
            "/api/v1/tickets",
            &key,
            &json!({ "project_key": "LIN", "title": "Second issue", "status": "backlog" }),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let t2: Value = resp.json().await.unwrap();
    assert_eq!(t2["identifier"], "LIN-2", "got: {t2}");

    // 6. GET /api/v1/tickets?project=LIN&status=backlog → at least one ticket with identifier="LIN-1"
    let resp = s
        .get("/api/v1/tickets?project=LIN&status=backlog", &key)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "list_tickets with filters should 200");
    let tickets: Value = resp.json().await.unwrap();
    let arr = tickets.as_array().expect("tickets should be an array");
    assert!(!arr.is_empty(), "expected at least one backlog ticket");
    let identifiers: Vec<&str> = arr
        .iter()
        .filter_map(|t| t["identifier"].as_str())
        .collect();
    assert!(
        identifiers.contains(&"LIN-1"),
        "LIN-1 not found in filtered results: {identifiers:?}"
    );

    // 7. POST /api/v1/relations → blocks LIN-1 → LIN-2
    let resp = s
        .post_json(
            "/api/v1/relations",
            &key,
            &json!({
                "from_identifier": "LIN-1",
                "to_identifier": "LIN-2",
                "relation_type": "blocks"
            }),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "add_relation should 201");
    let relation: Value = resp.json().await.unwrap();
    let relation_id = relation["id"].as_str().expect("relation should have id").to_string();
    assert_eq!(relation["from_identifier"], "LIN-1");

    // 8. DELETE /api/v1/relations/:id → 200 or 204
    let resp = s
        .delete(&format!("/api/v1/relations/{relation_id}"), &key)
        .send()
        .await
        .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 204,
        "remove_relation should 200 or 204, got {}",
        resp.status()
    );

    // 9. DELETE /api/v1/tickets/LIN-1 → 200 or 204
    let resp = s.delete("/api/v1/tickets/LIN-1", &key).send().await.unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 204,
        "delete_ticket should 200 or 204, got {}",
        resp.status()
    );

    // 10. POST /api/v1/projects/LIN/cycles → create cycle
    let resp = s
        .post_json(
            "/api/v1/projects/LIN/cycles",
            &key,
            &json!({ "name": "Sprint 1" }),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "create_cycle should 201");
    let cycle: Value = resp.json().await.unwrap();
    let cycle_id = cycle["id"].as_str().expect("cycle should have id").to_string();
    assert_eq!(cycle["name"], "Sprint 1");

    // 11. PATCH /api/v1/cycles/:id → {"name":"Sprint 2"} → 200
    let resp = s
        .patch_json(
            &format!("/api/v1/cycles/{cycle_id}"),
            &key,
            &json!({ "name": "Sprint 2" }),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "update_cycle should 200");
    let updated_cycle: Value = resp.json().await.unwrap();
    assert_eq!(updated_cycle["name"], "Sprint 2", "got: {updated_cycle}");
}

#[tokio::test]
async fn log_returns_events() {
    let s = TestServer::start().await;
    let key = s.register("carol").await;

    s.post_json(
        "/api/v1/projects",
        &key,
        &json!({ "key": "LOG", "name": "Log Test" }),
    )
    .send()
    .await
    .unwrap();

    let log: Value = s
        .get("/api/v1/log", &key)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let items = log["items"].as_array().unwrap();
    assert!(!items.is_empty(), "log should have events");
    let kinds: Vec<&str> = items
        .iter()
        .filter_map(|e| e["kind"].as_str())
        .collect();
    assert!(kinds.contains(&"project.create"), "kinds: {kinds:?}");
}
