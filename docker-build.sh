#!/bin/bash

# Docker build and push script for RustBot
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

# Validate username
if [ -z "$DOCKER_USERNAME" ] || [ "$DOCKER_USERNAME" = "your_docker_username" ]; then
    echo "❌ Error: Please provide a valid Docker Hub username"
    echo "Usage: $0 [docker_username]"
    exit 1
fi

echo "🐳 Building Docker image for RustBot..."

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
echo "🏗️  Building for linux/amd64 and linux/arm64..."

# Check if user wants to push to Docker Hub first
echo ""
read -p "Do you want to push this image to Docker Hub? (y/N): " -n 1 -r
echo

if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "🚀 Building and pushing image to Docker Hub..."

    # Check if user is logged in to Docker Hub
    if ! docker info | grep -q "Username:"; then
        echo "🔐 Logging in to Docker Hub..."
        docker login
    fi

    # Build and push the multi-platform image
    echo "📤 Building and pushing multi-platform image ${DOCKER_USERNAME}/${IMAGE_NAME}:${TAG}..."
    echo "🌍 Platforms: linux/amd64, linux/arm64"

    if docker buildx build \
        --platform linux/amd64,linux/arm64 \
        --tag "${DOCKER_USERNAME}/${IMAGE_NAME}:${TAG}" \
        --push \
        .; then

        echo "✅ Image pushed successfully!"
        echo "🌐 Your image is now available at: https://hub.docker.com/r/${DOCKER_USERNAME}/${IMAGE_NAME}"

        echo ""
        echo "🔍 To inspect the multi-platform manifest:"
        echo "docker buildx imagetools inspect ${DOCKER_USERNAME}/${IMAGE_NAME}:${TAG}"
    else
        echo "❌ Failed to push image to Docker Hub"
        exit 1
    fi
else
    # Build locally for current platform only
    echo "🏗️  Building for local platform only..."

    if docker buildx build \
        --tag "${DOCKER_USERNAME}/${IMAGE_NAME}:${TAG}" \
        --load \
        .; then

        echo "✅ Docker image built locally!"
        echo "📦 Image: ${DOCKER_USERNAME}/${IMAGE_NAME}:${TAG}"
        echo "ℹ️  Image built locally but not pushed to Docker Hub."
    else
        echo "❌ Failed to build image locally"
        exit 1
    fi
fi

echo ""
echo "🏃 To run the container locally:"
echo "docker run -e DISCORD_TOKEN=your_token_here ${DOCKER_USERNAME}/${IMAGE_NAME}:${TAG}"
echo ""
echo "🏃 To run the container with docker-compose:"
echo "DISCORD_TOKEN=your_token_here docker-compose up -d"
echo ""
echo "🌍 Multi-platform support (when pushed to Docker Hub):"
echo "  • linux/amd64 (Intel/AMD x86_64)"
echo "  • linux/arm64 (Apple Silicon M1/M2, ARM servers)"
echo ""
echo "📋 For CasaOS deployment:"
echo "  1. Use image: ${DOCKER_USERNAME}/${IMAGE_NAME}:${TAG}"
echo "  2. Set environment variable: DISCORD_TOKEN=your_bot_token"
echo "  3. Optional: RUST_LOG=info (or debug for more verbose logging)"
