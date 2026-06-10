//! `relations` table access.

use chrono::Utc;
use sqlx::{Row, SqlitePool};

use crate::error::{AppError, Result};

/// Row representation of the `relations` table.
#[derive(Debug, Clone)]
pub struct RelationRow {
    pub id: String,
    pub user_id: String,
    pub from_ticket_id: String,
    pub to_ticket_id: String,
    pub relation_type: String,
    pub created_at: String,
}

impl RelationRow {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self> {
        Ok(Self {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            from_ticket_id: row.try_get("from_ticket_id")?,
            to_ticket_id: row.try_get("to_ticket_id")?,
            relation_type: row.try_get("relation_type")?,
            created_at: row.try_get("created_at")?,
        })
    }
}

/// Insert a new relation. Maps UNIQUE violation → AppError::Conflict.
pub async fn insert(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    from_ticket_id: &str,
    to_ticket_id: &str,
    relation_type: &str,
) -> Result<RelationRow> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO relations (id, user_id, from_ticket_id, to_ticket_id, \"type\", created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )
    .bind(id)
    .bind(user_id)
    .bind(from_ticket_id)
    .bind(to_ticket_id)
    .bind(relation_type)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
            AppError::Conflict(format!("relation ({from_ticket_id} → {to_ticket_id}: {relation_type}) already exists"))
        }
        _ => AppError::Db(e),
    })?;

    Ok(RelationRow {
        id: id.to_string(),
        user_id: user_id.to_string(),
        from_ticket_id: from_ticket_id.to_string(),
        to_ticket_id: to_ticket_id.to_string(),
        relation_type: relation_type.to_string(),
        created_at: now,
    })
}

/// Delete a relation by primary key. Idempotent — no error if not found.
pub async fn delete(pool: &SqlitePool, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM relations WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// List all relations where from_ticket_id = ticket_id OR to_ticket_id = ticket_id.
pub async fn list_for_ticket(
    pool: &SqlitePool,
    user_id: &str,
    ticket_id: &str,
) -> Result<Vec<RelationRow>> {
    let rows = sqlx::query(
        "SELECT id, user_id, from_ticket_id, to_ticket_id, \"type\" AS relation_type, created_at \
         FROM relations \
         WHERE user_id = ?1 AND (from_ticket_id = ?2 OR to_ticket_id = ?2) \
         ORDER BY created_at ASC",
    )
    .bind(user_id)
    .bind(ticket_id)
    .fetch_all(pool)
    .await?;
    rows.iter().map(RelationRow::from_row).collect()
}
