//! `comment list | add`.

use crate::cli::client::Client;
use crate::cli::config::CliConfig;
use crate::cli::output::{print_json, print_line, CliResult};
use crate::cli::CommentCmd;

pub async fn run(cmd: &CommentCmd, cfg: &CliConfig, json: bool) -> CliResult<()> {
    let client = Client::new(cfg)?;
    match cmd {
        CommentCmd::List { ticket_id } => {
            let comments: serde_json::Value = client
                .get(&format!("/api/v1/tickets/{ticket_id}/comments"))
                .await?;
            if json {
                print_json(&comments)?;
            } else {
                let arr = comments.as_array().cloned().unwrap_or_default();
                for c in &arr {
                    let author = c["author"].as_str().unwrap_or("?");
                    let body = c["body"].as_str().unwrap_or("?");
                    print_line(format!("[{author}] {body}"));
                }
            }
        }
        CommentCmd::Add {
            ticket_id,
            body,
            author,
        } => {
            let payload = serde_json::json!({
                "body": body,
                "author": author,
            });
            let c: serde_json::Value = client
                .post(&format!("/api/v1/tickets/{ticket_id}/comments"), &payload)
                .await?;
            if json {
                print_json(&c)?;
            } else {
                print_line(format!("added comment {}", c["id"].as_str().unwrap_or("?")));
            }
        }
    }
    Ok(())
}
