# AGENTS.md — Instructions for LLM Agents Using LineAgent

> **You are an LLM agent. LineAgent is your task tracker.** Read this file
> before you start any work session. Treat it as a system prompt for a new
> tool: it is non-negotiable unless the human explicitly overrides it.

This file is normative. If a workflow below conflicts with a behaviour you
would otherwise default to, the workflow wins. If the workflow is ambiguous,
ask the human before guessing.

---

## 1. What is LineAgent

LineAgent is a **persistent task-tracking layer** for LLM agents. It is not
a knowledge base, not a wiki, not a chat log. It is:

- A **service** that stores projects, tickets, comments, and relations, scoped
  per user.
- A **search engine** over tickets (SQLite FTS5, BM25).
- A **graph** of tickets connected by typed relations (`blocks`,
  `duplicates`, `relates_to`).
- An **audit log** of every mutation, so future agents can understand what was
  done and why.
- A **cycle tracker** (sprints / iterations) for time-boxed planning.

It is intentionally dumb. The "intelligence" is yours: you decide what work
to break into tickets, you track progress, you leave notes for your future
self and for parallel agents. LineAgent stores what you wrote and helps
future agents find it.

---

## 2. When to use it

### Always create a ticket when you start a discrete unit of work

If a task will take more than one tool call, has a clear outcome, and the
human or another agent might care whether it was done — create a ticket.

Examples of things that deserve tickets:
- Implementing a feature
- Investigating a bug
- Running a research task
- Doing a code review
- Executing a deployment step

Examples of things that do **not** deserve tickets:
- A single grep/read that takes one tool call
- A conversational clarification
- A task that is complete in under 30 seconds with no output

### Always update a ticket when its status changes

Status transitions are the primary signal downstream agents use to understand
project state. Never leave a ticket in `in_progress` after you have finished.

### Leave comments when you make a non-obvious decision

Comments are persistent. Your context window is not. If you made a choice that
future-you or another agent might question, write a comment explaining why.

```
ticket_identifier: LIN-7
body: "Chose argon2id over bcrypt — argon2id is memory-hard,
       resists GPU attacks. bcrypt has no memory-hardness. See OWASP 2025."
author: "agent"
```

### Use relations to model dependencies

If ticket A cannot start until ticket B is done, add a `blocks` relation.
This lets you query "what is blocking me right now?" and gives humans a graph
of the work.

---

## 3. Ticket lifecycle

```
backlog → todo → in_progress → review → done
                                      ↘ cancelled
```

Rules:
- Newly created tickets start in `backlog` by default.
- Move to `todo` when the ticket is committed to the current cycle.
- Move to `in_progress` when you begin execution.
- Move to `review` when execution is complete and output is ready for human check.
- Move to `done` when confirmed correct.
- Use `cancelled` only when the ticket is explicitly abandoned (not just deprioritised).

Never move a ticket directly from `backlog` to `done` without intermediate steps
unless the work was trivially small and you completed it in this turn.

---

## 4. The three workflows

### 4.1 Plan — decompose a task into tickets

Goal: turn a human request into a set of trackable units of work.

```
1. Create a project if one does not exist.
     create_project { key: "PROJ", name: "..." }

2. Break the work into tickets.
     - Each ticket is one unit of work with a single clear outcome.
     - Use parent_identifier for sub-tasks.
     - Set priority: critical / high / medium / low.
     - Set status: todo (committed this session) or backlog (later).

3. Add blocking relations where relevant.
     add_relation { from: "PROJ-3", to: "PROJ-1", relation_type: "blocks" }

4. Assign to a cycle if you are working in iterations.
     create_cycle { project_key: "PROJ", name: "Sprint 1", ... }
     — then set cycle_id on each ticket.

5. Call get_index to confirm the plan looks right.
```

### 4.2 Execute — work through tickets

Goal: complete the committed work, leaving a full audit trail.

```
1. list_tickets { status: "todo" } — find what needs doing.

2. For each ticket:
   a. update_ticket { identifier: "PROJ-1", status: "in_progress" }
   b. Do the actual work.
   c. add_comment with any non-obvious decisions made.
   d. update_ticket { identifier: "PROJ-1", status: "review" or "done" }

3. If blocked: update status to backlog, add a comment explaining the block,
   add a "blocks" relation from the blocking ticket to this one.

4. Call get_index at the end of a session to check overall state.
```

### 4.3 Resume — pick up where a previous session left off

Goal: re-establish context without re-reading everything.

```
1. get_index — see all projects and per-status counts at a glance.

2. list_tickets { status: "in_progress" } — find interrupted work.

3. For any in_progress ticket, read its comments to understand
   what was done and what was left.

4. list_tickets { status: "todo" } — see what is next.

5. search_tickets { query: "<relevant topic>" } — find related tickets
   if you are starting a new sub-task.

6. get_log { limit: 20 } — see recent mutations across all tickets.
```

---

## 5. Naming conventions

### Project keys

- Uppercase ASCII letters and digits only: `LIN`, `API`, `INFRA`, `ML2`.
- 1–8 characters.
- One project per logical system or product. Do not create a project per task.

### Ticket titles

- Imperative verb phrase: "Add rate limiting to /auth/register".
- Short: under 80 chars. The identifier plus title should fit one terminal line.
- Specific: "Fix null pointer in ticket_repo::update" > "Fix bug".

### Comments

- Write for future-you who has no memory of this conversation.
- Include: what you decided, why, and what you ruled out.
- Do not write "done" as a comment. Status changes communicate done.

### Assignee field

- Use a stable agent identifier: model name, pipeline name, or role.
  E.g. `claude-sonnet-4-6`, `ci-bot`, `search-agent`.
- Leave blank if no specific agent is assigned.

---

## 6. Available MCP tools

| Tool | When to use |
|------|-------------|
| `create_ticket` | Start a new unit of work |
| `update_ticket` | Status change, reassign, set cycle |
| `get_ticket` | Read a ticket + its comments and relations |
| `list_tickets` | Find tickets by status / project / priority |
| `delete_ticket` | Remove a ticket that should never have existed |
| `add_comment` | Leave a non-obvious decision or finding |
| `list_comments` | Read the history of a ticket |
| `add_relation` | Model a blocking or duplicate relationship |
| `remove_relation` | Fix a wrong relation |
| `search_tickets` | Find tickets by full-text query (BM25) |
| `create_project` | One-time setup for a new project |
| `get_project` | Read project metadata |
| `update_project` | Rename or redescribe a project |
| `list_projects` | See all projects |
| `create_cycle` | Start a new sprint |
| `update_cycle` | Adjust cycle dates or name |
| `list_cycles` | See all cycles for a project |
| `get_index` | Dashboard: per-project status counts |
| `get_log` | Audit trail of recent mutations |

Do not call the HTTP API directly. Use the MCP tools — they handle auth,
error normalisation, and response formatting.

---

## 7. Schema discipline

### One ticket, one outcome

A ticket should describe one thing that will be either done or not done.
"Refactor auth and fix the rate limiter and update docs" is three tickets.

### Status must reflect truth

A ticket in `in_progress` means an agent is actively working on it right now.
If you are done for the session, move it to `review` or `backlog`. Do not
leave phantom in-progress tickets — they mislead every future agent that
calls `list_tickets { status: "in_progress" }`.

### Comments are append-only truth

Never use a comment to contradict a previous comment. If a previous decision
was wrong, write a new comment explaining the correction. The log is an
audit trail, not a scratchpad.

### Never invent ticket identifiers

Identifiers like `LIN-7` are assigned by the server. Do not guess or
hard-code them. Always read the identifier from the create response.

---

## 8. Example session

```
Human: Build a rate limiter for the login endpoint.

Agent: I'll plan this as tickets before starting.

  → create_ticket { project_key: "LIN", title: "Research rate-limiting strategies",
      status: "todo", priority: "high" }
  ← { identifier: "LIN-11", ... }

  → create_ticket { project_key: "LIN", title: "Implement token-bucket rate limiter",
      status: "todo", priority: "high", parent_identifier: null }
  ← { identifier: "LIN-12", ... }

  → create_ticket { project_key: "LIN", title: "Add integration test for rate limiter",
      status: "todo", priority: "medium", parent_identifier: "LIN-12" }
  ← { identifier: "LIN-13", ... }

  → add_relation { from_identifier: "LIN-12", to_identifier: "LIN-11",
      relation_type: "blocks" }

Starting with LIN-11.

  → update_ticket { identifier: "LIN-11", status: "in_progress" }

[Does research...]

  → add_comment { ticket_identifier: "LIN-11", body: "Token bucket chosen over
      sliding window log — O(1) space vs O(n). See RFC 6585 for 429 semantics.",
      author: "claude-sonnet-4-6" }
  → update_ticket { identifier: "LIN-11", status: "done" }
  → update_ticket { identifier: "LIN-12", status: "in_progress" }

[Implements...]

  → update_ticket { identifier: "LIN-12", status: "review" }
  → update_ticket { identifier: "LIN-13", status: "in_progress" }

[Writes tests...]

  → update_ticket { identifier: "LIN-13", status: "done" }
  → update_ticket { identifier: "LIN-12", status: "done" }

  → get_index
  ← LIN: backlog=8, todo=0, in_progress=0, review=0, done=3, cancelled=0

Done. Implemented token-bucket rate limiter (LIN-11 → LIN-12 → LIN-13).
```

Things to notice:
- The agent planned before doing.
- It used a blocking relation to model the dependency.
- It left a comment with the *why* of the decision, not just the *what*.
- It moved every ticket to a terminal state before declaring done.
- It called `get_index` at the end for a sanity check.

---

## 9. Failure modes

- **Cannot reach LineAgent.** Stop. Do not track tasks locally and promise to
  "sync later". Tell the human the server is unavailable.
- **Ticket already exists for this work.** Use `search_tickets` first. If found,
  update the existing ticket — do not create a duplicate.
- **Blocked mid-task.** Move the ticket to `backlog`, add a comment explaining
  the blocker, add a relation from the blocking ticket. Do not leave it `in_progress`.
- **Identifier collision.** Identifiers are assigned by the server. If you see a
  conflict error on create, you have a bug in your logic — read the error, do not retry blindly.
- **You do not know which project to use.** Call `list_projects` first. If none
  fits, ask the human before creating a new one.

---

## 10. Integrations (operator-managed)

These are server-side features configured by the human operator. As an agent
you do not configure them — you just benefit from them.

### Slack notifications

If `LINEAGENT_SLACK_TOKEN` and `LINEAGENT_SLACK_CHANNEL` are set on the server,
every `create_ticket` and `update_ticket` call automatically posts a message to
the configured Slack channel. You do not need to do anything special — the
notifications fire on the same tool calls you already make.

### Importing from Linear

A workspace can be seeded from Linear using the CLI:

```bash
lineagent import linear --linear-key lin_api_... --team ENG
```

This creates projects, tickets (with status/priority mapping), and comments
from a Linear workspace. As an agent you will find these tickets already
present; treat them like any other tickets.

### MCP against a remote server

The `lineagent mcp` binary can target a remote LineAgent instance. No local
database is needed. Configure via env:

```
LINEAGENT_API_URL=https://your-lineagent.example.com
LINEAGENT_API_KEY=lineagent_...
```

All tool calls are proxied over HTTPS. The `.mcp.json` template in the repo
root shows the configuration format.

---

## 11. TL;DR

1. Read this file before starting any session.
2. Plan = create tickets → set relations → assign to cycle (if any).
3. Execute = `in_progress` while working → comment decisions → `done` when complete.
4. Resume = `get_index` → `list_tickets { status: "in_progress" }` → read comments → continue.
5. A ticket in `in_progress` is a live claim. Clean up before you leave.

If you only remember one thing: **move every ticket you open to a terminal status
before the session ends**.

---

*LineAgent is open-source: https://github.com/deepfounder-ai/LineAgent*
