use serde::{Deserialize, Serialize};
use uuid::Uuid;
use geojson::{GeoJson, Geometry};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct Dataset {
    pub id: Uuid,
    pub name: String,
    pub api_key: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Feature {
    pub id: Uuid,
    pub dataset_id: Uuid,
    pub feature_id: String,
    pub geometry: Option<Geometry>,
    pub attributes: Value,
    pub input_srid: i32,
    pub api_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpatialQuery {
    pub operation: String, // intersects, contains, within, etc.
    pub geometry: GeoJson,
    pub dataset_name: String,
    pub api_key: String,
    #[serde(default = "default_srid")]
    pub input_srid: i32,
    #[serde(default = "default_srid")]
    pub output_srid: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AttributeQuery {
    pub dataset_name: String,
    pub conditions: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateApiKeyRequest {
    pub new_key: String,
    #[serde(default = "default_key_expiry")]
    pub key_expires_in_seconds: i32,
    #[serde(default = "default_data_expiry")]
    pub data_expires_in_seconds: i32,
}

fn default_key_expiry() -> i32 {
    7776000 // 3 months in seconds
}

fn default_data_expiry() -> i32 {
    3600 // 1 hour in seconds
}

fn default_srid() -> i32 {
    4326 // WGS84
} 