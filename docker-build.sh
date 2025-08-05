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

# Build the Docker image
docker build -t "${DOCKER_USERNAME}/${IMAGE_NAME}:${TAG}" .

echo "✅ Docker image built successfully!"
echo "📦 Image: ${DOCKER_USERNAME}/${IMAGE_NAME}:${TAG}"

# Ask if user wants to push to Docker Hub
echo ""
read -p "Do you want to push this image to Docker Hub? (y/N): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "🚀 Pushing image to Docker Hub..."

    # Check if user is logged in to Docker Hub
    if ! docker info | grep -q "Username:"; then
        echo "🔐 Logging in to Docker Hub..."
        docker login
    fi

    # Push the image
    echo "📤 Pushing ${DOCKER_USERNAME}/${IMAGE_NAME}:${TAG}..."
    if docker push "${DOCKER_USERNAME}/${IMAGE_NAME}:${TAG}"; then
        echo "✅ Image pushed successfully!"
        echo "🌐 Your image is now available at: https://hub.docker.com/r/${DOCKER_USERNAME}/${IMAGE_NAME}"

        # Also tag and push as 'latest' if not already latest
        if [ "$TAG" != "latest" ]; then
            docker tag "${DOCKER_USERNAME}/${IMAGE_NAME}:${TAG}" "${DOCKER_USERNAME}/${IMAGE_NAME}:latest"
            docker push "${DOCKER_USERNAME}/${IMAGE_NAME}:latest"
            echo "🏷️  Also tagged and pushed as 'latest'"
        fi
    else
        echo "❌ Failed to push image to Docker Hub"
        exit 1
    fi
else
    echo "ℹ️  Image built locally but not pushed to Docker Hub."
fi

echo ""
echo "🏃 To run the container locally:"
echo "docker run -e DISCORD_TOKEN=your_token_here ${DOCKER_USERNAME}/${IMAGE_NAME}:${TAG}"
echo ""
echo "🏃 To run the container with a .env file:"
echo "docker run --env-file .env ${DOCKER_USERNAME}/${IMAGE_NAME}:${TAG}"
