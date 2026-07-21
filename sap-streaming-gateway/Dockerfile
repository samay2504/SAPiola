# syntax=docker/dockerfile:1

# Stage 1: Build the Rust streaming gateway
FROM rust:bookworm AS builder

# Install build dependencies (cmake, build-essential for rdkafka-sys)
RUN apt-get update && apt-get install -y --no-install-recommends \
    cmake \
    build-essential \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    libprotobuf-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the entire workspace root for Cargo workspace resolution
COPY . .

# Build the specific package with release profile
RUN cargo build --release --package sap-streaming-gateway

# Stage 2: Create the minimal production image
FROM debian:bookworm-slim

# Install runtime dependencies (OpenSSL, ca-certificates)
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Run as non-root user
RUN useradd -ms /bin/bash appuser
USER appuser

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/sap-streaming-gateway /app/sap-streaming-gateway

ENTRYPOINT ["/app/sap-streaming-gateway"]
