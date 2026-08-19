# Multi-stage Dockerfile for Axiomatic
# Stage 1: Build release binary
FROM rust:1.82-slim AS builder

WORKDIR /usr/src/axiomatic
COPY Cargo.toml Cargo.lock ./
COPY models ./models
COPY src ./src

RUN cargo build --release

# Stage 2: Minimal runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /usr/src/axiomatic/target/release/axiomatic /usr/local/bin/axiomatic
COPY --from=builder /usr/src/axiomatic/models ./models

ENV RUST_LOG=info
EXPOSE 3000

ENTRYPOINT ["/usr/local/bin/axiomatic"]
CMD ["serve", "3000"]
