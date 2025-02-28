FROM rust:1.81 as builder

WORKDIR /usr/src/app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim

# Create required directories and set permissions
RUN set -eux; \
    mkdir -p /var/lib/apt/lists/partial; \
    mkdir -p /var/cache/apt/archives/partial; \
    mkdir -p /var/log/apt; \
    chmod -R 777 /var/lib/apt/lists; \
    chmod -R 777 /var/cache/apt; \
    chmod -R 777 /var/log/apt

# Install required packages
RUN apt-get update && apt-get install -y \
    postgresql-client \
    libpq5 \
    redis-tools \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/release/geospatial-api /usr/local/bin/

ENV RUST_LOG=info
EXPOSE 8080

CMD ["geospatial-api"] 