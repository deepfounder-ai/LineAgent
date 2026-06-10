use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::Result;
use crate::mcp::AuthedContext;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextContent {
    #[serde(rename = "type")]
    pub kind: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

impl TextContent {
    pub fn text(s: impl Into<String>) -> Self {
        Self { kind: "text".into(), text: s.into(), mime_type: None }
    }

    pub fn json(v: impl serde::Serialize) -> Self {
        Self::text(serde_json::to_string_pretty(&serde_json::to_value(v).unwrap()).unwrap())
    }
}

pub fn list_tools() -> Vec<Tool> {
    vec![]
}

pub async fn call_tool(name: &str, _args: Value, _user: &AuthedContext) -> Result<Vec<TextContent>> {
    Err(crate::error::AppError::Validation(format!("unknown tool '{name}'")))
}
