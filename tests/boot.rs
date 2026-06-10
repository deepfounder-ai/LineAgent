use lineagent::config::Config;
use lineagent::storage::init_pool;
use tempfile::TempDir;

#[tokio::test]
async fn pool_init_ok() {
    let dir = TempDir::new().expect("tempdir");
    let config = Config::for_test(dir.path().to_path_buf());
    let state = init_pool(config).await;
    assert!(state.is_ok(), "pool init failed: {:?}", state.err());
}
