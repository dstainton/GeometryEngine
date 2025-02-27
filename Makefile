.PHONY: dev test coverage audit clean build deploy

dev:
	docker-compose -f docker-compose.dev.yml up

test:
	cargo test

coverage:
	cargo tarpaulin --out Html

audit:
	cargo audit

clean:
	cargo clean
	rm -rf target/

build:
	docker build -t geospatial-api .

deploy:
	helm upgrade --install geospatial-api ./helm/geospatial-api

lint:
	cargo fmt -- --check
	cargo clippy -- -D warnings

check: lint test audit 