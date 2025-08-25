# Backend image (Rust -> slim runtime)
FROM rust:latest AS builder
WORKDIR /build
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev build-essential ca-certificates && rm -rf /var/lib/apt/lists/*
COPY backend/Cargo.toml backend/Cargo.lock ./backend/
RUN mkdir -p backend/src && echo "fn main(){}" > backend/src/main.rs && cd backend && cargo build --release && rm -rf src
COPY backend ./backend
RUN cd backend && cargo build --release

FROM ubuntu:24.04 AS runtime
WORKDIR /app/backend
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libssl3 tini && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/backend/target/release/samplefarm-backend /app/backend/samplefarm-backend
COPY --from=builder /build/backend/config.toml /app/backend/config.toml
EXPOSE 8080
ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["/app/backend/samplefarm-backend"]


