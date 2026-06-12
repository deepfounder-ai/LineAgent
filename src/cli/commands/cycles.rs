//! `cycle list | create | update`.

use crate::cli::client::Client;
use crate::cli::config::CliConfig;
use crate::cli::output::{print_json, print_line, CliError, CliResult};
use crate::cli::CycleCmd;

pub async fn run(cmd: &CycleCmd, cfg: &CliConfig, json: bool) -> CliResult<()> {
    let client = Client::new(cfg)?;
    match cmd {
        CycleCmd::List { project } => {
            let Some(project) = project else {
                return Err(CliError::Other(
                    "project key required; use --project KEY".to_string(),
                ));
            };
            let cycles: serde_json::Value = client
                .get(&format!("/api/v1/projects/{project}/cycles"))
                .await?;
            if json {
                print_json(&cycles)?;
            } else {
                let arr = cycles.as_array().cloned().unwrap_or_default();
                for c in &arr {
                    print_line(format!(
                        "{} — {}",
                        c["id"].as_str().unwrap_or("?"),
                        c["name"].as_str().unwrap_or("?"),
                    ));
                }
            }
        }
        CycleCmd::Create {
            project,
            name,
            starts_at,
            ends_at,
        } => {
            let body = serde_json::json!({
                "name": name,
                "starts_at": starts_at,
                "ends_at": ends_at,
            });
            let c: serde_json::Value = client
                .post(&format!("/api/v1/projects/{project}/cycles"), &body)
                .await?;
            if json {
                print_json(&c)?;
            } else {
                print_line(format!("created cycle {}", c["id"].as_str().unwrap_or("?")));
            }
        }
        CycleCmd::Update {
            cycle_id,
            name,
            starts_at,
            ends_at,
        } => {
            let body = serde_json::json!({
                "name": name,
                "starts_at": starts_at,
                "ends_at": ends_at,
            });
            let c: serde_json::Value = client
                .patch(&format!("/api/v1/cycles/{cycle_id}"), &body)
                .await?;
            if json {
                print_json(&c)?;
            } else {
                print_line(format!("updated cycle {cycle_id}"));
            }
        }
    }
    Ok(())
}
