use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::storage::{
    comment_repo, event_repo, project_repo, relation_repo, ticket_repo, AppState,
};

// ---------------------------------------------------------------------------
// Status
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Status {
    #[default]
    Backlog,
    Todo,
    InProgress,
    Review,
    Done,
    Cancelled,
}

impl Status {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Backlog => "backlog",
            Self::Todo => "todo",
            Self::InProgress => "in_progress",
            Self::Review => "review",
            Self::Done => "done",
            Self::Cancelled => "cancelled",
        }
    }
}

impl FromStr for Status {
    type Err = AppError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "backlog" => Ok(Self::Backlog),
            "todo" => Ok(Self::Todo),
            "in_progress" => Ok(Self::InProgress),
            "review" => Ok(Self::Review),
            "done" => Ok(Self::Done),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(AppError::Validation(format!("invalid status: {s:?}"))),
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for Status {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Status {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

// ---------------------------------------------------------------------------
// Priority
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Priority {
    Critical,
    High,
    #[default]
    Medium,
    Low,
}

impl Priority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }
}

impl FromStr for Priority {
    type Err = AppError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "critical" => Ok(Self::Critical),
            "high" => Ok(Self::High),
            "medium" => Ok(Self::Medium),
            "low" => Ok(Self::Low),
            _ => Err(AppError::Validation(format!("invalid priority: {s:?}"))),
        }
    }
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for Priority {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Priority {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

// ---------------------------------------------------------------------------
// RelationType
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelationType {
    Blocks,
    Duplicates,
    RelatesTo,
}

impl RelationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Blocks => "blocks",
            Self::Duplicates => "duplicates",
            Self::RelatesTo => "relates_to",
        }
    }
}

impl FromStr for RelationType {
    type Err = AppError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "blocks" => Ok(Self::Blocks),
            "duplicates" => Ok(Self::Duplicates),
            "relates_to" => Ok(Self::RelatesTo),
            _ => Err(AppError::Validation(format!(
                "invalid relation type: {s:?}; expected blocks | duplicates | relates_to"
            ))),
        }
    }
}

impl fmt::Display for RelationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for RelationType {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for RelationType {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

// ---------------------------------------------------------------------------
// View structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct Comment {
    pub id: String,
    pub author: Option<String>,
    pub body: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RelationView {
    pub id: String,
    pub from_identifier: String,
    pub to_identifier: String,
    pub relation_type: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Ticket {
    pub id: String,
    pub user_id: String,
    pub project_id: String,
    pub identifier: String,
    pub number: i64,
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

impl From<ticket_repo::TicketRow> for Ticket {
    fn from(r: ticket_repo::TicketRow) -> Self {
        Self {
            id: r.id,
            user_id: r.user_id,
            project_id: r.project_id,
            identifier: r.identifier,
            number: r.number,
            title: r.title,
            description: r.description,
            status: r.status,
            priority: r.priority,
            assignee: r.assignee,
            parent_id: r.parent_id,
            cycle_id: r.cycle_id,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TicketView {
    #[serde(flatten)]
    pub ticket: Ticket,
    pub comments: Vec<Comment>,
    pub relations: Vec<RelationView>,
}

// ---------------------------------------------------------------------------
// Input structs
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct CreateTicket {
    pub project_key: String,
    pub title: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub assignee: Option<String>,
    pub parent_identifier: Option<String>,
    pub cycle_id: Option<String>,
}

#[derive(Debug, Default)]
pub struct UpdateTicket {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub assignee: Option<String>,
    pub parent_identifier: Option<String>,
    pub cycle_id: Option<String>,
}

#[derive(Debug, Default)]
pub struct TicketServiceFilter {
    pub project_key: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub assignee: Option<String>,
    pub cycle_id: Option<String>,
    pub parent_identifier: Option<String>,
    pub limit: Option<i64>,
}

// ---------------------------------------------------------------------------
// TicketService
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct TicketService {
    state: AppState,
}

impl TicketService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    pub async fn create(&self, user_id: &str, input: CreateTicket) -> Result<Ticket> {
        let db = &self.state.db;

        // 1. Get project by key
        let key = input.project_key.to_uppercase();
        let project = project_repo::get_by_key(db, user_id, &key)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("project key={key}")))?;

        // 2. Validate status and priority
        let status = input
            .status
            .as_deref()
            .map(|s| s.parse::<Status>())
            .transpose()?
            .unwrap_or_default();
        let priority = input
            .priority
            .as_deref()
            .map(|p| p.parse::<Priority>())
            .transpose()?
            .unwrap_or_default();

        // 3. Resolve parent_identifier → parent_id
        let parent_id = if let Some(ref pi) = input.parent_identifier {
            let row = ticket_repo::get_by_identifier(db, user_id, pi)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("parent ticket identifier={pi}")))?;
            Some(row.id)
        } else {
            None
        };

        // 4. Get next ticket number
        let number = project_repo::next_ticket_number(db, &project.id).await?;

        // 5. Build identifier
        let identifier = format!("{}-{}", project.key, number);

        // 6. Insert ticket
        let id = Uuid::now_v7().to_string();
        let row = ticket_repo::insert(
            db,
            &id,
            user_id,
            &project.id,
            number,
            &identifier,
            &input.title,
            input.description.as_deref(),
            status.as_str(),
            priority.as_str(),
            input.assignee.as_deref(),
            parent_id.as_deref(),
            input.cycle_id.as_deref(),
        )
        .await?;

        // 7. Append event
        event_repo::append(db, user_id, "ticket.create", Some(&identifier), None).await?;

        Ok(Ticket::from(row))
    }

    pub async fn get(&self, user_id: &str, identifier: &str) -> Result<TicketView> {
        let db = &self.state.db;

        let row = ticket_repo::get_by_identifier(db, user_id, identifier)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("ticket identifier={identifier}")))?;

        let ticket = Ticket::from(row.clone());

        // List comments
        let comment_rows = comment_repo::list_for_ticket(db, user_id, &row.id).await?;
        let comments = comment_rows
            .into_iter()
            .map(|c| Comment {
                id: c.id,
                author: c.author,
                body: c.body,
                created_at: c.created_at,
            })
            .collect();

        // List relations and resolve identifiers
        let relation_rows = relation_repo::list_for_ticket(db, user_id, &row.id).await?;
        let mut relations = Vec::new();
        for r in relation_rows {
            let from = ticket_repo::get_by_id(db, &r.from_ticket_id).await?;
            let to = ticket_repo::get_by_id(db, &r.to_ticket_id).await?;
            if let (Some(from_t), Some(to_t)) = (from, to) {
                relations.push(RelationView {
                    id: r.id,
                    from_identifier: from_t.identifier,
                    to_identifier: to_t.identifier,
                    relation_type: r.relation_type,
                });
            }
        }

        Ok(TicketView {
            ticket,
            comments,
            relations,
        })
    }

    pub async fn list(&self, user_id: &str, filter: TicketServiceFilter) -> Result<Vec<Ticket>> {
        let db = &self.state.db;

        // Resolve project_key to project_id if provided
        let project_id = if let Some(ref key) = filter.project_key {
            let k = key.to_uppercase();
            let proj = project_repo::get_by_key(db, user_id, &k)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("project key={k}")))?;
            Some(proj.id)
        } else {
            None
        };

        // Resolve parent_identifier if provided
        let parent_id = if let Some(ref pi) = filter.parent_identifier {
            let row = ticket_repo::get_by_identifier(db, user_id, pi)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("parent ticket identifier={pi}")))?;
            Some(row.id)
        } else {
            None
        };

        let repo_filter = ticket_repo::TicketFilter {
            project_id,
            status: filter.status,
            priority: filter.priority,
            assignee: filter.assignee,
            cycle_id: filter.cycle_id,
            parent_id,
            limit: filter.limit,
        };

        let rows = ticket_repo::list(db, user_id, &repo_filter).await?;
        Ok(rows.into_iter().map(Ticket::from).collect())
    }

    pub async fn update(
        &self,
        user_id: &str,
        identifier: &str,
        patch: UpdateTicket,
    ) -> Result<Ticket> {
        let db = &self.state.db;

        // Verify ticket exists
        let row = ticket_repo::get_by_identifier(db, user_id, identifier)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("ticket identifier={identifier}")))?;

        // Validate status/priority if provided
        if let Some(ref s) = patch.status {
            s.parse::<Status>()?;
        }
        if let Some(ref p) = patch.priority {
            p.parse::<Priority>()?;
        }

        // Resolve parent_identifier if provided
        let parent_id = if let Some(ref pi) = patch.parent_identifier {
            let parent = ticket_repo::get_by_identifier(db, user_id, pi)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("parent ticket identifier={pi}")))?;
            Some(parent.id)
        } else {
            None
        };

        let repo_patch = ticket_repo::TicketPatch {
            title: patch.title,
            description: patch.description,
            status: patch.status,
            priority: patch.priority,
            assignee: patch.assignee,
            parent_id,
            cycle_id: patch.cycle_id,
        };

        ticket_repo::update(db, &row.id, &repo_patch).await?;

        event_repo::append(db, user_id, "ticket.update", Some(identifier), None).await?;

        // Fetch and return fresh data
        let updated = ticket_repo::get_by_id(db, &row.id)
            .await?
            .ok_or_else(|| AppError::Internal("updated ticket not found".into()))?;

        Ok(Ticket::from(updated))
    }

    pub async fn delete(&self, user_id: &str, identifier: &str) -> Result<()> {
        let db = &self.state.db;

        // Idempotent: if already gone, return Ok
        let row = ticket_repo::get_by_identifier(db, user_id, identifier).await?;
        let Some(row) = row else {
            return Ok(());
        };

        ticket_repo::delete(db, &row.id).await?;
        event_repo::append(db, user_id, "ticket.delete", Some(identifier), None).await?;

        Ok(())
    }
}
