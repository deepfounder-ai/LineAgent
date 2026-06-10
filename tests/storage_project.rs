use lineagent::config::Config;
use lineagent::storage::init_pool;
use lineagent::storage::project_repo;

async fn setup() -> sqlx::SqlitePool {
    // Use an in-memory database so the TempDir lifetime does not matter.
    let mut cfg = Config::for_test(std::path::PathBuf::from("/tmp"));
    cfg.db_url = "sqlite::memory:".to_string();
    let pool = init_pool(cfg).await.unwrap().db;
    // Seed the users referenced by tests (FK: projects.user_id → users.id).
    for (id, name) in &[("user1", "user1"), ("u1", "u1")] {
        sqlx::query(
            "INSERT OR IGNORE INTO users (id, username, password_hash, created_at) VALUES (?1, ?2, '', '2024-01-01T00:00:00+00:00')",
        )
        .bind(id)
        .bind(name)
        .execute(&pool)
        .await
        .unwrap();
    }
    pool
}

#[tokio::test]
async fn insert_and_get_by_key() {
    let pool = setup().await;
    let id = uuid::Uuid::now_v7().to_string();
    project_repo::insert(
        &pool, &id, "user1", "LIN", "LineAgent", None,
    ).await.unwrap();
    let proj = project_repo::get_by_key(&pool, "user1", "LIN").await.unwrap().unwrap();
    assert_eq!(proj.id, id);
    assert_eq!(proj.key, "LIN");
    assert_eq!(proj.name, "LineAgent");
    assert_eq!(proj.ticket_counter, 0);
}

#[tokio::test]
async fn next_ticket_number_increments() {
    let pool = setup().await;
    let id = uuid::Uuid::now_v7().to_string();
    project_repo::insert(&pool, &id, "user1", "OPS", "Ops", None).await.unwrap();
    let n1 = project_repo::next_ticket_number(&pool, &id).await.unwrap();
    let n2 = project_repo::next_ticket_number(&pool, &id).await.unwrap();
    assert_eq!(n1, 1);
    assert_eq!(n2, 2);
}

#[tokio::test]
async fn next_cycle_number_increments() {
    let pool = setup().await;
    let id = uuid::Uuid::now_v7().to_string();
    project_repo::insert(&pool, &id, "user1", "PROJ", "Project", None).await.unwrap();
    let n1 = project_repo::next_cycle_number(&pool, &id).await.unwrap();
    assert_eq!(n1, 1);
}

#[tokio::test]
async fn update_project() {
    let pool = setup().await;
    let id = uuid::Uuid::now_v7().to_string();
    project_repo::insert(&pool, &id, "user1", "UPD", "Original", None).await.unwrap();
    project_repo::update(&pool, &id, Some("Updated"), None).await.unwrap();
    let proj = project_repo::get_by_id(&pool, &id).await.unwrap().unwrap();
    assert_eq!(proj.name, "Updated");
}

#[tokio::test]
async fn list_for_user() {
    let pool = setup().await;
    let id1 = uuid::Uuid::now_v7().to_string();
    let id2 = uuid::Uuid::now_v7().to_string();
    project_repo::insert(&pool, &id1, "u1", "AA", "Alpha", None).await.unwrap();
    project_repo::insert(&pool, &id2, "u1", "BB", "Beta", None).await.unwrap();
    let list = project_repo::list_for_user(&pool, "u1").await.unwrap();
    assert_eq!(list.len(), 2);
}
