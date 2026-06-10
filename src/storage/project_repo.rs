//! `projects` table access.

use chrono::Utc;
use sqlx::{Row, SqlitePool};

use crate::error::{AppError, Result};

/// Row representation of the `projects` table.
#[derive(Debug, Clone)]
pub struct ProjectRow {
    pub id: String,
    pub user_id: String,
    pub key: String,
    pub name: String,
    pub description: Option<String>,
    pub ticket_counter: i64,
    pub cycle_counter: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl ProjectRow {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self> {
        Ok(Self {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            key: row.try_get("key")?,
            name: row.try_get("name")?,
            description: row.try_get("description")?,
            ticket_counter: row.try_get("ticket_counter")?,
            cycle_counter: row.try_get("cycle_counter")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

/// Insert a new project row. Returns the inserted row.
pub async fn insert(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    key: &str,
    name: &str,
    description: Option<&str>,
) -> Result<ProjectRow> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO projects \
         (id, user_id, key, name, description, ticket_counter, cycle_counter, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, 0, 0, ?6, ?7)",
    )
    .bind(id)
    .bind(user_id)
    .bind(key)
    .bind(name)
    .bind(description)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
            AppError::Conflict(format!("project key already exists: {key}"))
        }
        _ => AppError::Db(e),
    })?;

    get_by_id(pool, id)
        .await?
        .ok_or_else(|| AppError::Internal("inserted project not found".into()))
}

/// Fetch a project by primary key.
pub async fn get_by_id(pool: &SqlitePool, id: &str) -> Result<Option<ProjectRow>> {
    let row = sqlx::query(
        "SELECT id, user_id, key, name, description, ticket_counter, cycle_counter, created_at, updated_at \
         FROM projects WHERE id = ?1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    match row {
        Some(r) => Ok(Some(ProjectRow::from_row(&r)?)),
        None => Ok(None),
    }
}

/// Fetch a project by (user_id, key).
pub async fn get_by_key(pool: &SqlitePool, user_id: &str, key: &str) -> Result<Option<ProjectRow>> {
    let row = sqlx::query(
        "SELECT id, user_id, key, name, description, ticket_counter, cycle_counter, created_at, updated_at \
         FROM projects WHERE user_id = ?1 AND key = ?2",
    )
    .bind(user_id)
    .bind(key)
    .fetch_optional(pool)
    .await?;
    match row {
        Some(r) => Ok(Some(ProjectRow::from_row(&r)?)),
        None => Ok(None),
    }
}

/// List all projects for a user, ordered by created_at ascending.
pub async fn list_for_user(pool: &SqlitePool, user_id: &str) -> Result<Vec<ProjectRow>> {
    let rows = sqlx::query(
        "SELECT id, user_id, key, name, description, ticket_counter, cycle_counter, created_at, updated_at \
         FROM projects WHERE user_id = ?1 ORDER BY created_at ASC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    rows.iter().map(ProjectRow::from_row).collect()
}

/// Update name and/or description. `updated_at` is refreshed.
/// Passing `None` for a field keeps the existing value.
pub async fn update(
    pool: &SqlitePool,
    id: &str,
    name: Option<&str>,
    description: Option<&str>,
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    let result = sqlx::query(
        "UPDATE projects SET \
         name = COALESCE(?2, name), \
         description = COALESCE(?3, description), \
         updated_at = ?4 \
         WHERE id = ?1",
    )
    .bind(id)
    .bind(name)
    .bind(description)
    .bind(&now)
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("project id={id}")));
    }
    Ok(())
}

/// Atomically increment and return the next per-project ticket number.
pub async fn next_ticket_number(pool: &SqlitePool, project_id: &str) -> Result<i64> {
    let row = sqlx::query(
        "UPDATE projects SET ticket_counter = ticket_counter + 1, updated_at = ?2 \
         WHERE id = ?1 RETURNING ticket_counter",
    )
    .bind(project_id)
    .bind(Utc::now().to_rfc3339())
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("project id={project_id}")))?;
    Ok(row.try_get("ticket_counter")?)
}

/// Atomically increment and return the next per-project cycle number.
pub async fn next_cycle_number(pool: &SqlitePool, project_id: &str) -> Result<i64> {
    let row = sqlx::query(
        "UPDATE projects SET cycle_counter = cycle_counter + 1, updated_at = ?2 \
         WHERE id = ?1 RETURNING cycle_counter",
    )
    .bind(project_id)
    .bind(Utc::now().to_rfc3339())
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("project id={project_id}")))?;
    Ok(row.try_get("cycle_counter")?)
}
