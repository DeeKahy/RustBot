# Multi-stage build for optimized RustBot Docker image
FROM rust:1.75 AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /app

# Copy dependency files first for better caching
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release && rm -rf src

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

# Copy the binary from the builder stage
COPY --from=builder /app/target/release/rustbot /usr/local/bin/rustbot

# Make sure the binary is executable
RUN chmod +x /usr/local/bin/rustbot

# Switch to non-root user
USER rustbot

# Set default environment variables
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1

# Add health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=15s --retries=3 \
    CMD pgrep rustbot || exit 1

# Run the bot
CMD ["rustbot"]
