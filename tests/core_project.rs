use lineagent::{config::Config, storage::pool::init_pool};
use lineagent::core::project::ProjectService;

async fn setup() -> lineagent::storage::AppState {
    let mut cfg = Config::for_test(std::path::PathBuf::from("/tmp"));
    cfg.db_url = "sqlite::memory:".to_string();
    init_pool(cfg).await.unwrap()
}

#[tokio::test]
async fn create_project_uppercases_key() {
    let state = setup().await;
    let user_id = create_test_user(&state).await;
    let svc = ProjectService::new(state);
    let proj = svc.create(&user_id, "lin", "LineAgent", None).await.unwrap();
    assert_eq!(proj.key, "LIN");
    assert_eq!(proj.name, "LineAgent");
}

#[tokio::test]
async fn create_duplicate_key_returns_conflict() {
    let state = setup().await;
    let user_id = create_test_user(&state).await;
    let svc = ProjectService::new(state);
    svc.create(&user_id, "LIN", "First", None).await.unwrap();
    let err = svc.create(&user_id, "LIN", "Second", None).await.unwrap_err();
    assert!(matches!(err, lineagent::error::AppError::Conflict(_)));
}

#[tokio::test]
async fn get_and_list() {
    let state = setup().await;
    let user_id = create_test_user(&state).await;
    let svc = ProjectService::new(state);
    svc.create(&user_id, "AA", "Alpha", None).await.unwrap();
    svc.create(&user_id, "BB", "Beta", None).await.unwrap();
    let proj = svc.get(&user_id, "AA").await.unwrap();
    assert_eq!(proj.name, "Alpha");
    let list = svc.list(&user_id).await.unwrap();
    assert_eq!(list.len(), 2);
}

#[tokio::test]
async fn update_project() {
    let state = setup().await;
    let user_id = create_test_user(&state).await;
    let svc = ProjectService::new(state);
    svc.create(&user_id, "UPD", "Original", None).await.unwrap();
    svc.update(&user_id, "UPD", Some("Updated"), None).await.unwrap();
    let proj = svc.get(&user_id, "UPD").await.unwrap();
    assert_eq!(proj.name, "Updated");
}

#[tokio::test]
async fn create_appends_event() {
    let state = setup().await;
    let user_id = create_test_user(&state).await;
    let svc = ProjectService::new(state.clone());
    svc.create(&user_id, "EVT", "EventTest", None).await.unwrap();
    // verify event row exists
    let events = sqlx::query("SELECT kind FROM events WHERE user_id = ?1")
        .bind(&user_id)
        .fetch_all(&state.db)
        .await.unwrap();
    assert!(!events.is_empty());
    use sqlx::Row;
    let kind: String = events[0].try_get("kind").unwrap();
    assert_eq!(kind, "project.create");
}

// Helper: insert a user row directly
async fn create_test_user(state: &lineagent::storage::AppState) -> String {
    let id = uuid::Uuid::now_v7().to_string();
    sqlx::query("INSERT INTO users (id, username, password_hash, created_at) VALUES (?1,'testuser','hash', datetime('now'))")
        .bind(&id)
        .execute(&state.db)
        .await.unwrap();
    id
}
