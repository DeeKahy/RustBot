# RustBot Deployment Guide for Ubuntu Server

This guide provides the easiest and most reliable way to deploy RustBot on an Ubuntu server with persistent data storage.

## Prerequisites

- Ubuntu Server (18.04 or later)
- Root or sudo access
- Internet connection

## Quick Start (Recommended)

### 1. Install Docker

```bash
# Update package list
sudo apt update

# Install Docker
sudo apt install -y docker.io docker-compose

# Start and enable Docker
sudo systemctl start docker
sudo systemctl enable docker

# Add your user to docker group (optional, allows running docker without sudo)
sudo usermod -aG docker $USER
# Log out and back in for this to take effect
```

### 2. Clone the Repository

```bash
cd ~
git clone https://github.com/DeeKahy/RustBot.git
cd RustBot
git checkout developing  # or main, depending on which branch you want
```

### 3. Configure Environment Variables

```bash
# Copy the example environment file
cp .env.example .env

# Edit the .env file with your Discord token
nano .env
```

Add your Discord token:
```
DISCORD_TOKEN=your_actual_discord_token_here
PROTECTED_USERS=your_discord_username another_username
```

### 4. Start the Bot

```bash
docker-compose up -d
```

That's it! Your bot is now running with persistent data storage.

## Understanding Persistent Data

The Docker setup now includes two persistent volumes:

1. **`rustbot-data`** - Mounted at `/var/lib/rustbot/`
   - `parking_data.json` - Encrypted parking information
   - `parking_key` - Encryption key for parking data
   
2. **`rustbot-tmp`** - Mounted at `/tmp/`
   - `rustbot_reminders.json` - User reminders

These volumes persist even when you update or restart the container, so your users' data is never lost.

## Managing Your Bot

### View Logs

```bash
# Follow logs in real-time
docker logs -f rustbot

# View last 100 lines
docker logs --tail 100 rustbot
```

### Stop the Bot

```bash
docker-compose down
```

### Restart the Bot

```bash
docker-compose restart
```

### Update the Bot

Use the included update script:

```bash
./update-bot.sh
```

Or manually:

```bash
# Pull the latest image
docker-compose pull

# Restart with new image (keeps all data)
docker-compose up -d
```

## Data Backup

Your bot's data is stored in Docker volumes. To back it up:

```bash
# Create backup directory
mkdir -p ~/rustbot-backups

# Backup parking data
docker run --rm -v rustbot_rustbot-data:/data -v ~/rustbot-backups:/backup ubuntu tar czf /backup/parking-backup-$(date +%Y%m%d-%H%M%S).tar.gz -C /data .

# Backup reminders
docker run --rm -v rustbot_rustbot-tmp:/data -v ~/rustbot-backups:/backup ubuntu tar czf /backup/reminders-backup-$(date +%Y%m%d-%H%M%S).tar.gz -C /data rustbot_reminders.json
```

## Data Restore

To restore from a backup:

```bash
# Stop the bot
docker-compose down

# Restore parking data
docker run --rm -v rustbot_rustbot-data:/data -v ~/rustbot-backups:/backup ubuntu tar xzf /backup/parking-backup-YYYYMMDD-HHMMSS.tar.gz -C /data

# Restore reminders
docker run --rm -v rustbot_rustbot-tmp:/data -v ~/rustbot-backups:/backup ubuntu tar xzf /backup/reminders-backup-YYYYMMDD-HHMMSS.tar.gz -C /data

# Start the bot
docker-compose up -d
```

## Troubleshooting

### Bot won't start

Check logs:
```bash
docker logs rustbot
```

Common issues:
- Invalid Discord token in `.env`
- Port conflicts (unlikely for this bot)
- Out of memory (increase Docker limits)

### Data not persisting

Verify volumes are mounted:
```bash
docker inspect rustbot | grep -A 10 Mounts
```

You should see volumes mounted at `/var/lib/rustbot` and `/tmp`.

### View volume data

```bash
# List parking data files
docker run --rm -v rustbot_rustbot-data:/data ubuntu ls -lah /data

# View reminders file
docker run --rm -v rustbot_rustbot-tmp:/data ubuntu cat /data/rustbot_reminders.json
```

### Recreate volumes (WARNING: This deletes all data!)

```bash
docker-compose down -v  # -v removes volumes
docker-compose up -d    # Creates fresh volumes
```

## Security Notes

1. **Encryption Keys**: The parking data encryption key is stored in the volume. Back it up securely!
2. **File Permissions**: Files in `/var/lib/rustbot/` are set to mode 0600 (owner read/write only)
3. **Environment Variables**: Never commit your `.env` file with real tokens to Git
4. **Network Security**: The bot runs on an isolated Docker network

## Advanced: Using Pre-built Images

If you want to use the pre-built image from GitHub Container Registry:

```bash
# Pull the image
docker pull ghcr.io/deekahy/rustbot:latest

# Use docker-compose.yml as-is (it already references this image)
docker-compose up -d
```

The image automatically:
- Pulls the latest code from GitHub on startup
- Builds the bot inside the container
- Restarts automatically on crashes
- Updates when you use exit code 42 (from `-update` command)

## CasaOS Deployment

If using CasaOS:

1. **Add new application**
2. **Custom Install** → **Docker Compose**
3. **Paste the contents of `docker-compose.yml`**
4. **Set environment variables** in the UI:
   - `DISCORD_TOKEN`: Your bot token
   - `PROTECTED_USERS`: Your username
   - `GIT_BRANCH`: `developing` or `main`
5. **Deploy**

CasaOS will automatically manage the volumes for you.

## Monitoring

### Check bot status
```bash
docker ps | grep rustbot
```

### Check resource usage
```bash
docker stats rustbot
```

### Health check
```bash
docker inspect rustbot | grep -A 5 Health
```

## Updating Environment Variables

```bash
# Edit .env
nano .env

# Restart to apply changes
docker-compose up -d
```

## Getting Help

- Check logs first: `docker logs rustbot`
- Review the [main README](README.md) for bot features
- Check [GitHub Issues](https://github.com/DeeKahy/RustBot/issues)
