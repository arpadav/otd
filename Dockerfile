FROM rust:1.94-slim AS builder

WORKDIR /build

# cache deps
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main(){}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# real source
COPY . .
RUN cargo build --release

# ---------- runtime stage ----------
FROM debian:bookworm-slim

WORKDIR /app

# copy binary + config
COPY --from=builder /build/target/release/otd /app/otd

# ensure executable
RUN chmod +x /app/otd

# run
CMD ["./otd"]
