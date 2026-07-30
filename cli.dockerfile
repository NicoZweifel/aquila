FROM rust:1.93-bookworm AS chef
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY --from=planner /app/recipe.json recipe.json

RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
RUN cargo build --release -p aquila_cli

RUN strip target/release/aquila


FROM debian:bookworm-slim AS  runner

WORKDIR /app

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd -r aquila && useradd -r -g aquila aquila

COPY --from=builder /app/target/release/aquila /usr/local/bin/aquila

USER aquila

RUN aquila --version

ENTRYPOINT ["aquila"]

CMD ["help"]