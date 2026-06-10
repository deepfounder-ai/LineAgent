//! `ticket list | get | create | update | delete`.

use crate::cli::client::Client;
use crate::cli::config::CliConfig;
use crate::cli::output::{print_json, print_line, CliResult};
use crate::cli::TicketCmd;

pub async fn run(cmd: &TicketCmd, cfg: &CliConfig, json: bool) -> CliResult<()> {
    let client = Client::new(cfg)?;
    match cmd {
        TicketCmd::List {
            project,
            status,
            priority,
            assignee,
            limit,
        } => {
            let mut params: Vec<String> = Vec::new();
            if let Some(p) = project {
                params.push(format!("project={p}"));
            }
            if let Some(s) = status {
                params.push(format!("status={s}"));
            }
            if let Some(p) = priority {
                params.push(format!("priority={p}"));
            }
            if let Some(a) = assignee {
                params.push(format!("assignee={a}"));
            }
            if let Some(l) = limit {
                params.push(format!("limit={l}"));
            }
            let path = if params.is_empty() {
                "/api/v1/tickets".to_string()
            } else {
                format!("/api/v1/tickets?{}", params.join("&"))
            };
            let tickets: serde_json::Value = client.get(&path).await?;
            if json {
                print_json(&tickets)?;
            } else {
                let arr = tickets
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
                for t in &arr {
                    print_line(format!(
                        "{} [{}] {}",
                        t["id"].as_str().unwrap_or("?"),
                        t["status"].as_str().unwrap_or("?"),
                        t["title"].as_str().unwrap_or("?"),
                    ));
                }
            }
        }
        TicketCmd::Get { id } => {
            let t: serde_json::Value =
                client.get(&format!("/api/v1/tickets/{id}")).await?;
            print_json(&t)?;
        }
        TicketCmd::Create {
            project,
            title,
            description,
            status,
            priority,
            assignee,
        } => {
            let body = serde_json::json!({
                "project_key": project,
                "title": title,
                "description": description,
                "status": status,
                "priority": priority,
                "assignee": assignee,
            });
            let t: serde_json::Value = client.post("/api/v1/tickets", &body).await?;
            if json {
                print_json(&t)?;
            } else {
                print_line(format!(
                    "created ticket {}",
                    t["id"].as_str().unwrap_or("?")
                ));
            }
        }
        TicketCmd::Update {
            id,
            title,
            status,
            priority,
        } => {
            let body = serde_json::json!({
                "title": title,
                "status": status,
                "priority": priority,
            });
            let t: serde_json::Value =
                client.patch(&format!("/api/v1/tickets/{id}"), &body).await?;
            if json {
                print_json(&t)?;
            } else {
                print_line(format!("updated ticket {id}"));
            }
        }
        TicketCmd::Delete { id } => {
            let _: serde_json::Value =
                client.delete(&format!("/api/v1/tickets/{id}")).await?;
            if json {
                print_json(&serde_json::json!({ "deleted": id }))?;
            } else {
                print_line(format!("deleted ticket {id}"));
            }
        }
    }
    Ok(())
}
