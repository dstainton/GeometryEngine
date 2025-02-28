use thiserror::Error;
use actix_web::{ResponseError, HttpResponse};
use serde_json::json;
use deadpool_postgres::{PoolError, CreatePoolError};
use tokio_postgres::Error as PgError;
use redis::RedisError;
use prometheus;
use utoipa::ToSchema;
use serde::Serialize;

#[derive(Error, Debug, ToSchema, Serialize)]
pub enum ApiError {
    // Database-related errors
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("Pool error: {0}")]
    Pool(String),
    
    #[error("Failed to create connection pool: {0}")]
    CreatePool(String),
    
    // Cache-related errors
    #[error("Cache error: {0}")]
    Cache(String),
    
    // Authentication and authorization errors
    #[error("Invalid API key")]
    InvalidApiKey,
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    // Resource not found errors
    #[error("Dataset not found: {0}")]
    DatasetNotFound(String),
    
    #[error("Feature not found: {0}")]
    FeatureNotFound(String),
    
    // Resource conflict errors
    #[error("Dataset already exists: {0}")]
    DatasetExists(String),
    
    #[error("Feature ID already exists in dataset: {0}")]
    FeatureExists(String),
    
    // Validation errors
    #[error("Invalid geometry: {0}")]
    InvalidGeometry(String),
    
    #[error("Invalid coordinate system {srid}: {message}")]
    InvalidCoordinateSystem {
        srid: i32,
        message: String,
        supported_systems: Vec<i32>,
    },
    
    #[error("Coordinate transformation error: {0}")]
    CoordinateTransformError(String),
    
    // Serialization errors
    #[error("JSON error: {0}")]
    Json(String),
    
    #[error("Invalid query: {0}")]
    InvalidQuery(String),
}

impl From<PgError> for ApiError {
    fn from(err: PgError) -> Self {
        ApiError::Database(err.to_string())
    }
}

impl From<PoolError> for ApiError {
    fn from(err: PoolError) -> Self {
        ApiError::Pool(err.to_string())
    }
}

impl From<CreatePoolError> for ApiError {
    fn from(err: CreatePoolError) -> Self {
        ApiError::CreatePool(err.to_string())
    }
}

impl From<RedisError> for ApiError {
    fn from(err: RedisError) -> Self {
        ApiError::Cache(err.to_string())
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        ApiError::Json(err.to_string())
    }
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        match self {
            // Authentication errors
            Self::InvalidApiKey => HttpResponse::Unauthorized().json(json!({
                "error": self.to_string()
            })),
            
            // Not found errors
            Self::DatasetNotFound(_) | Self::FeatureNotFound(_) => {
                HttpResponse::NotFound().json(json!({
                    "error": self.to_string()
                }))
            }
            
            // Rate limiting
            Self::RateLimitExceeded => {
                HttpResponse::TooManyRequests()
                    .append_header(("X-RateLimit-Limit", "100"))
                    .append_header(("X-RateLimit-Reset", "60"))
                    .json(json!({
                        "error": self.to_string()
                    }))
            },
            
            // Invalid coordinate system
            Self::InvalidCoordinateSystem { srid, message, supported_systems } => {
                HttpResponse::BadRequest().json(json!({
                    "error": format!("Invalid SRID {}: {}", srid, message),
                    "supported_systems": supported_systems,
                    "documentation_url": "https://api.yourdomain.com/docs/coordinate-systems"
                }))
            },
            
            Self::CoordinateTransformError(msg) => {
                HttpResponse::BadRequest().json(json!({
                    "error": format!("Coordinate transformation failed: {}", msg)
                }))
            },
            
            // All other errors
            _ => HttpResponse::InternalServerError().json(json!({
                "error": self.to_string()
            })),
        }
    }
}

#[derive(Debug, Error)]
pub enum MetricsError {
    #[error("Prometheus error: {0}")]
    Prometheus(#[from] prometheus::Error),
} 