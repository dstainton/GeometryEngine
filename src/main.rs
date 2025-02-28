use actix_web::{web, App, HttpServer, HttpResponse, HttpRequest};
use actix_web::dev::Payload;
use crate::models::{Dataset, Feature, SpatialQuery, CreateApiKeyRequest, AttributeQuery};
use crate::error::ApiError;
use std::time::Duration;
use serde_json::json;
use actix_web::FromRequest;
use futures::future::{ready, Ready};
use crate::rate_limit::RateLimiter;
use std::env;
use utoipa::{OpenApi, Modify};
use utoipa::openapi::security::{SecurityScheme, HttpAuthScheme, HttpBuilder};
use utoipa_swagger_ui::SwaggerUi;
use log::error;

mod models;
mod db;
mod cache;
mod error;
mod rate_limit;
mod projection;

// Create a custom header type
#[derive(Debug)]
struct ApiKey(String);

impl FromRequest for ApiKey {
    type Error = ApiError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let api_key = req
            .headers()
            .get("X-API-Key")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string())
            .ok_or(ApiError::InvalidApiKey);

        ready(api_key.map(ApiKey))
    }
}

// Add documentation for the health check endpoint
/// Check API health status
#[utoipa::path(
    get,
    path = "/health-check",
    responses(
        (status = 200, description = "API is healthy"),
        (status = 503, description = "API is unhealthy", body = ApiError)
    ),
    tag = "health"
)]
async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}

// Add documentation for readiness check
/// Check if API and its dependencies are ready
#[utoipa::path(
    get,
    path = "/readiness-check",
    responses(
        (status = 200, description = "API is ready"),
        (status = 503, description = "API is not ready", body = ApiError)
    ),
    tag = "health"
)]
async fn readiness_check(
    db: web::Data<db::Database>,
    cache: web::Data<cache::Cache>
) -> Result<HttpResponse, ApiError> {
    // Check DB connection
    db.check_connection().await?;
    
    // Check Redis connection
    cache.check_connection().await?;
    
    Ok(HttpResponse::Ok().finish())
}

// Add documentation for store feature endpoint
/// Store a new feature in a dataset
#[utoipa::path(
    post,
    path = "/api/v1/datasets/{dataset_name}/features",
    request_body(
        content = Feature,
        example = json!({
            "feature_id": "building_123",
            "geometry": {
                "type": "Polygon",
                "coordinates": [[
                    [-122.4194, 37.7749],
                    [-122.4195, 37.7749],
                    [-122.4195, 37.7750],
                    [-122.4194, 37.7750],
                    [-122.4194, 37.7749]
                ]]
            },
            "attributes": {
                "name": "Ferry Building",
                "height": 75.0,
                "year_built": 1898,
                "use_type": "commercial",
                "floors": 4,
                "historic": true
            },
            "input_srid": 4326
        })
    ),
    responses(
        (status = 201, description = "Feature created successfully",
         example = json!({
             "id": "123e4567-e89b-12d3-a456-426614174000",
             "feature_id": "building_123",
             "dataset_id": "123e4567-e89b-12d3-a456-426614174001"
         })
        ),
        (status = 400, description = "Invalid feature data", body = ApiError),
        (status = 401, description = "Invalid API key", body = ApiError),
        (status = 429, description = "Rate limit exceeded", body = ApiError)
    ),
    tag = "features"
)]
async fn store_feature(
    api_key: ApiKey,
    dataset_name: web::Path<String>,
    feature: web::Json<Feature>,
    db: web::Data<db::Database>,
    cache: web::Data<cache::Cache>,
    rate_limiter: web::Data<RateLimiter>,
) -> Result<HttpResponse, ApiError> {
    rate_limiter.check_rate_limit(api_key.0.as_str()).await?;

    // Validate API key
    if !db.validate_api_key(api_key.0.as_str()).await? {
        return Err(ApiError::InvalidApiKey);
    }

    // Store in database (using 4326 as default SRID if not specified)
    let feature_id = db.store_feature(
        feature.0.clone(),
        &dataset_name,
        &api_key.0,
        4326,  // Default to WGS84
    ).await?;

    // Cache the result
    cache.set_feature(&dataset_name, &feature.0).await?;

    Ok(HttpResponse::Created().json(feature_id))
}

/// Update an existing feature in a dataset
#[utoipa::path(
    put,
    path = "/api/v1/datasets/{dataset_name}/features/{feature_id}",
    request_body = Feature,
    params(
        ("dataset_name" = String, Path, description = "Name of the dataset"),
        ("feature_id" = String, Path, description = "ID of the feature to update"),
        ("X-API-Key" = String, Header, description = "API key for authentication")
    ),
    responses(
        (status = 200, description = "Feature updated successfully"),
        (status = 400, description = "Invalid feature data", body = ApiError),
        (status = 401, description = "Invalid API key", body = ApiError),
        (status = 404, description = "Feature not found", body = ApiError),
        (status = 429, description = "Rate limit exceeded", body = ApiError)
    ),
    tag = "features"
)]
async fn update_feature(
    api_key: ApiKey,
    path: web::Path<(String, String)>,
    feature: web::Json<Feature>,
    db: web::Data<db::Database>,
    cache: web::Data<cache::Cache>,
    rate_limiter: web::Data<RateLimiter>,
) -> Result<HttpResponse, ApiError> {
    rate_limiter.check_rate_limit(api_key.0.as_str()).await?;
    let (dataset_name, feature_id) = path.into_inner();

    // Validate API key
    if !db.validate_api_key(api_key.0.as_str()).await? {
        return Err(ApiError::InvalidApiKey);
    }

    // Update database with SRID
    db.update_feature(
        feature.0.clone(),
        &dataset_name,
        &api_key.0,
        feature.0.input_srid,
    ).await?;

    // Invalidate cache
    cache.invalidate_feature(&dataset_name, &feature_id).await?;

    Ok(HttpResponse::Ok().finish())
}

/// Perform a spatial query on features
#[utoipa::path(
    post,
    path = "/api/v1/spatial-query",
    request_body(
        content = SpatialQuery,
        example = json!({
            "operation": "intersects",
            "geometry": {
                "type": "Polygon",
                "coordinates": [[
                    [-122.4, 37.7],
                    [-122.4, 37.8],
                    [-122.3, 37.8],
                    [-122.3, 37.7],
                    [-122.4, 37.7]
                ]]
            },
            "dataset_name": "my_dataset",
            "input_srid": 4326,
            "output_srid": 4326
        })
    ),
    params(
        ("X-API-Key" = String, Header, description = "API key for authentication")
    ),
    responses(
        (status = 200, description = "Query results", body = Vec<Feature>),
        (status = 400, description = "Invalid query parameters", body = ApiError),
        (status = 401, description = "Invalid API key", body = ApiError),
        (status = 429, description = "Rate limit exceeded", body = ApiError)
    ),
    tag = "spatial"
)]
async fn spatial_query(
    api_key: ApiKey,
    query: web::Json<SpatialQuery>,
    db: web::Data<db::Database>,
    cache: web::Data<cache::Cache>,
    rate_limiter: web::Data<RateLimiter>,
) -> Result<HttpResponse, ApiError> {
    rate_limiter.check_rate_limit(api_key.0.as_str()).await?;
    // Validate API key
    if !db.validate_api_key(api_key.0.as_str()).await? {
        return Err(ApiError::InvalidApiKey);
    }

    // Set api_key in query for dataset validation
    let mut query = query.into_inner();
    query.api_key = api_key.0;

    // Perform spatial query
    let results = db.spatial_query(&query).await?;

    // Cache results for each feature
    for feature in &results {
        cache.set_feature(&query.dataset_name, feature).await?;
    }

    Ok(HttpResponse::Ok().json(results))
}

/// Create a new dataset
#[utoipa::path(
    post,
    path = "/api/v1/datasets",
    request_body(
        content = Dataset,
        example = json!({
            "name": "san_francisco_buildings",
            "description": "Building footprints in San Francisco",
            "metadata": {
                "source": "City of San Francisco",
                "last_updated": "2024-02-27",
                "crs": "EPSG:4326"
            }
        })
    ),
    params(
        ("X-API-Key" = String, Header, description = "API key for authentication")
    ),
    responses(
        (status = 201, description = "Dataset created successfully",
         example = json!({
             "id": "123e4567-e89b-12d3-a456-426614174000",
             "name": "san_francisco_buildings",
             "api_key": "dataset_key_123"
         })
        ),
        (status = 401, description = "Invalid API key", body = ApiError),
        (status = 409, description = "Dataset already exists", body = ApiError),
        (status = 429, description = "Rate limit exceeded", body = ApiError)
    ),
    tag = "datasets"
)]
async fn create_dataset(
    api_key: ApiKey,
    dataset: web::Json<Dataset>,
    db: web::Data<db::Database>,
    rate_limiter: web::Data<RateLimiter>,
) -> Result<HttpResponse, ApiError> {
    rate_limiter.check_rate_limit(api_key.0.as_str()).await?;
    
    if !db.validate_api_key(api_key.0.as_str()).await? {
        return Err(ApiError::InvalidApiKey);
    }

    db.create_dataset(&dataset).await?;
    Ok(HttpResponse::Created().json(dataset.0))
}

/// List all datasets accessible with the provided API key
#[utoipa::path(
    get,
    path = "/api/v1/datasets",
    params(
        ("X-API-Key" = String, Header, description = "API key for authentication")
    ),
    responses(
        (status = 200, description = "List of datasets", body = Vec<Dataset>),
        (status = 401, description = "Invalid API key", body = ApiError),
        (status = 429, description = "Rate limit exceeded", body = ApiError)
    ),
    tag = "datasets"
)]
async fn list_datasets(
    api_key: ApiKey,
    db: web::Data<db::Database>,
    rate_limiter: web::Data<RateLimiter>,
) -> Result<HttpResponse, ApiError> {
    rate_limiter.check_rate_limit(api_key.0.as_str()).await?;
    
    if !db.validate_api_key(api_key.0.as_str()).await? {
        return Err(ApiError::InvalidApiKey);
    }

    let datasets = db.get_datasets(api_key.0.as_str()).await?;
    Ok(HttpResponse::Ok().json(datasets))
}

/// Create a new API key
#[utoipa::path(
    post,
    path = "/api/v1/api-keys",
    request_body(
        content = CreateApiKeyRequest,
        example = json!({
            "new_key": "user_key_123",
            "key_expires_in_seconds": 2592000,  // 30 days
            "data_expires_in_seconds": 3600     // 1 hour
        })
    ),
    responses(
        (status = 201, description = "API key created successfully",
         example = json!({
             "key": "user_key_123",
             "key_expires_in_seconds": 2592000,
             "data_expires_in_seconds": 3600,
             "created_at": "2024-02-27T15:00:00Z"
         })
        ),
        (status = 401, description = "Invalid master API key", body = ApiError),
        (status = 429, description = "Rate limit exceeded", body = ApiError)
    ),
    tag = "api-keys"
)]
async fn create_api_key(
    master_key: ApiKey,
    request: web::Json<CreateApiKeyRequest>,
    db: web::Data<db::Database>,
    rate_limiter: web::Data<RateLimiter>,
) -> Result<HttpResponse, ApiError> {
    rate_limiter.check_rate_limit(master_key.0.as_str()).await?;
    
    db.create_api_key(
        &request.new_key,
        request.key_expires_in_seconds,
        request.data_expires_in_seconds,
        master_key.0.as_str()
    ).await?;

    Ok(HttpResponse::Created().json(json!({ 
        "key": request.new_key,
        "key_expires_in_seconds": request.key_expires_in_seconds,
        "data_expires_in_seconds": request.data_expires_in_seconds
    })))
}

async fn run_cleanup_task(db: db::Database) {
    let cleanup_interval = Duration::from_secs(3600); // Run cleanup every hour
    loop {
        if let Err(e) = db.cleanup_expired_keys().await {
            error!("Failed to cleanup expired keys: ({:?})", e);
        }
        tokio::time::sleep(cleanup_interval).await;
    }
}

// Add documentation for get feature endpoint
/// Get a feature by ID from a dataset
#[utoipa::path(
    get,
    path = "/api/v1/datasets/{dataset_name}/features/{feature_id}",
    params(
        ("dataset_name" = String, Path, description = "Name of the dataset"),
        ("feature_id" = String, Path, description = "ID of the feature"),
        ("X-API-Key" = String, Header, description = "API key for authentication")
    ),
    responses(
        (status = 200, description = "Feature found", body = Feature,
         example = json!({
             "id": "123e4567-e89b-12d3-a456-426614174000",
             "dataset_id": "123e4567-e89b-12d3-a456-426614174001",
             "feature_id": "building_123",
             "geometry": {
                 "type": "Point",
                 "coordinates": [-122.4194, 37.7749]
             },
             "attributes": {
                 "name": "Ferry Building",
                 "height": 75.0,
                 "year_built": 1898
             },
             "input_srid": 4326
         })
        ),
        (status = 401, description = "Invalid API key", body = ApiError),
        (status = 404, description = "Feature not found", body = ApiError),
        (status = 429, description = "Rate limit exceeded", body = ApiError)
    ),
    tag = "features"
)]
async fn get_feature(
    api_key: ApiKey,
    path: web::Path<(String, String)>,
    db: web::Data<db::Database>,
    cache: web::Data<cache::Cache>,
    rate_limiter: web::Data<RateLimiter>,
) -> Result<HttpResponse, ApiError> {
    rate_limiter.check_rate_limit(api_key.0.as_str()).await?;
    
    let (dataset_name, feature_id) = path.into_inner();

    // Validate API key
    if !db.validate_api_key(api_key.0.as_str()).await? {
        return Err(ApiError::InvalidApiKey);
    }

    // Try cache first
    if let Some(feature) = cache.get_feature(&dataset_name, &feature_id).await? {
        return Ok(HttpResponse::Ok().json(feature));
    }

    // If not in cache, get from DB
    // (You'll need to add a get_feature method to Database)
    let feature = db.get_feature(&dataset_name, &feature_id).await?
        .ok_or_else(|| ApiError::FeatureNotFound(feature_id.clone()))?;

    // Cache the result
    cache.set_feature(&dataset_name, &feature).await?;

    Ok(HttpResponse::Ok().json(feature))
}

/// Query features by attributes
#[utoipa::path(
    post,
    path = "/api/v1/attribute-query",
    request_body(
        content = AttributeQuery,
        example = json!({
            "dataset_name": "san_francisco_buildings",
            "conditions": {
                "height": { "gt": 50 },
                "year_built": { "lt": 1950 },
                "use_type": { "eq": "commercial" },
                "historic": { "eq": true }
            }
        })
    ),
    responses(
        (status = 200, description = "Query results",
         example = json!([
             {
                 "id": "123e4567-e89b-12d3-a456-426614174000",
                 "feature_id": "building_123",
                 "geometry": {
                     "type": "Point",
                     "coordinates": [-122.4194, 37.7749]
                 },
                 "attributes": {
                     "name": "Ferry Building",
                     "height": 75.0,
                     "year_built": 1898
                 }
             }
         ])
        ),
        (status = 400, description = "Invalid query parameters", body = ApiError),
        (status = 401, description = "Invalid API key", body = ApiError),
        (status = 429, description = "Rate limit exceeded", body = ApiError)
    ),
    tag = "attributes"
)]
async fn attribute_query(
    api_key: ApiKey,
    query: web::Json<AttributeQuery>,
    db: web::Data<db::Database>,
    cache: web::Data<cache::Cache>,
    rate_limiter: web::Data<RateLimiter>,
) -> Result<HttpResponse, ApiError> {
    rate_limiter.check_rate_limit(api_key.0.as_str()).await?;
    // Validate API key
    if !db.validate_api_key(api_key.0.as_str()).await? {
        return Err(ApiError::InvalidApiKey);
    }

    // Perform attribute query
    let results = db.attribute_query(&query).await?;

    // Cache results for each feature
    for feature in &results {
        cache.set_feature(&query.dataset_name, feature).await?;
    }

    Ok(HttpResponse::Ok().json(results))
}

// Generate API documentation
#[derive(OpenApi)]
#[openapi(
    paths(
        health_check,
        readiness_check,
        create_api_key,
        create_dataset,
        list_datasets,
        store_feature,
        update_feature,
        get_feature,
        spatial_query,
        attribute_query
    ),
    components(
        schemas(Feature, Dataset, SpatialQuery, CreateApiKeyRequest, ApiError, AttributeQuery)
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "api-keys", description = "API key management"),
        (name = "datasets", description = "Dataset operations"),
        (name = "features", description = "Feature operations"),
        (name = "spatial", description = "Spatial queries"),
        (name = "attributes", description = "Attribute-based queries")
    ),
    info(
        title = "Geospatial API",
        version = "1.0.0",
        description = "High-performance REST API for geospatial data management",
        license(
            name = "MIT",
            url = "https://opensource.org/licenses/MIT"
        ),
        contact(
            name = "API Support",
            email = "support@yourdomain.com",
            url = "https://api.yourdomain.com/support"
        )
    ),
    servers(
        (url = "http://localhost:8080", description = "Development server"),
        (url = "https://api.yourdomain.com", description = "Production server")
    ),
    external_docs(
        url = "https://api.yourdomain.com/docs",
        description = "Additional documentation"
    )
)]
struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "api_key",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("API Key")
                        .description(Some("API key for authentication"))
                        .build()
                )
            );
            components.add_security_scheme(
                "master_key",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("Master API Key")
                        .description(Some("Master API key for administrative operations"))
                        .build()
                )
            );
        }
        
        // Add security requirement for all operations except health check
        for (path, item) in openapi.paths.paths.iter_mut() {
            if !path.contains("health-check") {
                let security = if path.contains("api-keys") {
                    vec![utoipa::openapi::security::SecurityRequirement::new(
                        "master_key".to_string(),
                        Vec::<String>::new()
                    )]
                } else {
                    vec![utoipa::openapi::security::SecurityRequirement::new(
                        "api_key".to_string(),
                        Vec::<String>::new()
                    )]
                };
                
                if let Some(op) = item.operations.get_mut(&utoipa::openapi::PathItemType::Get) { op.security = Some(security.clone()); }
                if let Some(op) = item.operations.get_mut(&utoipa::openapi::PathItemType::Post) { op.security = Some(security.clone()); }
                if let Some(op) = item.operations.get_mut(&utoipa::openapi::PathItemType::Put) { op.security = Some(security.clone()); }
                if let Some(op) = item.operations.get_mut(&utoipa::openapi::PathItemType::Delete) { op.security = Some(security.clone()); }
            }
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    let db = db::Database::new()
        .await
        .expect("Failed to create database connection");
    
    // Initialize database and create master key if needed
    db.initialize()
        .await
        .expect("Failed to initialize database");
    
    let cache = cache::Cache::new()
        .expect("Failed to create cache");
    
    let requests_per_minute = env::var("RATE_LIMIT_PER_MINUTE")
        .unwrap_or_else(|_| "100".to_string())
        .parse::<u32>()
        .expect("RATE_LIMIT_PER_MINUTE must be a number");
    
    let rate_limiter = web::Data::new(
        RateLimiter::new(cache.clone(), requests_per_minute)
    );

    // Start cleanup task
    let cleanup_db = db.clone();
    tokio::spawn(async move {
        run_cleanup_task(cleanup_db).await;
    });

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(db.clone()))
            .app_data(web::Data::new(cache.clone()))
            .app_data(rate_limiter.clone())
            // Add Swagger UI
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-docs/openapi.json", ApiDoc::openapi())
            )
            .service(
                web::scope("/api/v1")
                    .route("/api-keys", web::post().to(create_api_key))
                    .route("/datasets", web::post().to(create_dataset))
                    .route("/datasets", web::get().to(list_datasets))
                    .route("/datasets/{dataset_name}/features", web::post().to(store_feature))
                    .route("/datasets/{dataset_name}/features/{feature_id}", web::put().to(update_feature))
                    .route("/spatial-query", web::post().to(spatial_query))
                    .route("/datasets/{dataset_name}/features/{feature_id}", web::get().to(get_feature))
                    .route("/attribute-query", web::post().to(attribute_query))
                    .route("/health-check", web::get().to(health_check))
                    .route("/readiness-check", web::get().to(readiness_check))
            )
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
} 