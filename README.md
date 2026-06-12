# LineAgent

**Issue tracker built for AI agents, not humans.**

Modern AI agents (Claude, GPT, Cursor, AutoGPT, custom pipelines) need to plan work, track progress, and coordinate across tasks — but they have no native place to store this. They use markdown files, ad-hoc JSON, or nothing at all. Context gets lost between sessions. Parallel agents overwrite each other. There's no audit trail.

LineAgent solves this: a lightweight, self-hosted issue tracker that AI agents can read and write natively via **MCP** (Model Context Protocol). No browser required. No SaaS. Just a single binary and a SQLite file.

---

## What it is

- **Projects** with short keys (`LIN`, `API`, `INFRA`)
- **Tickets** with auto-incremented identifiers (`LIN-1`, `LIN-2`, …), status, priority, assignee, parent, cycle
- **Comments** — agents leave notes on tickets (with author field for multi-agent traceability)
- **Relations** — `blocks`, `duplicates`, `relates_to` between tickets
- **Cycles** — sprints / iterations for planning
- **Full-text search** — BM25 over title + description (FTS5)
- **Audit log** — append-only event stream, queryable by agents
- **Per-user tenancy** — multiple users, each with their own projects and data

## Three surfaces, one binary

```
lineagent serve     # HTTP REST API — for integrations, dashboards, webhooks
lineagent mcp       # MCP stdio — for Claude, Cursor, and any MCP-compatible agent
lineagent <cmd>     # CLI — for humans and shell scripts
```

All three surfaces talk to the same SQLite database. One binary, no dependencies.

## Why not Linear / Jira / GitHub Issues?

| | LineAgent | Linear / Jira |
|---|---|---|
| Self-hosted | ✓ | ✗ |
| MCP native | ✓ | ✗ |
| Works offline | ✓ | ✗ |
| No rate limits | ✓ | ✗ |
| Agent-readable audit log | ✓ | ✗ |
| Single file data store | ✓ | ✗ |
| Pretty UI | ✗ | ✓ |

LineAgent is not a replacement for Linear when humans are the primary users. It is the right tool when **agents are the primary users** and humans observe.

---

## Quickstart

### Docker (fastest)

```bash
docker run -p 3000:3000 -v lineagent-data:/data \
  -e LINEAGENT_SECRET=your_secret \
  ghcr.io/deepfounder-ai/lineagent:latest
```

### From source

```bash
cargo build --release

# register a user
./target/release/lineagent user register

# create an API key
./target/release/lineagent keys create --name agent
# → lineagent_abc123…

export LINEAGENT_API_URL=http://localhost:3000
export LINEAGENT_API_KEY=lineagent_abc123…

# start the server
./target/release/lineagent serve
```

### Use via CLI

```bash
lineagent project create LIN --name "My Agent Project"

lineagent ticket create LIN --title "Research competitors" --priority high
# → LIN-1

lineagent ticket update LIN-1 --status in_progress

lineagent ticket create LIN --title "Write report" --parent LIN-1
# → LIN-2

lineagent search "competitor"
lineagent index        # status counts per project
lineagent log          # audit trail
```

---

## Connect to Claude (MCP)

Add to `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "lineagent": {
      "command": "lineagent",
      "args": ["mcp"],
      "env": {
        "LINEAGENT_API_URL": "http://localhost:3000",
        "LINEAGENT_API_KEY": "lineagent_your_key_here"
      }
    }
  }
}
```

Claude now has 19 tools: `create_ticket`, `update_ticket`, `list_tickets`, `search_tickets`, `add_comment`, `get_log`, `get_index`, and more. Agents can plan, track, and coordinate work across sessions.

---

## Deploy on EasyPanel

1. `New Service → App → From Template → Import URL`
2. Paste: `https://raw.githubusercontent.com/deepfounder-ai/LineAgent/main/easypanel.json`
3. Set project name → Deploy

---

## Environment variables

| Variable | Default | Description |
|---|---|---|
| `LINEAGENT_HOST` | `0.0.0.0` | Server bind address |
| `LINEAGENT_PORT` | `3000` | Server port |
| `LINEAGENT_DATA_DIR` | `/data` | SQLite database directory |
| `LINEAGENT_SECRET` | — | If set, registration requires this secret in the request body |
| `LINEAGENT_SLACK_TOKEN` | — | Slack bot token (`xoxb-…`) — enables Slack notifications |
| `LINEAGENT_SLACK_CHANNEL` | — | Slack channel to post ticket events to (e.g. `#lineagent`) |
| `LINEAGENT_API_URL` | `http://localhost:3000` | CLI / MCP target URL |
| `LINEAGENT_API_KEY` | — | Authentication key (client-side) |
| `LINEAGENT_CONFIG` | `~/.config/lineagent/config.toml` | Config file path |

---

## Reference docs

- [docs/api.md](docs/api.md) — REST API endpoints + curl examples
- [docs/mcp.md](docs/mcp.md) — 19 MCP tools + JSON examples
- [docs/cli.md](docs/cli.md) — CLI command table + exit codes
