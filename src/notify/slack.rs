//! Fire-and-forget Slack notifications for ticket events.

use std::sync::Arc;

use serde_json::{json, Value};

use crate::config::Config;

/// Post a Block Kit message to Slack. Spawned as a background task — errors are logged, never propagated.
///
/// `payload` must contain at least `"text"` (fallback) and optionally `"blocks"`.
pub fn notify(config: Arc<Config>, payload: Value) {
    let (token, channel) = match (&config.slack_token, &config.slack_channel) {
        (Some(t), Some(c)) => (t.clone(), c.clone()),
        _ => return,
    };

    tracing::info!(channel = %channel, "slack: sending notification");
    tokio::spawn(async move {
        let mut body = payload;
        body["channel"] = json!(channel);

        let client = reqwest::Client::new();
        match client
            .post("https://slack.com/api/chat.postMessage")
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
        {
            Err(e) => tracing::warn!(error = %e, "slack notification failed (transport)"),
            Ok(resp) => {
                match resp.json::<Value>().await {
                    Err(e) => tracing::warn!(error = %e, "slack notification failed (parse)"),
                    Ok(v) => {
                        if v["ok"] == true {
                            tracing::info!("slack notification sent ok");
                        } else {
                            tracing::warn!(error = %v["error"], "slack notification failed (api)");
                        }
                    }
                }
            }
        }
    });
}

fn priority_emoji(p: &str) -> &'static str {
    match p {
        "critical" => "🔴",
        "high"     => "🟠",
        "medium"   => "🟡",
        "low"      => "🔵",
        _          => "⚪",
    }
}

fn status_label(s: &str) -> &'static str {
    match s {
        "backlog"     => "Backlog",
        "todo"        => "Todo",
        "in_progress" => "In Progress",
        "review"      => "Review",
        "done"        => "Done",
        "cancelled"   => "Cancelled",
        _             => "unknown",
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max).collect();
        format!("{t}…")
    }
}

pub fn ticket_created(
    config: Arc<Config>,
    identifier: &str,
    title: &str,
    status: &str,
    priority: &str,
    assignee: Option<&str>,
    description: Option<&str>,
) {
    let pe = priority_emoji(priority);
    let sl = status_label(status);
    let fallback = format!("[{identifier}] created: {title}");

    let mut blocks: Vec<Value> = vec![
        json!({
            "type": "section",
            "text": {
                "type": "mrkdwn",
                "text": format!("{pe} *{identifier}* — New ticket\n*{title}*")
            }
        }),
    ];

    if let Some(desc) = description.filter(|d| !d.trim().is_empty()) {
        blocks.push(json!({
            "type": "section",
            "text": { "type": "mrkdwn", "text": truncate(desc, 200) }
        }));
    }

    let mut context: Vec<Value> = vec![
        json!({"type": "mrkdwn", "text": format!("Status: `{sl}`")}),
        json!({"type": "mrkdwn", "text": format!("Priority: `{priority}`")}),
    ];
    if let Some(a) = assignee {
        context.push(json!({"type": "mrkdwn", "text": format!("Assignee: {a}")}));
    }
    blocks.push(json!({ "type": "context", "elements": context }));

    notify(config, json!({ "text": fallback, "blocks": blocks }));
}

pub fn ticket_updated(
    config: Arc<Config>,
    identifier: &str,
    title: &str,
    status: &str,
    priority: &str,
    assignee: Option<&str>,
    description: Option<&str>,
) {
    let sl = status_label(status);
    let fallback = format!("[{identifier}] updated: {title}");

    let mut blocks: Vec<Value> = vec![
        json!({
            "type": "section",
            "text": {
                "type": "mrkdwn",
                "text": format!("✏️ *{identifier}* — Updated\n*{title}*")
            }
        }),
    ];

    if let Some(desc) = description.filter(|d| !d.trim().is_empty()) {
        blocks.push(json!({
            "type": "section",
            "text": { "type": "mrkdwn", "text": truncate(desc, 200) }
        }));
    }

    let mut context: Vec<Value> = vec![
        json!({"type": "mrkdwn", "text": format!("Status: `{sl}`")}),
        json!({"type": "mrkdwn", "text": format!("Priority: `{priority}`")}),
    ];
    if let Some(a) = assignee {
        context.push(json!({"type": "mrkdwn", "text": format!("Assignee: {a}")}));
    }
    blocks.push(json!({ "type": "context", "elements": context }));

    notify(config, json!({ "text": fallback, "blocks": blocks }));
}
