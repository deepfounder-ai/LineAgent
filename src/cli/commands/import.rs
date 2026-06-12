//! `lineagent import` subcommands.

use std::collections::HashMap;
use std::time::Duration;

use reqwest::Client as HttpClient;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::cli::client::Client;
use crate::cli::config::CliConfig;
use crate::cli::output::{CliError, CliResult};
use crate::cli::ImportCmd;

pub async fn run(cmd: &ImportCmd, config: &CliConfig) -> CliResult<()> {
    match cmd {
        ImportCmd::Linear {
            linear_key,
            teams: team_filter,
            dry_run,
        } => run_linear(linear_key.clone(), team_filter, *dry_run, config).await,
    }
}

// ---------------------------------------------------------------------------
// Linear GraphQL types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct LinearTeam {
    id: String,
    name: String,
    key: String,
}

#[derive(Debug, Deserialize)]
struct LinearState {
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Debug, Deserialize)]
struct LinearUser {
    #[serde(rename = "displayName")]
    display_name: String,
}

#[derive(Debug, Deserialize)]
struct LinearParent {
    id: String,
}

#[derive(Debug, Deserialize)]
struct LinearComment {
    body: String,
    user: Option<LinearUser>,
}

#[derive(Debug, Deserialize)]
struct LinearIssue {
    id: String,
    title: String,
    description: Option<String>,
    priority: u8,
    assignee: Option<LinearUser>,
    state: LinearState,
    parent: Option<LinearParent>,
    comments: LinearCommentPage,
}

#[derive(Debug, Deserialize)]
struct LinearCommentPage {
    nodes: Vec<LinearComment>,
}

#[derive(Debug, Deserialize)]
struct PageInfo {
    #[serde(rename = "hasNextPage")]
    has_next_page: bool,
    #[serde(rename = "endCursor")]
    end_cursor: Option<String>,
}

// ---------------------------------------------------------------------------
// Mapping helpers
// ---------------------------------------------------------------------------

fn map_status(kind: &str) -> &'static str {
    match kind {
        "backlog" => "backlog",
        "unstarted" => "todo",
        "started" => "in_progress",
        "completed" => "done",
        "cancelled" => "cancelled",
        _ => "backlog",
    }
}

fn map_priority(p: u8) -> &'static str {
    match p {
        1 => "critical",
        2 => "high",
        4 => "low",
        _ => "medium",
    }
}

// ---------------------------------------------------------------------------
// Linear API client
// ---------------------------------------------------------------------------

struct LinearClient {
    http: HttpClient,
    token: String,
}

impl LinearClient {
    fn new(token: String) -> CliResult<Self> {
        let http = HttpClient::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(concat!("lineagent-importer/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| CliError::Network(e.to_string()))?;
        Ok(Self { http, token })
    }

    async fn graphql(&self, query: &str, variables: Value) -> CliResult<Value> {
        let body = json!({ "query": query, "variables": variables });
        let resp = self
            .http
            .post("https://api.linear.app/graphql")
            .bearer_auth(&self.token)
            .json(&body)
            .send()
            .await
            .map_err(|e| CliError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(CliError::Http {
                status,
                code: "linear_error".into(),
                message: text,
            });
        }

        let v: Value = resp
            .json()
            .await
            .map_err(|e| CliError::Network(e.to_string()))?;

        if let Some(errors) = v.get("errors") {
            return Err(CliError::Usage(format!("linear api error: {errors}")));
        }

        Ok(v)
    }

    async fn fetch_teams(&self) -> CliResult<Vec<LinearTeam>> {
        let q = r#"
            query { teams { nodes { id name key } } }
        "#;
        let data = self.graphql(q, json!({})).await?;
        let nodes = &data["data"]["teams"]["nodes"];
        serde_json::from_value(nodes.clone())
            .map_err(|e| CliError::Usage(format!("failed to parse teams: {e}")))
    }

    async fn fetch_issues(&self, team_id: &str) -> CliResult<Vec<LinearIssue>> {
        let q_all = r#"
            query TeamIssuesAll($teamId: String!, $after: String) {
              team(id: $teamId) {
                issues(first: 100, after: $after) {
                  nodes {
                    id title description priority
                    assignee { displayName }
                    state { type }
                    parent { id }
                    comments { nodes { body user { displayName } } }
                  }
                  pageInfo { hasNextPage endCursor }
                }
              }
            }
        "#;
        let mut all: Vec<LinearIssue> = vec![];
        let mut cursor: Option<String> = None;

        loop {
            let vars = json!({ "teamId": team_id, "after": cursor });
            let data = self.graphql(q_all, vars).await?;
            let issues_obj = &data["data"]["team"]["issues"];

            let nodes: Vec<LinearIssue> = serde_json::from_value(issues_obj["nodes"].clone())
                .map_err(|e| CliError::Usage(format!("failed to parse issues: {e}")))?;
            let page_info: PageInfo =
                serde_json::from_value(issues_obj["pageInfo"].clone())
                    .map_err(|e| CliError::Usage(format!("failed to parse pageInfo: {e}")))?;

            all.extend(nodes);

            if page_info.has_next_page {
                cursor = page_info.end_cursor;
            } else {
                break;
            }
        }

        Ok(all)
    }
}

// ---------------------------------------------------------------------------
// Import runner
// ---------------------------------------------------------------------------

async fn run_linear(
    linear_key: Option<String>,
    team_filter: &[String],
    dry_run: bool,
    config: &CliConfig,
) -> CliResult<()> {
    let token = linear_key
        .or_else(|| std::env::var("LINEAGENT_LINEAR_API_KEY").ok())
        .ok_or_else(|| {
            CliError::Usage(
                "Linear API key required: use --linear-key or LINEAGENT_LINEAR_API_KEY".into(),
            )
        })?;

    let linear = LinearClient::new(token)?;
    let la = Client::new(config)?;

    eprintln!("Fetching teams from Linear…");
    let teams = linear.fetch_teams().await?;
    let teams: Vec<LinearTeam> = if team_filter.is_empty() {
        teams
    } else {
        let filter: Vec<String> = team_filter.iter().map(|k| k.to_uppercase()).collect();
        teams
            .into_iter()
            .filter(|t| filter.contains(&t.key.to_uppercase()))
            .collect()
    };

    if teams.is_empty() {
        eprintln!("No matching teams found.");
        return Ok(());
    }

    eprintln!("Found {} team(s): {}", teams.len(), teams.iter().map(|t| t.key.as_str()).collect::<Vec<_>>().join(", "));

    for team in &teams {
        eprintln!("\n→ Team: {} ({})", team.name, team.key);

        if !dry_run {
            ensure_project(&la, &team.key, &team.name).await?;
        } else {
            eprintln!("  [dry-run] would create/skip project {}", team.key);
        }

        eprintln!("  Fetching issues…");
        let issues = linear.fetch_issues(&team.id).await?;
        eprintln!("  {} issue(s)", issues.len());

        // Pass 1: create tickets (root issues first, then children)
        // Map linear_id → lineagent_identifier
        let mut id_map: HashMap<String, String> = HashMap::new();

        // Sort: parents before children (issues without parent come first)
        let (roots, children): (Vec<&LinearIssue>, Vec<&LinearIssue>) =
            issues.iter().partition(|i| i.parent.is_none());

        for issue in roots.iter().chain(children.iter()) {
            let status = map_status(&issue.state.kind);
            let priority = map_priority(issue.priority);
            let assignee = issue.assignee.as_ref().map(|a| a.display_name.as_str());

            // Resolve parent identifier (only if parent was already imported)
            let parent_identifier = issue
                .parent
                .as_ref()
                .and_then(|p| id_map.get(&p.id))
                .cloned();

            if dry_run {
                eprintln!(
                    "  [dry-run] ticket: {} | {} | {} | assignee={:?} | parent={:?}",
                    issue.title, status, priority, assignee, parent_identifier
                );
                // Record a placeholder so children can resolve parents in dry-run
                id_map.insert(issue.id.clone(), format!("{}-?", team.key));
                continue;
            }

            let body = {
                let mut b = json!({
                    "project_key": team.key,
                    "title": issue.title,
                    "status": status,
                    "priority": priority,
                });
                if let Some(desc) = &issue.description {
                    b["description"] = json!(desc);
                }
                if let Some(a) = assignee {
                    b["assignee"] = json!(a);
                }
                if let Some(ref pi) = parent_identifier {
                    b["parent_identifier"] = json!(pi);
                }
                b
            };

            let resp: Value = la.post("/api/v1/tickets", &body).await?;
            let identifier = resp["identifier"]
                .as_str()
                .unwrap_or("?")
                .to_string();
            eprintln!("  + {} → {}", issue.title, identifier);
            id_map.insert(issue.id.clone(), identifier.clone());

            // Pass 2: add comments
            for comment in &issue.comments.nodes {
                if comment.body.trim().is_empty() {
                    continue;
                }
                let author = comment.user.as_ref().map(|u| u.display_name.as_str());
                let mut cb = json!({ "body": comment.body });
                if let Some(a) = author {
                    cb["author"] = json!(a);
                }
                let ticket_id = resp["id"].as_str().unwrap_or("");
                let _: CliResult<Value> = la
                    .post(&format!("/api/v1/tickets/{ticket_id}/comments"), &cb)
                    .await;
            }
        }

        eprintln!(
            "  ✓ imported {} ticket(s) for {}",
            id_map.len(),
            team.key
        );
    }

    if dry_run {
        eprintln!("\n[dry-run] no data was written.");
    } else {
        eprintln!("\nImport complete.");
    }
    Ok(())
}

async fn ensure_project(la: &Client, key: &str, name: &str) -> CliResult<()> {
    // Try to GET the project first; if 404 create it.
    let path = format!("/api/v1/projects/{key}");
    match la.get_opt::<Value>(&path).await {
        Ok(Some(_)) => {
            eprintln!("  project {key} already exists, skipping");
        }
        Ok(None) => {
            la.post::<Value, _>(
                "/api/v1/projects",
                &json!({ "key": key, "name": name }),
            )
            .await?;
            eprintln!("  created project {key}");
        }
        Err(e) => return Err(e),
    }
    Ok(())
}
