-- Enable PostGIS extension
CREATE EXTENSION IF NOT EXISTS postgis;

-- Create API keys table
CREATE TABLE IF NOT EXISTS api_keys (
    key TEXT PRIMARY KEY,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    last_used_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    key_expires_in_seconds INTEGER DEFAULT 7776000, -- 3 months
    data_expires_in_seconds INTEGER DEFAULT 3600    -- 1 hour
);

-- Create datasets table
CREATE TABLE IF NOT EXISTS datasets (
    id UUID PRIMARY KEY,
    name TEXT UNIQUE NOT NULL,
    api_key TEXT REFERENCES api_keys(key)
);

-- Create features table with PostGIS geometry
CREATE TABLE IF NOT EXISTS features (
    id UUID PRIMARY KEY,
    dataset_id UUID REFERENCES datasets(id),
    feature_id TEXT NOT NULL,
    geometry GEOMETRY(Geometry, 4326),
    attributes JSONB,
    UNIQUE(dataset_id, feature_id)
);

-- Create spatial index
CREATE INDEX IF NOT EXISTS features_geometry_idx ON features USING GIST(geometry); 