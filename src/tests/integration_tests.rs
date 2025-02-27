use super::*;
use actix_web::{test, web, App};
use crate::models::{Dataset, Feature, SpatialQuery, CreateApiKeyRequest};
use geojson::{GeoJson, Geometry, Value};
use serde_json::json;

async fn setup_test_system() -> test::TestApp {
    dotenv::dotenv().ok();
    let db = setup_test_db().await;
    let cache = setup_test_cache();
    let rate_limiter = web::Data::new(RateLimiter::new(cache.clone(), 100));

    // Initialize database
    db.initialize().await.expect("Failed to initialize database");

    test::init_service(
        App::new()
            .app_data(web::Data::new(db))
            .app_data(web::Data::new(cache))
            .app_data(rate_limiter)
            .service(
                web::scope("/api/v1")
                    .route("/api-keys", web::post().to(crate::create_api_key))
                    .route("/datasets", web::post().to(crate::create_dataset))
                    .route("/datasets", web::get().to(crate::list_datasets))
                    .route("/datasets/{dataset_name}/features", web::post().to(crate::store_feature))
                    .route("/datasets/{dataset_name}/features/{feature_id}", web::put().to(crate::update_feature))
                    .route("/spatial-query", web::post().to(crate::spatial_query))
                    .route("/datasets/{dataset_name}/features/{feature_id}", web::get().to(crate::get_feature))
            )
    ).await
}

#[tokio::test]
async fn test_complete_workflow() {
    let app = setup_test_system().await;
    let master_key = std::env::var("MASTER_API_KEY").unwrap();

    // 1. Create new API key
    let new_key = format!("test_key_{}", Uuid::new_v4());
    let req = test::TestRequest::post()
        .uri("/api/v1/api-keys")
        .header("X-API-Key", &master_key)
        .set_json(&CreateApiKeyRequest {
            new_key: new_key.clone(),
            key_expires_in_seconds: 3600,
            data_expires_in_seconds: 3600,
        })
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    // 2. Create dataset
    let dataset_name = "integration_test_dataset";
    let dataset = Dataset {
        id: Uuid::new_v4(),
        name: dataset_name.to_string(),
        api_key: new_key.clone(),
    };

    let req = test::TestRequest::post()
        .uri("/api/v1/datasets")
        .header("X-API-Key", &new_key)
        .set_json(&dataset)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    // 3. Store multiple features
    let features = vec![
        Feature {
            id: Uuid::new_v4(),
            dataset_id: dataset.id,
            feature_id: "point1".to_string(),
            geometry: GeoJson::Geometry(Geometry::new(Value::Point(vec![0.0, 0.0]))),
            attributes: json!({"name": "Point 1"}),
        },
        Feature {
            id: Uuid::new_v4(),
            dataset_id: dataset.id,
            feature_id: "point2".to_string(),
            geometry: GeoJson::Geometry(Geometry::new(Value::Point(vec![1.0, 1.0]))),
            attributes: json!({"name": "Point 2"}),
        },
    ];

    for feature in &features {
        let req = test::TestRequest::post()
            .uri(&format!("/api/v1/datasets/{}/features", dataset_name))
            .header("X-API-Key", &new_key)
            .set_json(feature)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    // 4. Perform spatial query
    let query = SpatialQuery {
        operation: "intersects".to_string(),
        dataset_name: dataset_name.to_string(),
        geometry: GeoJson::Geometry(Geometry::new(Value::Polygon(vec![
            vec![
                vec![-1.0, -1.0],
                vec![-1.0, 2.0],
                vec![2.0, 2.0],
                vec![2.0, -1.0],
                vec![-1.0, -1.0],
            ]
        ]))),
        api_key: new_key.clone(),
    };

    let req = test::TestRequest::post()
        .uri("/api/v1/spatial-query")
        .header("X-API-Key", &new_key)
        .set_json(&query)
        .to_request();

    let resp: Vec<Feature> = test::call_and_read_body_json(&app, req).await;
    assert_eq!(resp.len(), 2); // Should find both points

    // 5. Update a feature
    let mut updated_feature = features[0].clone();
    updated_feature.geometry = GeoJson::Geometry(Geometry::new(Value::Point(vec![0.5, 0.5])));

    let req = test::TestRequest::put()
        .uri(&format!("/api/v1/datasets/{}/features/point1", dataset_name))
        .header("X-API-Key", &new_key)
        .set_json(&updated_feature)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    // 6. Verify cache invalidation and retrieval
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/datasets/{}/features/point1", dataset_name))
        .header("X-API-Key", &new_key)
        .to_request();

    let resp: Feature = test::call_and_read_body_json(&app, req).await;
    assert_eq!(
        resp.geometry,
        GeoJson::Geometry(Geometry::new(Value::Point(vec![0.5, 0.5])))
    );
}

#[tokio::test]
async fn test_error_handling() {
    let app = setup_test_system().await;
    let api_key = generate_test_api_key();

    // Test invalid API key
    let req = test::TestRequest::get()
        .uri("/api/v1/datasets")
        .header("X-API-Key", "invalid_key")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 401);

    // Test dataset not found
    let req = test::TestRequest::get()
        .uri("/api/v1/datasets/nonexistent/features/123")
        .header("X-API-Key", &api_key)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 404);

    // Test invalid geometry
    let query = SpatialQuery {
        operation: "invalid_op".to_string(),
        dataset_name: "test".to_string(),
        geometry: GeoJson::Geometry(Geometry::new(Value::Point(vec![0.0, 0.0]))),
        api_key: api_key.clone(),
    };

    let req = test::TestRequest::post()
        .uri("/api/v1/spatial-query")
        .header("X-API-Key", &api_key)
        .set_json(&query)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 422);
}

#[tokio::test]
async fn test_complete_workflow_with_projections() {
    let app = setup_test_system().await;
    let master_key = std::env::var("MASTER_API_KEY").unwrap();

    // 1. Create new API key
    let new_key = format!("test_key_{}", Uuid::new_v4());
    let req = test::TestRequest::post()
        .uri("/api/v1/api-keys")
        .header("X-API-Key", &master_key)
        .set_json(&CreateApiKeyRequest {
            new_key: new_key.clone(),
            key_expires_in_seconds: 3600,
            data_expires_in_seconds: 3600,
        })
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    // 2. Create dataset
    let dataset_name = "integration_test_dataset";
    let dataset = Dataset {
        id: Uuid::new_v4(),
        name: dataset_name.to_string(),
        api_key: new_key.clone(),
    };

    let req = test::TestRequest::post()
        .uri("/api/v1/datasets")
        .header("X-API-Key", &new_key)
        .set_json(&dataset)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    // 3. Store features in different projections
    let features = vec![
        // Feature in WGS84
        Feature {
            id: Uuid::new_v4(),
            dataset_id: dataset.id,
            feature_id: "portland".to_string(),
            geometry: GeoJson::Geometry(Geometry::new(Value::Point(vec![-122.0, 45.0]))),
            attributes: json!({"name": "Portland", "state": "OR"}),
            input_srid: 4326,
        },
        // Feature in Web Mercator
        Feature {
            id: Uuid::new_v4(),
            dataset_id: dataset.id,
            feature_id: "seattle".to_string(),
            geometry: GeoJson::Geometry(Geometry::new(Value::Point(vec![
                -13619098.0,  // roughly -122.3° longitude
                6044371.0,    // roughly 47.6° latitude
            ]))),
            attributes: json!({"name": "Seattle", "state": "WA"}),
            input_srid: 3857,
        },
    ];

    for feature in &features {
        let req = test::TestRequest::post()
            .uri(&format!("/api/v1/datasets/{}/features", dataset_name))
            .header("X-API-Key", &new_key)
            .set_json(feature)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    // 4. Query features in Web Mercator but get results in WGS84
    let query = SpatialQuery {
        operation: "intersects".to_string(),
        dataset_name: dataset_name.to_string(),
        geometry: GeoJson::Geometry(Geometry::new(Value::Polygon(vec![vec![
            vec![-13700000.0, 5600000.0],  // Bounding box in Web Mercator
            vec![-13700000.0, 6100000.0],
            vec![-13500000.0, 6100000.0],
            vec![-13500000.0, 5600000.0],
            vec![-13700000.0, 5600000.0],
        ]]))),
        api_key: new_key.clone(),
        input_srid: 3857,
        output_srid: 4326,
    };

    let req = test::TestRequest::post()
        .uri("/api/v1/spatial-query")
        .header("X-API-Key", &new_key)
        .set_json(&query)
        .to_request();

    let resp: Vec<Feature> = test::call_and_read_body_json(&app, req).await;
    assert_eq!(resp.len(), 2);  // Should find both Portland and Seattle

    // 5. Verify coordinates are in WGS84
    for feature in resp {
        if let GeoJson::Geometry(geom) = feature.geometry {
            if let Value::Point(coords) = geom.value {
                match feature.feature_id.as_str() {
                    "portland" => {
                        assert!((coords[0] - (-122.0)).abs() < 0.1);
                        assert!((coords[1] - 45.0).abs() < 0.1);
                    },
                    "seattle" => {
                        assert!((coords[0] - (-122.3)).abs() < 0.1);
                        assert!((coords[1] - 47.6).abs() < 0.1);
                    },
                    _ => panic!("Unexpected feature_id"),
                }
            }
        }
    }
}

#[tokio::test]
async fn test_invalid_projection_handling() {
    let app = setup_test_system().await;
    let api_key = generate_test_api_key();

    // Create dataset
    let dataset = Dataset {
        id: Uuid::new_v4(),
        name: "test_dataset".to_string(),
        api_key: api_key.clone(),
    };

    let req = test::TestRequest::post()
        .uri("/api/v1/datasets")
        .header("X-API-Key", &api_key)
        .set_json(&dataset)
        .to_request();

    test::call_service(&app, req).await;

    // Try to store feature with invalid SRID
    let feature = Feature {
        id: Uuid::new_v4(),
        dataset_id: dataset.id,
        feature_id: "test_feature".to_string(),
        geometry: GeoJson::Geometry(Geometry::new(Value::Point(vec![0.0, 0.0]))),
        attributes: json!({"name": "test"}),
        input_srid: 9999,  // Invalid SRID
    };

    let req = test::TestRequest::post()
        .uri("/api/v1/datasets/test_dataset/features")
        .header("X-API-Key", &api_key)
        .set_json(&feature)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 422);  // Unprocessable Entity

    // Try spatial query with invalid SRID
    let query = SpatialQuery {
        operation: "intersects".to_string(),
        dataset_name: "test_dataset".to_string(),
        geometry: GeoJson::Geometry(Geometry::new(Value::Point(vec![0.0, 0.0]))),
        api_key: api_key.clone(),
        input_srid: 9999,  // Invalid input SRID
        output_srid: 9998, // Invalid output SRID
    };

    let req = test::TestRequest::post()
        .uri("/api/v1/spatial-query")
        .header("X-API-Key", &api_key)
        .set_json(&query)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 422);
}

#[tokio::test]
async fn test_polygon_with_holes() {
    let app = setup_test_system().await;
    let api_key = generate_test_api_key();

    // Create dataset
    let dataset = Dataset {
        id: Uuid::new_v4(),
        name: "test_dataset".to_string(),
        api_key: api_key.clone(),
    };

    let req = test::TestRequest::post()
        .uri("/api/v1/datasets")
        .header("X-API-Key", &api_key)
        .set_json(&dataset)
        .to_request();

    test::call_service(&app, req).await;

    // Create a polygon with a hole (donut) in Web Mercator
    let feature = Feature {
        id: Uuid::new_v4(),
        dataset_id: dataset.id,
        feature_id: "donut".to_string(),
        geometry: GeoJson::Geometry(Geometry::new(Value::Polygon(vec![
            // Outer ring (counterclockwise)
            vec![
                vec![-13580000.0, 5620000.0],
                vec![-13580000.0, 5625000.0],
                vec![-13575000.0, 5625000.0],
                vec![-13575000.0, 5620000.0],
                vec![-13580000.0, 5620000.0],
            ],
            // Inner ring (clockwise)
            vec![
                vec![-13578000.0, 5622000.0],
                vec![-13577000.0, 5622000.0],
                vec![-13577000.0, 5623000.0],
                vec![-13578000.0, 5623000.0],
                vec![-13578000.0, 5622000.0],
            ],
        ]))),
        attributes: json!({"name": "Donut"}),
        input_srid: 3857,
    };

    let req = test::TestRequest::post()
        .uri("/api/v1/datasets/test_dataset/features")
        .header("X-API-Key", &api_key)
        .set_json(&feature)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    // Query with a point inside the hole (should not intersect)
    let query = SpatialQuery {
        operation: "intersects".to_string(),
        dataset_name: "test_dataset".to_string(),
        geometry: GeoJson::Geometry(Geometry::new(Value::Point(vec![-122.5, 45.5]))),
        api_key: api_key.clone(),
        input_srid: 4326,
        output_srid: 4326,
    };

    let req = test::TestRequest::post()
        .uri("/api/v1/spatial-query")
        .header("X-API-Key", &api_key)
        .set_json(&query)
        .to_request();

    let resp: Vec<Feature> = test::call_and_read_body_json(&app, req).await;
    assert_eq!(resp.len(), 0);
}

#[tokio::test]
async fn test_multipolygon_operations() {
    let app = setup_test_system().await;
    let api_key = generate_test_api_key();

    // Create dataset
    let dataset = Dataset {
        id: Uuid::new_v4(),
        name: "test_dataset".to_string(),
        api_key: api_key.clone(),
    };

    let req = test::TestRequest::post()
        .uri("/api/v1/datasets")
        .header("X-API-Key", &api_key)
        .set_json(&dataset)
        .to_request();

    test::call_service(&app, req).await;

    // Create a MultiPolygon feature (e.g., Hawaii islands)
    let feature = Feature {
        id: Uuid::new_v4(),
        dataset_id: dataset.id,
        feature_id: "hawaii".to_string(),
        geometry: GeoJson::Geometry(Geometry::new(Value::MultiPolygon(vec![
            // Big Island
            vec![vec![
                vec![-155.5, 19.1],
                vec![-155.5, 20.0],
                vec![-154.8, 20.0],
                vec![-154.8, 19.1],
                vec![-155.5, 19.1],
            ]],
            // Maui
            vec![vec![
                vec![-156.7, 20.5],
                vec![-156.7, 21.0],
                vec![-156.0, 21.0],
                vec![-156.0, 20.5],
                vec![-156.7, 20.5],
            ]],
        ]))),
        attributes: json!({"name": "Hawaii Islands"}),
        input_srid: 4326,
    };

    let req = test::TestRequest::post()
        .uri("/api/v1/datasets/test_dataset/features")
        .header("X-API-Key", &api_key)
        .set_json(&feature)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    // Test contains operation with a point in Big Island
    let query = SpatialQuery {
        operation: "contains".to_string(),
        dataset_name: "test_dataset".to_string(),
        geometry: GeoJson::Geometry(Geometry::new(Value::Point(vec![-155.0, 19.5]))),
        api_key: api_key.clone(),
        input_srid: 4326,
        output_srid: 4326,
    };

    let req = test::TestRequest::post()
        .uri("/api/v1/spatial-query")
        .header("X-API-Key", &api_key)
        .set_json(&query)
        .to_request();

    let resp: Vec<Feature> = test::call_and_read_body_json(&app, req).await;
    assert_eq!(resp.len(), 1);
}

#[tokio::test]
async fn test_linestring_operations() {
    let app = setup_test_system().await;
    let api_key = generate_test_api_key();

    // Create dataset
    let dataset = Dataset {
        id: Uuid::new_v4(),
        name: "test_dataset".to_string(),
        api_key: api_key.clone(),
    };

    let req = test::TestRequest::post()
        .uri("/api/v1/datasets")
        .header("X-API-Key", &api_key)
        .set_json(&dataset)
        .to_request();

    test::call_service(&app, req).await;

    // Create a LineString feature (e.g., I-5 highway)
    let feature = Feature {
        id: Uuid::new_v4(),
        dataset_id: dataset.id,
        feature_id: "i5".to_string(),
        geometry: GeoJson::Geometry(Geometry::new(Value::LineString(vec![
            vec![-122.68, 45.52],  // Portland
            vec![-122.33, 47.61],  // Seattle
            vec![-122.91, 49.26],  // Vancouver
        ]))),
        attributes: json!({"name": "I-5 Highway"}),
        input_srid: 4326,
    };

    let req = test::TestRequest::post()
        .uri("/api/v1/datasets/test_dataset/features")
        .header("X-API-Key", &api_key)
        .set_json(&feature)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    // Test intersects with a polygon around Seattle
    let query = SpatialQuery {
        operation: "intersects".to_string(),
        dataset_name: "test_dataset".to_string(),
        geometry: GeoJson::Geometry(Geometry::new(Value::Polygon(vec![vec![
            vec![-122.5, 47.4],
            vec![-122.5, 47.8],
            vec![-122.2, 47.8],
            vec![-122.2, 47.4],
            vec![-122.5, 47.4],
        ]]))),
        api_key: api_key.clone(),
        input_srid: 4326,
        output_srid: 4326,
    };

    let req = test::TestRequest::post()
        .uri("/api/v1/spatial-query")
        .header("X-API-Key", &api_key)
        .set_json(&query)
        .to_request();

    let resp: Vec<Feature> = test::call_and_read_body_json(&app, req).await;
    assert_eq!(resp.len(), 1);
} 