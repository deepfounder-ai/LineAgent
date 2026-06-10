use lineagent::core::comment::CommentService;
use lineagent::core::cycle::CycleService;
use lineagent::core::project::ProjectService;
use lineagent::core::relation::RelationService;
use lineagent::core::ticket::{CreateTicket, TicketService};

// ---------------------------------------------------------------------------
// Helpers (same pattern as core_ticket.rs)
// ---------------------------------------------------------------------------

async fn setup_state() -> lineagent::storage::AppState {
    let mut cfg = lineagent::config::Config::for_test(std::path::PathBuf::from("/tmp"));
    cfg.db_url = "sqlite::memory:".to_string();
    lineagent::storage::pool::init_pool(cfg).await.unwrap()
}

async fn create_test_user(state: &lineagent::storage::AppState) -> String {
    let id = uuid::Uuid::now_v7().to_string();
    let username = format!("user_{}", &id[..8]);
    sqlx::query(
        "INSERT INTO users (id, username, password_hash, created_at) VALUES (?1,?2,'hash',datetime('now'))",
    )
    .bind(&id)
    .bind(&username)
    .execute(&state.db)
    .await
    .unwrap();
    id
}

/// Create a project + two tickets for use in tests.
async fn setup_project_and_tickets(
    state: &lineagent::storage::AppState,
    user_id: &str,
) {
    let proj_svc = ProjectService::new(state.clone());
    proj_svc.create(user_id, "LIN", "LineAgent", None).await.unwrap();

    let ticket_svc = TicketService::new(state.clone());
    ticket_svc
        .create(
            user_id,
            CreateTicket {
                project_key: "LIN".to_string(),
                title: "Ticket one".to_string(),
                description: None,
                status: None,
                priority: None,
                assignee: None,
                parent_identifier: None,
                cycle_id: None,
            },
        )
        .await
        .unwrap();
    ticket_svc
        .create(
            user_id,
            CreateTicket {
                project_key: "LIN".to_string(),
                title: "Ticket two".to_string(),
                description: None,
                status: None,
                priority: None,
                assignee: None,
                parent_identifier: None,
                cycle_id: None,
            },
        )
        .await
        .unwrap();
}

// ---------------------------------------------------------------------------
// CommentService tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn comment_add_and_list() {
    let state = setup_state().await;
    let user_id = create_test_user(&state).await;
    setup_project_and_tickets(&state, &user_id).await;

    let svc = CommentService::new(state);

    svc.add(&user_id, "LIN-1", Some("alice"), "First comment").await.unwrap();
    svc.add(&user_id, "LIN-1", None, "Second comment").await.unwrap();

    let comments = svc.list(&user_id, "LIN-1").await.unwrap();
    assert_eq!(comments.len(), 2);
    assert_eq!(comments[0].body, "First comment");
    assert_eq!(comments[1].body, "Second comment");
}

// ---------------------------------------------------------------------------
// RelationService tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn relation_add_validates_type() {
    let state = setup_state().await;
    let user_id = create_test_user(&state).await;
    setup_project_and_tickets(&state, &user_id).await;

    let svc = RelationService::new(state);

    let err = svc
        .add(&user_id, "LIN-1", "LIN-2", "bad_type")
        .await
        .unwrap_err();

    assert!(
        matches!(err, lineagent::error::AppError::Validation(_)),
        "expected Validation error, got: {err:?}"
    );
}

#[tokio::test]
async fn relation_add_and_remove() {
    let state = setup_state().await;
    let user_id = create_test_user(&state).await;
    setup_project_and_tickets(&state, &user_id).await;

    let svc = RelationService::new(state);

    let rel = svc.add(&user_id, "LIN-1", "LIN-2", "blocks").await.unwrap();
    assert_eq!(rel.from_identifier, "LIN-1");
    assert_eq!(rel.to_identifier, "LIN-2");
    assert_eq!(rel.relation_type, "blocks");

    svc.remove(&user_id, &rel.id).await.unwrap();

    // Removing again should be idempotent (no error)
    svc.remove(&user_id, &rel.id).await.unwrap();
}

#[tokio::test]
async fn relation_duplicate_returns_conflict() {
    let state = setup_state().await;
    let user_id = create_test_user(&state).await;
    setup_project_and_tickets(&state, &user_id).await;

    let svc = RelationService::new(state);

    svc.add(&user_id, "LIN-1", "LIN-2", "blocks").await.unwrap();

    let err = svc
        .add(&user_id, "LIN-1", "LIN-2", "blocks")
        .await
        .unwrap_err();

    assert!(
        matches!(err, lineagent::error::AppError::Conflict(_)),
        "expected Conflict error, got: {err:?}"
    );
}

// ---------------------------------------------------------------------------
// CycleService tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn cycle_create_list_update() {
    let state = setup_state().await;
    let user_id = create_test_user(&state).await;

    let proj_svc = ProjectService::new(state.clone());
    proj_svc.create(&user_id, "LIN", "LineAgent", None).await.unwrap();

    let svc = CycleService::new(state);

    let cycle = svc
        .create(&user_id, "LIN", "Sprint 1", Some("2025-01-01"), Some("2025-01-14"))
        .await
        .unwrap();

    assert_eq!(cycle.number, 1);
    assert_eq!(cycle.name, "Sprint 1");

    let list = svc.list(&user_id, "LIN").await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, cycle.id);

    let updated = svc
        .update(&user_id, &cycle.id, Some("Sprint 1 - renamed"), None, None)
        .await
        .unwrap();

    assert_eq!(updated.name, "Sprint 1 - renamed");
    assert_eq!(updated.id, cycle.id);
}
