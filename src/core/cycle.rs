//! CycleService — sprint/cycle management per project.

use serde::Serialize;
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::storage::{cycle_repo, event_repo, project_repo, AppState};

/// View struct returned to callers.
#[derive(Debug, Clone, Serialize)]
pub struct Cycle {
    pub id: String,
    pub project_id: String,
    pub number: i64,
    pub name: String,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<cycle_repo::CycleRow> for Cycle {
    fn from(r: cycle_repo::CycleRow) -> Self {
        Self {
            id: r.id,
            project_id: r.project_id,
            number: r.number,
            name: r.name,
            starts_at: r.starts_at,
            ends_at: r.ends_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

/// Cycle service. All methods are scoped by `user_id` for tenant isolation.
#[derive(Clone, Debug)]
pub struct CycleService {
    state: AppState,
}

impl CycleService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    /// Create a new cycle in the given project.
    pub async fn create(
        &self,
        user_id: &str,
        project_key: &str,
        name: &str,
        starts_at: Option<&str>,
        ends_at: Option<&str>,
    ) -> Result<Cycle> {
        let db = &self.state.db;
        let key = project_key.to_uppercase();

        let project = project_repo::get_by_key(db, user_id, &key)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("project key={key}")))?;

        let number = project_repo::next_cycle_number(db, &project.id).await?;
        let id = Uuid::now_v7().to_string();

        let row = cycle_repo::insert(db, &id, user_id, &project.id, number, name, starts_at, ends_at).await?;

        event_repo::append(db, user_id, "cycle.create", Some(&key), None).await?;

        Ok(Cycle::from(row))
    }

    /// List all cycles for a project.
    pub async fn list(&self, user_id: &str, project_key: &str) -> Result<Vec<Cycle>> {
        let db = &self.state.db;
        let key = project_key.to_uppercase();

        let project = project_repo::get_by_key(db, user_id, &key)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("project key={key}")))?;

        let rows = cycle_repo::list_for_project(db, user_id, &project.id).await?;
        Ok(rows.into_iter().map(Cycle::from).collect())
    }

    /// Update a cycle's mutable fields.
    pub async fn update(
        &self,
        user_id: &str,
        cycle_id: &str,
        name: Option<&str>,
        starts_at: Option<&str>,
        ends_at: Option<&str>,
    ) -> Result<Cycle> {
        let db = &self.state.db;

        cycle_repo::update(db, cycle_id, name, starts_at, ends_at).await?;

        event_repo::append(db, user_id, "cycle.update", Some(cycle_id), None).await?;

        let updated = cycle_repo::get_by_id(db, cycle_id)
            .await?
            .ok_or_else(|| AppError::Internal("updated cycle not found".into()))?;

        Ok(Cycle::from(updated))
    }
}
