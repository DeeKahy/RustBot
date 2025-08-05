# RustBot CasaOS Deployment Guide

This guide will help you deploy RustBot on CasaOS with proper multi-platform support and environment variable configuration.

## Quick Setup

### 1. Using the Pre-built Docker Image (Recommended)

**Image**: `deekahy/rustbot:latest`
- ✅ Supports both `linux/amd64` and `linux/arm64`
- ✅ Optimized for production use
- ✅ Multi-stage build for smaller image size
- ✅ Runs as non-root user for security

### 2. Environment Variables

Set these environment variables in CasaOS:

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `DISCORD_TOKEN` | ✅ Yes | - | Your Discord bot token |
| `RUST_LOG` | ❌ No | `info` | Log level (error, warn, info, debug, trace) |
| `RUST_BACKTRACE` | ❌ No | `1` | Enable backtraces for debugging |

### 3. Getting Your Discord Bot Token

1. Go to [Discord Developer Portal](https://discord.com/developers/applications)
2. Create a new application or select an existing one
3. Go to the "Bot" section
4. Copy the bot token
5. **Important**: Keep this token secret and never share it publicly!

## CasaOS Configuration

### Method 1: Using CasaOS App Store (If Available)

If RustBot is available in the CasaOS App Store, simply:
1. Search for "RustBot"
2. Click Install
3. Set your `DISCORD_TOKEN` environment variable
4. Start the container

### Method 2: Manual Docker Compose

1. Create a new application in CasaOS
2. Use the following docker-compose configuration:

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
    
    # Resource limits
    mem_limit: 256m
    mem_reservation: 128m
    cpus: 0.5
    
    # Health check
    healthcheck:
      test: ["CMD", "pgrep", "rustbot"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 15s
    
    # Security
    security_opt:
      - no-new-privileges:true
    read_only: true
    
    # Logging
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"
```

### Method 3: Direct Docker Run

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

## Resource Requirements

### Minimum Requirements
- **RAM**: 128MB
- **CPU**: 0.1 cores
- **Storage**: 50MB (for image)

### Recommended Requirements
- **RAM**: 256MB
- **CPU**: 0.5 cores
- **Storage**: 100MB

## Bot Commands

RustBot comes with the following commands (prefix: `-`):

| Command | Description | Usage |
|---------|-------------|-------|
| `-ping` | Check bot latency | `-ping` |
| `-hello` | Friendly greeting | `-hello` |
| `-spamping` | Multiple pings | `-spamping [count]` |
| `-uwu` | UwU text transformation | `-uwu <text>` |
| `-coinflip` | Flip a coin | `-coinflip` |
| `-pfp` | Get user's profile picture | `-pfp [user]` |

## Monitoring and Troubleshooting

### Viewing Logs

In CasaOS:
1. Go to your RustBot container
2. Click on "Logs" tab
3. Monitor for any errors or issues

Via command line:
```bash
docker logs rustbot -f
```

### Health Check

The container includes a built-in health check that monitors if the bot process is running:
- **Interval**: 30 seconds
- **Timeout**: 10 seconds
- **Retries**: 3
- **Start Period**: 15 seconds

### Common Issues

#### Bot Not Starting
1. **Check Discord Token**: Ensure `DISCORD_TOKEN` is set correctly
2. **Check Token Permissions**: Make sure your bot has the necessary permissions
3. **Check Logs**: Look for error messages in the container logs

#### High Memory Usage
1. **Check Log Level**: Set `RUST_LOG=warn` or `RUST_LOG=error` to reduce logging
2. **Monitor Usage**: Use CasaOS resource monitoring

#### Bot Not Responding to Commands
1. **Check Bot Permissions**: Ensure bot has "Send Messages" and "Read Messages" permissions
2. **Check Prefix**: Commands must start with `-` (dash)
3. **Check Bot Status**: Ensure bot is online in Discord

## Security Best Practices

### Environment Variables
- ✅ **Do**: Store `DISCORD_TOKEN` in CasaOS environment variables
- ❌ **Don't**: Hard-code tokens in compose files
- ❌ **Don't**: Share your bot token publicly

### Container Security
- ✅ Runs as non-root user
- ✅ Read-only filesystem
- ✅ No new privileges
- ✅ Resource limits applied

### Network Security
- ✅ No exposed ports (Discord bots don't need incoming connections)
- ✅ Minimal attack surface

## Updating RustBot

### Automatic Updates (Recommended)
Use Watchtower to automatically update RustBot:

```yaml
version: "3.8"

services:
  rustbot:
    image: deekahy/rustbot:latest
    # ... (your existing config)
    
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

# Restart container in CasaOS or via command line:
docker restart rustbot
```

## Multi-Platform Support

This Docker image supports:
- **linux/amd64** (Intel/AMD 64-bit processors)
- **linux/arm64** (ARM 64-bit processors, including Raspberry Pi 4+)

CasaOS will automatically pull the correct image for your platform.

## Support and Contributing

- **Issues**: Report bugs on [GitHub Issues](https://github.com/your-username/RustBot/issues)
- **Documentation**: Check the main [README.md](README.md)
- **Contributing**: See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup

## Advanced Configuration

### Custom Build
If you need to modify the bot, you can build your own image:

```bash
# Clone the repository
git clone https://github.com/your-username/RustBot.git
cd RustBot

# Build multi-platform image
./docker-build.sh your_docker_username

# Use your custom image in CasaOS
# image: your_docker_username/rustbot:latest
```

### Development Mode
For development, you can build locally:

```bash
# Use local build in compose
build: .
# Instead of: image: deekahy/rustbot:latest
```

---

## Quick Reference

**Image**: `deekahy/rustbot:latest`  
**Required Environment**: `DISCORD_TOKEN=your_token_here`  
**Memory Limit**: `256m`  
**CPU Limit**: `0.5`  
**Health Check**: Built-in process monitoring  
**Security**: Non-root user, read-only filesystem  
**Platforms**: linux/amd64, linux/arm64  
