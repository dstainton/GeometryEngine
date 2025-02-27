use super::*;
use crate::models::Feature;
use geojson::{GeoJson, Geometry, Value};
use serde_json::json;

#[tokio::test]
async fn test_cache_operations() {
    let cache = setup_test_cache();
    let dataset_name = "test_dataset";
    let feature_id = "test_feature";

    // Create test feature
    let feature = Feature {
        id: Uuid::new_v4(),
        dataset_id: Uuid::new_v4(),
        feature_id: feature_id.to_string(),
        geometry: GeoJson::Geometry(Geometry::new(Value::Point(vec![0.0, 0.0]))),
        attributes: json!({"name": "test point"}),
    };

    // Test setting feature in cache
    cache.set_feature(dataset_name, &feature)
        .await
        .expect("Failed to cache feature");

    // Test getting feature from cache
    let cached = cache.get_feature(dataset_name, feature_id)
        .await
        .expect("Failed to get cached feature")
        .expect("Feature not found in cache");
    
    assert_eq!(cached.id, feature.id);
    assert_eq!(cached.feature_id, feature.feature_id);

    // Test invalidating feature
    cache.invalidate_feature(dataset_name, feature_id)
        .await
        .expect("Failed to invalidate cache");

    // Verify feature was removed
    let invalid = cache.get_feature(dataset_name, feature_id)
        .await
        .expect("Failed to check cache");
    assert!(invalid.is_none());
}

#[tokio::test]
async fn test_cache_expiry() {
    let cache = setup_test_cache();
    let dataset_name = "test_dataset";
    let feature_id = "test_feature";

    let feature = Feature {
        id: Uuid::new_v4(),
        dataset_id: Uuid::new_v4(),
        feature_id: feature_id.to_string(),
        geometry: GeoJson::Geometry(Geometry::new(Value::Point(vec![0.0, 0.0]))),
        attributes: json!({"name": "test point"}),
    };

    // Set feature with 1 second expiry
    let mut conn = cache.get_connection().await.expect("Failed to get Redis connection");
    let json = serde_json::to_string(&feature).expect("Failed to serialize feature");
    let _: () = redis::AsyncCommands::set_ex(&mut conn, "test_key", json, 1)
        .await
        .expect("Failed to set cache with expiry");

    // Verify feature exists
    let exists: bool = redis::AsyncCommands::exists(&mut conn, "test_key")
        .await
        .expect("Failed to check key existence");
    assert!(exists);

    // Wait for expiry
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Verify feature was removed
    let exists: bool = redis::AsyncCommands::exists(&mut conn, "test_key")
        .await
        .expect("Failed to check key existence");
    assert!(!exists);
} 