---
name: lineagent
description: Task tracking for AI agents backed by a LineAgent server, used via
  the lineagent MCP tools. Use BEFORE starting any discrete work (create_ticket),
  DURING work (update_ticket status, add_comment for decisions), and AFTER
  (move to done, call get_index). Also for resuming sessions (list_tickets
  in_progress, get_log).
---

# LineAgent task tracking

Task state lives in the `lineagent` MCP server (registered in `.mcp.json`).
Always use MCP tools — never call the HTTP API directly.

## When to use this skill

- **PLAN**: before starting work, create tickets for each discrete unit.
  Set blocking relations. Assign to a cycle if working in sprints.
- **EXECUTE**: move ticket to `in_progress` → do work → comment decisions →
  move to `done` or `review`.
- **RESUME**: call `get_index` → `list_tickets { status: "in_progress" }` →
  read comments → continue.
- **SEARCH**: before creating a ticket, call `search_tickets` to check it
  doesn't already exist.

## MCP tools (quick reference)

**Tickets:** `create_ticket`, `update_ticket`, `get_ticket`, `list_tickets`,
`delete_ticket`

**Comments:** `add_comment`, `list_comments`

**Relations:** `add_relation`, `remove_relation`

**Search:** `search_tickets`

**Projects:** `create_project`, `get_project`, `update_project`, `list_projects`

**Cycles:** `create_cycle`, `update_cycle`, `list_cycles`

**Dashboard:** `get_index`, `get_log`

## Status lifecycle

`backlog` → `todo` → `in_progress` → `review` → `done`
                                              ↘ `cancelled`

## Rules

- Create a ticket before starting any work that takes more than one tool call.
- Move ticket to `in_progress` at the moment you begin; to `done` when complete.
- Never leave a ticket in `in_progress` at the end of a session.
- Leave a comment for every non-obvious decision (what + why + what you ruled out).
- Call `get_index` at the end of a session as a sanity check.
- Do not invent identifiers — read them from create responses.

Full instructions: `AGENTS.md` in the project root.
