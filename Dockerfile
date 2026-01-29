FROM rust:slim-bookworm AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY crates/aquila_core crates/aquila_core
COPY crates/aquila_server crates/aquila_server
COPY crates/aquila_auth_github crates/aquila_auth_github
COPY crates/aquila_s3 crates/aquila_s3
COPY crates/aquila_compute_aws crates/aquila_compute_aws

RUN cargo build --release --bin aquila_server

FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/aquila_server /usr/local/bin/server

ENV PORT=3000
EXPOSE 3000

CMD ["server"]


