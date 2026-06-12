//! Fire-and-forget Slack notifications for ticket events.

use std::sync::Arc;

use crate::config::Config;

/// Post a message to Slack. Spawned as a background task — errors are logged, never propagated.
pub fn notify(config: Arc<Config>, text: String) {
    let (token, channel) = match (&config.slack_token, &config.slack_channel) {
        (Some(t), Some(c)) => (t.clone(), c.clone()),
        _ => return,
    };

    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "channel": channel,
            "text": text,
        });
        if let Err(e) = client
            .post("https://slack.com/api/chat.postMessage")
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
        {
            tracing::warn!(error = %e, "slack notification failed");
        }
    });
}
