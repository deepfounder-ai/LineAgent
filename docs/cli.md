# LineAgent CLI Reference

```
lineagent [OPTIONS] <COMMAND>
```

## Global options

| Flag | Description |
|------|-------------|
| `--api-url <URL>` | Override `LINEAGENT_API_URL` |
| `--api-key <KEY>` | Override `LINEAGENT_API_KEY` |
| `--config <PATH>` | Override `LINEAGENT_CONFIG` |
| `-h, --help` | Print help |
| `-V, --version` | Print version |

## Environment variables

| Variable | Description |
|----------|-------------|
| `LINEAGENT_API_URL` | Server base URL (default `http://localhost:8080`) |
| `LINEAGENT_API_KEY` | Authentication key |
| `LINEAGENT_CONFIG` | Config file path (default `~/.config/lineagent/config.toml`) |

---

## Commands

### `serve`

Start the HTTP server.

```
lineagent serve [--host <HOST>] [--port <PORT>] [--data-dir <DIR>]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--host` | `127.0.0.1` | Bind address |
| `--port` | `8080` | Port |
| `--data-dir` | `./data` | SQLite database directory |

---

### `mcp`

Start the MCP stdio server.

```
lineagent mcp
```

Reads JSON-RPC 2.0 from stdin, writes to stdout. Requires a running `lineagent serve`.

---

### `user`

```
lineagent user <SUBCOMMAND>
```

| Subcommand | Description |
|------------|-------------|
| `register` | Register a new user (interactive) |
| `login` | Log in and save credentials |
| `whoami` | Show current user info |

```bash
lineagent user register
# prompts: username, password

lineagent user login
# prompts: username, password; saves API key to config file

lineagent user whoami
# {"id":"...","username":"agent","created_at":"..."}
```

---

### `keys`

```
lineagent keys <SUBCOMMAND>
```

| Subcommand | Args | Description |
|------------|------|-------------|
| `list` | | List API keys |
| `create` | `--name <NAME>` | Create an API key |
| `revoke` | `<ID>` | Revoke an API key by id |

```bash
lineagent keys create --name ci-bot
# key: lineagent_abc123...

lineagent keys list

lineagent keys revoke 01932...
```

---

### `project`

```
lineagent project <SUBCOMMAND>
```

| Subcommand | Args | Description |
|------------|------|-------------|
| `create` | `<KEY> --name <NAME> [--description <DESC>]` | Create project |
| `list` | | List all projects |
| `get` | `<KEY>` | Get project by key |
| `update` | `<KEY> [--name <N>] [--description <D>]` | Update project |

```bash
lineagent project create LIN --name "LineAgent" --description "Core tracker"
lineagent project list
lineagent project get LIN
lineagent project update LIN --name "LineAgent v2"
```

---

### `ticket`

```
lineagent ticket <SUBCOMMAND>
```

| Subcommand | Description |
|------------|-------------|
| `create` | Create a ticket |
| `list` | List tickets |
| `get` | Get a ticket |
| `update` | Update a ticket |
| `delete` | Delete a ticket |

#### `ticket create`

```
lineagent ticket create <PROJECT_KEY>
  --title <TITLE>
  [--description <DESC>]
  [--status <STATUS>]
  [--priority <PRIORITY>]
  [--assignee <ASSIGNEE>]
  [--parent <PARENT_IDENTIFIER>]
  [--cycle <CYCLE_ID>]
```

Status values: `backlog` (default), `todo`, `in_progress`, `review`, `done`, `cancelled`
Priority values: `critical`, `high`, `medium` (default), `low`

```bash
lineagent ticket create LIN --title "Implement search" --priority high
# LIN-1
```

#### `ticket list`

```
lineagent ticket list
  [--project <KEY>]
  [--status <STATUS>]
  [--priority <PRIORITY>]
  [--assignee <ASSIGNEE>]
  [--cycle <CYCLE_ID>]
  [--limit <N>]
```

#### `ticket get`

```
lineagent ticket get <IDENTIFIER>
```

```bash
lineagent ticket get LIN-1
```

#### `ticket update`

```
lineagent ticket update <IDENTIFIER>
  [--title <T>] [--description <D>] [--status <S>]
  [--priority <P>] [--assignee <A>]
  [--parent <PARENT_IDENTIFIER>] [--cycle <CYCLE_ID>]
```

#### `ticket delete`

```
lineagent ticket delete <IDENTIFIER>
```

---

### `comment`

```
lineagent comment <SUBCOMMAND>
```

| Subcommand | Args | Description |
|------------|------|-------------|
| `add` | `<IDENTIFIER> --body <BODY> [--author <AUTHOR>]` | Add comment |
| `list` | `<IDENTIFIER>` | List comments |

```bash
lineagent comment add LIN-1 --body "Fixed in #abc" --author agent
lineagent comment list LIN-1
```

---

### `relation`

```
lineagent relation <SUBCOMMAND>
```

| Subcommand | Args | Description |
|------------|------|-------------|
| `add` | `<FROM> <TO> --type <TYPE>` | Add relation |
| `remove` | `<RELATION_ID>` | Remove relation |
| `list` | `<IDENTIFIER>` | List relations for ticket |

Relation types: `blocks`, `duplicates`, `relates_to`

```bash
lineagent relation add LIN-1 LIN-2 --type blocks
lineagent relation list LIN-1
lineagent relation remove 01932...
```

---

### `cycle`

```
lineagent cycle <SUBCOMMAND>
```

| Subcommand | Args | Description |
|------------|------|-------------|
| `create` | `<PROJECT_KEY> --name <N> [--starts-at <S>] [--ends-at <E>]` | Create cycle |
| `list` | `[--project <KEY>]` | List cycles |
| `update` | `<ID> [--name <N>] [--starts-at <S>] [--ends-at <E>]` | Update cycle |

```bash
lineagent cycle create LIN --name "Sprint 1" \
  --starts-at 2026-06-01T00:00:00Z \
  --ends-at   2026-06-14T23:59:59Z

lineagent cycle list --project LIN
lineagent cycle update 01932... --name "Sprint 1 revised"
```

---

### `search`

```
lineagent search <QUERY> [--limit <N>]
```

Full-text BM25 search.

```bash
lineagent search "memory leak" --limit 5
```

---

### `index`

```
lineagent index
```

Print all projects with per-status ticket counts.

---

### `log`

```
lineagent log [--since <RFC3339>] [--limit <N>]
```

Print recent audit events.

```bash
lineagent log --since 2026-06-10T00:00:00Z --limit 50
```

---

### `completions`

```
lineagent completions <SHELL>
```

Generate shell completions. Shells: `bash`, `zsh`, `fish`, `powershell`.

```bash
lineagent completions zsh > ~/.zsh/completions/_lineagent
```

---

## Exit codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | General error (auth failure, not found, conflict, server error) |
| `2` | CLI usage error (invalid arguments) |
