//! ProjectService — CRUD entry points for projects.

use serde::Serialize;
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::storage::{event_repo, project_repo, AppState};

/// View struct returned to callers.
#[derive(Debug, Clone, Serialize)]
pub struct Project {
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

impl From<project_repo::ProjectRow> for Project {
    fn from(r: project_repo::ProjectRow) -> Self {
        Self {
            id: r.id,
            user_id: r.user_id,
            key: r.key,
            name: r.name,
            description: r.description,
            ticket_counter: r.ticket_counter,
            cycle_counter: r.cycle_counter,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

/// Project service. All methods are scoped by `user_id` for tenant isolation.
#[derive(Clone, Debug)]
pub struct ProjectService {
    state: AppState,
}

impl ProjectService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    /// Create a new project.
    ///
    /// - `key` is uppercased automatically.
    /// - Returns `AppError::Validation` if the key is empty or contains
    ///   non-alphanumeric characters.
    /// - Returns `AppError::Conflict` if the key already exists for this user.
    pub async fn create(
        &self,
        user_id: &str,
        key: &str,
        name: &str,
        description: Option<&str>,
    ) -> Result<Project> {
        let key = key.to_uppercase();
        validate_key(&key)?;

        let id = Uuid::now_v7().to_string();
        let row =
            project_repo::insert(&self.state.db, &id, user_id, &key, name, description).await?;

        event_repo::append(&self.state.db, user_id, "project.create", Some(&key), None).await?;

        Ok(Project::from(row))
    }

    /// Fetch a project by key for a given user.
    pub async fn get(&self, user_id: &str, key: &str) -> Result<Project> {
        let key = key.to_uppercase();
        project_repo::get_by_key(&self.state.db, user_id, &key)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("project key={key}")))
            .map(Project::from)
    }

    /// List all projects for a user.
    pub async fn list(&self, user_id: &str) -> Result<Vec<Project>> {
        let rows = project_repo::list_for_user(&self.state.db, user_id).await?;
        Ok(rows.into_iter().map(Project::from).collect())
    }

    /// Update name and/or description of a project.
    ///
    /// Returns `AppError::NotFound` if the project does not exist.
    pub async fn update(
        &self,
        user_id: &str,
        key: &str,
        name: Option<&str>,
        description: Option<&str>,
    ) -> Result<Project> {
        let key = key.to_uppercase();
        // Verify the project exists and belongs to this user.
        let existing = project_repo::get_by_key(&self.state.db, user_id, &key)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("project key={key}")))?;

        project_repo::update(&self.state.db, &existing.id, name, description).await?;

        event_repo::append(&self.state.db, user_id, "project.update", Some(&key), None).await?;

        let updated = project_repo::get_by_id(&self.state.db, &existing.id)
            .await?
            .ok_or_else(|| AppError::Internal("updated project not found".into()))?;

        Ok(Project::from(updated))
    }
}

/// Validate a project key: must be non-empty and contain only `[A-Z0-9]`.
fn validate_key(key: &str) -> Result<()> {
    if key.is_empty() {
        return Err(AppError::Validation("project key must not be empty".into()));
    }
    if !key
        .chars()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
    {
        return Err(AppError::Validation(
            "project key may only contain [A-Z0-9]".into(),
        ));
    }
    Ok(())
}
