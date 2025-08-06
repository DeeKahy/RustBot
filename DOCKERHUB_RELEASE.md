# RustBot Auto-Update Release 🚀

## Docker Hub Images Available

- **Latest**: `deekahy/rustbot:latest`
- **Auto-Update**: `deekahy/rustbot:autoupdate`

Both tags contain the same auto-updating functionality and support multi-platform deployment.

## 🆕 What's New in This Release

### Auto-Update Functionality
- 🔄 **Automatic GitHub Updates**: Container pulls latest changes from `https://github.com/DeeKahy/RustBot` on startup
- 🎯 **Smart Rebuilding**: Only recompiles when source code actually changes
- ⚡ **Instant Updates**: Just restart the container to get the latest version
- 🎮 **Discord Remote Updates**: New `-update` command for remote updates (deekahy only)

### Enhanced Docker Experience
- 🌍 **Multi-Platform**: Supports both `linux/amd64` and `linux/arm64`
- 🐳 **Self-Contained**: No more dependency on manual Docker Hub pushes
- 🔧 **Better Error Handling**: Graceful fallbacks when updates fail
- 📊 **Enhanced Logging**: Detailed startup and update logs

## 🚀 Quick Start

### Using Docker Run
```bash
docker run -d \
  --name rustbot \
  --restart unless-stopped \
  -e DISCORD_TOKEN=your_discord_token_here \
  -e RUST_LOG=info \
  deekahy/rustbot:latest
```

### Using Docker Compose
```yaml
version: "3.8"
services:
  rustbot:
    image: deekahy/rustbot:latest
    container_name: rustbot
    restart: unless-stopped
    environment:
      - DISCORD_TOKEN=${DISCORD_TOKEN}
      - RUST_LOG=info
      - RUST_BACKTRACE=1
```

### For CasaOS Users
1. **Image**: `deekahy/rustbot:latest`
2. **Environment Variables**:
   - `DISCORD_TOKEN`: Your Discord bot token
   - `RUST_LOG`: `info` (optional)
3. **Restart Policy**: `unless-stopped`
4. **Resources**: 256MB RAM, 0.5 CPU recommended

## 🔄 Auto-Update Features

### Automatic Updates on Container Start
Every time you start or restart the container:
1. 📥 Pulls latest changes from GitHub main branch
2. 🔍 Detects if source code changed
3. 🔨 Rebuilds only if necessary
4. 🚀 Starts bot with latest version

### Discord Remote Updates
The bot now includes a special `-update` command:
- **Who can use it**: Only user "deekahy"
- **What it does**: 
  - Pulls latest GitHub changes
  - Rebuilds if needed
  - Restarts with new version
  - Provides real-time status updates in Discord

### Manual Updates
```bash
# Simply restart the container to trigger update
docker restart rustbot

# Or using docker-compose
docker-compose restart
```

## 📋 Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `DISCORD_TOKEN` | ✅ | - | Your Discord bot token |
| `RUST_LOG` | ❌ | `info` | Log level (debug, info, warn, error) |
| `RUST_BACKTRACE` | ❌ | `1` | Enable backtraces for debugging |

## 🔧 Platform Support

This image supports multiple architectures:
- **linux/amd64**: Intel/AMD x86_64 processors
- **linux/arm64**: ARM64 processors (Apple Silicon M1/M2, ARM servers)

## 🎯 Migration from Previous Versions

### If you're using an older version:
1. **Stop current container**: `docker stop rustbot`
2. **Remove old container**: `docker rm rustbot`
3. **Pull new image**: `docker pull deekahy/rustbot:latest`
4. **Start with new image**: Use the quick start commands above

### Benefits of upgrading:
- ✅ No more manual updates needed
- ✅ Always runs latest code automatically
- ✅ Remote updates via Discord
- ✅ Better error handling and logging
- ✅ Multi-platform support

## 🔍 Verification

After deployment, verify everything is working:

### Check Container Status
```bash
docker ps | grep rustbot
```

### View Logs
```bash
docker logs rustbot -f
```

### Look for these startup messages:
- `🔄 Checking for updates from GitHub...`
- `✅ Successfully pulled latest changes`
- `🚀 Starting RustBot...`
- `🤖 [BotName] is online and ready!`

## 🛠️ Troubleshooting

### Common Issues

**Bot not starting**
- Check if `DISCORD_TOKEN` is set correctly
- Verify the token has proper bot permissions
- Check logs: `docker logs rustbot`

**Update failures**
- Bot continues with current version if updates fail
- Check network connectivity to GitHub
- Review logs for specific error messages

**Permission errors**
- Ensure container has internet access
- Check that GitHub repository is accessible

### Getting Help

1. **Check logs**: `docker logs rustbot -f`
2. **Verify environment**: `docker exec rustbot printenv`
3. **Test connectivity**: `docker exec rustbot git --version`
4. **Repository issues**: Check [GitHub Repository](https://github.com/DeeKahy/RustBot)

## 📈 Performance Notes

### Resource Usage
- **Memory**: ~128-256MB during normal operation
- **CPU**: Minimal usage during operation, higher during builds
- **Storage**: ~500MB for image + build cache
- **Network**: Only pulls changes when source code is updated

### Build Times
- **First startup**: 2-5 minutes (full build)
- **Updates with changes**: 1-3 minutes (incremental build)
- **Updates without changes**: 10-30 seconds (no rebuild)

## 🔐 Security

- Runs as non-privileged user (`rustbot`)
- Limited file system access
- Only pulls from public GitHub repository
- Discord update command restricted to specific user
- No sensitive data stored in container

## 🎉 What's Next?

This auto-update system means:
1. **You never have to manually update again** - just restart the container
2. **Remote updates** - use `-update` in Discord when you can't access the server
3. **Always latest features** - automatically gets new commands and bug fixes
4. **Zero maintenance** - updates happen automatically and safely

---

**Enjoy your self-updating RustBot! 🤖✨**

For more information, visit the [GitHub repository](https://github.com/DeeKahy/RustBot).