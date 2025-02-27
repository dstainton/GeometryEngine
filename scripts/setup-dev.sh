#!/bin/bash
set -e

# Install required tools
cargo install cargo-watch
cargo install cargo-tarpaulin
cargo install cargo-audit

# Setup git hooks
cat > .git/hooks/pre-commit << 'EOF'
#!/bin/bash
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test
EOF
chmod +x .git/hooks/pre-commit

# Create development environment file
cat > .env.development << 'EOF'
POSTGRES_HOST=localhost
POSTGRES_PORT=5432
POSTGRES_DB=geospatial
POSTGRES_USER=postgres
POSTGRES_PASSWORD=development
REDIS_URL=redis://localhost:6379
MASTER_API_KEY=dev-master-key
RUST_LOG=debug
EOF

# Initialize database
cat > init-dev-db.sql << 'EOF'
CREATE EXTENSION IF NOT EXISTS postgis;
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
EOF 