# Docker Deployment Guide 🐳

This guide covers everything you need to know about deploying RustBot using Docker.

## Quick Start

1. **Clone the repository:**
```bash
git clone <your-repo-url>
cd RustBot
```

2. **Build and push (interactive script):**
```bash
./docker-build.sh
```

3. **Run your bot:**
```bash
docker run -e DISCORD_TOKEN=your_token_here your_username/rustbot:latest
```

## Building the Docker Image

### Option 1: Using the Build Script (Recommended)

The included `docker-build.sh` script makes it easy to build and push your image:

```bash
# Interactive mode (will prompt for username)
./docker-build.sh

# Or provide username directly
./docker-build.sh your_docker_username
```

The script will:
- Build the Docker image with proper tagging
- Optionally push to Docker Hub
- Handle Docker login if needed
- Provide helpful output and error messages

### Option 2: Manual Docker Commands

```bash
# Build the image
docker build -t your_username/rustbot:latest .

# Push to Docker Hub
docker login
docker push your_username/rustbot:latest
```

## Running the Container

### Basic Run Command

```bash
docker run -e DISCORD_TOKEN=your_discord_token your_username/rustbot:latest
```

### With Environment File

Create a `.env` file with your configuration:
```env
DISCORD_TOKEN=your_discord_token_here
RUST_LOG=info
```

Then run:
```bash
docker run --env-file .env your_username/rustbot:latest
```

### With Docker Compose

Use the included `docker-compose.yml`:

```bash
# Set your token
export DISCORD_TOKEN=your_discord_token_here

# Run with compose
docker-compose up -d

# View logs
docker-compose logs -f

# Stop the bot
docker-compose down
```

## Docker Image Details

### Base Image
- **Base**: `rust:latest` (official Rust Docker image)
- **Size**: ~1.2GB (includes full Rust toolchain for native compilation)
- **Architecture**: Supports both AMD64 and ARM64

### Build Process
1. **Dependency Layer**: Copies `Cargo.toml` and `Cargo.lock`, builds dependencies
2. **Source Layer**: Copies source code and builds the final binary
3. **Optimization**: Uses Docker layer caching for faster rebuilds

### Runtime Environment
- **Working Directory**: `/app`
- **Binary Location**: `/app/target/release/rustbot`
- **Default Log Level**: `info`
- **User**: `root` (container only)

## Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `DISCORD_TOKEN` | ✅ Yes | - | Your Discord bot token |
| `RUST_LOG` | ❌ No | `info` | Log level (error, warn, info, debug, trace) |

## Production Deployment

### Using Docker Compose (Recommended)

```yaml
version: '3.8'

services:
  rustbot:
    image: your_username/rustbot:latest
    container_name: rustbot-prod
    restart: unless-stopped
    environment:
      - DISCORD_TOKEN=${DISCORD_TOKEN}
      - RUST_LOG=info
    # Optional: Resource limits
    deploy:
      resources:
        limits:
          memory: 256M
        reservations:
          memory: 128M
```

### With Health Checks

```yaml
services:
  rustbot:
    image: your_username/rustbot:latest
    container_name: rustbot
    restart: unless-stopped
    environment:
      - DISCORD_TOKEN=${DISCORD_TOKEN}
    healthcheck:
      test: ["CMD", "pgrep", "rustbot"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
```

### Using Docker Swarm

```bash
# Initialize swarm (if not already done)
docker swarm init

# Deploy as a service
docker service create \
  --name rustbot \
  --env DISCORD_TOKEN=your_token \
  --restart-condition on-failure \
  --replicas 1 \
  your_username/rustbot:latest
```

## Multi-Architecture Support

The image supports both AMD64 and ARM64 architectures:

```bash
# Build for multiple architectures
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -t your_username/rustbot:latest \
  --push .
```

This allows the bot to run on:
- x86_64 servers and development machines
- ARM64 servers (like AWS Graviton instances)
- Apple Silicon Macs (M1/M2)
- Raspberry Pi 4 (with 64-bit OS)

## CI/CD Integration

### GitHub Actions

The included `.github/workflows/docker.yml` automatically:
- Builds the image on every push to main
- Pushes to Docker Hub with proper tagging
- Supports both AMD64 and ARM64 architectures
- Uses Docker layer caching for speed

**Required Secrets:**
- `DOCKER_USERNAME`: Your Docker Hub username
- `DOCKER_PASSWORD`: Your Docker Hub password or access token

### GitLab CI

```yaml
# .gitlab-ci.yml
docker-build:
  stage: build
  image: docker:latest
  services:
    - docker:dind
  script:
    - docker build -t $CI_REGISTRY_IMAGE:$CI_COMMIT_SHA .
    - docker push $CI_REGISTRY_IMAGE:$CI_COMMIT_SHA
  only:
    - main
```

## Troubleshooting

### Common Issues

**Build fails with "Cargo.lock version error":**
- Solution: Use `rust:latest` instead of a specific version
- The project uses newer Cargo features

**Container exits immediately:**
- Check that `DISCORD_TOKEN` is set correctly
- Verify the token is valid and not expired
- Check logs: `docker logs <container_name>`

**"Permission denied" errors:**
- The container runs as root by default
- For production, consider creating a non-root user

**High memory usage:**
- The Rust image is large (~1.2GB) due to the full toolchain
- For production, consider a multi-stage build with a smaller runtime image

### Debugging

**View container logs:**
```bash
# Follow logs in real-time
docker logs -f rustbot

# With Docker Compose
docker-compose logs -f rustbot
```

**Access container shell:**
```bash
docker exec -it rustbot /bin/bash
```

**Check environment variables:**
```bash
docker exec rustbot env | grep DISCORD
```

## Security Considerations

### Token Security
- Never hardcode tokens in the Dockerfile
- Use environment variables or Docker secrets
- Consider using Docker secrets in production:

```bash
# Create a secret
echo "your_discord_token" | docker secret create discord_token -

# Use in service
docker service create \
  --name rustbot \
  --secret discord_token \
  your_username/rustbot:latest
```

### Network Security
- Discord bots don't need exposed ports
- Run on isolated Docker networks in production
- Consider using a reverse proxy if adding web features

### Image Security
- Regularly update the base image: `docker pull rust:latest`
- Scan images for vulnerabilities
- Use minimal base images for production deployments

## Performance Optimization

### Build Optimization
```dockerfile
# Add to Dockerfile for smaller final image
FROM rust:latest as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/rustbot /usr/local/bin/rustbot
CMD ["rustbot"]
```

### Runtime Optimization
- Set appropriate resource limits
- Use Docker's `--memory` flag to limit RAM usage
- Monitor with `docker stats`

## Monitoring

### Basic Monitoring
```bash
# Resource usage
docker stats rustbot

# Container health
docker inspect rustbot | jq '.[0].State'
```

### Advanced Monitoring
Consider integrating with:
- Prometheus + Grafana
- Docker's built-in logging drivers
- External monitoring services (New Relic, DataDog, etc.)

## Scaling

### Horizontal Scaling
Discord bots typically don't need horizontal scaling, but if required:

```yaml
# Docker Compose with replicas
services:
  rustbot:
    image: your_username/rustbot:latest
    deploy:
      replicas: 3
    environment:
      - DISCORD_TOKEN=${DISCORD_TOKEN}
```

**Note**: Multiple instances of the same bot token will conflict. Use Discord's sharding for large bots.

## Updates and Maintenance

### Updating the Bot
```bash
# Pull latest image
docker pull your_username/rustbot:latest

# Restart container
docker-compose down && docker-compose up -d
```

### Automatic Updates
Use Watchtower for automatic updates:

```yaml
services:
  rustbot:
    image: your_username/rustbot:latest
    # ... other config

  watchtower:
    image: containrrr/watchtower
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
    command: --interval 3600 rustbot
```

## Support

If you encounter issues:
1. Check the container logs: `docker logs rustbot`
2. Verify environment variables are set correctly
3. Ensure the Discord token is valid and has proper permissions
4. Check Discord's API status: https://discordstatus.com/
5. Open an issue on the GitHub repository with logs and error details