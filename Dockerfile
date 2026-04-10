# --------------------------------------------------
# stage 1: build frontend
# --------------------------------------------------
FROM node:22-slim AS frontend
WORKDIR /build/frontend
COPY frontend/package.json frontend/package-lock.json* ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

# --------------------------------------------------
# stage 2: build rust binary
# --------------------------------------------------
FROM rust:1.87-slim AS backend
WORKDIR /build
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# cache deps
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src && echo "fn main(){}" > src/main.rs
RUN cargo build --release || true
RUN rm -rf src

# copy frontend build output and real source
COPY --from=frontend /build/frontend/build frontend/build/
COPY src/ src/

RUN cargo build --release

# --------------------------------------------------
# stage 3: runtime
# --------------------------------------------------
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app

COPY --from=backend /build/target/release/otd /app/otd

EXPOSE 15204
EXPOSE 15205
CMD ["./otd"]
