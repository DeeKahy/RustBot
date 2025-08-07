#!/bin/bash
set -e

echo "🤖 RustBot GitHub-based Container Starting..."

# Clone repo on first run, pull updates on subsequent runs
if [ ! -d "/app/RustBot" ]; then
    echo "🔄 First startup - cloning repository from GitHub..."
    git clone -b "${GIT_BRANCH:-developing}" "${REPO_URL:-https://github.com/DeeKahy/RustBot.git}" /app/RustBot
    cd /app/RustBot
    echo "🔨 Building application (this may take a few minutes)..."
    cargo build --release
    echo "✅ Build complete!"
else
    echo "🔄 Checking for updates from GitHub..."
    cd /app/RustBot

    # Configure git to trust the directory
    git config --global --add safe.directory /app/RustBot 2>/dev/null || true

    # Pull latest changes
    if git pull origin "${GIT_BRANCH:-developing}"; then
        echo "✅ Successfully pulled latest changes"
        echo "🔨 Rebuilding application..."
        if cargo build --release; then
            echo "✅ Build successful!"
        else
            echo "❌ Build failed, using previous version"
        fi
    else
        echo "⚠️ Failed to pull updates, using existing version"
    fi
fi

echo "🚀 Starting RustBot..."
cd /app/RustBot

# Start the bot with restart capability for -kys and -update commands
while true; do
    ./target/release/rustbot
    exit_code=$?

    # Exit code 42 means update was requested
    if [ $exit_code -eq 42 ]; then
        echo "🔄 Update requested, pulling latest changes..."

        # Pull latest changes
        if git pull origin "${GIT_BRANCH:-developing}"; then
            echo "✅ Successfully pulled latest changes"
            echo "🔨 Rebuilding with latest changes..."
            if cargo build --release; then
                echo "✅ Build successful, restarting with new version!"
            else
                echo "❌ Build failed, restarting with previous version"
            fi
        else
            echo "⚠️ Failed to pull updates, restarting with current version"
        fi

        sleep 2
        continue

    # Exit code 43 means kys command was used (1-hour cooldown)
    elif [ $exit_code -eq 43 ]; then
        echo "💤 KYS command used, sleeping for 1 hour before restart..."
        echo "⏰ Bot will restart automatically in 1 hour"
        sleep 3600  # Sleep for 1 hour (3600 seconds)
        echo "🌅 1-hour cooldown complete, restarting bot..."
        continue
    else
        echo "🛑 Bot stopped with exit code $exit_code"
        exit $exit_code
    fi
done
