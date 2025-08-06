#!/bin/bash

# Auto-updating RustBot Deployment Script
# This script deploys the RustBot with auto-update capabilities

set -e

echo "🚀 Deploying RustBot with Auto-Update Capabilities"
echo "================================================="

# Check if .env file exists
if [ ! -f .env ]; then
    echo "⚠️  .env file not found!"
    echo "Please create a .env file with your DISCORD_TOKEN"
    echo "You can copy from env.example:"
    echo "cp env.example .env"
    echo "Then edit .env and add your Discord bot token"
    exit 1
fi

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    echo "❌ Docker is not running or not accessible"
    echo "Please start Docker and try again"
    exit 1
fi

# Check if docker-compose is available
if ! command -v docker-compose > /dev/null 2>&1; then
    echo "❌ docker-compose is not installed"
    echo "Please install docker-compose and try again"
    exit 1
fi

# Function to deploy with specific environment
deploy() {
    local env=$1
    local compose_file=$2

    echo "🔧 Deploying in $env mode..."

    # Stop existing containers
    echo "🛑 Stopping existing containers..."
    docker-compose -f $compose_file down --remove-orphans 2>/dev/null || true

    # Remove existing images to force rebuild
    echo "🧹 Cleaning up old images..."
    docker image rm rustbot-rustbot 2>/dev/null || true
    docker image rm rustbot_rustbot 2>/dev/null || true

    # Build and start
    echo "🔨 Building and starting containers..."
    docker-compose -f $compose_file up --build -d

    # Show logs
    echo "📋 Showing startup logs..."
    docker-compose -f $compose_file logs -f --tail=50
}

# Parse command line arguments
case "${1:-dev}" in
    "prod"|"production")
        echo "🏭 Production deployment selected"
        deploy "production" "docker-compose.prod.yml"
        ;;
    "dev"|"development"|"")
        echo "🛠️  Development deployment selected"
        deploy "development" "docker-compose.yml"
        ;;
    "casaos")
        echo "🏠 CasaOS deployment selected"
        if [ -f "docker-compose.casaos.yml" ]; then
            deploy "casaos" "docker-compose.casaos.yml"
        else
            echo "❌ docker-compose.casaos.yml not found"
            exit 1
        fi
        ;;
    "stop")
        echo "🛑 Stopping all RustBot containers..."
        docker-compose down --remove-orphans 2>/dev/null || true
        docker-compose -f docker-compose.prod.yml down --remove-orphans 2>/dev/null || true
        docker-compose -f docker-compose.casaos.yml down --remove-orphans 2>/dev/null || true
        echo "✅ All containers stopped"
        ;;
    "logs")
        echo "📋 Showing logs for running containers..."
        # Try to show logs from any running compose setup
        if docker-compose ps -q > /dev/null 2>&1; then
            docker-compose logs -f --tail=100
        elif docker-compose -f docker-compose.prod.yml ps -q > /dev/null 2>&1; then
            docker-compose -f docker-compose.prod.yml logs -f --tail=100
        elif docker-compose -f docker-compose.casaos.yml ps -q > /dev/null 2>&1; then
            docker-compose -f docker-compose.casaos.yml logs -f --tail=100
        else
            echo "❌ No running containers found"
        fi
        ;;
    "status")
        echo "📊 Container Status:"
        echo "==================="
        docker ps --filter "name=rustbot" --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"
        ;;
    "update")
        echo "🔄 Triggering manual update..."
        # Find the running container and restart it
        container=$(docker ps --filter "name=rustbot" --format "{{.Names}}" | head -1)
        if [ -n "$container" ]; then
            echo "🔄 Restarting $container to trigger update..."
            docker restart "$container"
            docker logs -f "$container"
        else
            echo "❌ No running RustBot container found"
        fi
        ;;
    "shell")
        echo "🐚 Opening shell in RustBot container..."
        container=$(docker ps --filter "name=rustbot" --format "{{.Names}}" | head -1)
        if [ -n "$container" ]; then
            docker exec -it "$container" /bin/bash
        else
            echo "❌ No running RustBot container found"
        fi
        ;;
    "help"|"-h"|"--help")
        echo "Usage: $0 [COMMAND]"
        echo ""
        echo "Commands:"
        echo "  dev, development    Deploy in development mode (default)"
        echo "  prod, production    Deploy in production mode"
        echo "  casaos             Deploy for CasaOS"
        echo "  stop               Stop all RustBot containers"
        echo "  logs               Show logs from running containers"
        echo "  status             Show container status"
        echo "  update             Trigger manual update by restarting container"
        echo "  shell              Open shell in running container"
        echo "  help               Show this help message"
        echo ""
        echo "Examples:"
        echo "  $0                 # Deploy in development mode"
        echo "  $0 prod           # Deploy in production mode"
        echo "  $0 stop           # Stop all containers"
        echo "  $0 logs           # Follow logs"
        ;;
    *)
        echo "❌ Unknown command: $1"
        echo "Use '$0 help' for usage information"
        exit 1
        ;;
esac

echo ""
echo "✅ Operation completed!"
echo ""
echo "💡 Useful commands:"
echo "   View logs: $0 logs"
echo "   Check status: $0 status"
echo "   Update bot: $0 update"
echo "   Stop bot: $0 stop"
echo ""
echo "🔄 The bot will automatically check for updates on startup and can be updated"
echo "   by using the '-update' command in Discord (only available to 'deekahy')"
