# --------------------------------------------------
# build stage
# --------------------------------------------------
FROM rust:1.94-slim AS builder
WORKDIR /build
# --------------------------------------------------
# cache deps
# --------------------------------------------------
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main(){}" > src/main.rs
RUN cargo build --release
RUN rm -rf src
# --------------------------------------------------
# copy real source
# --------------------------------------------------
COPY src ./src
COPY static ./static
RUN cargo build --release
# --------------------------------------------------
# runtime stage
# --------------------------------------------------
FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /build/target/release/otd /app/otd
RUN chmod +x /app/otd
# --------------------------------------------------
# entry
# --------------------------------------------------
CMD ["./otd"]
