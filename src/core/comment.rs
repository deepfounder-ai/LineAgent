//! CommentService — add and list comments on tickets.

use serde::Serialize;
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::storage::{comment_repo, event_repo, ticket_repo, AppState};

/// View struct returned to callers.
#[derive(Debug, Clone, Serialize)]
pub struct Comment {
    pub id: String,
    pub ticket_id: String,
    pub author: Option<String>,
    pub body: String,
    pub created_at: String,
}

/// Comment service. All methods are scoped by `user_id` for tenant isolation.
#[derive(Clone, Debug)]
pub struct CommentService {
    state: AppState,
}

impl CommentService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    /// Add a comment to a ticket identified by its human-readable identifier (e.g. "LIN-1").
    pub async fn add(
        &self,
        user_id: &str,
        ticket_identifier: &str,
        author: Option<&str>,
        body: &str,
    ) -> Result<Comment> {
        let db = &self.state.db;

        let ticket = ticket_repo::get_by_identifier(db, user_id, ticket_identifier)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("ticket identifier={ticket_identifier}")))?;

        let id = Uuid::now_v7().to_string();
        let row = comment_repo::insert(db, &id, user_id, &ticket.id, author, body).await?;

        event_repo::append(db, user_id, "comment.add", Some(ticket_identifier), None).await?;

        Ok(Comment {
            id: row.id,
            ticket_id: row.ticket_id,
            author: row.author,
            body: row.body,
            created_at: row.created_at,
        })
    }

    /// List all comments for a ticket.
    pub async fn list(&self, user_id: &str, ticket_identifier: &str) -> Result<Vec<Comment>> {
        let db = &self.state.db;

        let ticket = ticket_repo::get_by_identifier(db, user_id, ticket_identifier)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("ticket identifier={ticket_identifier}")))?;

        let rows = comment_repo::list_for_ticket(db, user_id, &ticket.id).await?;
        Ok(rows
            .into_iter()
            .map(|r| Comment {
                id: r.id,
                ticket_id: r.ticket_id,
                author: r.author,
                body: r.body,
                created_at: r.created_at,
            })
            .collect())
    }
}
