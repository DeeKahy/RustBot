# syntax=docker/dockerfile:1

# ---- Build Stage ----
FROM rust:1.85-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy manifests and source for caching
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release || true
RUN rm -rf src

# Copy the rest of the source and .git for self-update
COPY . .
# Ensure .git is present for update commands
RUN [ -d ".git" ] || (echo "Missing .git directory! Clone with --recurse-submodules and include .git." && false)

# Build the actual binary
RUN cargo build --release

# ---- Runtime Stage ----
FROM debian:bookworm-slim

# Install runtime dependencies: bash, git, ca-certificates, tini, curl, and build tools for Rust
RUN apt-get update && apt-get install -y \
    bash \
    git \
    ca-certificates \
    tini \
    curl \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Install Rust toolchain
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.85.0
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /app

# Copy built binary and scripts from builder
COPY --from=builder /app/target/release/rustbot /app/target/release/rustbot
COPY --from=builder /app/startup.sh /app/startup.sh
COPY --from=builder /app/.git /app/.git
COPY --from=builder /app/Cargo.toml /app/Cargo.toml
COPY --from=builder /app/Cargo.lock /app/Cargo.lock
COPY --from=builder /app/src /app/src

# Make sure startup.sh is executable
RUN chmod +x /app/startup.sh

# Environment defaults (can be overridden)
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1
ENV GIT_BRANCH=develop

# Use tini for proper signal handling, run startup.sh
ENTRYPOINT ["/usr/bin/tini", "--", "/app/startup.sh"]
