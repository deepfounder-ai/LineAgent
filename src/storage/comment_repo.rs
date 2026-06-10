//! `comments` table access.

use chrono::Utc;
use sqlx::{Row, SqlitePool};

use crate::error::{AppError, Result};

/// Row representation of the `comments` table.
#[derive(Debug, Clone)]
pub struct CommentRow {
    pub id: String,
    pub user_id: String,
    pub ticket_id: String,
    pub author: Option<String>,
    pub body: String,
    pub created_at: String,
}

impl CommentRow {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self> {
        Ok(Self {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            ticket_id: row.try_get("ticket_id")?,
            author: row.try_get("author")?,
            body: row.try_get("body")?,
            created_at: row.try_get("created_at")?,
        })
    }
}

/// Insert a new comment and return the inserted row.
pub async fn insert(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    ticket_id: &str,
    author: Option<&str>,
    body: &str,
) -> Result<CommentRow> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO comments (id, user_id, ticket_id, author, body, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )
    .bind(id)
    .bind(user_id)
    .bind(ticket_id)
    .bind(author)
    .bind(body)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| AppError::Db(e))?;

    Ok(CommentRow {
        id: id.to_string(),
        user_id: user_id.to_string(),
        ticket_id: ticket_id.to_string(),
        author: author.map(str::to_string),
        body: body.to_string(),
        created_at: now,
    })
}

/// List all comments for a ticket, ordered by created_at ASC.
pub async fn list_for_ticket(
    pool: &SqlitePool,
    user_id: &str,
    ticket_id: &str,
) -> Result<Vec<CommentRow>> {
    let rows = sqlx::query(
        "SELECT id, user_id, ticket_id, author, body, created_at \
         FROM comments WHERE user_id = ?1 AND ticket_id = ?2 ORDER BY created_at ASC",
    )
    .bind(user_id)
    .bind(ticket_id)
    .fetch_all(pool)
    .await?;
    rows.iter().map(CommentRow::from_row).collect()
}
