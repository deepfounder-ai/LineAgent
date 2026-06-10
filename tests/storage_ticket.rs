use lineagent::config::Config;
use lineagent::storage::init_pool;
use lineagent::storage::project_repo;
use lineagent::storage::ticket_repo::{self, TicketFilter, TicketPatch};

async fn setup() -> sqlx::SqlitePool {
    let mut cfg = Config::for_test(std::path::PathBuf::from("/tmp"));
    cfg.db_url = "sqlite::memory:".to_string();
    let pool = init_pool(cfg).await.unwrap().db;

    // Seed user row (tickets.user_id → users.id).
    sqlx::query(
        "INSERT OR IGNORE INTO users (id, username, password_hash, created_at) \
         VALUES ('user1', 'user1', '', '2024-01-01T00:00:00+00:00')",
    )
    .execute(&pool)
    .await
    .unwrap();

    // Seed project row (tickets.project_id → projects.id).
    sqlx::query(
        "INSERT OR IGNORE INTO projects \
         (id, user_id, key, name, ticket_counter, cycle_counter, created_at, updated_at) \
         VALUES ('proj1', 'user1', 'LIN', 'LineAgent', 0, 0, \
                 '2024-01-01T00:00:00+00:00', '2024-01-01T00:00:00+00:00')",
    )
    .execute(&pool)
    .await
    .unwrap();

    pool
}

async fn insert_test_ticket(
    pool: &sqlx::SqlitePool,
    identifier: &str,
    title: &str,
) -> (project_repo::ProjectRow, ticket_repo::TicketRow) {
    let pid = uuid::Uuid::now_v7().to_string();
    // Use a unique key derived from the project id to avoid conflicts with the
    // "LIN" key already seeded by setup() or prior calls in the same pool.
    let key = format!("T{}", &pid[..8].to_uppercase().replace('-', ""));
    let proj = project_repo::insert(pool, &pid, "user1", &key, "LineAgent", None)
        .await
        .unwrap();
    let tid = uuid::Uuid::now_v7().to_string();
    let num: i64 = identifier.split('-').last().unwrap().parse().unwrap();
    let ticket = ticket_repo::insert(
        pool, &tid, "user1", &pid, num, identifier, title, None, "backlog", "medium", None, None,
        None,
    )
    .await
    .unwrap();
    (proj, ticket)
}

#[tokio::test]
async fn insert_and_get_by_identifier() {
    let pool = setup().await;
    let id = uuid::Uuid::now_v7().to_string();

    ticket_repo::insert(
        &pool, &id, "user1", "proj1", 1, "LIN-1", "Fix the bug",
        None, "backlog", "medium", None, None, None,
    )
    .await
    .unwrap();

    let ticket = ticket_repo::get_by_identifier(&pool, "user1", "LIN-1")
        .await
        .unwrap()
        .expect("ticket should be found");

    assert_eq!(ticket.title, "Fix the bug");
    assert_eq!(ticket.identifier, "LIN-1");
    assert_eq!(ticket.id, id);
}

#[tokio::test]
async fn list_with_status_filter() {
    let pool = setup().await;
    let id1 = uuid::Uuid::now_v7().to_string();
    let id2 = uuid::Uuid::now_v7().to_string();

    ticket_repo::insert(
        &pool, &id1, "user1", "proj1", 1, "LIN-1", "Backlog ticket",
        None, "backlog", "medium", None, None, None,
    )
    .await
    .unwrap();

    ticket_repo::insert(
        &pool, &id2, "user1", "proj1", 2, "LIN-2", "Done ticket",
        None, "done", "medium", None, None, None,
    )
    .await
    .unwrap();

    let filter = TicketFilter {
        status: Some("backlog".to_string()),
        ..Default::default()
    };
    let tickets = ticket_repo::list(&pool, "user1", &filter).await.unwrap();

    assert_eq!(tickets.len(), 1);
    assert_eq!(tickets[0].status, "backlog");
    assert_eq!(tickets[0].identifier, "LIN-1");
}

#[tokio::test]
async fn update_ticket() {
    let pool = setup().await;
    let id = uuid::Uuid::now_v7().to_string();

    ticket_repo::insert(
        &pool, &id, "user1", "proj1", 1, "LIN-1", "Original title",
        None, "backlog", "medium", None, None, None,
    )
    .await
    .unwrap();

    let patch = TicketPatch {
        title: Some("Updated title".to_string()),
        ..Default::default()
    };
    ticket_repo::update(&pool, &id, &patch).await.unwrap();

    let ticket = ticket_repo::get_by_id(&pool, &id)
        .await
        .unwrap()
        .expect("ticket should exist");

    assert_eq!(ticket.title, "Updated title");
    assert_eq!(ticket.status, "backlog"); // unchanged via COALESCE
}

#[tokio::test]
async fn delete_ticket() {
    let pool = setup().await;
    let id = uuid::Uuid::now_v7().to_string();

    ticket_repo::insert(
        &pool, &id, "user1", "proj1", 1, "LIN-1", "To be deleted",
        None, "backlog", "medium", None, None, None,
    )
    .await
    .unwrap();

    ticket_repo::delete(&pool, &id).await.unwrap();

    let result = ticket_repo::get_by_identifier(&pool, "user1", "LIN-1")
        .await
        .unwrap();
    assert!(result.is_none(), "ticket should be deleted");
}

#[tokio::test]
async fn get_by_id_returns_none_for_missing() {
    let pool = setup().await;
    let result = ticket_repo::get_by_id(&pool, "nonexistent-id")
        .await
        .unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn update_returns_not_found_for_missing_id() {
    let pool = setup().await;
    let patch = ticket_repo::TicketPatch {
        title: Some("new".to_string()),
        ..Default::default()
    };
    let err = ticket_repo::update(&pool, "nonexistent-id", &patch)
        .await
        .unwrap_err();
    assert!(
        matches!(err, lineagent::error::AppError::NotFound(_)),
        "expected NotFound, got {:?}",
        err
    );
}

#[tokio::test]
async fn delete_is_idempotent() {
    let pool = setup().await;
    let (_, t) = insert_test_ticket(&pool, "LIN-1", "Ticket").await;
    ticket_repo::delete(&pool, &t.id).await.unwrap();
    // second delete should also succeed
    ticket_repo::delete(&pool, &t.id).await.unwrap();
}

#[tokio::test]
async fn insert_duplicate_identifier_returns_conflict() {
    let pool = setup().await;
    let (p, _) = insert_test_ticket(&pool, "LIN-1", "First").await;
    let id2 = uuid::Uuid::now_v7().to_string();
    let err = ticket_repo::insert(
        &pool, &id2, "user1", &p.id, 2, "LIN-1", "Second", None, "backlog", "medium", None, None,
        None,
    )
    .await
    .unwrap_err();
    assert!(
        matches!(err, lineagent::error::AppError::Conflict(_)),
        "expected Conflict, got {:?}",
        err
    );
}

#[tokio::test]
async fn list_with_multi_filter() {
    let pool = setup().await;
    let (p, _) = insert_test_ticket(&pool, "LIN-1", "Ticket A").await;
    let id2 = uuid::Uuid::now_v7().to_string();
    ticket_repo::insert(
        &pool, &id2, "user1", &p.id, 2, "LIN-2", "Ticket B", None, "done", "high", None, None,
        None,
    )
    .await
    .unwrap();
    let filter = ticket_repo::TicketFilter {
        status: Some("done".to_string()),
        priority: Some("high".to_string()),
        ..Default::default()
    };
    let list = ticket_repo::list(&pool, "user1", &filter).await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].identifier, "LIN-2");
}

#[tokio::test]
async fn list_respects_limit() {
    let pool = setup().await;
    let (p, _) = insert_test_ticket(&pool, "LIN-1", "T1").await;
    for i in 2..=5i64 {
        let id = uuid::Uuid::now_v7().to_string();
        ticket_repo::insert(
            &pool,
            &id,
            "user1",
            &p.id,
            i,
            &format!("LIN-{i}"),
            &format!("T{i}"),
            None,
            "backlog",
            "medium",
            None,
            None,
            None,
        )
        .await
        .unwrap();
    }
    let filter = ticket_repo::TicketFilter {
        limit: Some(2),
        ..Default::default()
    };
    let list = ticket_repo::list(&pool, "user1", &filter).await.unwrap();
    assert_eq!(list.len(), 2);
}
