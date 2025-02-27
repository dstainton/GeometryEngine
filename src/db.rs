use crate::models::{Dataset, Feature, SpatialQuery};
use crate::error::ApiError;
use deadpool_postgres::{Pool, Config, Runtime};
use tokio_postgres::NoTls;
use std::env;
use serde_json::Value;
use postgres_types::Json;
use geojson::GeoJson;
use crate::projection::ProjectionTransformer;
use uuid::Uuid;

#[derive(Clone)]
pub struct Database {
    // Connection pool for PostgreSQL with PostGIS
    pool: Pool,
}

#[allow(dead_code)]
impl Database {
    // Initialize database connection pool from environment variables
    pub async fn new() -> Result<Self, ApiError> {
        let mut cfg = Config::new();
        cfg.host = Some(env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string()));
        cfg.port = Some(env::var("POSTGRES_PORT").unwrap_or_else(|_| "5432".to_string()).parse().unwrap());
        cfg.dbname = Some(env::var("POSTGRES_DB").unwrap_or_else(|_| "geospatial".to_string()));
        cfg.user = Some(env::var("POSTGRES_USER").unwrap_or_else(|_| "postgres".to_string()));
        cfg.password = Some(env::var("POSTGRES_PASSWORD").unwrap_or_default());

        let pool = cfg.create_pool(Some(Runtime::Tokio1), NoTls)
            .map_err(ApiError::CreatePool)?;
        Ok(Self { pool })
    }

    // Ensure master API key exists in database
    pub async fn initialize(&self) -> Result<(), ApiError> {
        let client = self.pool.get().await?;
        
        // Get master API key from environment
        let master_key = env::var("MASTER_API_KEY")
            .expect("MASTER_API_KEY must be set");

        // Check if master key exists, if not create it
        let exists: bool = client
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM api_keys WHERE key = $1)",
                &[&master_key]
            )
            .await?
            .get(0);

        if !exists {
            client
                .execute(
                    "INSERT INTO api_keys (key) VALUES ($1)",
                    &[&master_key]
                )
                .await?;
            log::info!("Created master API key");
        }

        Ok(())
    }

    pub async fn is_master_key(&self, api_key: &str) -> bool {
        api_key == env::var("MASTER_API_KEY").unwrap_or_default()
    }

    pub async fn validate_api_key(&self, api_key: &str) -> Result<bool, ApiError> {
        if self.is_master_key(api_key).await {
            return Ok(true);
        }

        let client = self.pool.get().await?;
        
        // Check if key exists and hasn't expired
        let row = client
            .query_one(
                "SELECT EXISTS(
                    SELECT 1 
                    FROM api_keys 
                    WHERE key = $1 
                    AND last_used_at + (key_expires_in_seconds || ' seconds')::interval >= CURRENT_TIMESTAMP
                )",
                &[&api_key]
            )
            .await?;

        let valid = row.get::<_, bool>(0);
        
        if valid {
            // Update last_used_at if key is valid
            self.update_key_last_used(api_key).await?;
        }

        Ok(valid)
    }

    pub async fn create_api_key(
        &self, 
        new_key: &str, 
        key_expires_in_seconds: i32,
        data_expires_in_seconds: i32,
        master_key: &str
    ) -> Result<(), ApiError> {
        if !self.is_master_key(master_key).await {
            return Err(ApiError::InvalidApiKey);
        }

        let client = self.pool.get().await?;
        client
            .execute(
                "INSERT INTO api_keys (key, key_expires_in_seconds, data_expires_in_seconds) 
                 VALUES ($1, $2, $3)",
                &[&new_key, &key_expires_in_seconds, &data_expires_in_seconds]
            )
            .await?;
        
        Ok(())
    }

    pub async fn update_key_last_used(&self, api_key: &str) -> Result<(), ApiError> {
        let client = self.pool.get().await?;
        client
            .execute(
                "UPDATE api_keys SET last_used_at = CURRENT_TIMESTAMP WHERE key = $1",
                &[&api_key]
            )
            .await?;
        Ok(())
    }

    pub async fn cleanup_expired_keys(&self) -> Result<(), ApiError> {
        let client = self.pool.get().await?;
        
        // Don't delete the master key
        let master_key = env::var("MASTER_API_KEY").expect("MASTER_API_KEY must be set");
        
        // Delete features associated with expired data or expired keys
        client
            .execute(
                "DELETE FROM features 
                 WHERE dataset_id IN (
                     SELECT d.id 
                     FROM datasets d 
                     JOIN api_keys k ON d.api_key = k.key 
                     WHERE k.key != $1 
                     AND (
                         k.last_used_at + (k.key_expires_in_seconds || ' seconds')::interval < CURRENT_TIMESTAMP
                         OR 
                         k.last_used_at + (k.data_expires_in_seconds || ' seconds')::interval < CURRENT_TIMESTAMP
                     )
                 )",
                &[&master_key]
            )
            .await?;

        // Delete datasets with expired data or expired keys
        client
            .execute(
                "DELETE FROM datasets 
                 WHERE api_key IN (
                     SELECT key 
                     FROM api_keys 
                     WHERE key != $1 
                     AND (
                         last_used_at + (key_expires_in_seconds || ' seconds')::interval < CURRENT_TIMESTAMP
                         OR 
                         last_used_at + (data_expires_in_seconds || ' seconds')::interval < CURRENT_TIMESTAMP
                     )
                 )",
                &[&master_key]
            )
            .await?;

        // Delete expired API keys
        client
            .execute(
                "DELETE FROM api_keys 
                 WHERE key != $1 
                 AND last_used_at + (key_expires_in_seconds || ' seconds')::interval < CURRENT_TIMESTAMP",
                &[&master_key]
            )
            .await?;

        Ok(())
    }

    // Store feature with SRID transformation to WGS84
    pub async fn store_feature(&self, mut feature: Feature, dataset_name: &str, api_key: &str, input_srid: i32) -> Result<Uuid, ApiError> {
        let client = self.pool.get().await?;
        
        // Validate GeoJSON before storing
        let geometry = feature.geometry.clone()
            .ok_or_else(|| ApiError::InvalidGeometry("Missing geometry".to_string()))?;
        validate_geojson(&GeoJson::Geometry(geometry))?;
        
        // Validate and transform coordinates
        ProjectionTransformer::validate_srid(input_srid)?;
        
        // Check if dataset exists and get its ID
        let dataset = self.get_dataset_by_name(dataset_name, api_key).await?
            .ok_or_else(|| ApiError::DatasetNotFound(dataset_name.to_string()))?;

        // Check if feature_id already exists in this dataset
        let exists = client
            .query_one(
                "SELECT EXISTS(
                    SELECT 1 FROM features f 
                    JOIN datasets d ON f.dataset_id = d.id 
                    WHERE d.name = $1 AND f.feature_id = $2
                )",
                &[&dataset_name, &feature.feature_id],
            )
            .await?
            .get::<_, bool>(0);

        if exists {
            return Err(ApiError::FeatureExists(feature.feature_id.clone()));
        }

        // Insert the feature
        feature.dataset_id = dataset.id;
        let id = Uuid::new_v4();
        
        // Convert geometry to GeoJSON string with explicit error handling
        let geometry_json = serde_json::to_string(&feature.geometry
            .ok_or_else(|| ApiError::InvalidGeometry("Missing geometry".to_string()))?
        ).map_err(|e| ApiError::CoordinateTransformError(format!("Failed to serialize geometry: {}", e)))?;

        client.execute(
            "INSERT INTO features (id, dataset_id, feature_id, geometry, attributes) 
             VALUES ($1, $2, $3, ST_Transform(ST_SetSRID(ST_GeomFromGeoJSON($4), $5), 4326), $6)",
            &[
                &id,
                &feature.dataset_id,
                &feature.feature_id,
                &geometry_json,
                &input_srid,
                &Json(feature.attributes),
            ],
        ).await?;

        Ok(id)
    }

    pub async fn update_feature(&self, feature: Feature, dataset_name: &str, api_key: &str, input_srid: i32) -> Result<(), ApiError> {
        let client = self.pool.get().await?;

        // Validate GeoJSON before updating
        let geometry = feature.geometry.clone()
            .ok_or_else(|| ApiError::InvalidGeometry("Missing geometry".to_string()))?;
        validate_geojson(&GeoJson::Geometry(geometry))?;
        
        // Check if dataset exists and get its ID
        let dataset = self.get_dataset_by_name(dataset_name, api_key).await?
            .ok_or_else(|| ApiError::DatasetNotFound(dataset_name.to_string()))?;

        // Convert geometry to GeoJSON string with explicit error handling
        let geometry_json = serde_json::to_string(&feature.geometry
            .ok_or_else(|| ApiError::InvalidGeometry("Missing geometry".to_string()))?
        ).map_err(|e| ApiError::InvalidGeometry(format!("Failed to serialize geometry: {}", e)))?;

        // Update the feature
        let rows_affected = client
            .execute(
                "UPDATE features 
                 SET geometry = ST_Transform(ST_SetSRID(ST_GeomFromGeoJSON($1), $2), 4326), 
                     attributes = $3 
                 WHERE dataset_id = $4 AND feature_id = $5",
                &[
                    &geometry_json,
                    &input_srid,
                    &Json(feature.attributes.clone()),
                    &dataset.id,
                    &feature.feature_id,
                ],
            )
            .await?;

        if rows_affected == 0 {
            return Err(ApiError::FeatureNotFound(feature.feature_id.clone()));
        }

        Ok(())
    }

    // Execute spatial query with SRID transformations
    pub async fn spatial_query(&self, query: &SpatialQuery) -> Result<Vec<Feature>, ApiError> {
        // Validate SRIDs
        ProjectionTransformer::validate_srid(query.input_srid)?;
        ProjectionTransformer::validate_srid(query.output_srid)?;

        let client = self.pool.get().await?;
        
        let sql = match query.operation.as_str() {
            "intersects" => "ST_Intersects",
            "contains" => "ST_Contains",
            "within" => "ST_Within",
            _ => return Err(ApiError::InvalidGeometry("Invalid spatial operation".to_string())),
        };

        let query_sql = format!(
            "SELECT f.id, f.dataset_id, f.feature_id, d.api_key,
             ST_AsGeoJSON(ST_Transform(f.geometry, {})) as geometry, 
             f.attributes 
             FROM features f 
             JOIN datasets d ON f.dataset_id = d.id 
             WHERE d.name = $1 
             AND {}(
                 ST_Transform(f.geometry, {}),
                 ST_Transform(ST_SetSRID(ST_GeomFromGeoJSON($2), {}), {})
             )",
            query.output_srid,
            sql,
            query.input_srid,
            query.input_srid,
            query.input_srid
        );

        let rows = client
            .query(
                &query_sql,
                &[&query.dataset_name, &query.geometry.to_string()],
            )
            .await?;

        let features = rows
            .into_iter()
            .map(|row| Feature {
                id: row.get("id"),
                dataset_id: row.get("dataset_id"),
                feature_id: row.get("feature_id"),
                geometry: serde_json::from_str(&row.get::<_, String>("geometry")).unwrap(),
                attributes: row.get::<_, Json<Value>>("attributes").0,
                input_srid: 4326,  // Features from DB are always in WGS84
                api_key: row.get("api_key"),
            })
            .collect();

        Ok(features)
    }

    pub async fn create_dataset(&self, dataset: &Dataset) -> Result<(), ApiError> {
        let mut client = self.pool.get().await?;  // Make client mutable
        let transaction = client.transaction().await?;

        // Check if dataset name already exists for this API key
        let exists: bool = transaction
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM datasets WHERE name = $1 AND api_key = $2)",
                &[&dataset.name, &dataset.api_key],
            )
            .await?
            .get(0);

        if exists {
            return Err(ApiError::DatasetExists(dataset.name.clone()));
        }
        
        transaction
            .execute(
                "INSERT INTO datasets (id, name, api_key) VALUES ($1, $2, $3)",
                &[&dataset.id, &dataset.name, &dataset.api_key],
            )
            .await?;

        transaction.commit().await?;
        Ok(())
    }

    pub async fn get_datasets(&self, api_key: &str) -> Result<Vec<Dataset>, ApiError> {
        let client = self.pool.get().await?;
        
        let rows = client
            .query(
                "SELECT id, name, api_key FROM datasets WHERE api_key = $1",
                &[&api_key],
            )
            .await?;

        let datasets = rows
            .into_iter()
            .map(|row| Dataset {
                id: row.get("id"),
                name: row.get("name"),
                api_key: row.get("api_key"),
            })
            .collect();

        Ok(datasets)
    }

    pub async fn get_dataset_by_name(&self, name: &str, api_key: &str) -> Result<Option<Dataset>, ApiError> {
        let client = self.pool.get().await?;
        
        let row = client
            .query_opt(
                "SELECT id, name, api_key FROM datasets WHERE name = $1 AND api_key = $2",
                &[&name, &api_key],
            )
            .await?;

        Ok(row.map(|row| Dataset {
            id: row.get("id"),
            name: row.get("name"),
            api_key: row.get("api_key"),
        }))
    }

    pub async fn get_feature(&self, dataset_name: &str, feature_id: &str) -> Result<Option<Feature>, ApiError> {
        let client = self.pool.get().await?;
        
        let row = client
            .query_opt(
                "SELECT f.id, f.dataset_id, f.feature_id, ST_AsGeoJSON(f.geometry) as geometry, f.attributes, d.api_key 
                 FROM features f
                 JOIN datasets d ON f.dataset_id = d.id
                 WHERE d.name = $1 AND f.feature_id = $2",
                &[&dataset_name, &feature_id],
            )
            .await?;

        Ok(row.map(|row| Feature {
            id: row.get("id"),
            dataset_id: row.get("dataset_id"),
            feature_id: row.get("feature_id"),
            geometry: serde_json::from_str(&row.get::<_, String>("geometry")).unwrap(),
            attributes: row.get::<_, Json<Value>>("attributes").0,
            input_srid: 4326,  // Features from DB are always in WGS84
            api_key: row.get("api_key"),
        }))
    }

    pub async fn check_connection(&self) -> Result<(), ApiError> {
        let client = self.pool.get().await?;
        client.execute("SELECT 1", &[]).await?;
        Ok(())
    }
}

#[allow(dead_code)]
fn validate_geojson(geojson: &GeoJson) -> Result<(), ApiError> {
    match geojson {
        GeoJson::Geometry(g) => {
            if g.value.type_name() == "GeometryCollection" {
                return Err(ApiError::InvalidGeometry(
                    "GeometryCollection is not supported".to_string()
                ));
            }
        },
        _ => {
            return Err(ApiError::InvalidGeometry(
                "Only Geometry type is supported".to_string()
            ));
        }
    }
    Ok(())
} 