use super::*;
use crate::error::ApiError;
use actix_web::http::StatusCode;

#[test]
fn test_invalid_coordinate_system_error() {
    let error = ApiError::InvalidCoordinateSystem {
        srid: 1234,
        message: "Unsupported coordinate system".to_string(),
        supported_systems: vec![4326, 3857],
    };

    let response = error.error_response();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = response.into_body();
    let json: serde_json::Value = serde_json::from_slice(body.as_ref()).unwrap();
    
    assert!(json["error"].as_str().unwrap().contains("Invalid SRID 1234"));
    assert!(json["supported_systems"].as_array().unwrap().contains(&json!(4326)));
    assert_eq!(
        json["documentation_url"].as_str().unwrap(),
        "https://api.yourdomain.com/docs/coordinate-systems"
    );
}

#[test]
fn test_coordinate_transform_error() {
    let error = ApiError::CoordinateTransformError("Failed to transform".to_string());
    let response = error.error_response();
    
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    
    let body = response.into_body();
    let json: serde_json::Value = serde_json::from_slice(body.as_ref()).unwrap();
    assert!(json["error"].as_str().unwrap().contains("Failed to transform"));
} 