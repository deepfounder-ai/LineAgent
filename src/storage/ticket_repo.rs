//! `tickets` table access.

use chrono::Utc;
use sqlx::{Row, SqlitePool};

use crate::error::{AppError, Result};

/// Row representation of the `tickets` table.
#[derive(Debug, Clone)]
pub struct TicketRow {
    pub id: String,
    pub user_id: String,
    pub project_id: String,
    pub number: i64,
    pub identifier: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub assignee: Option<String>,
    pub parent_id: Option<String>,
    pub cycle_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl TicketRow {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self> {
        Ok(Self {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            project_id: row.try_get("project_id")?,
            number: row.try_get("number")?,
            identifier: row.try_get("identifier")?,
            title: row.try_get("title")?,
            description: row.try_get("description")?,
            status: row.try_get("status")?,
            priority: row.try_get("priority")?,
            assignee: row.try_get("assignee")?,
            parent_id: row.try_get("parent_id")?,
            cycle_id: row.try_get("cycle_id")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

/// Optional filters for `list`.
#[derive(Debug, Default)]
pub struct TicketFilter {
    pub project_id: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub assignee: Option<String>,
    pub cycle_id: Option<String>,
    pub parent_id: Option<String>,
    pub limit: Option<i64>,
}

/// Fields that can be patched via `update`.
#[derive(Debug, Default)]
pub struct TicketPatch {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub assignee: Option<String>,
    pub parent_id: Option<String>,
    pub cycle_id: Option<String>,
}

/// Insert a new ticket row and return it.
pub async fn insert(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    project_id: &str,
    number: i64,
    identifier: &str,
    title: &str,
    description: Option<&str>,
    status: &str,
    priority: &str,
    assignee: Option<&str>,
    parent_id: Option<&str>,
    cycle_id: Option<&str>,
) -> Result<TicketRow> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO tickets \
         (id, user_id, project_id, number, identifier, title, description, status, priority, \
          assignee, parent_id, cycle_id, created_at, updated_at) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
    )
    .bind(id)
    .bind(user_id)
    .bind(project_id)
    .bind(number)
    .bind(identifier)
    .bind(title)
    .bind(description)
    .bind(status)
    .bind(priority)
    .bind(assignee)
    .bind(parent_id)
    .bind(cycle_id)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(TicketRow {
        id: id.to_string(),
        user_id: user_id.to_string(),
        project_id: project_id.to_string(),
        number,
        identifier: identifier.to_string(),
        title: title.to_string(),
        description: description.map(str::to_string),
        status: status.to_string(),
        priority: priority.to_string(),
        assignee: assignee.map(str::to_string),
        parent_id: parent_id.map(str::to_string),
        cycle_id: cycle_id.map(str::to_string),
        created_at: now.clone(),
        updated_at: now,
    })
}

/// Fetch a ticket by primary key.
pub async fn get_by_id(pool: &SqlitePool, id: &str) -> Result<Option<TicketRow>> {
    let row = sqlx::query(
        "SELECT id,user_id,project_id,number,identifier,title,description,status,priority,\
         assignee,parent_id,cycle_id,created_at,updated_at FROM tickets WHERE id=?1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    match row {
        Some(r) => Ok(Some(TicketRow::from_row(&r)?)),
        None => Ok(None),
    }
}

/// Fetch a ticket by (user_id, identifier).
pub async fn get_by_identifier(
    pool: &SqlitePool,
    user_id: &str,
    identifier: &str,
) -> Result<Option<TicketRow>> {
    let row = sqlx::query(
        "SELECT id,user_id,project_id,number,identifier,title,description,status,priority,\
         assignee,parent_id,cycle_id,created_at,updated_at FROM tickets \
         WHERE user_id=?1 AND identifier=?2",
    )
    .bind(user_id)
    .bind(identifier)
    .fetch_optional(pool)
    .await?;
    match row {
        Some(r) => Ok(Some(TicketRow::from_row(&r)?)),
        None => Ok(None),
    }
}

/// List tickets for a user, with optional filters. Max 100 results by default.
///
/// Dynamic filtering is done by building the SQL string manually and binding
/// parameters positionally; this avoids the sqlx compile-time macro restrictions.
pub async fn list(pool: &SqlitePool, user_id: &str, filter: &TicketFilter) -> Result<Vec<TicketRow>> {
    let limit = filter.limit.unwrap_or(100);

    let mut sql = String::from(
        "SELECT id,user_id,project_id,number,identifier,title,description,status,priority,\
         assignee,parent_id,cycle_id,created_at,updated_at FROM tickets WHERE user_id=?1",
    );
    let mut idx: i32 = 2;
    let mut params: Vec<String> = Vec::new();

    if let Some(ref v) = filter.project_id {
        sql.push_str(&format!(" AND project_id=?{idx}"));
        idx += 1;
        params.push(v.clone());
    }
    if let Some(ref v) = filter.status {
        sql.push_str(&format!(" AND status=?{idx}"));
        idx += 1;
        params.push(v.clone());
    }
    if let Some(ref v) = filter.priority {
        sql.push_str(&format!(" AND priority=?{idx}"));
        idx += 1;
        params.push(v.clone());
    }
    if let Some(ref v) = filter.assignee {
        sql.push_str(&format!(" AND assignee=?{idx}"));
        idx += 1;
        params.push(v.clone());
    }
    if let Some(ref v) = filter.cycle_id {
        sql.push_str(&format!(" AND cycle_id=?{idx}"));
        idx += 1;
        params.push(v.clone());
    }
    if let Some(ref v) = filter.parent_id {
        sql.push_str(&format!(" AND parent_id=?{idx}"));
        idx += 1;
        params.push(v.clone());
    }
    sql.push_str(&format!(" ORDER BY updated_at DESC LIMIT ?{idx}"));

    let mut q = sqlx::query(&sql).bind(user_id);
    for p in &params {
        q = q.bind(p.as_str());
    }
    q = q.bind(limit);

    let rows = q.fetch_all(pool).await?;
    rows.iter().map(TicketRow::from_row).collect()
}

/// Update mutable fields of a ticket. `updated_at` is refreshed.
/// Passing `None` for a field keeps the existing value (via COALESCE).
pub async fn update(pool: &SqlitePool, id: &str, patch: &TicketPatch) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    let res = sqlx::query(
        "UPDATE tickets SET \
         title=COALESCE(?2,title), description=COALESCE(?3,description), \
         status=COALESCE(?4,status), priority=COALESCE(?5,priority), \
         assignee=COALESCE(?6,assignee), parent_id=COALESCE(?7,parent_id), \
         cycle_id=COALESCE(?8,cycle_id), updated_at=?9 WHERE id=?1",
    )
    .bind(id)
    .bind(patch.title.as_deref())
    .bind(patch.description.as_deref())
    .bind(patch.status.as_deref())
    .bind(patch.priority.as_deref())
    .bind(patch.assignee.as_deref())
    .bind(patch.parent_id.as_deref())
    .bind(patch.cycle_id.as_deref())
    .bind(&now)
    .execute(pool)
    .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("ticket id={id}")));
    }
    Ok(())
}

/// Delete a ticket by primary key. Idempotent — no error if not found.
pub async fn delete(pool: &SqlitePool, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM tickets WHERE id=?1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
