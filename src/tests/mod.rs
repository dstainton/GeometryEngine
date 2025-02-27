mod api_tests;
mod db_tests;
mod cache_tests;
mod rate_limit_tests;
mod integration_tests;

use crate::{db::Database, cache::Cache, rate_limit::RateLimiter};
use tokio_postgres::NoTls;
use redis::Client;
use uuid::Uuid;

// Test helpers
pub async fn setup_test_db() -> Database {
    let mut cfg = deadpool_postgres::Config::new();
    cfg.host = Some("localhost".to_string());
    cfg.port = Some(5432);
    cfg.dbname = Some("geospatial_test".to_string());
    cfg.user = Some("postgres".to_string());
    cfg.password = Some("your_password".to_string());

    Database::new().await.expect("Failed to create test database")
}

pub fn setup_test_cache() -> Cache {
    Cache::new().expect("Failed to create test cache")
}

pub fn generate_test_api_key() -> String {
    format!("test_key_{}", Uuid::new_v4())
} 