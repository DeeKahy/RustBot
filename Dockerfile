# Multi-stage build for optimized RustBot Docker image
FROM rust:1.85-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    binutils \
    && rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /app

# Copy dependency files first for better caching
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies (this layer will be cached)
# Remove Cargo.lock temporarily to avoid version conflicts, then rebuild it
RUN rm -f Cargo.lock && cargo build --release && rm -rf src

# Copy the actual source code
COPY src ./src

# Build the application
RUN cargo build --release

# Strip the binary to reduce size
RUN strip /app/target/release/rustbot

# Runtime stage - use distroless for minimal size
FROM gcr.io/distroless/cc-debian12

# Copy CA certificates from builder
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

# Copy the binary from the builder stage
COPY --from=builder /app/target/release/rustbot /rustbot

# Set default environment variables
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1

# Run the bot
CMD ["/rustbot"]
