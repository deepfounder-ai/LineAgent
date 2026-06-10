# LineAgent Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build LineAgent — a self-hosted Rust issue tracker for AI agents (Linear-for-agents) with REST + MCP-stdio + CLI surfaces over one SQLite file.

**Architecture:** Port the `mnemos` layered single-binary architecture (transport → core → storage/auth → error/config). Reuse mnemos auth/config/error/pool/MCP-loop/CLI-client unchanged; replace the wiki domain (pages/sources) with a ticket domain (projects/tickets/comments/relations/cycles). No filesystem — pure SQLite; the audit log is the `events` table read directly.

**Tech Stack:** Rust 2021, axum 0.7, sqlx 0.8 (SQLite + FTS5), clap 4, tokio, uuid v7, chrono, serde, argon2.

**Reference source:** mnemos lives at `/Users/kirleshkevich/Documents/GitHub/pn+mcp+cli`. When a step says "port from mnemos", copy that file and apply the listed edits. **Spec:** `docs/superpowers/specs/2026-06-10-lineagent-design.md`.

**Conventions every task follows:**
- TDD: failing test → run-it-fails → minimal impl → run-it-passes → commit.
- Every `core` mutating method is scoped by `user_id` and appends an audit row by calling **`storage::event_repo::append(db, user_id, kind, ref_, payload_json)` directly** (the verbatim-ported repo fn). mnemos's `core/log.rs` `LogBuilder` wrapper is filesystem-based and **not** ported — there is no on-disk log; `event_repo::append` is the only append path.
- Timestamps are RFC3339 TEXT. IDs are `uuid::Uuid::now_v7().to_string()` — **except** `events.id`, which keeps mnemos's `INTEGER PRIMARY KEY AUTOINCREMENT`.
- Run the full suite with `cargo test` from the repo root; run one test with `cargo test --test <file> <name> -- --nocapture`.

---

## Phase 0 — Scaffolding (compiles, healthz works)

### Task 0.1: Cargo manifest + crate skeleton

**Files:**
- Create: `Cargo.toml`, `src/lib.rs`, `src/main.rs`

- [ ] **Step 1: Write `Cargo.toml`** — copy mnemos `Cargo.toml` verbatim, then change: `name = "lineagent"`, `[lib] name = "lineagent"`, `[[bin]] name = "lineagent"`, `description = "Issue tracker for AI agents — REST/MCP/CLI over SQLite"`. **Remove** the `pulldown-cmark` dependency (no markdown rendering). Keep everything else.

- [ ] **Step 2: Write `src/lib.rs`** — port mnemos `src/lib.rs`. Module list becomes: `pub mod api; pub mod auth; pub mod cli; pub mod config; pub mod core; pub mod error; pub mod mcp; pub mod storage;`. Keep `pub use error::{AppError, Result};`, `VERSION`, `BUILD_REV`. Update the doc-comment architecture diagram to the ticket domain.

- [ ] **Step 3: Write `src/main.rs`** — port mnemos `src/main.rs` verbatim, replacing `mnemos::` with `lineagent::`.

- [ ] **Step 4: Commit**
```bash
git add Cargo.toml src/lib.rs src/main.rs && git commit -m "chore: crate skeleton"
```

### Task 0.2: Port unchanged foundation modules

**Files (port verbatim, `mnemos`→`lineagent` only):**
- Create: `src/error.rs` (verbatim — no mnemos-specific strings)
- Create: `src/storage/pool.rs` (verbatim)
- Create: `src/storage/mod.rs` — module list: `api_key_repo, event_repo, project_repo, ticket_repo, comment_repo, relation_repo, cycle_repo, pool, user_repo; pub use pool::{init_pool, AppState};`
- Create: `src/storage/{user_repo,api_key_repo,event_repo}.rs` (verbatim from mnemos)
- Create: `src/auth/{mod,user,api_key,password,middleware}.rs` (verbatim)

- [ ] **Step 1:** Copy each file listed above from mnemos, replacing the crate name and any `mnemos`-literal log strings with `lineagent`. Do **not** copy `storage/{page_repo,source_repo,fs_layout}.rs`.

- [ ] **Step 2:** `pool.rs` references `config.resolved_db_url()` → handled in Task 0.3. Leave as-is.

- [ ] **Step 3: Commit**
```bash
git add src/error.rs src/storage src/auth && git commit -m "chore: port auth/error/pool/repos from mnemos"
```

### Task 0.3: Config (edited)

**Files:**
- Create: `src/config.rs`
- Test: `tests/config.rs`

- [ ] **Step 1: Write failing test** `tests/config.rs`:
```rust
use lineagent::config::Config;
use std::path::PathBuf;

#[test]
fn resolved_db_url_uses_lineagent_db() {
    let c = Config::for_test(PathBuf::from("/tmp/x"));
    assert!(c.resolved_db_url().contains("lineagent.db"));
}
```

- [ ] **Step 2: Run, expect fail** (`Config` not found): `cargo test --test config`

- [ ] **Step 3: Write `src/config.rs`** — port mnemos `config.rs` with edits: env prefix `MNEMOS_`→`LINEAGENT_` everywhere; default `log_filter = "lineagent=info,tower_http=info,axum=info"`; DB filename `lineagent.db`; **remove** the `max_source_bytes` and `source_timeout_secs` fields + their env reads + defaults (no URL fetching). Keep `host`, `port`, `data_dir`, `db_url`, `log_filter`, `from_env`, `for_test`, `resolved_db_url`, `ConfigError`.

- [ ] **Step 4: Run, expect pass**: `cargo test --test config`

- [ ] **Step 5: Commit**

### Task 0.4: Empty migration + boot smoke test

**Files:**
- Create: `migrations/0001_init.sql` (users + api_keys only for now)
- Create: `src/core/mod.rs` (empty module list, fill later), `src/mcp/mod.rs`, `src/api/mod.rs`, `src/cli/mod.rs` stubs sufficient to compile

- [ ] **Step 1:** Write `migrations/0001_init.sql` with **only** the mnemos `users` and `api_keys` tables + their indexes (copy those two blocks verbatim, including the header PRAGMA comment). Remaining tables added in Phase 1.

- [ ] **Step 2:** Create minimal `src/api/mod.rs`, `src/mcp/mod.rs`, `src/cli/mod.rs`, `src/core/mod.rs` stubs. For `api/mod.rs` port the mnemos router but with only `/`, `/healthz`, and the auth routes wired (drop page/source routes). For `cli/mod.rs` define `Cli` with just `Serve`, `Mcp`, `Completions`, `User`, `Keys` subcommands (port those enum variants). For `mcp/mod.rs` port the loop but have `tools::list_tools()` return `vec![]` and `call_tool` return an "unknown tool" error (stub `src/mcp/tools.rs` with just `Tool`, `TextContent`, `list_tools`, `call_tool`). Port `cli/{client,output,config}.rs` and `cli/commands/{mod,user,keys,misc}.rs` (misc keeps only `serve`/`mcp`/`completions`) verbatim from mnemos.

- [ ] **Step 3: Write failing test** `tests/boot.rs`: spin up `init_pool(Config::for_test(tmp))`, assert it returns `Ok`. (Mirror mnemos test harness in `tests/`.)

- [ ] **Step 4:** `cargo test --test boot` → pass. `cargo build` clean.

- [ ] **Step 5: Commit** `chore: boot scaffold — healthz + auth routes compile`

---

## Phase 1 — Schema + storage repos

### Task 1.1: Full migration schema

**Files:**
- Modify: `migrations/0001_init.sql`

- [ ] **Step 1:** Append the `projects`, `tickets`, `comments`, `relations`, `cycles`, `events`, and `tickets_fts` definitions per the spec's Data Model. Key DDL:

```sql
CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    key TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    ticket_counter INTEGER NOT NULL DEFAULT 0,
    cycle_counter INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(user_id, key),
    FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS cycles (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    number INTEGER NOT NULL,
    name TEXT NOT NULL,
    starts_at TEXT,
    ends_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS tickets (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    number INTEGER NOT NULL,
    identifier TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'backlog',
    priority TEXT NOT NULL DEFAULT 'medium',
    assignee TEXT,
    parent_id TEXT,
    cycle_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(user_id, identifier),
    UNIQUE(project_id, number),
    FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE,
    FOREIGN KEY(parent_id) REFERENCES tickets(id) ON DELETE SET NULL,
    FOREIGN KEY(cycle_id) REFERENCES cycles(id) ON DELETE SET NULL
);
CREATE INDEX IF NOT EXISTS idx_tickets_user_status ON tickets(user_id, status);
CREATE INDEX IF NOT EXISTS idx_tickets_user_project ON tickets(user_id, project_id);
CREATE INDEX IF NOT EXISTS idx_tickets_user_assignee ON tickets(user_id, assignee);
CREATE INDEX IF NOT EXISTS idx_tickets_user_parent ON tickets(user_id, parent_id);
CREATE INDEX IF NOT EXISTS idx_tickets_user_cycle ON tickets(user_id, cycle_id);

CREATE TABLE IF NOT EXISTS comments (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    ticket_id TEXT NOT NULL,
    author TEXT,
    body TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY(ticket_id) REFERENCES tickets(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_comments_ticket ON comments(ticket_id);

CREATE TABLE IF NOT EXISTS relations (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    from_ticket_id TEXT NOT NULL,
    to_ticket_id TEXT NOT NULL,
    type TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(from_ticket_id, to_ticket_id, type),
    FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY(from_ticket_id) REFERENCES tickets(id) ON DELETE CASCADE,
    FOREIGN KEY(to_ticket_id) REFERENCES tickets(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_relations_from ON relations(user_id, from_ticket_id);
CREATE INDEX IF NOT EXISTS idx_relations_to ON relations(user_id, to_ticket_id);

-- events: copy mnemos events table + indexes verbatim.

CREATE VIRTUAL TABLE IF NOT EXISTS tickets_fts USING fts5(
    title, description, tokenize='unicode61 remove_diacritics 2'
);
CREATE TRIGGER IF NOT EXISTS tickets_ai AFTER INSERT ON tickets BEGIN
    INSERT INTO tickets_fts(rowid, title, description)
    VALUES (new.rowid, new.title, COALESCE(new.description,''));
END;
CREATE TRIGGER IF NOT EXISTS tickets_ad AFTER DELETE ON tickets BEGIN
    DELETE FROM tickets_fts WHERE rowid = old.rowid;
END;
CREATE TRIGGER IF NOT EXISTS tickets_au AFTER UPDATE ON tickets BEGIN
    DELETE FROM tickets_fts WHERE rowid = old.rowid;
    INSERT INTO tickets_fts(rowid, title, description)
    VALUES (new.rowid, new.title, COALESCE(new.description,''));
END;
```

- [ ] **Step 2:** `cargo test --test boot` still passes (migration runs clean).

- [ ] **Step 3: Commit** `feat: full ticket schema migration`

### Task 1.2: project_repo

**Files:**
- Create: `src/storage/project_repo.rs`
- Test: `tests/storage_project.rs`

- [ ] **Step 1: Failing test** — insert a project, `get_by_key`, assert round-trip; call `next_ticket_number` twice, assert it returns 1 then 2.

- [ ] **Step 2:** Run → fail.

- [ ] **Step 3: Implement** following the mnemos `page_repo.rs` shape: `ProjectRow { id, user_id, key, name, description, ticket_counter, cycle_counter, created_at, updated_at }` + `from_row`. Functions: `insert`, `get_by_id`, `get_by_key(pool, user_id, key)`, `list_for_user`, `update(name, description)`, and the atomic counter:
```rust
/// Atomically increment and return the next per-project ticket number.
pub async fn next_ticket_number(pool: &SqlitePool, project_id: &str) -> Result<i64> {
    let row = sqlx::query(
        "UPDATE projects SET ticket_counter = ticket_counter + 1, updated_at = ?2 \
         WHERE id = ?1 RETURNING ticket_counter",
    )
    .bind(project_id)
    .bind(Utc::now().to_rfc3339())
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("project id={project_id}")))?;
    Ok(row.try_get("ticket_counter")?)
}
```
Add an analogous `next_cycle_number` using `cycle_counter`.

- [ ] **Step 4:** Run → pass.
- [ ] **Step 5: Commit**

### Task 1.3: ticket_repo

**Files:**
- Create: `src/storage/ticket_repo.rs`
- Test: `tests/storage_ticket.rs`

- [ ] **Step 1: Failing test** — insert a project + ticket, `get_by_identifier`, assert fields; test `list` with a status filter returns only matching rows.

- [ ] **Step 2:** Run → fail.

- [ ] **Step 3: Implement** `TicketRow` (all columns) + `from_row`. Functions: `insert(... all fields incl. number, identifier ...)`, `get_by_id`, `get_by_identifier(pool, user_id, identifier)`, `list(pool, user_id, &TicketFilter)`, `update(pool, id, &TicketPatch)` (COALESCE-style partial update — build SQL with only provided fields, or simplest: fetch-merge-write), `delete(pool, id)`. `TicketFilter { project_id, status, priority, assignee, cycle_id, parent_id, limit }`. Build the list query dynamically appending `AND col = ?` clauses for each `Some` filter; always `ORDER BY updated_at DESC LIMIT`.

- [ ] **Step 4:** Run → pass.
- [ ] **Step 5: Commit**

### Task 1.4: comment_repo, relation_repo, cycle_repo

**Files:**
- Create: `src/storage/{comment_repo,relation_repo,cycle_repo}.rs`
- Test: `tests/storage_misc.rs`

- [ ] **Step 1: Failing test** covering: add+list comments for a ticket (ordered by created_at ASC); add a relation + `list_for_ticket` returns it in both directions; duplicate relation insert returns `Conflict`; insert+list+update a cycle.

- [ ] **Step 2:** Run → fail.

- [ ] **Step 3: Implement** each repo on the mnemos repo pattern:
  - `comment_repo`: `CommentRow`, `insert`, `list_for_ticket`.
  - `relation_repo`: `RelationRow`, `insert` (map SQLite UNIQUE violation → `AppError::Conflict`), `delete(id)`, `list_for_ticket(pool, user_id, ticket_id)` = `WHERE from_ticket_id = ?2 OR to_ticket_id = ?2`.
  - `cycle_repo`: `CycleRow`, `insert`, `get_by_id`, `list_for_project`, `update(name, starts_at, ends_at)`.

- [ ] **Step 4:** Run → pass.
- [ ] **Step 5: Commit**

---

## Phase 2 — Core services

### Task 2.1: Enums + validation (`core/ticket.rs` types)

**Files:**
- Create: `src/core/ticket.rs` (start with enums)
- Test: `tests/core_ticket.rs`

- [ ] **Step 1: Failing test**:
```rust
use lineagent::core::ticket::{Status, Priority};
#[test]
fn parses_valid_and_rejects_invalid() {
    assert!("in_progress".parse::<Status>().is_ok());
    assert!("nope".parse::<Status>().is_err());
    assert!("critical".parse::<Priority>().is_ok());
}
```

- [ ] **Step 2:** Run → fail.

- [ ] **Step 3: Implement** `Status` (backlog, todo, in_progress, review, done, cancelled) and `Priority` (critical, high, medium, low) as enums with `FromStr` (→ `AppError::Validation` on miss), `as_str`, `Serialize`/`Deserialize` via the string form, and `Default` (backlog / medium). Add `RelationType` (blocks, duplicates, relates_to) the same way.

- [ ] **Step 4:** Run → pass.
- [ ] **Step 5: Commit**

### Task 2.2: ProjectService (`core/project.rs`)

**Files:**
- Create: `src/core/project.rs`
- Test: `tests/core_project.rs`

- [ ] **Step 1: Failing test** — `create` a project with key `lin` → stored uppercased `LIN`; duplicate key → `Conflict`; `get`/`list`/`update` round-trip; event `project.create` appended.

- [ ] **Step 2:** Run → fail.

- [ ] **Step 3: Implement** `ProjectService { state }` mirroring mnemos `PageService`: `create(user_id, key, name, description)` (uppercase + validate key non-empty/alnum, map dup → Conflict, append event), `get(user_id, key)`, `list(user_id)`, `update(user_id, key, name?, description?)`. Return a `Project` view struct (`Serialize`).

- [ ] **Step 4:** Run → pass.
- [ ] **Step 5: Commit**

### Task 2.3: TicketService — create + identifier generation

**Files:**
- Modify: `src/core/ticket.rs`
- Test: `tests/core_ticket.rs`

- [ ] **Step 1: Failing test** — create two tickets in project `LIN`, assert identifiers `LIN-1`, `LIN-2`; create in second project `OPS`, assert `OPS-1` (independent counters). Validate bad status string → `Validation`.

- [ ] **Step 2:** Run → fail.

- [ ] **Step 3: Implement** `TicketService::create(user_id, CreateTicket{ project_key, title, description, status?, priority?, assignee?, parent_identifier?, cycle_id? })`:
  1. Resolve project by key (NotFound if missing).
  2. Validate `status`/`priority` via the enums (default if None).
  3. Resolve `parent_identifier` → parent ticket id (NotFound if given but missing).
  4. `let number = project_repo::next_ticket_number(db, project.id).await?;`
  5. `let identifier = format!("{}-{}", project.key, number);`
  6. `ticket_repo::insert(...)`.
  7. Append event `ticket.create` ref=identifier.
  8. Return the `Ticket` view.

- [ ] **Step 4:** Run → pass.
- [ ] **Step 5: Commit**

### Task 2.4: TicketService — get (with comments+relations), list, update, delete

**Files:**
- Modify: `src/core/ticket.rs`
- Test: `tests/core_ticket.rs`

- [ ] **Step 1: Failing tests** — `get` returns ticket with attached comments + relations; `update` changes status and bumps `updated_at` + appends `ticket.update`; `list` filters by status/assignee; `delete` removes + appends `ticket.delete` and is idempotent.

- [ ] **Step 2:** Run → fail.

- [ ] **Step 3: Implement**:
  - `TicketView { ...fields, comments: Vec<Comment>, relations: Vec<RelationView> }`. `get(user_id, identifier)` joins via `comment_repo::list_for_ticket` + `relation_repo::list_for_ticket` (resolve the other-side identifier for each relation row).
  - `update(user_id, identifier, TicketPatch{ title?, description?, status?, priority?, assignee?, parent_identifier?, cycle_id?, project? (no — project move out of scope) })` — validate enums, resolve parent, write, event.
  - `list(user_id, TicketFilter input keyed by project_key)` — resolve project_key→id, delegate to repo.
  - `delete(user_id, identifier)` — idempotent, event.

- [ ] **Step 4:** Run → pass.
- [ ] **Step 5: Commit**

### Task 2.5: CommentService, RelationService, CycleService

**Files:**
- Create: `src/core/{comment,relation,cycle}.rs`
- Test: `tests/core_misc.rs`

- [ ] **Step 1: Failing tests** — add+list comment (event `comment.add`); add relation by two identifiers + validate type enum + dup→Conflict + remove; create cycle in project (number `1`), list, update.

- [ ] **Step 2:** Run → fail.

- [ ] **Step 3: Implement** three services on the same pattern:
  - `CommentService::add(user_id, ticket_identifier, author?, body)` → resolve ticket, insert, event. `list(user_id, ticket_identifier)`.
  - `RelationService::add(user_id, from_identifier, to_identifier, type_str)` → validate `RelationType`, resolve both tickets, insert (dup→Conflict), event `relation.add`. `remove(user_id, id)`.
  - `CycleService::create(user_id, project_key, name, starts_at?, ends_at?)` → `next_cycle_number`, insert, event. `list(user_id, project_key)`, `update(user_id, id, name?, starts_at?, ends_at?)`.

- [ ] **Step 4:** Run → pass.
- [ ] **Step 5: Commit**

### Task 2.6: SearchService + IndexService

**Files:**
- Create: `src/core/search.rs`, `src/core/index.rs`
- Modify: `src/core/mod.rs` (final module list: `comment, cycle, index, project, relation, search, ticket`)
- Test: `tests/core_search.rs`

- [ ] **Step 1: Failing test** — create tickets, `search("payment")` returns the matching ticket identifier ranked first; `index()` returns per-project counts grouped by status.

- [ ] **Step 2:** Run → fail.

- [ ] **Step 3: Implement**:
  - `search.rs`: port mnemos `SearchService` but query `tickets_fts` JOIN `tickets`, select `identifier, title, snippet(tickets_fts,1,...), bm25() AS rank`, scope by `user_id`. Hit struct `{ identifier, title, snippet, rank }`.
  - `index.rs`: no filesystem. `IndexService::build(user_id)` returns a serializable summary: `Vec<ProjectIndex { key, name, counts: { backlog, todo, in_progress, review, done, cancelled, total } }>` computed with a `SELECT project_id, status, COUNT(*) ... GROUP BY` query joined to projects.

- [ ] **Step 4:** Run → pass.
- [ ] **Step 5: Commit**

---

## Phase 3 — REST API

### Task 3.1: DTOs + AuthContext extractor

**Files:**
- Create: `src/api/dto.rs`, `src/api/extract.rs`
- Modify: `src/api/mod.rs` (add `pub mod dto; pub mod extract;`)

- [ ] **Step 1:** Port `src/api/extract.rs` verbatim from mnemos.

- [ ] **Step 2:** Write `src/api/dto.rs`: request/response structs — `CreateProjectReq`, `UpdateProjectReq`, `CreateTicketReq`, `UpdateTicketReq`, `ListTicketsQuery` (serde `Deserialize` with `Option` fields matching `axum::extract::Query`), `AddCommentReq`, `AddRelationReq`, `CreateCycleReq`, `UpdateCycleReq`, `SearchQuery { q, limit }`, `LogQuery { since, limit }`. Response views reuse the core view structs (`Ticket`, `TicketView`, `Project`, etc.) — re-export, don't duplicate.

- [ ] **Step 3: Commit**

### Task 3.2: Handlers + router

**Files:**
- Create: `src/api/handlers.rs`
- Modify: `src/api/mod.rs` (wire routes per spec)
- Test: `tests/api_integration.rs`

- [ ] **Step 1: Failing test** — boot the router in-process (mnemos `api_integration.rs` harness: register user → get key → bearer requests). Cover: create project, create ticket (assert `LIN-1`), get ticket, patch status, add comment, add relation, list with `?status=`, search, index, log, healthz (200, no auth).

- [ ] **Step 2:** Run → fail.

- [ ] **Step 3: Implement** handlers as thin adapters (mnemos pattern: extract `AuthContext`, call the core service scoped by `ctx.user_id`, `?`-propagate `AppError`→`ApiError`, `Json` out). One handler per route in the spec's REST list. Keep mnemos `root`/`healthz`/auth handlers (port the auth ones verbatim; replace the landing HTML copy with LineAgent text). **Two handlers are written fresh, NOT ported:** `get_log` (mnemos's renders text via the dropped `core::log::format_log_line`; instead query `event_repo::list_for_user(db, user_id, &EventFilter{since,limit})` and return the rows as JSON) and `get_index` (return `IndexService::build` output as JSON). Wire `api/mod.rs` router exactly per the spec's REST surface, all ticket routes behind `require_auth`. **Update routes use `patch(handler)`, not mnemos's `put(...)`** — the spec mandates PATCH for `/tickets/:identifier`, `/projects/:key`, `/cycles/:id`.

- [ ] **Step 4:** Run → pass. `cargo test` whole suite green.
- [ ] **Step 5: Commit** `feat: REST API surface`

---

## Phase 4 — MCP tools

### Task 4.1: Tool definitions (`tools/list`)

**Files:**
- Modify: `src/mcp/tools.rs`
- Test: `tests/mcp_stdio.rs`

- [ ] **Step 1: Failing test** — send `tools/list` over the stdio harness (port mnemos `mcp_stdio.rs`), assert all 19 tool names present.

- [ ] **Step 2:** Run → fail.

- [ ] **Step 3: Implement** `list_tools()` returning 19 `Tool`s (spec list) using the mnemos `tool(name, desc, json!(schema))` helper. Drop the `$defs/frontmatter` machinery; schemas are plain inline objects with `additionalProperties: false`. Enumerate `status`/`priority`/relation `type` with JSON Schema `enum`. Keep `TextContent` (+ add a `json` helper that serializes a value to a text item, as mnemos has).

- [ ] **Step 4:** Run → pass.
- [ ] **Step 5: Commit**

### Task 4.2: Tool handlers (`tools/call`)

**Files:**
- Modify: `src/mcp/tools.rs`
- Test: `tests/mcp_stdio.rs`

- [ ] **Step 1: Failing test** — over stdio: `create_project`, then `create_ticket` → assert returned identifier `LIN-1`; `get_ticket`; `add_comment`; `search_tickets`; `get_index`. Assert an invalid `status` arg returns `isError: true`.

- [ ] **Step 2:** Run → fail.

- [ ] **Step 3: Implement** `call_tool` dispatch (one arm per tool) using mnemos arg helpers (`arg_str`, `arg_str_required`, `arg_i64_opt`) — port those verbatim. Each handler builds the core service from `user.ctx.state`, calls it scoped by `user.user_id`, returns `vec![TextContent::json(result)]`. Errors propagate as `AppError` (the mnemos loop renders `isError`).

- [ ] **Step 4:** Run → pass.
- [ ] **Step 5: Commit** `feat: 19 MCP tools`

---

## Phase 5 — CLI

### Task 5.1: clap command tree

**Files:**
- Modify: `src/cli/mod.rs`

- [ ] **Step 1:** Replace mnemos `Pages`/`Sources`/`Search`/`Index`/`Log`/`Lint` variants with: `Project(ProjectCmd)`, `Ticket(TicketCmd)`, `Comment(CommentCmd)`, `Relation(RelationCmd)`, `Cycle(CycleCmd)`, `Search { query, limit }`, `Index`, `Log { since, limit }`. Keep `Serve`, `Mcp`, `Completions`, `User`, `Keys`. Update `#[command(name = "lineagent", ...)]`. Define each subcommand enum per the spec's CLI list.

- [ ] **Step 2:** `cargo build` clean.
- [ ] **Step 3: Commit**

### Task 5.2: CLI command handlers + dispatch

**Files:**
- Create: `src/cli/commands/{projects,tickets,comments,relations,cycles}.rs`
- Modify: `src/cli/client.rs` (add `patch`), `src/cli/commands/mod.rs` (dispatch), `src/cli/commands/misc.rs` (search/index/log)
- Test: `tests/cli_e2e.rs`

- [ ] **Step 1: Add `patch` to `client.rs`** — the ported mnemos `client.rs` exposes `get/post/put/delete` but **no `patch`** (mnemos updates via PUT; LineAgent uses PATCH). Add a convenience method mirroring `put`, delegating to the existing generic `request`:
```rust
pub async fn patch<T: DeserializeOwned, B: Serialize>(&self, path: &str, body: &B) -> CliResult<T> {
    self.request(reqwest::Method::PATCH, path, Some(body)).await
}
```

- [ ] **Step 2: Failing test** — port mnemos `cli_e2e.rs` harness (boot in-process server, point CLI at it). Cover: `project create`, `ticket create` (assert prints `LIN-1`), `ticket list`, `ticket update` (PATCH path), `comment add`, `search`, `index`, `--json` output is valid JSON.

- [ ] **Step 3:** Run → fail.

- [ ] **Step 4: Implement** each command module on the mnemos `pages.rs`/`misc.rs` pattern: build the `/api/v1/...` path, `client.get/post/patch/delete`, then `print_json` (if `--json`) or a human table via `print_line`. Wire all new families into `commands/mod.rs::dispatch`. Move `search`/`index`/`log` into `misc.rs` against the new endpoints.

- [ ] **Step 4:** Run → pass. Full `cargo test` green.
- [ ] **Step 5: Commit** `feat: CLI surface`

---

## Phase 6 — Packaging + docs

### Task 6.1: Docker + compose + CI + install script

**Files:**
- Create: `Dockerfile`, `docker-compose.yml`, `.dockerignore`, `.github/workflows/{ci,docker-publish}.yml`, `scripts/install.sh`

- [ ] **Step 1:** Port each from mnemos, replacing `mnemos`→`lineagent` (binary name, image name, default port env `LINEAGENT_PORT`, data volume path). CI: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`.

- [ ] **Step 2:** `docker build .` succeeds locally (or note as manual check if Docker absent).
- [ ] **Step 3: Commit**

### Task 6.2: Docs

**Files:**
- Create: `README.md`, `docs/{api,mcp,cli}.md`

- [ ] **Step 1:** Write `README.md` (what it is, quickstart: `lineagent user register`, `lineagent serve`, MCP config snippet with `LINEAGENT_API_KEY`). Write `docs/api.md` (endpoint table + example curl), `docs/mcp.md` (19 tools + JSON examples), `docs/cli.md` (command table + exit codes). Derive content from the implemented surfaces.

- [ ] **Step 2: Commit** `docs: README + api/mcp/cli reference`

### Task 6.3: Final verification

- [ ] **Step 1:** `cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test` — all green.
- [ ] **Step 2:** Manual smoke: `cargo run -- user register agent`, `cargo run -- serve` in one shell; in another `LINEAGENT_API_KEY=… cargo run -- project create LIN --name "LineAgent"`, `… ticket create LIN --title "first"` → prints `LIN-1`.
- [ ] **Step 3:** Use superpowers:verification-before-completion to confirm every claim with command output before declaring done.

---

## Concurrency test (correctness backstop)

Add to `tests/core_ticket.rs`: spawn N=20 concurrent `TicketService::create` calls on one project via `tokio::join!`/`JoinSet`, collect identifiers, assert the numeric suffixes are exactly `1..=20` with no duplicates and no gaps. This validates the `UPDATE … RETURNING` atomic counter under load.
