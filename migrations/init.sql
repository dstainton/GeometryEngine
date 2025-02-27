CREATE EXTENSION IF NOT EXISTS postgis;

CREATE TABLE api_keys (
    key TEXT PRIMARY KEY,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    key_expires_in_seconds INTEGER NOT NULL DEFAULT 7776000, -- 3 months in seconds
    data_expires_in_seconds INTEGER NOT NULL DEFAULT 3600,   -- 1 hour in seconds
    last_used_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE datasets (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    api_key TEXT REFERENCES api_keys(key),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(name, api_key)
);

CREATE TABLE features (
    id UUID PRIMARY KEY,
    dataset_id UUID REFERENCES datasets(id),
    feature_id TEXT NOT NULL,
    geometry geometry(Geometry, 4326),
    attributes JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(dataset_id, feature_id)
);

CREATE INDEX features_geometry_idx ON features USING GIST(geometry);
CREATE INDEX features_attributes_idx ON features USING GIN(attributes); 