use lineagent::config::Config;
use std::path::PathBuf;

#[test]
fn resolved_db_url_uses_lineagent_db() {
    let c = Config::for_test(PathBuf::from("/tmp/x"));
    assert!(c.resolved_db_url().contains("lineagent.db"));
}
