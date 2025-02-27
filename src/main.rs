use actix_web::{web, App, HttpServer, HttpResponse, HttpRequest};
use actix_web::dev::Payload;
use crate::models::{Dataset, Feature, SpatialQuery, CreateApiKeyRequest};
use crate::error::ApiError;
use std::time::Duration;
use serde_json::json;
use actix_web::FromRequest;
use futures::future::{ready, Ready};
use crate::rate_limit::RateLimiter;
use std::env;

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
        tokio::time::sleep(cleanup_interval).await;
        if let Err(e) = db.cleanup_expired_keys().await {
            log::error!("Error during cleanup: {}", e);
        }
    }
}

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

async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}

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
            .service(
                web::scope("/api/v1")
                    .route("/api-keys", web::post().to(create_api_key))
                    .route("/datasets", web::post().to(create_dataset))
                    .route("/datasets", web::get().to(list_datasets))
                    .route("/datasets/{dataset_name}/features", web::post().to(store_feature))
                    .route("/datasets/{dataset_name}/features/{feature_id}", web::put().to(update_feature))
                    .route("/spatial-query", web::post().to(spatial_query))
                    .route("/datasets/{dataset_name}/features/{feature_id}", web::get().to(get_feature))
                    .route("/health-check", web::get().to(health_check))
                    .route("/readiness-check", web::get().to(readiness_check))
            )
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
} 