# LineAgent REST API Reference

Base URL: `http://localhost:3000/api/v1`

All protected endpoints require `Authorization: Bearer <LINEAGENT_API_KEY>`.

---

## Auth

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `POST` | `/auth/register` | — | Register a new user |
| `POST` | `/auth/login` | — | Log in, get API key |
| `GET` | `/auth/whoami` | ✓ | Current user info |
| `GET` | `/auth/keys` | ✓ | List API keys |
| `POST` | `/auth/keys` | ✓ | Create an API key |
| `DELETE` | `/auth/keys/:id` | ✓ | Revoke an API key |

### Register

If `LINEAGENT_SECRET` is set on the server, the `secret` field is required.

```bash
# server has LINEAGENT_SECRET set
curl -X POST http://localhost:3000/api/v1/auth/register \
  -H 'Content-Type: application/json' \
  -d '{"username":"agent","password":"s3cr3t","secret":"your_secret"}'

# open registration (no LINEAGENT_SECRET)
curl -X POST http://localhost:3000/api/v1/auth/register \
  -H 'Content-Type: application/json' \
  -d '{"username":"agent","password":"s3cr3t"}'
```

```json
{"id":"01932...","username":"agent","created_at":"2026-06-10T12:00:00Z"}
```

Returns `403 Forbidden` if secret is wrong or missing when required.

### Login

```bash
curl -X POST http://localhost:3000/api/v1/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"username":"agent","password":"s3cr3t"}'
```

```json
{"key":"lineagent_abc123...","created_at":"2026-06-10T12:00:00Z"}
```

---

## Projects

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/projects` | ✓ | List all projects |
| `POST` | `/projects` | ✓ | Create a project |
| `GET` | `/projects/:key` | ✓ | Get project by key |
| `PATCH` | `/projects/:key` | ✓ | Update project |

### Create project

```bash
curl -X POST http://localhost:3000/api/v1/projects \
  -H 'Authorization: Bearer lineagent_...' \
  -H 'Content-Type: application/json' \
  -d '{"key":"LIN","name":"LineAgent","description":"Main project"}'
```

```json
{
  "id": "01932...",
  "key": "LIN",
  "name": "LineAgent",
  "description": "Main project",
  "ticket_counter": 0,
  "cycle_counter": 0,
  "created_at": "2026-06-10T12:00:00Z",
  "updated_at": "2026-06-10T12:00:00Z"
}
```

---

## Tickets

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/tickets` | ✓ | List tickets (with filters) |
| `POST` | `/tickets` | ✓ | Create a ticket |
| `GET` | `/tickets/:id` | ✓ | Get ticket by ID |
| `PATCH` | `/tickets/:id` | ✓ | Update ticket |
| `DELETE` | `/tickets/:id` | ✓ | Delete ticket |

Query parameters for `GET /tickets`:
- `project_key` — filter by project
- `status` — `backlog|todo|in_progress|review|done|cancelled`
- `priority` — `critical|high|medium|low`
- `assignee` — filter by assignee
- `cycle_id` — filter by cycle
- `limit` — max results (default 20, max 1000)

### Create ticket

```bash
curl -X POST http://localhost:3000/api/v1/tickets \
  -H 'Authorization: Bearer lineagent_...' \
  -H 'Content-Type: application/json' \
  -d '{"project_key":"LIN","title":"Implement auth","status":"backlog","priority":"high"}'
```

```json
{
  "id": "01932...",
  "identifier": "LIN-1",
  "project_key": "LIN",
  "title": "Implement auth",
  "description": null,
  "status": "backlog",
  "priority": "high",
  "assignee": null,
  "parent_identifier": null,
  "cycle_id": null,
  "created_at": "2026-06-10T12:00:00Z",
  "updated_at": "2026-06-10T12:00:00Z"
}
```

### Update ticket

```bash
curl -X PATCH http://localhost:3000/api/v1/tickets/01932... \
  -H 'Authorization: Bearer lineagent_...' \
  -H 'Content-Type: application/json' \
  -d '{"status":"done"}'
```

---

## Comments

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/tickets/:id/comments` | ✓ | List comments for ticket |
| `POST` | `/tickets/:id/comments` | ✓ | Add comment |

```bash
curl -X POST http://localhost:3000/api/v1/tickets/01932.../comments \
  -H 'Authorization: Bearer lineagent_...' \
  -H 'Content-Type: application/json' \
  -d '{"body":"Fixed in commit abc123","author":"agent"}'
```

---

## Relations

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/tickets/:id/relations` | ✓ | List relations for ticket |
| `POST` | `/relations` | ✓ | Add relation |
| `DELETE` | `/relations/:id` | ✓ | Remove relation |

Relation types: `blocks`, `duplicates`, `relates_to`

```bash
curl -X POST http://localhost:3000/api/v1/relations \
  -H 'Authorization: Bearer lineagent_...' \
  -H 'Content-Type: application/json' \
  -d '{"from_identifier":"LIN-1","to_identifier":"LIN-2","relation_type":"blocks"}'
```

---

## Cycles

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/projects/:key/cycles` | ✓ | List cycles for project |
| `POST` | `/projects/:key/cycles` | ✓ | Create cycle |
| `PATCH` | `/cycles/:id` | ✓ | Update cycle |

```bash
curl -X POST http://localhost:3000/api/v1/projects/LIN/cycles \
  -H 'Authorization: Bearer lineagent_...' \
  -H 'Content-Type: application/json' \
  -d '{"name":"Sprint 1","starts_at":"2026-06-01T00:00:00Z","ends_at":"2026-06-14T23:59:59Z"}'
```

---

## Search

```
GET /search?q=<query>&limit=<n>
```

Full-text BM25 search over ticket titles and descriptions. Returns snippets with highlights.

```bash
curl 'http://localhost:3000/api/v1/search?q=auth+middleware' \
  -H 'Authorization: Bearer lineagent_...'
```

```json
[
  {
    "identifier": "LIN-1",
    "title": "Implement auth",
    "snippet": "Implement <b>auth</b> middleware for the REST API",
    "rank": -1.23
  }
]
```

---

## Index

```
GET /index
```

Returns all projects with per-status ticket counts.

```json
[
  {
    "key": "LIN",
    "name": "LineAgent",
    "counts": {
      "backlog": 3,
      "todo": 1,
      "in_progress": 2,
      "review": 0,
      "done": 5,
      "cancelled": 0
    }
  }
]
```

---

## Audit Log

```
GET /log?since=<rfc3339>&limit=<n>
```

Returns recent audit events (default limit 100).

---

## Public

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/` | Service info |
| `GET` | `/healthz` | Health check (`{"status":"ok"}`) |

---

## Error format

```json
{
  "error": {
    "code": "not_found",
    "message": "ticket id=01932..."
  }
}
```

Error codes: `not_found`, `conflict`, `unauthorized`, `forbidden`, `unprocessable_entity`, `internal`.

---

## Slack integration

Set `LINEAGENT_SLACK_TOKEN` (bot token, `xoxb-…`) and `LINEAGENT_SLACK_CHANNEL` (e.g. `#lineagent`) on the server to receive ticket create/update notifications in Slack. Both env vars must be set; either alone is ignored.
