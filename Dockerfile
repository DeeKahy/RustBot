# Multi-stage build for optimized Docker image with auto-update capability
# Supports both linux/amd64 and linux/arm64 platforms

# Build stage
FROM rust:latest AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    git \
    && rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /app

# Clone the repository
RUN git clone https://github.com/DeeKahy/RustBot.git .

# Build the application
RUN cargo build --release

# Runtime stage
FROM rust:slim

# Install runtime dependencies and git for updates
RUN apt-get update && apt-get install -y \
    ca-certificates \
    git \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/* \
    && update-ca-certificates

# Create a non-root user for security with proper home directory
RUN useradd -r -s /bin/bash rustbot && \
    mkdir -p /home/rustbot && \
    chown rustbot:rustbot /home/rustbot

# Set the working directory
WORKDIR /app

# Copy the repository from builder
COPY --from=builder /app /app

# Create startup script
RUN cat > /app/startup.sh << 'EOF'
#!/bin/bash

echo "🔄 Checking for updates from GitHub..."

# Set up git environment
export HOME=/home/rustbot
export GIT_CONFIG_GLOBAL=/app/.gitconfig

# Create a local git config file that doesn't require home directory write access
cat > /app/.gitconfig << 'GITEOF'
[safe]
    directory = /app
[user]
    name = RustBot Container
    email = rustbot@container.local
[init]
    defaultBranch = main
GITEOF

# Configure git to use the local config file
export GIT_CONFIG_GLOBAL=/app/.gitconfig

# Pull latest changes (non-fatal if it fails)
echo "📥 Attempting to pull latest changes..."
if git pull origin main 2>/dev/null; then
    echo "✅ Successfully pulled latest changes"

    # Check if there are any changes to source files
    if git diff --name-only HEAD@{1} HEAD 2>/dev/null | grep -E '\.(rs|toml)$' > /dev/null 2>&1; then
        echo "🔨 Source code changes detected, rebuilding..."
        if cargo build --release; then
            echo "✅ Build successful!"
        else
            echo "❌ Build failed, using previous version"
        fi
    else
        echo "ℹ️ No source code changes detected, using existing build"
    fi
else
    echo "⚠️ Failed to pull updates (this is normal for the first run), using existing version"
fi

echo "🚀 Starting RustBot..."

# Start the bot with restart capability
while true; do
    /app/target/release/rustbot
    exit_code=$?

    # Exit code 42 means update was requested
    if [ $exit_code -eq 42 ]; then
        echo "🔄 Update requested, pulling latest changes..."

        # Pull latest changes
        if git pull origin main 2>/dev/null; then
            echo "✅ Successfully pulled latest changes"

            # Rebuild if source files changed
            if git diff --name-only HEAD@{1} HEAD 2>/dev/null | grep -E '\.(rs|toml)$' > /dev/null 2>&1; then
                echo "🔨 Rebuilding with latest changes..."
                if cargo build --release; then
                    echo "✅ Build successful, restarting with new version!"
                else
                    echo "❌ Build failed, restarting with previous version"
                fi
            else
                echo "ℹ️ No source changes, restarting with same version"
            fi
        else
            echo "⚠️ Failed to pull updates, restarting with current version"
        fi

        sleep 2
        continue
    else
        echo "🛑 Bot stopped with exit code $exit_code"
        exit $exit_code
    fi
done
EOF

# Make startup script executable
RUN chmod +x /app/startup.sh

# Change ownership to the rustbot user
RUN chown -R rustbot:rustbot /app

# Create a git config file in the app directory (after copying repository)
RUN echo '[safe]' > /app/.gitconfig && \
    echo '    directory = /app' >> /app/.gitconfig && \
    echo '[user]' >> /app/.gitconfig && \
    echo '    name = RustBot Container' >> /app/.gitconfig && \
    echo '    email = rustbot@container.local' >> /app/.gitconfig && \
    echo '[init]' >> /app/.gitconfig && \
    echo '    defaultBranch = main' >> /app/.gitconfig && \
    chown rustbot:rustbot /app/.gitconfig

# Switch to non-root user
USER rustbot

# Set default environment variables (can be overridden)
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1

# Add health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=15s --retries=3 \
    CMD pgrep rustbot || exit 1

# Run the startup script
CMD ["/app/startup.sh"]
