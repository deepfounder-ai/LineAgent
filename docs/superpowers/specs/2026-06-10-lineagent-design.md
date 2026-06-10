# LineAgent — Design Spec (v1)

**Date:** 2026-06-10
**Status:** Approved (brainstorm)

## Summary

LineAgent is "Linear, but for AI agents" — a self-hosted issue tracker that
agents drive through MCP (stdio), a REST API, or a CLI. It is a single Rust
binary over SQLite, modeled on the `mnemos` codebase (same layered
architecture and transport patterns), shipped as a Docker container.

Agents create tickets, organize them into projects and cycles, comment on
progress, link tickets with relations, and read an append-only audit log.

## Goals

- One process, one SQLite file, three surfaces: REST + MCP stdio + CLI.
- Faithful reuse of mnemos scaffolding (auth, config, error, pool, MCP loop,
  CLI client/output/config, Docker/CI) with minimal edits.
- Human-readable ticket identifiers (`LIN-1`) for agent cross-referencing.

## Non-Goals (v1)

- No web UI. Backend only.
- No filesystem artifacts. Pure SQLite (mnemos's on-disk markdown/log mirror
  is removed).
- No labels, attachments, reactions, or notifications.
- No real-time/subscription transport.

## Architecture

Layered, identical to mnemos (lower layers depend only on layers below):

```
cli / mcp / api          (transport)
        │
        ▼
core (ticket, comment, relation, project, cycle, search, index)
        │
        ▼
storage (sqlx repos)     auth (user, api_key, middleware)
        │
        ▼
error, config
```

**Key difference from mnemos:** no filesystem. mnemos stores markdown pages
and a `log.md` on disk and mirrors them into SQLite. LineAgent is SQLite-only.
`get_log` reads the `events` table directly. The following mnemos modules are
**dropped**: `core/{frontmatter,page,slug,source,lint}`,
`storage/{page_repo,source_repo,fs_layout}`, `api/extract` (multipart upload),
and the filesystem half of `core/log.rs`.

## Tenancy

Per-user scoping, unchanged from mnemos. Every row carries `user_id`. An agent
authenticates with an API key (`Authorization: Bearer …`) which resolves to a
`user_id`; all queries are scoped to it. The MCP server resolves a single
`LINEAGENT_API_KEY` at startup and scopes the whole session to that user.

`auth/` (user, api_key, middleware, password) is reused **unchanged** from
mnemos.

## Data Model

### Enums (validated in `core/ticket.rs`)

- **status:** `backlog → todo → in_progress → review → done | cancelled`
- **priority:** `critical | high | medium | low`
- **relation type:** `blocks | duplicates | relates_to`

`status` and `priority` are stored as TEXT and validated on write; invalid
values return `AppError::Validation`.

### Tables (`migrations/0001_init.sql`)

Reused from mnemos unchanged: `users`, `api_keys`.

**projects**
| col | type | notes |
|---|---|---|
| id | TEXT PK | uuid v7 |
| user_id | TEXT FK→users | ON DELETE CASCADE |
| key | TEXT | identifier prefix, e.g. `LIN`; uppercased on write |
| name | TEXT | |
| description | TEXT | nullable |
| ticket_counter | INTEGER | per-project ticket sequence, starts 0 |
| cycle_counter | INTEGER | per-project cycle sequence, starts 0 |
| created_at, updated_at | TEXT | RFC3339 |

`UNIQUE(user_id, key)`.

**tickets**
| col | type | notes |
|---|---|---|
| id | TEXT PK | uuid v7 |
| user_id | TEXT FK→users | CASCADE |
| project_id | TEXT FK→projects | CASCADE |
| number | INTEGER | per-project sequence |
| identifier | TEXT | `key-number`, e.g. `LIN-1`; stored for lookup |
| title | TEXT | |
| description | TEXT | nullable |
| status | TEXT | default `backlog` |
| priority | TEXT | default `medium` |
| assignee | TEXT | free-text agent name; nullable |
| parent_id | TEXT FK→tickets | nullable; canonical hierarchy |
| cycle_id | TEXT FK→cycles | nullable |
| created_at, updated_at | TEXT | |

`UNIQUE(user_id, identifier)`, `UNIQUE(project_id, number)`. Indexes on
`(user_id, status)`, `(user_id, project_id)`, `(user_id, assignee)`,
`(user_id, parent_id)`, `(user_id, cycle_id)`.

**comments**
| col | type | notes |
|---|---|---|
| id | TEXT PK | uuid v7 |
| user_id | TEXT FK→users | CASCADE |
| ticket_id | TEXT FK→tickets | CASCADE |
| author | TEXT | free-text agent name; nullable |
| body | TEXT | |
| created_at | TEXT | |

**relations**
| col | type | notes |
|---|---|---|
| id | TEXT PK | uuid v7 |
| user_id | TEXT FK→users | CASCADE |
| from_ticket_id | TEXT FK→tickets | CASCADE |
| to_ticket_id | TEXT FK→tickets | CASCADE |
| type | TEXT | `blocks | duplicates | relates_to` |
| created_at | TEXT | |

`UNIQUE(from_ticket_id, to_ticket_id, type)`. Indexes on `(user_id, from_ticket_id)`
and `(user_id, to_ticket_id)` — `get_ticket` looks up relations in both
directions, so the reverse edge needs its own index.

**cycles**
| col | type | notes |
|---|---|---|
| id | TEXT PK | uuid v7 |
| user_id | TEXT FK→users | CASCADE |
| project_id | TEXT FK→projects | CASCADE |
| number | INTEGER | per-project sequence |
| name | TEXT | |
| starts_at | TEXT | nullable RFC3339 |
| ends_at | TEXT | nullable RFC3339 |
| created_at, updated_at | TEXT | |

**events** (reused from mnemos, generalized)
Append-only audit log. `kind` like `ticket.create`, `ticket.update`,
`comment.add`, `relation.add`, `project.create`, `cycle.create`. `ref` holds
the ticket identifier / project key / cycle id. `payload_json` holds the change
delta. Read directly by `get_log` (no on-disk mirror).

**tickets_fts** — FTS5 virtual table over `(title, description)` with
`tokenize='unicode61 remove_diacritics 2'`, kept in sync by AFTER
INSERT/UPDATE/DELETE triggers (same DELETE+INSERT pattern as mnemos
`pages_fts`).

### Hierarchy vs relations (resolved)

`parent_id` on `tickets` is the **single canonical** source of truth for
parent/child hierarchy — set via `create_ticket` / `update_ticket`, enabling
fast tree queries. `child-of` is therefore **not** a relation type; the
`relations` table holds only `blocks | duplicates | relates_to`. This avoids
two sources of truth for hierarchy.

### Identifier generation (race-free)

`create_ticket` runs in a single SQLite transaction:

1. `UPDATE projects SET ticket_counter = ticket_counter + 1 WHERE id = ? RETURNING ticket_counter`
2. `number = ticket_counter`, `identifier = key || '-' || number`
3. `INSERT INTO tickets (...)`

WAL journal mode + `busy_timeout` (from mnemos `pool.rs`) plus the
single-statement atomic increment guarantee no duplicate numbers under
concurrency. Cycles use the same pattern with `cycle_counter`.

## Surfaces

### MCP tools (19)

| group | tools |
|---|---|
| tickets | `create_ticket`, `update_ticket`, `get_ticket`, `list_tickets`, `delete_ticket` |
| comments | `add_comment`, `list_comments` |
| relations | `add_relation`, `remove_relation` |
| search | `search_tickets` (FTS5 over title+description) |
| projects | `create_project`, `get_project`, `update_project`, `list_projects` |
| cycles | `create_cycle`, `update_cycle`, `list_cycles` |
| misc | `get_log`, `get_index` |

Three-surface parity is deliberate: every capability on REST/CLI has an MCP
tool, hence `get_project` + `update_project` are present.

- `get_ticket` returns the ticket plus its comments and relations inline.
- `list_tickets` filters: `project`, `status`, `priority`, `assignee`,
  `cycle`, `parent`.
- `get_index` returns a summary: projects → ticket counts grouped by status.
- Each tool has a JSON Schema in `tools/list`, dispatched through the mnemos
  MCP loop (`mcp/mod.rs` reused unchanged; `mcp/tools.rs` rewritten).

### REST API (`/api/v1`, behind `require_auth`)

```
projects:  GET /projects           POST /projects
           GET /projects/:key       PATCH /projects/:key
tickets:   GET /tickets            POST /tickets
           GET /tickets/:identifier PATCH /tickets/:identifier  DELETE /tickets/:identifier
comments:  GET /tickets/:identifier/comments   POST /tickets/:identifier/comments
relations: GET /tickets/:identifier/relations   POST /relations   DELETE /relations/:id
cycles:    GET /projects/:key/cycles   POST /projects/:key/cycles   PATCH /cycles/:id
search:    GET /search?q=&limit=
misc:      GET /index   GET /log?since=&limit=
public:    GET /healthz
auth:      POST /auth/{register,login}  GET /auth/whoami  GET/POST /auth/keys  DELETE /auth/keys/:id
```

`auth` routes reused unchanged from mnemos. Error mapping via mnemos
`error.rs` `ApiError` (reused unchanged).

### CLI

Thin HTTP client over the REST API (mnemos `client.rs`, `output.rs`,
`config.rs` reused). In-process subcommands: `serve`, `mcp`, `completions`.

```
lineagent project   {list, get, create, update}
lineagent ticket    {list, get, create, update, delete}
lineagent comment   {list, add}
lineagent relation  {list, add, remove}
lineagent cycle      {list, create, update}
lineagent search <query> [--limit]
lineagent index
lineagent log [--since] [--limit]
lineagent user/keys ...   (from mnemos)
```

`--json` global flag for raw output. Env prefix `MNEMOS_*` → `LINEAGENT_*`.
Binary name `lineagent`.

## Reused-Unchanged vs Rewritten

**Reused unchanged from mnemos:**
`auth/` (user, api_key, middleware, password), `error.rs`,
`storage/pool.rs`, `storage/{user_repo,api_key_repo,event_repo}.rs`,
`mcp/mod.rs` (JSON-RPC loop), `cli/{client,output,config}.rs`,
`cli/commands/{user,keys}.rs`, scaffolding (`lib.rs`, `main.rs` minus dropped
modules).

**Edited minimally:** `config.rs` (env prefix, db filename, drop
source-fetch fields), `Cargo.toml` / `Dockerfile` / `docker-compose.yml` / CI
(binary name `lineagent`), `scripts/install.sh`.

**Written fresh:**
`core/{ticket,comment,relation,project,cycle,search,index}.rs`,
`storage/{ticket_repo,comment_repo,relation_repo,project_repo,cycle_repo}.rs`,
`migrations/0001_init.sql`, `mcp/tools.rs`, `mcp/resources.rs` (optional),
`api/{handlers,dto,mod}.rs`, `cli/mod.rs` (clap defs),
`cli/commands/{tickets,comments,relations,projects,cycles,misc}.rs`.

## Error Handling

Reuse mnemos `AppError` / `ApiError`. New validation cases use existing
variants: `Validation` (bad enum/missing field), `NotFound` (unknown
identifier/key), `Conflict` (duplicate project key, duplicate relation).
Internal errors logged, never leaked. MCP tool errors returned as
`isError: true` content per mnemos convention.

## Testing

Mirror mnemos `tests/`:

- `auth_password.rs`, `auth_api_key.rs` — reused from mnemos.
- `core_ticket.rs` — enum validation, identifier generation, hierarchy.
- `core_search.rs` — FTS5 ranking over tickets.
- `api_integration.rs` — REST CRUD across projects/tickets/comments/relations/cycles.
- `mcp_stdio.rs` — JSON-RPC tool calls end-to-end.
- `cli_e2e.rs` — CLI against an in-process server.

Identifier-generation concurrency test: spawn N concurrent `create_ticket`
calls on one project, assert numbers are a contiguous `1..=N` with no
duplicates.

## Open Questions

None. All brainstorm forks resolved:
- Ticket ID: project-prefixed (`LIN-1`) + internal uuid.
- Cycles: included in v1 (belong to a project).
- Tenancy: per-user scoping.
- `project`: real entity table.
- `child-of`: dropped from relations; hierarchy via `parent_id`.
- Binary/env: `lineagent` / `LINEAGENT_*`.
- `assignee`: free-text agent name.
