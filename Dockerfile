# Lightweight self-updating RustBot container
# Clones and builds from GitHub on startup
FROM rust:1.85-slim

# Install runtime dependencies: git, ca-certificates, tini for signal handling
RUN apt-get update && apt-get install -y \
    git \
    ca-certificates \
    tini \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy startup script
COPY startup.sh /app/startup.sh
RUN chmod +x /app/startup.sh

# Environment defaults (can be overridden)
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1
ENV GIT_BRANCH=developing
ENV REPO_URL=https://github.com/DeeKahy/RustBot.git

# Use tini for proper signal handling and run startup script
ENTRYPOINT ["/usr/bin/tini", "--", "/app/startup.sh"]
