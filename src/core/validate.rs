//! Shared validation helpers used by both the `api` and `auth` layers.

use crate::error::{AppError, Result};

pub const MIN_USERNAME_LEN: usize = 3;
pub const MAX_USERNAME_LEN: usize = 32;

/// Validate a username: 3-32 chars, `[a-z0-9_-]` only.
pub fn validate_username(username: &str) -> Result<()> {
    if username.len() < MIN_USERNAME_LEN {
        return Err(AppError::Validation(format!(
            "username must be at least {MIN_USERNAME_LEN} characters"
        )));
    }
    if username.len() > MAX_USERNAME_LEN {
        return Err(AppError::Validation(format!(
            "username must be at most {MAX_USERNAME_LEN} characters"
        )));
    }
    if !username
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
    {
        return Err(AppError::Validation(
            "username may only contain [a-z0-9_-]".to_string(),
        ));
    }
    Ok(())
}
