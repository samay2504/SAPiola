# syntax=docker/dockerfile:1

# Stage 1: Build the Rust graph layer
FROM rust:bookworm AS builder

# Install build dependencies
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
RUN cargo build --release --bin sap-graph-layer

# Stage 2: Create the minimal production image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Run as non-root user
RUN useradd -ms /bin/bash appuser
RUN mkdir -p /app/data && chown -R appuser:appuser /app/data
USER appuser

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/sap-graph-layer /app/sap-graph-layer

ENTRYPOINT ["/app/sap-graph-layer"]
