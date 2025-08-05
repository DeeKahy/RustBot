# Use the official Rust image as the base image
FROM rust:latest

# Set the working directory inside the container
WORKDIR /app

# Install any additional system dependencies if needed
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy the Cargo.toml and Cargo.lock files first for better caching
COPY Cargo.toml Cargo.lock ./

# Create a dummy src/main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies (this will be cached unless Cargo.toml changes)
RUN cargo build --release && rm src/main.rs

# Copy the source code
COPY src ./src

# Build the application
RUN cargo build --release

# Set environment variables
ENV RUST_LOG=info

# Expose any ports if needed (Discord bots typically don't need this)
# EXPOSE 8080

# Run the bot
CMD ["./target/release/rustbot"]
