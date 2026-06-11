# LineAgent MCP Reference

LineAgent exposes 19 tools via MCP stdio JSON-RPC 2.0. The `lineagent mcp` subcommand reads from stdin and writes to stdout. `LINEAGENT_API_KEY` is passed via environment — it is never written to stdout.

## Configuration

```json
{
  "mcpServers": {
    "lineagent": {
      "command": "lineagent",
      "args": ["mcp"],
      "env": {
        "LINEAGENT_API_URL": "http://localhost:8080",
        "LINEAGENT_API_KEY": "lineagent_your_key_here"
      }
    }
  }
}
```

A running `lineagent serve` is required. The MCP process is a thin JSON-RPC → REST proxy.

---

## Tools

### Tickets

#### `create_ticket`

Create a new ticket in a project.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `project_key` | string | ✓ | Project key (e.g. `LIN`) |
| `title` | string | ✓ | Ticket title |
| `description` | string | — | Markdown description |
| `status` | string | — | `backlog` (default) \| `todo` \| `in_progress` \| `review` \| `done` \| `cancelled` |
| `priority` | string | — | `critical` \| `high` \| `medium` (default) \| `low` |
| `assignee` | string | — | Assignee username/id |
| `parent_identifier` | string | — | Parent ticket identifier (e.g. `LIN-1`) |
| `cycle_id` | string | — | Cycle id |

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "create_ticket",
    "arguments": {
      "project_key": "LIN",
      "title": "Fix memory leak in event loop",
      "priority": "high",
      "status": "todo"
    }
  }
}
```

Response content: JSON ticket object including `identifier` (e.g. `LIN-7`).

---

#### `update_ticket`

Update an existing ticket.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `identifier` | string | ✓ | Ticket identifier (e.g. `LIN-1`) |
| `title` | string | — | |
| `description` | string | — | |
| `status` | string | — | |
| `priority` | string | — | |
| `assignee` | string | — | |
| `parent_identifier` | string | — | |
| `cycle_id` | string | — | |

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/call",
  "params": {
    "name": "update_ticket",
    "arguments": {
      "identifier": "LIN-7",
      "status": "in_progress",
      "assignee": "agent-alpha"
    }
  }
}
```

---

#### `get_ticket`

Get a single ticket with its comments and relations.

| Field | Type | Required |
|-------|------|----------|
| `identifier` | string | ✓ |

```json
{"name":"get_ticket","arguments":{"identifier":"LIN-7"}}
```

---

#### `list_tickets`

List tickets with optional filters.

| Field | Type | Default |
|-------|------|---------|
| `project_key` | string | — |
| `status` | string | — |
| `priority` | string | — |
| `assignee` | string | — |
| `cycle_id` | string | — |
| `parent_identifier` | string | — |
| `limit` | integer | 20 |

```json
{
  "name": "list_tickets",
  "arguments": {"project_key":"LIN","status":"in_progress","limit":50}
}
```

---

#### `delete_ticket`

Delete a ticket. Idempotent.

```json
{"name":"delete_ticket","arguments":{"identifier":"LIN-7"}}
```

---

### Comments

#### `add_comment`

| Field | Type | Required |
|-------|------|----------|
| `ticket_identifier` | string | ✓ |
| `body` | string | ✓ |
| `author` | string | — |

```json
{
  "name": "add_comment",
  "arguments": {
    "ticket_identifier": "LIN-7",
    "body": "Reproduced on main. Stack trace attached.",
    "author": "agent-alpha"
  }
}
```

---

#### `list_comments`

```json
{"name":"list_comments","arguments":{"ticket_identifier":"LIN-7"}}
```

---

### Relations

#### `add_relation`

| Field | Type | Required | Values |
|-------|------|----------|--------|
| `from_identifier` | string | ✓ | |
| `to_identifier` | string | ✓ | |
| `relation_type` | string | ✓ | `blocks` \| `duplicates` \| `relates_to` |

```json
{
  "name": "add_relation",
  "arguments": {
    "from_identifier": "LIN-7",
    "to_identifier": "LIN-3",
    "relation_type": "blocks"
  }
}
```

Returns `{"id":"...","relation_type":"blocks",...}` — save the `id` to remove it later.

---

#### `remove_relation`

```json
{"name":"remove_relation","arguments":{"relation_id":"01932..."}}
```

---

### Search

#### `search_tickets`

Full-text BM25 search over ticket title + description.

| Field | Type | Required | Default |
|-------|------|----------|---------|
| `query` | string | ✓ | |
| `limit` | integer | — | 20 |

```json
{
  "name": "search_tickets",
  "arguments": {"query":"memory leak event loop","limit":5}
}
```

Response:

```json
[
  {
    "identifier": "LIN-7",
    "title": "Fix memory leak in event loop",
    "snippet": "Fix <b>memory</b> <b>leak</b> in <b>event</b> <b>loop</b>",
    "rank": -2.41
  }
]
```

---

### Projects

#### `create_project`

| Field | Type | Required |
|-------|------|----------|
| `key` | string | ✓ |
| `name` | string | ✓ |
| `description` | string | — |

Key is auto-uppercased. Valid chars: `A-Z`, `0-9`.

```json
{"name":"create_project","arguments":{"key":"api","name":"API Service"}}
```

---

#### `get_project`

```json
{"name":"get_project","arguments":{"key":"LIN"}}
```

---

#### `update_project`

```json
{
  "name": "update_project",
  "arguments": {"key":"LIN","description":"Core issue tracker"}
}
```

---

#### `list_projects`

```json
{"name":"list_projects","arguments":{}}
```

---

### Cycles

#### `create_cycle`

| Field | Type | Required |
|-------|------|----------|
| `project_key` | string | ✓ |
| `name` | string | ✓ |
| `starts_at` | string (ISO 8601) | — |
| `ends_at` | string (ISO 8601) | — |

```json
{
  "name": "create_cycle",
  "arguments": {
    "project_key": "LIN",
    "name": "Sprint 1",
    "starts_at": "2026-06-01T00:00:00Z",
    "ends_at": "2026-06-14T23:59:59Z"
  }
}
```

---

#### `update_cycle`

| Field | Type | Required |
|-------|------|----------|
| `cycle_id` | string | ✓ |
| `name` | string | — |
| `starts_at` | string | — |
| `ends_at` | string | — |

```json
{"name":"update_cycle","arguments":{"cycle_id":"01932...","name":"Sprint 1 (revised)"}}
```

---

#### `list_cycles`

```json
{"name":"list_cycles","arguments":{"project_key":"LIN"}}
```

---

### Misc

#### `get_index`

Returns all projects with per-status ticket counts.

```json
{"name":"get_index","arguments":{}}
```

---

#### `get_log`

Recent audit events.

| Field | Type | Default |
|-------|------|---------|
| `since` | string (RFC3339) | — |
| `limit` | integer | 100 |

```json
{"name":"get_log","arguments":{"limit":20}}
```

---

## Protocol notes

- JSON-RPC 2.0 over stdin/stdout.
- `initialize` / `initialized` handshake is handled automatically.
- Tool results are returned as `content: [{type:"text", text:"<json>"}]`.
- Errors surface as `isError: true` in the response content, not as JSON-RPC errors.
