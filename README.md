# LineAgent

Self-hosted issue tracker for AI agents. Linear-for-agents: REST + MCP stdio + CLI over a single SQLite file.

## Features

- **REST API** — full CRUD for projects, tickets, comments, relations, cycles
- **MCP stdio** — 19 tools for AI agents via JSON-RPC 2.0 (works with Claude, Cursor, etc.)
- **CLI** — scripting and human use
- **FTS5 full-text search** — BM25 ranked over ticket title + description
- **Audit log** — append-only events table
- **Per-user tenancy** — all data scoped to authenticated user

## Quickstart

### 1. Build

```bash
cargo build --release
# binary at target/release/lineagent
```

### 2. Register a user

```bash
lineagent user register
# prompts for username + password; prints "Registered."
```

### 3. Create an API key

```bash
lineagent keys create --name agent
# prints key: lineagent_…
export LINEAGENT_API_KEY=lineagent_…
```

### 4. Start the server

```bash
lineagent serve
# listening on 0.0.0.0:8080 by default
```

### 5. Use the CLI

```bash
# create a project
lineagent project create LIN --name "LineAgent"

# create a ticket
lineagent ticket create LIN --title "First ticket"
# → LIN-1

# list tickets
lineagent ticket list --project LIN

# update a ticket
lineagent ticket update LIN-1 --status in_progress

# full-text search
lineagent search "first ticket"

# project index (status counts)
lineagent index
```

## MCP Configuration (Claude Desktop)

Add to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "lineagent": {
      "command": "/usr/local/bin/lineagent",
      "args": ["mcp"],
      "env": {
        "LINEAGENT_API_URL": "http://localhost:8080",
        "LINEAGENT_API_KEY": "lineagent_your_key_here"
      }
    }
  }
}
```

The MCP process connects to a running `lineagent serve` instance. `LINEAGENT_API_KEY` is never written to stdout — MCP stdout is a clean JSON-RPC stream.

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `LINEAGENT_HOST` | `127.0.0.1` | Server bind address |
| `LINEAGENT_PORT` | `8080` | Server port |
| `LINEAGENT_DATA_DIR` | `./data` | Directory for `lineagent.db` |
| `LINEAGENT_API_URL` | `http://localhost:8080` | CLI / MCP target |
| `LINEAGENT_API_KEY` | — | Authentication key |
| `LINEAGENT_CONFIG` | `~/.config/lineagent/config.toml` | Config file path |

## Docker

```bash
docker run -p 8080:8080 -v lineagent-data:/data \
  -e LINEAGENT_API_KEY=lineagent_… \
  ghcr.io/your-org/lineagent:latest
```

See [docs/api.md](docs/api.md), [docs/mcp.md](docs/mcp.md), [docs/cli.md](docs/cli.md) for full reference.
