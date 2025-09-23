# Multi-stage Dockerfile for Atomic Bundler
FROM rust:1.75-slim-bullseye AS builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    sqlite3 \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy workspace configuration
COPY Cargo.toml Cargo.lock ./
COPY rust-toolchain.toml ./

# Copy all crate manifests first (for better caching)
COPY crates/middleware/Cargo.toml ./crates/middleware/
COPY crates/relay_client/Cargo.toml ./crates/relay_client/
COPY crates/simulator/Cargo.toml ./crates/simulator/
COPY crates/payment/Cargo.toml ./crates/payment/
COPY crates/config/Cargo.toml ./crates/config/
COPY crates/types/Cargo.toml ./crates/types/

# Create stub source files for dependency caching
RUN mkdir -p crates/middleware/src crates/relay_client/src crates/simulator/src \
    crates/payment/src crates/config/src crates/types/src && \
    echo "fn main() {}" > crates/middleware/src/main.rs && \
    echo "// stub" > crates/relay_client/src/lib.rs && \
    echo "// stub" > crates/simulator/src/lib.rs && \
    echo "// stub" > crates/payment/src/lib.rs && \
    echo "// stub" > crates/config/src/lib.rs && \
    echo "// stub" > crates/types/src/lib.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release && rm -rf crates/*/src

# Copy actual source code
COPY crates/ ./crates/

# Build the application
RUN cargo build --release --bin middleware

# Runtime stage
FROM debian:bullseye-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    sqlite3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 bundler

# Set working directory
WORKDIR /app

# Copy binary from builder stage
COPY --from=builder /app/target/release/middleware /app/middleware

# Create directories and set permissions
RUN mkdir -p /app/data /app/logs && \
    chown -R bundler:bundler /app

# Copy configuration template
COPY config.example.yaml /app/config.example.yaml

# Switch to non-root user
USER bundler

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/healthz || exit 1

# Set environment variables
ENV RUST_LOG=info
ENV CONFIG_PATH=/app/config.yaml

# Run the application
CMD ["./middleware"]
