//! Database connection pool + application state.

pub mod api_key_repo;
pub mod event_repo;
pub mod project_repo;   // stub — just declare, file doesn't exist yet
pub mod ticket_repo;    // stub
pub mod comment_repo;   // stub
pub mod relation_repo;  // stub
pub mod cycle_repo;     // stub
pub mod pool;
pub mod user_repo;

pub use pool::{init_pool, AppState};
