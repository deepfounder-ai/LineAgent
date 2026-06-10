use lineagent::config::Config;
use lineagent::core::{
    index::IndexService,
    project::ProjectService,
    search::SearchService,
    ticket::{CreateTicket, TicketService},
};
use lineagent::storage::pool::init_pool;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn setup() -> lineagent::storage::AppState {
    let mut cfg = Config::for_test(std::path::PathBuf::from("/tmp"));
    cfg.db_url = "sqlite::memory:".to_string();
    init_pool(cfg).await.unwrap()
}

async fn create_user(state: &lineagent::storage::AppState) -> String {
    let id = uuid::Uuid::now_v7().to_string();
    let uname = format!("u{}", &id[..8]);
    sqlx::query(
        "INSERT INTO users (id, username, password_hash, created_at) \
         VALUES (?1, ?2, 'hash', datetime('now'))",
    )
    .bind(&id)
    .bind(&uname)
    .execute(&state.db)
    .await
    .unwrap();
    id
}

// ---------------------------------------------------------------------------
// SearchService tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn search_finds_ticket_by_title() {
    let state = setup().await;
    let uid = create_user(&state).await;

    let psvc = ProjectService::new(state.clone());
    psvc.create(&uid, "LIN", "LineAgent", None).await.unwrap();

    let tsvc = TicketService::new(state.clone());
    tsvc.create(
        &uid,
        CreateTicket {
            project_key: "LIN".into(),
            title: "payment processing".into(),
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
    tsvc.create(
        &uid,
        CreateTicket {
            project_key: "LIN".into(),
            title: "user authentication".into(),
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

    let ssvc = SearchService::new(state);
    let results = ssvc.search(&uid, "payment", None).await.unwrap();
    assert!(!results.is_empty(), "expected at least one result");
    assert_eq!(
        results[0].identifier, "LIN-1",
        "payment ticket should rank first"
    );
}

#[tokio::test]
async fn search_returns_empty_for_no_match() {
    let state = setup().await;
    let uid = create_user(&state).await;

    let psvc = ProjectService::new(state.clone());
    psvc.create(&uid, "LIN", "LineAgent", None).await.unwrap();

    let tsvc = TicketService::new(state.clone());
    tsvc.create(
        &uid,
        CreateTicket {
            project_key: "LIN".into(),
            title: "payment processing".into(),
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

    let ssvc = SearchService::new(state);
    let results = ssvc.search(&uid, "zzznomatch", None).await.unwrap();
    assert!(results.is_empty());
}

// ---------------------------------------------------------------------------
// IndexService tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn index_returns_status_counts() {
    let state = setup().await;
    let uid = create_user(&state).await;

    let psvc = ProjectService::new(state.clone());
    psvc.create(&uid, "LIN", "LineAgent", None).await.unwrap();

    let tsvc = TicketService::new(state.clone());
    tsvc.create(
        &uid,
        CreateTicket {
            project_key: "LIN".into(),
            title: "T1".into(),
            description: None,
            status: Some("done".into()),
            priority: None,
            assignee: None,
            parent_identifier: None,
            cycle_id: None,
        },
    )
    .await
    .unwrap();
    tsvc.create(
        &uid,
        CreateTicket {
            project_key: "LIN".into(),
            title: "T2".into(),
            description: None,
            status: None, // defaults to backlog
            priority: None,
            assignee: None,
            parent_identifier: None,
            cycle_id: None,
        },
    )
    .await
    .unwrap();

    let isvc = IndexService::new(state);
    let index = isvc.build(&uid).await.unwrap();
    assert_eq!(index.len(), 1);
    let proj = &index[0];
    assert_eq!(proj.key, "LIN");
    assert_eq!(proj.counts.done, 1);
    assert_eq!(proj.counts.backlog, 1);
    assert_eq!(proj.counts.total, 2);
}
