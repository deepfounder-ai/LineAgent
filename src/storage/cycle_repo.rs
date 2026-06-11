//! `cycles` table access.

use chrono::Utc;
use sqlx::{Row, SqlitePool};

use crate::error::{AppError, Result};

/// Row representation of the `cycles` table.
#[derive(Debug, Clone)]
pub struct CycleRow {
    pub id: String,
    pub user_id: String,
    pub project_id: String,
    pub number: i64,
    pub name: String,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl CycleRow {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self> {
        Ok(Self {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            project_id: row.try_get("project_id")?,
            number: row.try_get("number")?,
            name: row.try_get("name")?,
            starts_at: row.try_get("starts_at")?,
            ends_at: row.try_get("ends_at")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

/// Insert a new cycle and return the inserted row.
#[allow(clippy::too_many_arguments)]
pub async fn insert(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    project_id: &str,
    number: i64,
    name: &str,
    starts_at: Option<&str>,
    ends_at: Option<&str>,
) -> Result<CycleRow> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO cycles (id, user_id, project_id, number, name, starts_at, ends_at, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
    )
    .bind(id)
    .bind(user_id)
    .bind(project_id)
    .bind(number)
    .bind(name)
    .bind(starts_at)
    .bind(ends_at)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(AppError::Db)?;

    Ok(CycleRow {
        id: id.to_string(),
        user_id: user_id.to_string(),
        project_id: project_id.to_string(),
        number,
        name: name.to_string(),
        starts_at: starts_at.map(str::to_string),
        ends_at: ends_at.map(str::to_string),
        created_at: now.clone(),
        updated_at: now,
    })
}

/// Fetch a cycle by its primary key.
///
/// Note: does not filter by user_id — caller must verify ownership at the service layer.
pub async fn get_by_id(pool: &SqlitePool, id: &str) -> Result<Option<CycleRow>> {
    let row = sqlx::query(
        "SELECT id, user_id, project_id, number, name, starts_at, ends_at, created_at, updated_at \
         FROM cycles WHERE id = ?1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    match row {
        Some(r) => Ok(Some(CycleRow::from_row(&r)?)),
        None => Ok(None),
    }
}

/// List all cycles for a project, ordered by number ASC.
pub async fn list_for_project(
    pool: &SqlitePool,
    user_id: &str,
    project_id: &str,
) -> Result<Vec<CycleRow>> {
    let rows = sqlx::query(
        "SELECT id, user_id, project_id, number, name, starts_at, ends_at, created_at, updated_at \
         FROM cycles WHERE user_id = ?1 AND project_id = ?2 ORDER BY number ASC",
    )
    .bind(user_id)
    .bind(project_id)
    .fetch_all(pool)
    .await?;
    rows.iter().map(CycleRow::from_row).collect()
}

/// Update a cycle's mutable fields. All parameters default to their current values
/// when `None` is passed (COALESCE semantics).
///
/// **Limitation:** `starts_at` and `ends_at` cannot be cleared to `NULL` via this
/// function — passing `None` preserves the existing value. Use a dedicated clear
/// function if that is needed.
///
/// Returns `NotFound` if no row was updated.
pub async fn update(
    pool: &SqlitePool,
    id: &str,
    name: Option<&str>,
    starts_at: Option<&str>,
    ends_at: Option<&str>,
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    let result = sqlx::query(
        "UPDATE cycles SET \
         name = COALESCE(?2, name), \
         starts_at = COALESCE(?3, starts_at), \
         ends_at = COALESCE(?4, ends_at), \
         updated_at = ?5 \
         WHERE id = ?1",
    )
    .bind(id)
    .bind(name)
    .bind(starts_at)
    .bind(ends_at)
    .bind(&now)
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("cycle id={id}")));
    }
    Ok(())
}
