use super::*;
use actix_web::{test, web, App};
use crate::models::{Dataset, Feature, CreateApiKeyRequest};
use geojson::{GeoJson, Geometry, Value};
use serde_json::json;

async fn setup_test_app() -> test::TestApp {
    let db = setup_test_db().await;
    let cache = setup_test_cache();
    let rate_limiter = web::Data::new(RateLimiter::new(cache.clone(), 100));

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
async fn test_create_api_key() {
    let app = setup_test_app().await;
    let master_key = std::env::var("MASTER_API_KEY").unwrap();

    let req = test::TestRequest::post()
        .uri("/api/v1/api-keys")
        .header("X-API-Key", &master_key)
        .set_json(&CreateApiKeyRequest {
            new_key: "test_key".to_string(),
            key_expires_in_seconds: 3600,
            data_expires_in_seconds: 3600,
        })
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}

#[tokio::test]
async fn test_create_and_list_datasets() {
    let app = setup_test_app().await;
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

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    // List datasets
    let req = test::TestRequest::get()
        .uri("/api/v1/datasets")
        .header("X-API-Key", &api_key)
        .to_request();

    let resp: Vec<Dataset> = test::call_and_read_body_json(&app, req).await;
    assert_eq!(resp.len(), 1);
    assert_eq!(resp[0].name, "test_dataset");
}

#[tokio::test]
async fn test_feature_crud() {
    let app = setup_test_app().await;
    let api_key = generate_test_api_key();
    let dataset_name = "test_dataset";

    // First create a dataset
    let dataset = Dataset {
        id: Uuid::new_v4(),
        name: dataset_name.to_string(),
        api_key: api_key.clone(),
    };

    let req = test::TestRequest::post()
        .uri("/api/v1/datasets")
        .header("X-API-Key", &api_key)
        .set_json(&dataset)
        .to_request();

    test::call_service(&app, req).await;

    // Create feature
    let feature = Feature {
        id: Uuid::new_v4(),
        dataset_id: dataset.id,
        feature_id: "test_feature".to_string(),
        geometry: GeoJson::Geometry(Geometry::new(Value::Point(vec![0.0, 0.0]))),
        attributes: json!({"name": "test point"}),
    };

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/datasets/{}/features", dataset_name))
        .header("X-API-Key", &api_key)
        .set_json(&feature)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    // Get feature
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/datasets/{}/features/test_feature", dataset_name))
        .header("X-API-Key", &api_key)
        .to_request();

    let resp: Feature = test::call_and_read_body_json(&app, req).await;
    assert_eq!(resp.feature_id, "test_feature");
}

#[tokio::test]
async fn test_store_feature_with_projection() {
    let app = setup_test_app().await;
    let api_key = generate_test_api_key();

    // First create a dataset
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

    // Create feature in Web Mercator
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

    let req = test::TestRequest::post()
        .uri("/api/v1/datasets/test_dataset/features")
        .header("X-API-Key", &api_key)
        .set_json(&feature)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    // Get feature and verify it's in WGS84
    let req = test::TestRequest::get()
        .uri("/api/v1/datasets/test_dataset/features/test_feature")
        .header("X-API-Key", &api_key)
        .to_request();

    let resp: Feature = test::call_and_read_body_json(&app, req).await;
    if let GeoJson::Geometry(geom) = resp.geometry {
        if let Value::Point(coords) = geom.value {
            // Check if coordinates are roughly -122° longitude, 45° latitude
            assert!((coords[0] - (-122.0)).abs() < 0.1);
            assert!((coords[1] - 45.0).abs() < 0.1);
        }
    }
}

#[tokio::test]
async fn test_spatial_query_with_projection() {
    let app = setup_test_app().await;
    let api_key = generate_test_api_key();

    // Create dataset and feature in WGS84
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

    let feature = Feature {
        id: Uuid::new_v4(),
        dataset_id: dataset.id,
        feature_id: "test_feature".to_string(),
        geometry: GeoJson::Geometry(Geometry::new(Value::Point(vec![-122.0, 45.0]))),
        attributes: json!({"name": "Portland"}),
        input_srid: 4326,
    };

    let req = test::TestRequest::post()
        .uri("/api/v1/datasets/test_dataset/features")
        .header("X-API-Key", &api_key)
        .set_json(&feature)
        .to_request();

    test::call_service(&app, req).await;

    // Query in Web Mercator and get results in WGS84
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

    let req = test::TestRequest::post()
        .uri("/api/v1/spatial-query")
        .header("X-API-Key", &api_key)
        .set_json(&query)
        .to_request();

    let resp: Vec<Feature> = test::call_and_read_body_json(&app, req).await;
    assert_eq!(resp.len(), 1);

    // Verify the returned feature is in WGS84
    if let GeoJson::Geometry(geom) = &resp[0].geometry {
        if let Value::Point(coords) = &geom.value {
            assert!((coords[0] - (-122.0)).abs() < 0.1);
            assert!((coords[1] - 45.0).abs() < 0.1);
        }
    }
} 