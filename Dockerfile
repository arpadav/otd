# --------------------------------------------------
# build stage
# --------------------------------------------------
FROM rust:1.87-slim AS builder
WORKDIR /build

# --------------------------------------------------
# install system deps for dx cli and wasm target
# --------------------------------------------------
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev curl \
    && rm -rf /var/lib/apt/lists/*
RUN rustup target add wasm32-unknown-unknown
RUN cargo install dioxus-cli@0.7.4 --locked

# --------------------------------------------------
# install tailwindcss standalone cli
# --------------------------------------------------
RUN curl -sL https://github.com/nicolo-ribaudo/tailwindcss-cli/releases/latest/download/tailwindcss-linux-x64 -o /usr/local/bin/tailwindcss \
    && chmod +x /usr/local/bin/tailwindcss

# --------------------------------------------------
# cache workspace deps
# --------------------------------------------------
COPY Cargo.toml Cargo.lock ./
COPY crates/otd-tailwind/Cargo.toml crates/otd-tailwind/Cargo.toml
COPY crates/otd-web/Cargo.toml crates/otd-web/Cargo.toml
RUN mkdir -p crates/otd-tailwind/src && echo "" > crates/otd-tailwind/src/lib.rs
RUN mkdir -p crates/otd-web/src && echo "fn main(){}" > crates/otd-web/src/main.rs
RUN cargo build --release -p otd-web --features server || true
RUN rm -rf crates/otd-tailwind/src crates/otd-web/src

# --------------------------------------------------
# copy real source and build
# --------------------------------------------------
COPY input.css ./input.css
COPY crates ./crates

# Generate tailwind CSS
RUN tailwindcss -i input.css -o crates/otd-web/assets/tailwind.css --optimize --minify

# Build with dx (fullstack: server binary + WASM client)
RUN cd crates/otd-web && dx build --release --fullstack

# --------------------------------------------------
# runtime stage
# --------------------------------------------------
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app

# Copy the dx build output (server binary + public assets)
COPY --from=builder /build/target/dx/otd-web/release/web /app

ENV PORT=15204
ENV IP=0.0.0.0
EXPOSE 15204
EXPOSE 15205
CMD ["./server"]
