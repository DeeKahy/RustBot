# Multi-stage build for optimized Docker image
# Supports both linux/amd64 and linux/arm64 platforms

# Build stage
FROM rust:latest AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /app

# Copy dependency files first for better layer caching
COPY Cargo.toml ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies (this layer will be cached unless Cargo files change)
RUN cargo build --release && rm -rf src target/release/deps/rustbot*

# Copy the actual source code
COPY src ./src

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && update-ca-certificates

# Create a non-root user for security
RUN useradd -r -s /bin/false rustbot

# Set the working directory
WORKDIR /app

# Copy the binary from the builder stage
COPY --from=builder /app/target/release/rustbot /app/rustbot

# Change ownership to the rustbot user
RUN chown rustbot:rustbot /app/rustbot

# Switch to non-root user
USER rustbot

# Set default environment variables (can be overridden)
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1

# Add health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD pgrep rustbot || exit 1

# Run the application
CMD ["/app/rustbot"]
