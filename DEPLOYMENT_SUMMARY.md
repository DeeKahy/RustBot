# RustBot Deployment Summary

## What Was Fixed

### 1. Docker Multi-Platform Support
- ✅ **Fixed Dockerfile**: Updated to support both `linux/amd64` and `linux/arm64` architectures
- ✅ **Multi-stage Build**: Optimized build process with smaller final image size
- ✅ **Security Improvements**: Non-root user, read-only filesystem, no new privileges
- ✅ **Health Checks**: Built-in process monitoring for container health

### 2. Environment Variable Handling
- ✅ **CasaOS Compatible**: Environment variables properly configured for CasaOS deployment
- ✅ **Flexible Configuration**: Supports both direct environment variables and .env files
- ✅ **Default Values**: Sensible defaults with override capability

### 3. Docker Hub Deployment
- ✅ **Published Image**: `deekahy/rustbot:latest` available on Docker Hub
- ✅ **Multi-Platform**: Supports both Intel/AMD (x86_64) and ARM (ARM64) processors
- ✅ **Optimized Size**: ~80MB final image size (vs ~1GB+ unoptimized)

## Deployment Options

### Option 1: CasaOS Deployment (Recommended)

**Docker Image**: `deekahy/rustbot:latest`

**Required Environment Variables**:
```
DISCORD_TOKEN=your_discord_bot_token_here
```

**Optional Environment Variables**:
```
RUST_LOG=info                    # Log level: error, warn, info, debug, trace
RUST_BACKTRACE=1                # Enable backtraces for debugging
```

**Resource Requirements**:
- Memory: 256MB limit, 128MB reservation
- CPU: 0.5 cores limit
- Storage: ~100MB for image

**CasaOS Configuration**:
1. Add new application in CasaOS
2. Use image: `deekahy/rustbot:latest`
3. Set environment variable: `DISCORD_TOKEN=your_token`
4. Set memory limit: `256m`
5. Set CPU limit: `0.5`
6. Enable restart policy: `unless-stopped`

### Option 2: Docker Compose

Use the provided `docker-compose.casaos.yml`:

```yaml
version: "3.8"

services:
  rustbot:
    image: deekahy/rustbot:latest
    container_name: rustbot
    restart: unless-stopped
    
    environment:
      - DISCORD_TOKEN=${DISCORD_TOKEN}
      - RUST_LOG=${RUST_LOG:-info}
    
    mem_limit: 256m
    mem_reservation: 128m
    cpus: 0.5
    
    healthcheck:
      test: ["CMD", "pgrep", "rustbot"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 15s
    
    security_opt:
      - no-new-privileges:true
    read_only: true
    
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"
```

**Deploy with**:
```bash
DISCORD_TOKEN=your_token_here docker-compose -f docker-compose.casaos.yml up -d
```

### Option 3: Direct Docker Run

```bash
docker run -d \
  --name rustbot \
  --restart unless-stopped \
  -e DISCORD_TOKEN=your_discord_token_here \
  -e RUST_LOG=info \
  --memory=256m \
  --cpus=0.5 \
  --security-opt no-new-privileges:true \
  --read-only \
  deekahy/rustbot:latest
```

## Platform Compatibility

### ✅ Supported Platforms
- **Linux AMD64** (Intel/AMD x86_64 processors)
- **Linux ARM64** (ARM 64-bit processors, Raspberry Pi 4+, Apple Silicon servers)

### 🔍 Automatic Platform Selection
Docker will automatically pull the correct image for your platform:
- Intel/AMD servers → `linux/amd64` image
- ARM servers (Pi, etc.) → `linux/arm64` image
- CasaOS automatically selects the right architecture

## Environment Variables Guide

### Required Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `DISCORD_TOKEN` | Your Discord bot token | `MTIzNDU2Nzg5MDEyMzQ1Njc4.GhIjKl.MnOp...` |

### Optional Variables

| Variable | Default | Options | Description |
|----------|---------|---------|-------------|
| `RUST_LOG` | `info` | `error`, `warn`, `info`, `debug`, `trace` | Logging verbosity |
| `RUST_BACKTRACE` | `1` | `0`, `1`, `full` | Enable error backtraces |

### Getting Your Discord Token

1. Go to [Discord Developer Portal](https://discord.com/developers/applications)
2. Create a new application or select existing one
3. Navigate to "Bot" section
4. Copy the bot token
5. **Important**: Keep this token secret!

## Security Features

### Container Security
- ✅ **Non-root user**: Runs as `rustbot` user (not root)
- ✅ **Read-only filesystem**: Container filesystem is read-only
- ✅ **No new privileges**: Cannot escalate privileges
- ✅ **Resource limits**: Memory and CPU limits enforced
- ✅ **Minimal image**: Only essential components included

### Network Security
- ✅ **No exposed ports**: Discord bots don't need incoming connections
- ✅ **Outbound only**: Only connects to Discord API
- ✅ **TLS encryption**: All Discord communication encrypted

## Monitoring & Troubleshooting

### Health Monitoring
The container includes automatic health checks:
- **Check**: Process monitoring (`pgrep rustbot`)
- **Interval**: Every 30 seconds
- **Timeout**: 10 seconds
- **Retries**: 3 attempts
- **Start period**: 15 seconds grace period

### Viewing Logs

**CasaOS**: Use the built-in log viewer in the container interface

**Docker Compose**:
```bash
docker-compose -f docker-compose.casaos.yml logs -f rustbot
```

**Direct Docker**:
```bash
docker logs rustbot -f
```

### Common Issues

#### Bot Not Starting
1. **Check Token**: Verify `DISCORD_TOKEN` is correctly set
2. **Check Permissions**: Ensure bot has necessary Discord permissions
3. **Check Logs**: Look for authentication errors

#### High Memory Usage
1. **Reduce Logging**: Set `RUST_LOG=warn` or `RUST_LOG=error`
2. **Monitor Usage**: Use CasaOS resource monitoring

#### Bot Not Responding
1. **Check Permissions**: Bot needs "Send Messages" and "Read Messages"
2. **Check Prefix**: Commands start with `-` (dash)
3. **Check Status**: Verify bot is online in Discord

## Bot Commands

Available commands (prefix: `-`):

| Command | Description | Usage |
|---------|-------------|-------|
| `-ping` | Check bot latency | `-ping` |
| `-hello` | Friendly greeting | `-hello` |
| `-spamping` | Multiple pings | `-spamping [count]` |
| `-uwu` | UwU text transformation | `-uwu <text>` |
| `-coinflip` | Flip a coin | `-coinflip` |
| `-pfp` | Get profile picture | `-pfp [user]` |

## Updating the Bot

### Automatic Updates with Watchtower

Add to your docker-compose.yml:
```yaml
services:
  # ... rustbot service ...
  
  watchtower:
    image: containrrr/watchtower
    container_name: rustbot-watchtower
    restart: unless-stopped
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
    command: --interval 3600 rustbot
    environment:
      - WATCHTOWER_CLEANUP=true
```

### Manual Updates

```bash
# Pull latest image
docker pull deekahy/rustbot:latest

# Restart container (CasaOS or command line)
docker restart rustbot
```

## Development and Custom Builds

### Building Locally

If you need to modify the bot:

```bash
# Clone repository
git clone https://github.com/your-username/RustBot.git
cd RustBot

# Build multi-platform image
./docker-build.sh your_docker_username

# Or build for local platform only
docker build -t your_username/rustbot:latest .
```

### Development Mode

For development with live code changes:

```bash
# Use local build in docker-compose.yml
build: .
# Instead of: image: deekahy/rustbot:latest
```

## Support & Resources

- **Documentation**: [README.md](README.md)
- **CasaOS Guide**: [CASAOS_DEPLOYMENT.md](CASAOS_DEPLOYMENT.md)
- **Docker Commands**: [DOCKER.md](DOCKER.md)
- **Issues**: Report on GitHub Issues
- **Discord Bot Setup**: [Discord Developer Portal](https://discord.com/developers/applications)

## Quick Start Checklist

1. ✅ Get Discord bot token from Discord Developer Portal
2. ✅ Deploy using one of the methods above
3. ✅ Set `DISCORD_TOKEN` environment variable
4. ✅ Verify bot comes online in Discord
5. ✅ Test with `-ping` command
6. ✅ Invite bot to your server with proper permissions
7. ✅ Monitor logs for any issues

---

**Image**: `deekahy/rustbot:latest`  
**Platforms**: `linux/amd64`, `linux/arm64`  
**Size**: ~80MB  
**Security**: Non-root, read-only, resource-limited  
**Health**: Built-in monitoring  
**Updates**: Watchtower compatible  
