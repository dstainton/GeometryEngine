use redis::{Client, AsyncCommands, RedisResult};
use std::env;
use crate::models::Feature;
use crate::error::ApiError;
use serde_json;

#[derive(Clone)]
pub struct Cache {
    client: Client,
}

#[allow(dead_code)]
impl Cache {
    pub fn new() -> Result<Self, ApiError> {
        let redis_url = env::var("REDIS_URL").expect("REDIS_URL must be set");
        let client = Client::open(redis_url).map_err(ApiError::Cache)?;
        Ok(Self { client })
    }

    fn cache_key(dataset_name: &str, feature_id: &str) -> String {
        format!("feature:{}:{}", dataset_name, feature_id)
    }

    pub async fn get_feature(&self, dataset_name: &str, feature_id: &str) -> Result<Option<Feature>, ApiError> {
        let mut conn = self.client.get_async_connection().await.map_err(ApiError::Cache)?;
        let key = Self::cache_key(dataset_name, feature_id);
        
        let data: RedisResult<Option<String>> = conn.get(&key).await;
        let data = data.map_err(ApiError::Cache)?;
        
        match data {
            Some(json) => serde_json::from_str(&json).map(Some).map_err(ApiError::Json),
            None => Ok(None),
        }
    }

    pub async fn set_feature(&self, dataset_name: &str, feature: &Feature) -> Result<(), ApiError> {
        let mut conn = self.client.get_async_connection().await.map_err(ApiError::Cache)?;
        let key = Self::cache_key(dataset_name, &feature.feature_id);
        let json = serde_json::to_string(feature).map_err(ApiError::Json)?;
        
        let _: RedisResult<()> = conn.set_ex(&key, json, 3600).await;
        Ok(())
    }

    pub async fn invalidate_feature(&self, dataset_name: &str, feature_id: &str) -> Result<(), ApiError> {
        let mut conn = self.client.get_async_connection().await.map_err(ApiError::Cache)?;
        let key = Self::cache_key(dataset_name, feature_id);
        
        let _: RedisResult<()> = conn.del(&key).await;
        Ok(())
    }

    pub async fn get_connection(&self) -> Result<redis::aio::Connection, ApiError> {
        self.client.get_async_connection().await.map_err(ApiError::Cache)
    }

    pub async fn check_connection(&self) -> Result<(), ApiError> {
        let mut conn = self.get_connection().await?;
        let _: () = redis::cmd("PING")
            .query_async(&mut conn)
            .await
            .map_err(ApiError::Cache)?;
        Ok(())
    }
} 