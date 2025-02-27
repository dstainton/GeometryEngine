use super::*;
use crate::models::{Dataset, Feature, SpatialQuery};
use geojson::{GeoJson, Geometry, Value};
use serde_json::json;
use crate::error::ApiError;

#[tokio::test]
async fn test_api_key_management() {
    let db = setup_test_db().await;
    let test_key = generate_test_api_key();
    
    // Test creating API key
    db.create_api_key(&test_key, 3600, 3600, &std::env::var("MASTER_API_KEY").unwrap())
        .await
        .expect("Failed to create API key");
    
    // Test validating API key
    assert!(db.validate_api_key(&test_key).await.unwrap());
    
    // Test invalid API key
    assert!(!db.validate_api_key("invalid_key").await.unwrap());
}

#[tokio::test]
async fn test_dataset_operations() {
    let db = setup_test_db().await;
    let api_key = generate_test_api_key();
    
    // Create test dataset
    let dataset = Dataset {
        id: Uuid::new_v4(),
        name: "test_dataset".to_string(),
        api_key: api_key.clone(),
    };
    
    db.create_dataset(&dataset).await.expect("Failed to create dataset");
    
    // Test listing datasets
    let datasets = db.get_datasets(&api_key).await.expect("Failed to list datasets");
    assert_eq!(datasets.len(), 1);
    assert_eq!(datasets[0].name, "test_dataset");
    
    // Test getting dataset by name
    let found = db.get_dataset_by_name("test_dataset", &api_key)
        .await
        .expect("Failed to get dataset")
        .expect("Dataset not found");
    assert_eq!(found.id, dataset.id);
}

#[tokio::test]
async fn test_feature_operations() {
    let db = setup_test_db().await;
    let api_key = generate_test_api_key();
    
    // Create test dataset
    let dataset = Dataset {
        id: Uuid::new_v4(),
        name: "test_dataset".to_string(),
        api_key: api_key.clone(),
    };
    db.create_dataset(&dataset).await.expect("Failed to create dataset");
    
    // Create test feature
    let feature = Feature {
        id: Uuid::new_v4(),
        dataset_id: dataset.id,
        feature_id: "test_feature".to_string(),
        geometry: GeoJson::Geometry(Geometry::new(Value::Point(vec![0.0, 0.0]))),
        attributes: json!({"name": "test point"}),
    };
    
    // Test storing feature
    let id = db.store_feature(feature.clone(), &dataset.name, &api_key)
        .await
        .expect("Failed to store feature");
    
    // Test retrieving feature
    let retrieved = db.get_feature(&dataset.name, &feature.feature_id)
        .await
        .expect("Failed to get feature")
        .expect("Feature not found");
    assert_eq!(retrieved.feature_id, feature.feature_id);
}

#[tokio::test]
async fn test_feature_operations_with_projections() {
    let db = setup_test_db().await;
    let api_key = generate_test_api_key();
    
    // Create test dataset
    let dataset = Dataset {
        id: Uuid::new_v4(),
        name: "test_dataset".to_string(),
        api_key: api_key.clone(),
    };
    db.create_dataset(&dataset).await.expect("Failed to create dataset");
    
    // Test storing feature in Web Mercator (EPSG:3857)
    let feature = Feature {
        id: Uuid::new_v4(),
        dataset_id: dataset.id,
        feature_id: "test_feature".to_string(),
        geometry: GeoJson::Geometry(Geometry::new(Value::Point(vec![
            -13580978.0,  // roughly -122° longitude in Web Mercator
            5621521.0,    // roughly 45° latitude in Web Mercator
        ]))),
        attributes: json!({"name": "Portland"}),
        input_srid: 3857,
    };
    
    // Store feature (will be converted to WGS84 internally)
    let id = db.store_feature(feature.clone(), &dataset.name, &api_key, 3857)
        .await
        .expect("Failed to store feature");
    
    // Retrieve feature and verify coordinates are in WGS84
    let retrieved = db.get_feature(&dataset.name, &feature.feature_id)
        .await
        .expect("Failed to get feature")
        .expect("Feature not found");

    if let GeoJson::Geometry(geom) = retrieved.geometry {
        if let Value::Point(coords) = geom.value {
            // Check if coordinates are roughly -122° longitude, 45° latitude
            assert!((coords[0] - (-122.0)).abs() < 0.1);
            assert!((coords[1] - 45.0).abs() < 0.1);
        } else {
            panic!("Expected Point geometry");
        }
    } else {
        panic!("Expected Geometry");
    }
}

#[tokio::test]
async fn test_spatial_query() {
    let db = setup_test_db().await;
    let api_key = generate_test_api_key();
    
    // Create test dataset with a point feature
    let dataset = Dataset {
        id: Uuid::new_v4(),
        name: "test_dataset".to_string(),
        api_key: api_key.clone(),
    };
    db.create_dataset(&dataset).await.expect("Failed to create dataset");
    
    let feature = Feature {
        id: Uuid::new_v4(),
        dataset_id: dataset.id,
        feature_id: "test_feature".to_string(),
        geometry: GeoJson::Geometry(Geometry::new(Value::Point(vec![1.0, 1.0]))),
        attributes: json!({"name": "test point"}),
    };
    db.store_feature(feature, &dataset.name, &api_key).await.expect("Failed to store feature");
    
    // Test spatial query
    let query = SpatialQuery {
        operation: "intersects".to_string(),
        dataset_name: "test_dataset".to_string(),
        geometry: GeoJson::Geometry(Geometry::new(Value::Polygon(vec![
            vec![vec![0.0, 0.0], vec![0.0, 2.0], vec![2.0, 2.0], vec![2.0, 0.0], vec![0.0, 0.0]]
        ]))),
        api_key: api_key.clone(),
    };
    
    let results = db.spatial_query(&query).await.expect("Failed to execute spatial query");
    assert_eq!(results.len(), 1);
}

#[tokio::test]
async fn test_spatial_query_with_projections() {
    let db = setup_test_db().await;
    let api_key = generate_test_api_key();
    
    // Create test dataset with a point in WGS84
    let dataset = Dataset {
        id: Uuid::new_v4(),
        name: "test_dataset".to_string(),
        api_key: api_key.clone(),
    };
    db.create_dataset(&dataset).await.expect("Failed to create dataset");
    
    let feature = Feature {
        id: Uuid::new_v4(),
        dataset_id: dataset.id,
        feature_id: "test_feature".to_string(),
        geometry: GeoJson::Geometry(Geometry::new(Value::Point(vec![-122.0, 45.0]))),
        attributes: json!({"name": "Portland"}),
        input_srid: 4326,
    };
    
    db.store_feature(feature, &dataset.name, &api_key, 4326)
        .await
        .expect("Failed to store feature");
    
    // Query using Web Mercator coordinates
    let query = SpatialQuery {
        operation: "intersects".to_string(),
        dataset_name: "test_dataset".to_string(),
        geometry: GeoJson::Geometry(Geometry::new(Value::Polygon(vec![vec![
            vec![-13581978.0, 5620521.0],
            vec![-13581978.0, 5622521.0],
            vec![-13579978.0, 5622521.0],
            vec![-13579978.0, 5620521.0],
            vec![-13581978.0, 5620521.0],
        ]]))),
        api_key: api_key.clone(),
        input_srid: 3857,
        output_srid: 4326,
    };
    
    let results = db.spatial_query(&query)
        .await
        .expect("Failed to execute spatial query");
    
    assert_eq!(results.len(), 1);
}

#[tokio::test]
async fn test_store_feature_with_invalid_srid() {
    let db = setup_test_db().await;
    let feature = Feature {
        id: Uuid::new_v4(),
        dataset_id: Uuid::new_v4(),
        feature_id: "test".to_string(),
        geometry: Some(create_test_geometry()),
        attributes: json!({}),
        input_srid: 1234,  // Invalid SRID
        api_key: "test-key".to_string(),
    };

    let result = db.store_feature(feature, "test-dataset", "test-key", 1234).await;
    match result {
        Err(ApiError::InvalidCoordinateSystem { srid, .. }) => {
            assert_eq!(srid, 1234);
        },
        _ => panic!("Expected InvalidCoordinateSystem error"),
    }
}

#[tokio::test]
async fn test_store_feature_with_valid_srid() {
    let db = setup_test_db().await;
    let feature = Feature {
        id: Uuid::new_v4(),
        dataset_id: Uuid::new_v4(),
        feature_id: "test".to_string(),
        geometry: Some(create_test_geometry()),
        attributes: json!({}),
        input_srid: 3005,  // BC Albers
        api_key: "test-key".to_string(),
    };

    let result = db.store_feature(feature, "test-dataset", "test-key", 3005).await;
    assert!(result.is_ok());
} 