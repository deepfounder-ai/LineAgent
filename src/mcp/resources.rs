use crate::error::Result;
use crate::mcp::AuthedContext;
use serde_json::Value;

pub fn list_resources() -> Vec<Value> {
    vec![]
}
pub async fn read(_uri: &str, _user: &AuthedContext) -> Result<Vec<Value>> {
    Ok(vec![])
}
