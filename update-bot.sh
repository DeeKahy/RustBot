#!/bin/bash
# RustBot Update Script
# This script safely updates your RustBot container while preserving all data

set -e

echo "🤖 RustBot Update Script"
echo "========================"
echo ""

# Check if docker-compose exists
if ! command -v docker-compose &> /dev/null; then
    echo "❌ Error: docker-compose is not installed"
    exit 1
fi

# Check if docker-compose.yml exists
if [ ! -f "docker-compose.yml" ]; then
    echo "❌ Error: docker-compose.yml not found"
    echo "Please run this script from the RustBot directory"
    exit 1
fi

echo "📊 Current bot status:"
docker-compose ps
echo ""

# Ask for confirmation
read -p "🔄 Pull latest image and restart the bot? (y/N) " -n 1 -r
echo ""
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "❌ Update cancelled"
    exit 0
fi

echo ""
echo "📥 Pulling latest image..."
if ! docker-compose pull; then
    echo "❌ Failed to pull latest image"
    exit 1
fi

echo ""
echo "🔄 Stopping old container..."
docker-compose down

echo ""
echo "🚀 Starting updated container..."
if ! docker-compose up -d; then
    echo "❌ Failed to start container"
    exit 1
fi

echo ""
echo "⏳ Waiting for container to start..."
sleep 5

echo ""
echo "📊 New bot status:"
docker-compose ps

echo ""
echo "📋 Recent logs:"
docker logs --tail 20 rustbot

echo ""
echo "✅ Update complete!"
echo ""
echo "💡 Helpful commands:"
echo "  View logs:       docker logs -f rustbot"
echo "  Restart bot:     docker-compose restart"
echo "  Stop bot:        docker-compose down"
echo "  Check status:    docker-compose ps"
