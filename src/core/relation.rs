//! RelationService — link tickets with typed directed relations.

use std::str::FromStr;

use serde::Serialize;
use uuid::Uuid;

use crate::core::ticket::RelationType;
use crate::error::{AppError, Result};
use crate::storage::{event_repo, relation_repo, ticket_repo, AppState};

/// View struct returned to callers.
#[derive(Debug, Clone, Serialize)]
pub struct Relation {
    pub id: String,
    pub from_identifier: String,
    pub to_identifier: String,
    pub relation_type: String,
    pub created_at: String,
}

/// Relation service. All methods are scoped by `user_id` for tenant isolation.
#[derive(Clone, Debug)]
pub struct RelationService {
    state: AppState,
}

impl RelationService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    /// Add a directed relation between two tickets.
    pub async fn add(
        &self,
        user_id: &str,
        from_identifier: &str,
        to_identifier: &str,
        relation_type_str: &str,
    ) -> Result<Relation> {
        let db = &self.state.db;

        // Validate relation type first
        let relation_type = RelationType::from_str(relation_type_str)?;

        // Resolve from ticket
        let from_ticket = ticket_repo::get_by_identifier(db, user_id, from_identifier)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("ticket identifier={from_identifier}")))?;

        // Resolve to ticket
        let to_ticket = ticket_repo::get_by_identifier(db, user_id, to_identifier)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("ticket identifier={to_identifier}")))?;

        let id = Uuid::now_v7().to_string();
        let row = relation_repo::insert(
            db,
            &id,
            user_id,
            &from_ticket.id,
            &to_ticket.id,
            relation_type.as_str(),
        )
        .await?;

        event_repo::append(db, user_id, "relation.add", Some(from_identifier), None).await?;

        Ok(Relation {
            id: row.id,
            from_identifier: from_identifier.to_string(),
            to_identifier: to_identifier.to_string(),
            relation_type: row.relation_type,
            created_at: row.created_at,
        })
    }

    /// Remove a relation by its ID. Idempotent — no error if already gone.
    pub async fn remove(&self, user_id: &str, relation_id: &str) -> Result<()> {
        let db = &self.state.db;

        relation_repo::delete(db, relation_id).await?;
        event_repo::append(db, user_id, "relation.remove", Some(relation_id), None).await?;

        Ok(())
    }
}
