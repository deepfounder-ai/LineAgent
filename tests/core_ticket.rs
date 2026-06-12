use lineagent::core::ticket::{
    CreateTicket, Priority, RelationType, Status, TicketService, UpdateTicket,
};

#[test]
fn parses_valid_and_rejects_invalid() {
    assert!("in_progress".parse::<Status>().is_ok());
    assert!("nope".parse::<Status>().is_err());
    assert!("critical".parse::<Priority>().is_ok());
    assert!("high".parse::<Priority>().is_ok());
    assert!("bad_priority".parse::<Priority>().is_err());
}

#[test]
fn status_variants() {
    let cases = [
        "backlog",
        "todo",
        "in_progress",
        "review",
        "done",
        "cancelled",
    ];
    for s in cases {
        assert!(s.parse::<Status>().is_ok(), "failed to parse: {s}");
    }
}

#[test]
fn priority_variants() {
    for p in ["critical", "high", "medium", "low"] {
        assert!(p.parse::<Priority>().is_ok(), "failed to parse: {p}");
    }
}

#[test]
fn relation_type_variants() {
    for r in ["blocks", "duplicates", "relates_to"] {
        assert!(r.parse::<RelationType>().is_ok(), "failed to parse: {r}");
    }
    assert!("child_of".parse::<RelationType>().is_err()); // dropped from v1
}

#[test]
fn defaults() {
    assert_eq!(Status::default().as_str(), "backlog");
    assert_eq!(Priority::default().as_str(), "medium");
}

#[test]
fn serde_roundtrip() {
    let s: Status = serde_json::from_str("\"in_progress\"").unwrap();
    assert_eq!(s.as_str(), "in_progress");
    let json = serde_json::to_string(&s).unwrap();
    assert_eq!(json, "\"in_progress\"");
}

// ---------------------------------------------------------------------------
// TicketService helpers
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

// ---------------------------------------------------------------------------
// TicketService tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn ticket_identifiers_are_sequential() {
    let state = setup_state().await;
    let user_id = create_test_user(&state).await;
    let proj_svc = lineagent::core::project::ProjectService::new(state.clone());
    proj_svc
        .create(&user_id, "LIN", "LineAgent", None)
        .await
        .unwrap();

    let svc = TicketService::new(state);
    let t1 = svc
        .create(
            &user_id,
            CreateTicket {
                project_key: "LIN".to_string(),
                title: "First".to_string(),
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
    let t2 = svc
        .create(
            &user_id,
            CreateTicket {
                project_key: "LIN".to_string(),
                title: "Second".to_string(),
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
    assert_eq!(t1.identifier, "LIN-1");
    assert_eq!(t2.identifier, "LIN-2");
}

#[tokio::test]
async fn independent_project_counters() {
    let state = setup_state().await;
    let user_id = create_test_user(&state).await;
    let proj_svc = lineagent::core::project::ProjectService::new(state.clone());
    proj_svc
        .create(&user_id, "LIN", "LineAgent", None)
        .await
        .unwrap();
    proj_svc.create(&user_id, "OPS", "Ops", None).await.unwrap();

    let svc = TicketService::new(state);
    let t = svc
        .create(
            &user_id,
            CreateTicket {
                project_key: "OPS".to_string(),
                title: "Ops ticket".to_string(),
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
    assert_eq!(t.identifier, "OPS-1");
}

#[tokio::test]
async fn invalid_status_returns_validation_error() {
    let state = setup_state().await;
    let user_id = create_test_user(&state).await;
    let proj_svc = lineagent::core::project::ProjectService::new(state.clone());
    proj_svc
        .create(&user_id, "LIN", "LineAgent", None)
        .await
        .unwrap();

    let svc = TicketService::new(state);
    let err = svc
        .create(
            &user_id,
            CreateTicket {
                project_key: "LIN".to_string(),
                title: "Bad".to_string(),
                description: None,
                status: Some("nope".to_string()),
                priority: None,
                assignee: None,
                parent_identifier: None,
                cycle_id: None,
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err, lineagent::error::AppError::Validation(_)));
}

#[tokio::test]
async fn get_ticket_includes_comments_and_relations() {
    let state = setup_state().await;
    let user_id = create_test_user(&state).await;
    let proj_svc = lineagent::core::project::ProjectService::new(state.clone());
    proj_svc
        .create(&user_id, "LIN", "LineAgent", None)
        .await
        .unwrap();
    let svc = TicketService::new(state.clone());

    svc.create(
        &user_id,
        CreateTicket {
            project_key: "LIN".to_string(),
            title: "T1".to_string(),
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
    svc.create(
        &user_id,
        CreateTicket {
            project_key: "LIN".to_string(),
            title: "T2".to_string(),
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

    let t_view = svc.get(&user_id, "LIN-1").await.unwrap();
    assert_eq!(t_view.ticket.identifier, "LIN-1");
    // no comments yet
    assert!(t_view.comments.is_empty());
    assert!(t_view.relations.is_empty());
}

#[tokio::test]
async fn update_and_delete_ticket() {
    let state = setup_state().await;
    let user_id = create_test_user(&state).await;
    let proj_svc = lineagent::core::project::ProjectService::new(state.clone());
    proj_svc
        .create(&user_id, "LIN", "LineAgent", None)
        .await
        .unwrap();
    let svc = TicketService::new(state.clone());
    svc.create(
        &user_id,
        CreateTicket {
            project_key: "LIN".to_string(),
            title: "DeleteMe".to_string(),
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

    let updated = svc
        .update(
            &user_id,
            "LIN-1",
            UpdateTicket {
                title: Some("Updated".to_string()),
                status: Some("done".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.title, "Updated");
    assert_eq!(updated.status, "done");

    svc.delete(&user_id, "LIN-1").await.unwrap();
    let err = svc.get(&user_id, "LIN-1").await.unwrap_err();
    assert!(matches!(err, lineagent::error::AppError::NotFound(_)));
}
