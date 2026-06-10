//! `relation list | add | remove`.

use crate::cli::client::Client;
use crate::cli::config::CliConfig;
use crate::cli::output::{print_json, print_line, CliResult};
use crate::cli::RelationCmd;

pub async fn run(cmd: &RelationCmd, cfg: &CliConfig, json: bool) -> CliResult<()> {
    let client = Client::new(cfg)?;
    match cmd {
        RelationCmd::List { ticket_id } => {
            let relations: serde_json::Value = client
                .get(&format!("/api/v1/tickets/{ticket_id}/relations"))
                .await?;
            if json {
                print_json(&relations)?;
            } else {
                let arr = relations.as_array().cloned().unwrap_or_default();
                for r in &arr {
                    print_line(format!(
                        "{} {} → {}",
                        r["id"].as_str().unwrap_or("?"),
                        r["relation_type"].as_str().unwrap_or("?"),
                        r["to_identifier"].as_str().unwrap_or("?"),
                    ));
                }
            }
        }
        RelationCmd::Add { from, to, rtype } => {
            let body = serde_json::json!({
                "from_identifier": from,
                "to_identifier": to,
                "relation_type": rtype,
            });
            let r: serde_json::Value = client.post("/api/v1/relations", &body).await?;
            if json {
                print_json(&r)?;
            } else {
                print_line(format!(
                    "added relation {}",
                    r["id"].as_str().unwrap_or("?")
                ));
            }
        }
        RelationCmd::Remove { relation_id } => {
            let _: serde_json::Value = client
                .delete(&format!("/api/v1/relations/{relation_id}"))
                .await?;
            if json {
                print_json(&serde_json::json!({ "removed": relation_id }))?;
            } else {
                print_line(format!("removed relation {relation_id}"));
            }
        }
    }
    Ok(())
}
