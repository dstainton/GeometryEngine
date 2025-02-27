FROM rust:1.81 as builder

WORKDIR /usr/src/app
COPY . .
RUN cargo build --release

FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y \
    postgresql-client \
    libpq5 \
    redis-tools \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/release/geospatial-api /usr/local/bin/

ENV RUST_LOG=info
EXPOSE 8080

CMD ["geospatial-api"] 