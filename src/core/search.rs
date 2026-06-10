//! FTS5-backed full-text search over tickets.

use serde::Serialize;
use sqlx::Row;

use crate::error::Result;
use crate::storage::AppState;

/// A single search hit.
#[derive(Debug, Clone, Serialize)]
pub struct SearchHit {
    pub identifier: String,
    pub title: String,
    pub snippet: String,
    pub rank: f64,
}

/// Search service scoped by user_id for tenant isolation.
#[derive(Debug)]
pub struct SearchService {
    state: AppState,
}

impl SearchService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    /// Run a FTS5 full-text search against `tickets_fts` for a single user.
    ///
    /// Results are ordered by BM25 rank ascending (more relevant = lower BM25
    /// value in SQLite). Defaults to 20 results when `limit` is `None`.
    pub async fn search(
        &self,
        user_id: &str,
        query: &str,
        limit: Option<i64>,
    ) -> Result<Vec<SearchHit>> {
        let q = query.trim();
        if q.is_empty() {
            return Ok(Vec::new());
        }

        // Build an FTS5 query expression. If the caller already uses FTS5
        // operators, pass verbatim; otherwise OR-join quoted tokens so that
        // any single term matches (BM25 still ranks multi-term hits higher).
        let fts_query =
            if q.contains('"') || q.contains(':') || q.contains(" AND ") || q.contains(" OR ") {
                q.to_string()
            } else {
                let terms: Vec<String> = q
                    .split_whitespace()
                    .map(|t| format!("\"{}\"", t.replace('"', "\"\"")))
                    .collect();
                terms.join(" OR ")
            };

        let limit = limit.unwrap_or(20).clamp(1, 1000);

        let rows = sqlx::query(
            r#"
            SELECT t.identifier AS identifier,
                   t.title      AS title,
                   snippet(tickets_fts, 0, '<b>', '</b>', '...', 10) AS snippet,
                   bm25(tickets_fts) AS rank
            FROM tickets_fts
            JOIN tickets t ON t.rowid = tickets_fts.rowid
            WHERE tickets_fts MATCH ?1
              AND t.user_id = ?2
            ORDER BY rank
            LIMIT ?3
            "#,
        )
        .bind(&fts_query)
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.state.db)
        .await?;

        let mut hits = Vec::with_capacity(rows.len());
        for r in &rows {
            hits.push(SearchHit {
                identifier: r.try_get("identifier")?,
                title: r.try_get("title")?,
                snippet: r.try_get("snippet")?,
                rank: r.try_get("rank")?,
            });
        }
        Ok(hits)
    }
}
