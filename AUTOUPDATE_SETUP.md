# RustBot Auto-Update Docker Setup 🔄

This document explains how to use the new auto-update Docker setup for RustBot. With this setup, you can simply restart the Docker container to automatically pull the latest changes from GitHub and rebuild the bot.

## Features

- 🔄 **Automatic GitHub Updates**: Pulls latest changes from the main branch on startup
- 🔨 **Smart Rebuilding**: Only rebuilds when source code changes are detected
- 🎮 **Discord Update Command**: Use `-update` command in Discord to trigger updates (deekahy only)
- 🐳 **Docker-First**: Optimized for containerized deployment
- 📊 **Better Logging**: Enhanced logging and status reporting

## Quick Start

### 1. Prerequisites

- Docker and docker-compose installed
- `.env` file with your `DISCORD_TOKEN`

### 2. Deploy the Bot

Use the included deployment script:

```bash
# Development deployment (default)
./deploy-autoupdate.sh

# Production deployment
./deploy-autoupdate.sh prod

# CasaOS deployment
./deploy-autoupdate.sh casaos
```

### 3. Manage the Bot

```bash
# View logs
./deploy-autoupdate.sh logs

# Check status
./deploy-autoupdate.sh status

# Trigger update
./deploy-autoupdate.sh update

# Stop the bot
./deploy-autoupdate.sh stop

# Open shell in container
./deploy-autoupdate.sh shell
```

## How Auto-Updates Work

### On Container Startup

1. 📥 **Git Pull**: Pulls latest changes from `https://github.com/DeeKahy/RustBot`
2. 🔍 **Change Detection**: Checks if any `.rs` or `.toml` files changed
3. 🔨 **Conditional Build**: Only rebuilds if source code changed
4. 🚀 **Bot Start**: Launches the bot with the latest version

### Discord Update Command

The bot includes a special `-update` command that only user "deekahy" can execute:

```
-update
```

This command will:
1. Pull latest changes from GitHub
2. Rebuild the bot if needed
3. Restart with the new version
4. Provide status updates in Discord

### Manual Updates

To manually trigger an update without using Discord:

```bash
# Restart the container (triggers auto-update)
docker restart rustbot

# Or use the deployment script
./deploy-autoupdate.sh update
```

## Docker Compose Configurations

### Development (`docker-compose.yml`)

- Lower resource limits
- Development-friendly settings
- Detailed logging

### Production (`docker-compose.prod.yml`)

- Higher resource limits
- Production optimizations
- Enhanced monitoring

### CasaOS (`docker-compose.casaos.yml`)

- CasaOS-specific configuration
- UI integration friendly

## Environment Variables

Create a `.env` file with:

```env
DISCORD_TOKEN=your_discord_bot_token_here
RUST_LOG=info
RUST_BACKTRACE=1
```

## Container Structure

The Docker container includes:

- **Rust toolchain**: For rebuilding on updates
- **Git**: For pulling latest changes
- **Source code**: Cloned from GitHub
- **Startup script**: Handles the auto-update logic

## Troubleshooting

### Build Failures

If a build fails after an update:
- The bot will continue running with the previous version
- Check logs: `./deploy-autoupdate.sh logs`
- Manually fix issues and restart

### Git Pull Failures

If git pull fails:
- The bot will start with the current version
- Check network connectivity
- Verify GitHub repository access

### Permission Issues

If you get permission errors:
- Ensure the container has proper git configuration
- Check that the repository is publicly accessible

### Update Command Not Working

If `-update` command doesn't work:
- Verify you're using the username "deekahy"
- Check that the bot has necessary permissions in the container
- Review logs for error messages

## Logs and Monitoring

### View Real-time Logs

```bash
./deploy-autoupdate.sh logs
```

### Important Log Messages

- `🔄 Checking for updates from GitHub...` - Update check started
- `✅ Successfully pulled latest changes` - Git pull successful
- `🔨 Source code changes detected, rebuilding...` - Rebuild triggered
- `🚀 Starting RustBot...` - Bot starting
- `🔄 Update requested, pulling latest changes...` - Discord update triggered

### Health Checks

The container includes health checks that monitor:
- Bot process status
- Container responsiveness
- Startup completion

## Security Considerations

- The container runs as a non-privileged user (`rustbot`)
- No write access to sensitive directories
- Git operations are limited to the application directory
- Discord update command restricted to specific user

## Migration from Old Setup

To migrate from the previous Docker setup:

1. **Stop old containers**:
   ```bash
   docker-compose down
   ```

2. **Backup your .env file** (if you have one)

3. **Use new deployment script**:
   ```bash
   ./deploy-autoupdate.sh
   ```

4. **Verify functionality**:
   ```bash
   ./deploy-autoupdate.sh status
   ./deploy-autoupdate.sh logs
   ```

## Advanced Usage

### Custom Git Repository

To use a different repository, modify the Dockerfile:

```dockerfile
# Change this line in the Dockerfile
RUN git clone https://github.com/YourUsername/YourRepo.git .
```

### Custom Update Intervals

The current setup updates on:
- Container restart
- Discord `-update` command

To add scheduled updates, you could extend the startup script or use external tools like cron.

### Multi-branch Support

To use different branches:

1. Modify the startup script to pull from a different branch
2. Set environment variable for branch selection
3. Update the git pull commands accordingly

## Support

If you encounter issues:

1. Check logs: `./deploy-autoupdate.sh logs`
2. Verify status: `./deploy-autoupdate.sh status`
3. Test manual update: `./deploy-autoupdate.sh update`
4. Review this documentation
5. Check GitHub repository for latest changes

---

**Happy Botting! 🤖**

Remember: Just restart the container to get the latest updates automatically!