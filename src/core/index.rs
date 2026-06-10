//! Index service — per-project ticket status counts for dashboard views.

use serde::Serialize;
use sqlx::Row;

use crate::error::Result;
use crate::storage::{project_repo, AppState};

/// Ticket counts broken down by status for a single project.
#[derive(Debug, Clone, Serialize, Default)]
pub struct StatusCounts {
    pub backlog: i64,
    pub todo: i64,
    pub in_progress: i64,
    pub review: i64,
    pub done: i64,
    pub cancelled: i64,
    pub total: i64,
}

/// Summary of a project with its ticket status counts.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectIndex {
    pub key: String,
    pub name: String,
    pub counts: StatusCounts,
}

/// Index service. All methods are scoped by `user_id` for tenant isolation.
#[derive(Debug)]
pub struct IndexService {
    state: AppState,
}

impl IndexService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    /// Build an index of all projects for a user, each annotated with ticket
    /// status counts. Projects are returned in `created_at ASC` order
    /// (matching `project_repo::list_for_user`).
    pub async fn build(&self, user_id: &str) -> Result<Vec<ProjectIndex>> {
        let projects = project_repo::list_for_user(&self.state.db, user_id).await?;

        let mut result = Vec::with_capacity(projects.len());

        for proj in &projects {
            let rows = sqlx::query(
                "SELECT status, COUNT(*) AS cnt \
                 FROM tickets \
                 WHERE user_id = ?1 AND project_id = ?2 \
                 GROUP BY status",
            )
            .bind(user_id)
            .bind(&proj.id)
            .fetch_all(&self.state.db)
            .await?;

            let mut counts = StatusCounts::default();
            for row in &rows {
                let status: String = row.try_get("status")?;
                let cnt: i64 = row.try_get("cnt")?;
                match status.as_str() {
                    "backlog" => counts.backlog += cnt,
                    "todo" => counts.todo += cnt,
                    "in_progress" => counts.in_progress += cnt,
                    "review" => counts.review += cnt,
                    "done" => counts.done += cnt,
                    "cancelled" => counts.cancelled += cnt,
                    _ => {}
                }
                counts.total += cnt;
            }

            result.push(ProjectIndex {
                key: proj.key.clone(),
                name: proj.name.clone(),
                counts,
            });
        }

        Ok(result)
    }
}
