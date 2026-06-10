use serde_json::{Value};
use crate::error::Result;
use crate::mcp::AuthedContext;

pub fn list_resources() -> Vec<Value> { vec![] }
pub async fn read(_uri: &str, _user: &AuthedContext) -> Result<Vec<Value>> { Ok(vec![]) }
