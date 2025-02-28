use serde::{Deserialize, Serialize};
use uuid::Uuid;
use geojson::{GeoJson, Geometry};
use serde_json::Value;
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "name": "san_francisco_buildings",
    "description": "Building footprints in San Francisco"
}))]
pub struct Dataset {
    /// Unique identifier for the dataset
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub id: Uuid,
    /// Human-readable name for the dataset
    #[schema(example = "my_dataset")]
    pub name: String,
    /// API key with access to this dataset
    #[schema(example = "api_key_123")]
    pub api_key: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Feature {
    /// Unique identifier for the feature
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub id: Uuid,
    /// ID of the dataset this feature belongs to
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub dataset_id: Uuid,
    /// Human-readable identifier for the feature
    #[schema(example = "feature_123")]
    pub feature_id: String,
    /// GeoJSON geometry object
    #[schema(example = json!({
        "type": "Point",
        "coordinates": [-122.4194, 37.7749]
    }))]
    pub geometry: Option<Geometry>,
    /// Custom properties associated with the feature
    #[schema(example = json!({
        "name": "Ferry Building",
        "height": 75.0,
        "year_built": 1898,
        "historic": true
    }))]
    pub attributes: Value,
    /// Spatial Reference System Identifier (SRID) of input coordinates
    #[schema(example = 4326)]
    pub input_srid: i32,
    /// API key used to create or access this feature
    #[schema(example = "api_key_123")]
    pub api_key: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SpatialQuery {
    /// Spatial operation to perform
    #[schema(example = "intersects", value_type = String, format = "intersects|contains|within|touches|crosses|overlaps|disjoint")]
    pub operation: String,
    /// GeoJSON geometry to use for spatial query
    #[schema(example = json!({
        "type": "Polygon",
        "coordinates": [[
            [-122.4, 37.7],
            [-122.4, 37.8],
            [-122.3, 37.8],
            [-122.3, 37.7],
            [-122.4, 37.7]
        ]]
    }))]
    pub geometry: GeoJson,
    /// Name of the dataset to query
    #[schema(example = "my_dataset")]
    pub dataset_name: String,
    /// API key with access to the dataset
    #[schema(example = "api_key_123")]
    pub api_key: String,
    /// SRID of input geometry coordinates
    #[schema(default = 4326, example = 4326)]
    #[serde(default = "default_srid")]
    pub input_srid: i32,
    /// SRID for output geometries
    #[schema(default = 4326, example = 4326)]
    #[serde(default = "default_srid")]
    pub output_srid: i32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AttributeQuery {
    /// Name of the dataset to query
    #[schema(example = "my_dataset")]
    pub dataset_name: String,
    /// Query conditions for feature attributes. Supports operators: eq, gt, lt, gte, lte, in, like
    #[schema(example = json!({
        "height": { "gt": 50 },
        "year_built": { "lt": 1950 },
        "use_type": { "eq": "commercial" },
        "historic": { "eq": true }
    }))]
    pub conditions: Value,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateApiKeyRequest {
    /// Custom API key string. Must be unique
    #[schema(example = "new-api-key-123")]
    pub new_key: String,
    /// Time in seconds until the API key expires. Defaults to 3 months
    #[schema(default = 7776000, example = 7776000)]
    #[serde(default = "default_key_expiry")]
    pub key_expires_in_seconds: i32,
    /// Time in seconds that cached data remains valid. Defaults to 1 hour
    #[schema(default = 3600, example = 3600)]
    #[serde(default = "default_data_expiry")]
    pub data_expires_in_seconds: i32,
}

/// Default expiry time for API keys (3 months)
fn default_key_expiry() -> i32 {
    7776000 // 3 months in seconds
}

/// Default expiry time for cached data (1 hour)
fn default_data_expiry() -> i32 {
    3600 // 1 hour in seconds
}

/// Default Spatial Reference System Identifier (WGS84)
fn default_srid() -> i32 {
    4326 // WGS84 - World Geodetic System 1984
} 