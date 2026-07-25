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

RUN groupadd -g 10001 spontini \
    && useradd -u 10001 -g spontini -m -s /usr/sbin/nologin spontini \
    && mkdir -p /usr/local/cargo/registry /usr/local/cargo/git \
    && chown -R spontini:spontini /app /usr/local/cargo/registry /usr/local/cargo/git
USER spontini

EXPOSE 8080
CMD ["./target/release/backend"]

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd -g 10001 spontini \
    && useradd -u 10001 -g spontini -m -s /usr/sbin/nologin spontini

WORKDIR /app
COPY --from=build --chown=spontini:spontini /app/target/release/backend /app/backend
USER spontini

EXPOSE 8080
CMD ["/app/backend"]
