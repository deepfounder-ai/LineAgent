//! `project list | get | create | update`.

use crate::cli::client::Client;
use crate::cli::config::CliConfig;
use crate::cli::output::{print_json, print_line, CliResult};
use crate::cli::ProjectCmd;

pub async fn run(cmd: &ProjectCmd, cfg: &CliConfig, json: bool) -> CliResult<()> {
    let client = Client::new(cfg)?;
    match cmd {
        ProjectCmd::List => {
            let projects: Vec<serde_json::Value> = client.get("/api/v1/projects").await?;
            if json {
                print_json(&projects)?;
            } else {
                for p in &projects {
                    print_line(format!(
                        "{} — {}",
                        p["key"].as_str().unwrap_or("?"),
                        p["name"].as_str().unwrap_or("?")
                    ));
                }
            }
        }
        ProjectCmd::Get { key } => {
            let p: serde_json::Value =
                client.get(&format!("/api/v1/projects/{key}")).await?;
            print_json(&p)?;
        }
        ProjectCmd::Create {
            key,
            name,
            description,
        } => {
            let body = serde_json::json!({
                "key": key,
                "name": name,
                "description": description,
            });
            let p: serde_json::Value = client.post("/api/v1/projects", &body).await?;
            if json {
                print_json(&p)?;
            } else {
                print_line(format!(
                    "created project {}",
                    p["key"].as_str().unwrap_or("?")
                ));
            }
        }
        ProjectCmd::Update {
            key,
            name,
            description,
        } => {
            let body = serde_json::json!({
                "name": name,
                "description": description,
            });
            let p: serde_json::Value =
                client.patch(&format!("/api/v1/projects/{key}"), &body).await?;
            if json {
                print_json(&p)?;
            } else {
                print_line(format!("updated project {key}"));
            }
        }
    }
    Ok(())
}
