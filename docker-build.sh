#!/bin/bash

# Docker build and push script for RustBot with Auto-Update
# Usage: ./docker-build.sh [docker_username]

set -e  # Exit on any error

# Get Docker username from argument or prompt user
if [ -z "$1" ]; then
    echo "Please enter your Docker Hub username:"
    read -r DOCKER_USERNAME
else
    DOCKER_USERNAME="$1"
fi

IMAGE_NAME="rustbot"
TAG="latest"
AUTO_UPDATE_TAG="autoupdate"

# Validate username
if [ -z "$DOCKER_USERNAME" ] || [ "$DOCKER_USERNAME" = "your_docker_username" ]; then
    echo "❌ Error: Please provide a valid Docker Hub username"
    echo "Usage: $0 [docker_username]"
    exit 1
fi

echo "🐳 Building Docker image for RustBot with Auto-Update..."

# Check if buildx is available
if ! docker buildx version &> /dev/null; then
    echo "❌ Docker Buildx is required for multi-platform builds"
    echo "Please update Docker to a version that includes Buildx"
    exit 1
fi

# Create/use multiplatform builder
BUILDER_NAME="rustbot-builder"
if ! docker buildx ls | grep -q "$BUILDER_NAME"; then
    echo "🔧 Creating multiplatform builder..."
    docker buildx create --name "$BUILDER_NAME" --driver docker-container --bootstrap
    docker buildx use "$BUILDER_NAME"
else
    echo "🔧 Using existing multiplatform builder..."
    docker buildx use "$BUILDER_NAME"
fi

# Build the Docker image for multiple platforms
echo "🏗️  Building Auto-Update RustBot for linux/amd64 and linux/arm64..."
echo "🔄 This version includes auto-update functionality!"
echo "   - Pulls latest changes from GitHub on startup"
echo "   - Includes Discord -update command for deekahy"
echo "   - Smart rebuilding when source changes"

# Check if user wants to push to Docker Hub first
echo ""
read -p "Do you want to push this image to Docker Hub? (y/N): " -n 1 -r
echo

if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "🚀 Building and pushing Auto-Update RustBot to Docker Hub..."

    # Check if user is logged in to Docker Hub
    if ! docker info | grep -q "Username:"; then
        echo "🔐 Logging in to Docker Hub..."
        docker login
    fi

    # Build and push the multi-platform image
    echo "📤 Building and pushing multi-platform Auto-Update image ${DOCKER_USERNAME}/${IMAGE_NAME}:${TAG}..."
    echo "🌍 Platforms: linux/amd64, linux/arm64"
    echo "✨ Features: Auto-update from GitHub, Discord remote updates"

    if docker buildx build \
        --platform linux/amd64,linux/arm64 \
        --tag "${DOCKER_USERNAME}/${IMAGE_NAME}:${TAG}" \
        --push \
        .; then

        echo "✅ Auto-Update RustBot image pushed successfully!"
        echo "🌐 Your image is now available at: https://hub.docker.com/r/${DOCKER_USERNAME}/${IMAGE_NAME}"
        echo "🔄 This version automatically updates from GitHub on container restart!"
        echo "🎮 Use '-update' command in Discord to remotely update (deekahy only)"

        echo ""
        echo "🔍 To inspect the multi-platform manifest:"
        echo "docker buildx imagetools inspect ${DOCKER_USERNAME}/${IMAGE_NAME}:${TAG}"
    else
        echo "❌ Failed to push image to Docker Hub"
        exit 1
    fi
else
    # Build locally for current platform only
    echo "🏗️  Building Auto-Update RustBot for local platform only..."

    if docker buildx build \
        --tag "${DOCKER_USERNAME}/${IMAGE_NAME}:${TAG}" \
        --load \
        .; then

        echo "✅ Auto-Update Docker image built locally!"
        echo "📦 Image: ${DOCKER_USERNAME}/${IMAGE_NAME}:${TAG}"
        echo "ℹ️  Image built locally but not pushed to Docker Hub."
        echo "🔄 This version will auto-update from GitHub on container restart!"
    else
        echo "❌ Failed to build image locally"
        exit 1
    fi
fi

echo ""
echo "🏃 To run the Auto-Update container locally:"
echo "docker run -e DISCORD_TOKEN=your_token_here ${DOCKER_USERNAME}/${IMAGE_NAME}:${TAG}"
echo ""
echo "🏃 To run with docker-compose (recommended):"
echo "DISCORD_TOKEN=your_token_here docker-compose up -d"
echo ""
echo "🔄 Auto-Update Features:"
echo "  • Pulls latest code from GitHub on container start/restart"
echo "  • Use '-update' command in Discord for remote updates (deekahy only)"
echo "  • Smart rebuilding - only compiles when source code changes"
echo "  • No more manual Docker Hub management needed!"
echo ""
echo "🌍 Multi-platform support (when pushed to Docker Hub):"
echo "  • linux/amd64 (Intel/AMD x86_64)"
echo "  • linux/arm64 (Apple Silicon M1/M2, ARM servers)"
echo ""
echo "📋 For CasaOS deployment:"
echo "  1. Use image: ${DOCKER_USERNAME}/${IMAGE_NAME}:${TAG}"
echo "  2. Set environment variable: DISCORD_TOKEN=your_bot_token"
echo "  3. Optional: RUST_LOG=info (or debug for more verbose logging)"
echo "  4. The bot will auto-update from GitHub - just restart container for updates!"
