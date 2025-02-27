-- Indexes for API keys
CREATE INDEX IF NOT EXISTS idx_api_keys_last_used ON api_keys(last_used_at);
CREATE UNIQUE INDEX IF NOT EXISTS idx_api_keys_key ON api_keys(key);

-- Indexes for datasets
CREATE INDEX IF NOT EXISTS idx_datasets_api_key ON datasets(api_key);
CREATE UNIQUE INDEX IF NOT EXISTS idx_datasets_name_api_key ON datasets(name, api_key);

-- Indexes for features
CREATE INDEX IF NOT EXISTS idx_features_dataset_id ON features(dataset_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_features_dataset_feature ON features(dataset_id, feature_id);
CREATE INDEX IF NOT EXISTS idx_features_geometry ON features USING GIST(geometry); 