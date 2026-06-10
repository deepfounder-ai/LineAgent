use lineagent::config::Config;
use lineagent::storage::init_pool;
use lineagent::storage::{comment_repo, cycle_repo, relation_repo, ticket_repo};

async fn setup() -> sqlx::SqlitePool {
    let mut cfg = Config::for_test(std::path::PathBuf::from("/tmp"));
    cfg.db_url = "sqlite::memory:".to_string();
    let pool = init_pool(cfg).await.unwrap().db;

    // Seed user row.
    sqlx::query(
        "INSERT OR IGNORE INTO users (id, username, password_hash, created_at) \
         VALUES ('user1', 'user1', '', '2024-01-01T00:00:00+00:00')",
    )
    .execute(&pool)
    .await
    .unwrap();

    // Seed project row.
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

/// Seeds user → project → ticket. Returns (project_id, ticket_id).
async fn seed_ticket(pool: &sqlx::SqlitePool, identifier: &str) -> (String, String) {
    let project_id = "proj1".to_string();
    let ticket_id = uuid::Uuid::now_v7().to_string();
    let number: i64 = identifier.split('-').last().unwrap().parse().unwrap();
    ticket_repo::insert(
        pool,
        &ticket_id,
        "user1",
        &project_id,
        number,
        identifier,
        &format!("Ticket {identifier}"),
        None,
        "backlog",
        "medium",
        None,
        None,
        None,
    )
    .await
    .unwrap();
    (project_id, ticket_id)
}

// ---------------------------------------------------------------------------
// comment_repo tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn comment_add_and_list() {
    let pool = setup().await;
    let (_, ticket_id) = seed_ticket(&pool, "LIN-1").await;

    let id1 = uuid::Uuid::now_v7().to_string();
    let id2 = uuid::Uuid::now_v7().to_string();

    comment_repo::insert(&pool, &id1, "user1", &ticket_id, Some("agent"), "First comment")
        .await
        .unwrap();
    comment_repo::insert(&pool, &id2, "user1", &ticket_id, None, "Second comment")
        .await
        .unwrap();

    let comments = comment_repo::list_for_ticket(&pool, "user1", &ticket_id)
        .await
        .unwrap();

    assert_eq!(comments.len(), 2);
    assert_eq!(comments[0].body, "First comment");
    assert_eq!(comments[1].body, "Second comment");
}

#[tokio::test]
async fn comment_list_ordered_asc() {
    let pool = setup().await;
    let (_, ticket_id) = seed_ticket(&pool, "LIN-2").await;

    let id1 = uuid::Uuid::now_v7().to_string();
    comment_repo::insert(&pool, &id1, "user1", &ticket_id, None, "First")
        .await
        .unwrap();

    // Sleep >1 s to ensure different RFC3339 second-resolution timestamps.
    tokio::time::sleep(tokio::time::Duration::from_millis(1100)).await;

    let id2 = uuid::Uuid::now_v7().to_string();
    comment_repo::insert(&pool, &id2, "user1", &ticket_id, None, "Second")
        .await
        .unwrap();

    let comments = comment_repo::list_for_ticket(&pool, "user1", &ticket_id)
        .await
        .unwrap();

    assert_eq!(comments.len(), 2);
    assert_eq!(comments[0].body, "First");
    assert_eq!(comments[1].body, "Second", "second comment body should come last in ASC order");
}

// ---------------------------------------------------------------------------
// relation_repo tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn relation_add_and_list() {
    let pool = setup().await;
    let (_, ticket_a) = seed_ticket(&pool, "LIN-3").await;
    let (_, ticket_b) = seed_ticket(&pool, "LIN-4").await;

    let rel_id = uuid::Uuid::now_v7().to_string();
    relation_repo::insert(&pool, &rel_id, "user1", &ticket_a, &ticket_b, "blocks")
        .await
        .unwrap();

    // List from ticket_a perspective.
    let from_a = relation_repo::list_for_ticket(&pool, "user1", &ticket_a)
        .await
        .unwrap();
    assert_eq!(from_a.len(), 1);
    assert_eq!(from_a[0].relation_type, "blocks");

    // List from ticket_b perspective.
    let from_b = relation_repo::list_for_ticket(&pool, "user1", &ticket_b)
        .await
        .unwrap();
    assert_eq!(from_b.len(), 1);
    assert_eq!(from_b[0].relation_type, "blocks");
}

#[tokio::test]
async fn relation_duplicate_returns_conflict() {
    let pool = setup().await;
    let (_, ticket_a) = seed_ticket(&pool, "LIN-5").await;
    let (_, ticket_b) = seed_ticket(&pool, "LIN-6").await;

    let id1 = uuid::Uuid::now_v7().to_string();
    relation_repo::insert(&pool, &id1, "user1", &ticket_a, &ticket_b, "blocks")
        .await
        .unwrap();

    let id2 = uuid::Uuid::now_v7().to_string();
    let err = relation_repo::insert(&pool, &id2, "user1", &ticket_a, &ticket_b, "blocks")
        .await
        .unwrap_err();

    assert!(
        matches!(err, lineagent::error::AppError::Conflict(_)),
        "expected Conflict, got {:?}",
        err
    );
}

#[tokio::test]
async fn relation_delete() {
    let pool = setup().await;
    let (_, ticket_a) = seed_ticket(&pool, "LIN-7").await;
    let (_, ticket_b) = seed_ticket(&pool, "LIN-8").await;

    let rel_id = uuid::Uuid::now_v7().to_string();
    relation_repo::insert(&pool, &rel_id, "user1", &ticket_a, &ticket_b, "blocks")
        .await
        .unwrap();

    relation_repo::delete(&pool, &rel_id).await.unwrap();

    let relations = relation_repo::list_for_ticket(&pool, "user1", &ticket_a)
        .await
        .unwrap();
    assert!(relations.is_empty(), "list should be empty after delete");
}

// ---------------------------------------------------------------------------
// cycle_repo tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn cycle_insert_list_update() {
    let pool = setup().await;

    let cycle_id = uuid::Uuid::now_v7().to_string();
    let cycle = cycle_repo::insert(
        &pool,
        &cycle_id,
        "user1",
        "proj1",
        1,
        "Sprint 1",
        Some("2024-01-01T00:00:00+00:00"),
        Some("2024-01-14T00:00:00+00:00"),
    )
    .await
    .unwrap();

    assert_eq!(cycle.number, 1);
    assert_eq!(cycle.name, "Sprint 1");

    // List returns the cycle.
    let cycles = cycle_repo::list_for_project(&pool, "user1", "proj1")
        .await
        .unwrap();
    assert_eq!(cycles.len(), 1);
    assert_eq!(cycles[0].id, cycle_id);

    // Update name.
    cycle_repo::update(&pool, &cycle_id, Some("Sprint 1 Updated"), None, None)
        .await
        .unwrap();

    // get_by_id returns updated name.
    let updated = cycle_repo::get_by_id(&pool, &cycle_id)
        .await
        .unwrap()
        .expect("cycle should exist");
    assert_eq!(updated.name, "Sprint 1 Updated");
    // starts_at unchanged via COALESCE.
    assert_eq!(updated.starts_at.as_deref(), Some("2024-01-01T00:00:00+00:00"));
}

#[tokio::test]
async fn cycle_update_returns_not_found() {
    let pool = setup().await;
    let err = cycle_repo::update(&pool, "ghost-id", Some("x"), None, None).await.unwrap_err();
    assert!(matches!(err, lineagent::error::AppError::NotFound(_)), "expected NotFound, got {:?}", err);
}

#[tokio::test]
async fn cycle_get_by_id_returns_none_for_missing() {
    let pool = setup().await;
    let result = cycle_repo::get_by_id(&pool, "nonexistent").await.unwrap();
    assert!(result.is_none());
}
