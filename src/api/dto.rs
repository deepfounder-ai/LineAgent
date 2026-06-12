//! DTOs for the API.

use serde::{Deserialize, Serialize};

use crate::storage::api_key_repo::ApiKeyRow;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

pub use crate::core::validate::{MAX_USERNAME_LEN, MIN_USERNAME_LEN};
pub const MIN_PASSWORD_LEN: usize = 8;
pub const MAX_KEY_NAME_LEN: usize = 64;
pub const MAX_BODY_BYTES: usize = 10 * 1024 * 1024; // 10 MB

// ---------------------------------------------------------------------------
// Auth
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub secret: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub user_id: String,
    pub username: String,
    pub api_key: String,
    pub key_id: String,
}

impl AuthResponse {
    pub fn new(
        user_id: impl Into<String>,
        username: impl Into<String>,
        key_id: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        Self {
            user_id: user_id.into(),
            username: username.into(),
            api_key: api_key.into(),
            key_id: key_id.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateKeyRequest {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct KeyView {
    pub id: String,
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_used_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<ApiKeyRow> for KeyView {
    fn from(r: ApiKeyRow) -> Self {
        Self {
            id: r.id,
            name: r.name,
            created_at: r.created_at,
            last_used_at: r.last_used_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CreatedKeyView {
    #[serde(flatten)]
    pub view: KeyView,
    /// Plaintext key, shown exactly once.
    pub api_key: String,
}

// ---------------------------------------------------------------------------
// Health
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
    pub build_rev: Option<&'static str>,
}

// ---------------------------------------------------------------------------
// Projects
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CreateProjectReq {
    pub key: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProjectReq {
    pub name: Option<String>,
    pub description: Option<String>,
}

// ---------------------------------------------------------------------------
// Tickets
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CreateTicketReq {
    pub project_key: String,
    pub title: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub assignee: Option<String>,
    pub parent_identifier: Option<String>,
    pub cycle_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTicketReq {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub assignee: Option<String>,
    pub parent_identifier: Option<String>,
    pub cycle_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListTicketsQuery {
    pub project: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub assignee: Option<String>,
    pub cycle_id: Option<String>,
    pub parent: Option<String>,
    pub limit: Option<i64>,
}

// ---------------------------------------------------------------------------
// Comments
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct AddCommentReq {
    pub author: Option<String>,
    pub body: String,
}

// ---------------------------------------------------------------------------
// Relations
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct AddRelationReq {
    pub from_identifier: String,
    pub to_identifier: String,
    pub relation_type: String,
}

// ---------------------------------------------------------------------------
// Cycles
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CreateCycleReq {
    pub name: String,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCycleReq {
    pub name: Option<String>,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
}

// ---------------------------------------------------------------------------
// Search + Log
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct LogQuery {
    pub since: Option<String>,
    pub limit: Option<i64>,
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

/// Validate a username for the API layer. Delegates to the shared core validator.
pub fn validate_api_username(name: &str) -> Result<(), String> {
    crate::core::validate::validate_username(name).map_err(|e| e.to_string())
}

pub fn validate_api_password(pw: &str) -> Result<(), String> {
    if pw.len() < MIN_PASSWORD_LEN {
        return Err(format!(
            "password must be at least {MIN_PASSWORD_LEN} characters"
        ));
    }
    Ok(())
}

pub fn validate_key_name(name: &str) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("key name must not be empty".into());
    }
    if trimmed.len() > MAX_KEY_NAME_LEN {
        return Err(format!(
            "key name must be at most {MAX_KEY_NAME_LEN} characters"
        ));
    }
    Ok(())
}
