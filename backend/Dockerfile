FROM rust:1.96.1 AS build

WORKDIR /app

RUN rustup component add clippy rustfmt

COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY backend/ backend/
COPY ingest-core/ ingest-core/
COPY ingest/ ingest/
COPY ingest-cli/ ingest-cli/
COPY kb-store/ kb-store/
COPY features/ features/

RUN cargo build --release -p backend

EXPOSE 8080
CMD ["./target/release/backend"]

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=build /app/target/release/backend /app/backend

EXPOSE 8080
CMD ["/app/backend"]
