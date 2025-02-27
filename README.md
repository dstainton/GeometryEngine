# Geospatial API

A high-performance, scalable REST API for geospatial data management built with Rust and PostGIS.

Built completely using prompting in Cursor. The only manual edit was this line of text.

## Features

- 🗺️ Full GeoJSON support with PostGIS integration
- 🔒 Hierarchical API key authentication
- 🚀 High performance with Rust and async/await
- 💾 PostGIS spatial database with automatic SRID transformation
- 📦 Redis caching with circuit breaker pattern
- ⚡ Configurable rate limiting
- 📊 Prometheus metrics and Grafana dashboards
- 🔍 OpenAPI documentation
- 🛡️ Circuit breaker for external services
- 🎯 Comprehensive health checks
- 📝 Structured logging with request tracing
- 🔄 Automated backups with retention
- 🚢 Kubernetes/OpenShift deployment with Helm
- 🔐 Security-first design with least privilege

## Architecture

### Components
- Rust web service using actix-web
- PostgreSQL with PostGIS for spatial data
- Redis for caching and rate limiting
- Prometheus for metrics
- Grafana for monitoring

### Security Features
- API key authentication
- Network policies for pod isolation
- Security context constraints
- Regular security audits
- Automated vulnerability scanning

### Monitoring
- Prometheus metrics
- Grafana dashboards
- Health check endpoints
- Request tracing
- Error tracking

### Data Management
- Automated backups
- SRID transformation
- GeoJSON validation
- Attribute querying
- Spatial operations

### Supported Coordinate Systems
The API supports automatic transformation between the following coordinate systems:

#### Global Systems
- WGS84 (EPSG:4326)
- Web Mercator (EPSG:3857)

#### North American Systems
- BC Albers (EPSG:3005)
- UTM Zones 7-11N NAD83 (EPSG:26907-26911)
- NAD83(CSRS) BC Albers (EPSG:3157)
- NAD83 Geographic (EPSG:4269)
- NAD83(CSRS) Geographic (EPSG:4617)
- NAD83(CSRS) UTM Zones:
  - Zone 7N (EPSG:3155)
  - Zone 8N (EPSG:3156)
  - Zone 9N (EPSG:2955)
  - Zone 10N (EPSG:3158)
  - Zone 11N (EPSG:3159)
- US National Atlas Equal Area (EPSG:2163)

## Configuration

### Environment Variables
```env
POSTGRES_HOST=localhost
POSTGRES_PORT=5432
POSTGRES_DB=geospatial
POSTGRES_USER=postgres
POSTGRES_PASSWORD=secret
REDIS_URL=redis://redis:6379
MASTER_API_KEY=your-master-key
RATE_LIMIT_PER_MINUTE=100
RUST_LOG=info
BACKUP_RETENTION_DAYS=7
BACKUP_ENCRYPTION_KEY=your-backup-key
API_KEY_DEFAULT_EXPIRY=7776000
```

### Helm Values
```yaml
replicaCount: 2
image:
  repository: geospatial-api
  tag: latest

postgresql:
  auth:
    database: geospatial
  primary:
    persistence:
      size: 10Gi

redis:
  auth:
    enabled: true
  master:
    persistence:
      size: 5Gi

backup:
  enabled: true
  schedule: "0 1 * * *"
  retention: 7
  storage:
    size: 10Gi

metrics:
  enabled: true
  serviceMonitor:
    enabled: true
```

## API Endpoints

### Data Management
- `POST /api/v1/datasets` - Create dataset
- `GET /api/v1/datasets` - List datasets
- `POST /api/v1/datasets/{name}/features` - Store feature
- `PUT /api/v1/datasets/{name}/features/{id}` - Update feature
- `GET /api/v1/datasets/{name}/features/{id}` - Get feature

### Spatial Operations
- `POST /api/v1/spatial-query` - Spatial queries (intersects, contains, within)

### Administration
- `POST /api/v1/api-keys` - Create API key
- `GET /health-check` - Health check
- `GET /readiness-check` - Readiness check
- `GET /metrics` - Prometheus metrics

## API Examples

### Create API Key
```bash
curl -X POST http://localhost:8080/api/v1/api-keys \
  -H "X-API-Key: master-key" \
  -H "Content-Type: application/json" \
  -d '{
    "new_key": "user-api-key-1",
    "key_expires_in_seconds": 7776000,
    "data_expires_in_seconds": 3600
  }'
```

### Create Dataset
```bash
curl -X POST http://localhost:8080/api/v1/datasets \
  -H "X-API-Key: user-api-key-1" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "cities",
    "api_key": "user-api-key-1"
  }'
```

### Store Feature
```bash
curl -X POST http://localhost:8080/api/v1/datasets/cities/features \
  -H "X-API-Key: user-api-key-1" \
  -H "Content-Type: application/json" \
  -d '{
    "feature_id": "london",
    "geometry": {
      "type": "Point",
      "coordinates": [-0.1276, 51.5074]
    },
    "attributes": {
      "name": "London",
      "country": "UK",
      "population": 8982000
    },
    "input_srid": 4326  # Supported SRIDs:
    # - 4326 (WGS84)
    # - 3857 (Web Mercator)
    # - 2163 (US National Atlas)
    # - 3005 (BC Albers)
    # - 26907-26911 (UTM Zones 7-11N NAD83)
    # - 3157 (NAD83(CSRS) BC Albers)
    # - 4269 (NAD83 Geographic)
    # - 4617 (NAD83(CSRS) Geographic)
    # - 3155 (NAD83(CSRS) UTM Zone 7N)
    # - 3156 (NAD83(CSRS) UTM Zone 8N)
    # - 2955 (NAD83(CSRS) UTM Zone 9N)
    # - 3158 (NAD83(CSRS) UTM Zone 10N)
    # - 3159 (NAD83(CSRS) UTM Zone 11N)
  }'
```

### Spatial Query
```bash
curl -X POST http://localhost:8080/api/v1/spatial-query \
  -H "X-API-Key: user-api-key-1" \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "within",
    "dataset_name": "cities",
    "geometry": {
      "type": "Polygon",
      "coordinates": [[
        [-5.0, 50.0], [2.0, 50.0],
        [2.0, 54.0], [-5.0, 54.0],
        [-5.0, 50.0]
      ]]
    },
    "input_srid": 4326,
    "output_srid": 4326
  }'
```

### Get Feature
```bash
curl http://localhost:8080/api/v1/datasets/cities/features/london \
  -H "X-API-Key: user-api-key-1"
```

### Update Feature
```bash
curl -X PUT http://localhost:8080/api/v1/datasets/cities/features/london \
  -H "X-API-Key: user-api-key-1" \
  -H "Content-Type: application/json" \
  -d '{
    "feature_id": "london",
    "geometry": {
      "type": "Point",
      "coordinates": [-0.1276, 51.5074]
    },
    "attributes": {
      "name": "London",
      "country": "UK",
      "population": 9002000,
      "updated": true
    },
    "input_srid": 4326
  }'
```

### List Datasets
```bash
curl http://localhost:8080/api/v1/datasets \
  -H "X-API-Key: user-api-key-1"
```

### Health Check
```bash
curl http://localhost:8080/health-check
```

### Metrics
```bash
curl http://localhost:8080/metrics
```

## Development

### Prerequisites
- Rust 1.68+
- Docker & Docker Compose
- PostgreSQL 15 with PostGIS
- Redis 7
- Kubernetes/OpenShift (for production)

### Quick Start
```bash
# Setup development environment
./scripts/setup-dev.sh

# Start services
make dev

# Run tests
make test

# Build and deploy
make build
make deploy
```

### Available Make Commands
- `make dev` - Start development environment
- `make test` - Run tests
- `make coverage` - Generate coverage report
- `make audit` - Security audit
- `make lint` - Run linters
- `make build` - Build Docker image
- `make deploy` - Deploy to Kubernetes
- `make check` - Run all checks

## License

MIT License - see LICENSE file
