# syntax=docker/dockerfile:1

# Stage 1: Build the Go CDC agent
FROM golang:bookworm AS builder

# Add build tools required by confluent-kafka-go (needs CGO)
RUN apt-get update && apt-get install -y --no-install-recommends \
    gcc \
    libc6-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy go.mod and go.sum first to leverage Docker layer caching for dependencies
COPY go.mod go.sum ./
RUN go mod download

# Copy the rest of the source code
COPY . .

# Build the binary with CGO enabled (required by confluent-kafka-go)
RUN CGO_ENABLED=1 GOOS=linux go build -ldflags="-w -s" -o /cdc-agent ./cmd/cdc-agent

# Stage 2: Create the minimal production image
FROM debian:bookworm-slim

# Add CA certificates for TLS and timezone data
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    tzdata \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Run as a non-root user for security
RUN useradd -ms /bin/bash appuser
USER appuser

# Copy the compiled binary from the builder stage
COPY --from=builder /cdc-agent /app/cdc-agent

# Set the entrypoint
ENTRYPOINT ["/app/cdc-agent"]
