#!/bin/bash
set -e

echo "🔄 Checking for updates from GitHub..."

# Configure git to trust the directory
git config --global --add safe.directory /app 2>/dev/null || git config --global --add safe.directory $(pwd)

# Pull latest changes
if git pull origin developing; then
    echo "✅ Successfully pulled latest changes"

    # Check if there are any changes to source files
    if git diff --name-only HEAD@{1} HEAD | grep -E '\.(rs|toml)$' > /dev/null 2>&1; then
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
    echo "⚠️ Failed to pull updates, using existing version"
fi

echo "🚀 Starting RustBot..."

# Start the bot with restart capability
while true; do
    ./target/release/rustbot 2>/dev/null || cargo run
    exit_code=$?

    # Exit code 42 means update was requested
    if [ $exit_code -eq 42 ]; then
        echo "🔄 Update requested, pulling latest changes..."

        # Pull latest changes
        if git pull origin developing; then
            echo "✅ Successfully pulled latest changes"

            # Rebuild if source files changed
            if git diff --name-only HEAD@{1} HEAD | grep -E '\.(rs|toml)$' > /dev/null 2>&1; then
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
