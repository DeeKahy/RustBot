#!/bin/bash

# RustBot Deployment Script
# Easy deployment with Docker Compose

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check if Docker is running
check_docker() {
    if ! docker info > /dev/null 2>&1; then
        print_error "Docker is not running or not accessible"
        print_error "Please start Docker and try again"
        exit 1
    fi
}

# Function to check if .env file exists
check_env_file() {
    if [ ! -f .env ]; then
        print_warning ".env file not found"
        if [ -f env.example ]; then
            print_status "Creating .env from env.example..."
            cp env.example .env
            print_warning "Please edit .env file and add your Discord token:"
            print_warning "DISCORD_TOKEN=your_actual_discord_token_here"
            echo ""
            read -p "Press Enter to continue after editing .env file..."
        else
            print_error "No .env file found and no env.example to copy from"
            print_error "Please create a .env file with DISCORD_TOKEN=your_token"
            exit 1
        fi
    fi
}

# Function to validate Discord token
validate_token() {
    if [ -f .env ]; then
        # Source the .env file
        export $(grep -v '^#' .env | xargs)

        if [ -z "$DISCORD_TOKEN" ] || [ "$DISCORD_TOKEN" = "your_discord_bot_token_here" ]; then
            print_error "DISCORD_TOKEN is not set or still has placeholder value"
            print_error "Please edit .env file and set a valid Discord token"
            exit 1
        fi

        if [ ${#DISCORD_TOKEN} -lt 50 ]; then
            print_warning "Discord token seems too short (${#DISCORD_TOKEN} characters)"
            print_warning "Make sure you're using the correct bot token"
        fi
    fi
}

# Function to check platform compatibility
check_platform() {
    local platform=$(uname -m)
    case $platform in
        x86_64|amd64)
            print_status "Detected platform: linux/amd64"
            ;;
        aarch64|arm64)
            print_status "Detected platform: linux/arm64"
            ;;
        *)
            print_warning "Unsupported platform: $platform"
            print_warning "This bot is optimized for linux/amd64 and linux/arm64"
            ;;
    esac
}

# Function to show usage
show_usage() {
    echo "RustBot Deployment Script"
    echo ""
    echo "Usage: $0 [COMMAND]"
    echo ""
    echo "Commands:"
    echo "  start     Start the bot (development mode - builds from source)"
    echo "  prod      Start the bot (production mode - uses Docker Hub image)"
    echo "  casaos    Start the bot (CasaOS mode - optimized for CasaOS)"
    echo "  stop      Stop the bot"
    echo "  restart   Restart the bot"
    echo "  logs      Show bot logs"
    echo "  status    Show bot status"
    echo "  update    Pull latest image and restart (production mode only)"
    echo "  build     Build and optionally push multi-platform image"
    echo "  clean     Clean up containers and images"
    echo "  setup     Initial setup (create .env file)"
    echo ""
    echo "Examples:"
    echo "  $0 setup     # First time setup"
    echo "  $0 start     # Start development bot"
    echo "  $0 prod      # Start production bot"
    echo "  $0 casaos    # Start CasaOS optimized bot"
    echo "  $0 logs      # View logs"
    echo "  $0 stop      # Stop the bot"
}

# Function to setup environment
setup_env() {
    print_status "Setting up RustBot environment..."

    if [ -f .env ]; then
        print_warning ".env file already exists"
        read -p "Do you want to overwrite it? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            print_status "Keeping existing .env file"
            return
        fi
    fi

    if [ -f env.example ]; then
        cp env.example .env
        print_success ".env file created from env.example"
    else
        cat > .env << EOF
# RustBot Environment Variables
DISCORD_TOKEN=your_discord_bot_token_here
RUST_LOG=info
EOF
        print_success ".env file created"
    fi

    print_warning "Please edit .env file and set your Discord bot token:"
    print_warning "DISCORD_TOKEN=your_actual_discord_token_here"
    echo ""
    print_status "You can get a Discord bot token from:"
    print_status "https://discord.com/developers/applications"
}

# Main deployment functions
start_dev() {
    print_status "Starting RustBot in development mode..."
    check_docker
    check_platform
    check_env_file
    validate_token

    docker-compose up -d
    print_success "RustBot started in development mode!"
    print_status "View logs with: $0 logs"
}

start_prod() {
    print_status "Starting RustBot in production mode..."
    check_docker
    check_platform
    check_env_file
    validate_token

    docker-compose -f docker-compose.prod.yml up -d
    print_success "RustBot started in production mode!"
    print_success "Watchtower is running for automatic updates"
    print_status "View logs with: $0 logs"
}

start_casaos() {
    print_status "Starting RustBot in CasaOS mode..."
    check_docker
    check_platform
    check_env_file
    validate_token

    if [ ! -f docker-compose.casaos.yml ]; then
        print_error "docker-compose.casaos.yml not found!"
        print_error "This file is required for CasaOS deployment"
        exit 1
    fi

    docker-compose -f docker-compose.casaos.yml up -d
    print_success "RustBot started in CasaOS mode!"
    print_status "Optimized for CasaOS environment"
    print_status "View logs with: $0 logs"
}

stop_bot() {
    print_status "Stopping RustBot..."

    # Try to stop all versions
    docker-compose down 2>/dev/null || true
    docker-compose -f docker-compose.prod.yml down 2>/dev/null || true
    docker-compose -f docker-compose.casaos.yml down 2>/dev/null || true

    print_success "RustBot stopped!"
}

restart_bot() {
    print_status "Restarting RustBot..."
    stop_bot
    sleep 2

    # Check which compose file exists and was last used
    if docker ps -a --format "table {{.Names}}" | grep -q "rustbot-prod"; then
        start_prod
    elif docker ps -a --format "table {{.Names}}" | grep -q "rustbot.*casaos"; then
        start_casaos
    else
        start_dev
    fi
}

show_logs() {
    print_status "Showing RustBot logs..."

    # Check which container is running
    if docker ps --format "table {{.Names}}" | grep -q "rustbot-prod"; then
        docker-compose -f docker-compose.prod.yml logs -f rustbot
    elif docker ps --format "table {{.Names}}" | grep -q "rustbot.*casaos"; then
        docker-compose -f docker-compose.casaos.yml logs -f rustbot
    elif docker ps --format "table {{.Names}}" | grep -q "rustbot"; then
        docker-compose logs -f rustbot
    else
        print_error "No RustBot container is currently running"
        print_status "Start the bot with: $0 start, $0 prod, or $0 casaos"
        exit 1
    fi
}

show_status() {
    print_status "RustBot Status:"
    echo ""

    # Check for running containers
    if docker ps --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}" | grep -E "(rustbot|watchtower)"; then
        echo ""
        print_status "Container resource usage:"
        docker stats --no-stream --format "table {{.Container}}\t{{.CPUPerc}}\t{{.MemUsage}}" | grep -E "(rustbot|watchtower)" || true
    else
        print_warning "No RustBot containers are currently running"
        print_status "Start the bot with: $0 start or $0 prod"
    fi
}

update_bot() {
    print_status "Updating RustBot (production mode)..."
    check_docker

    # Pull latest image
    docker pull deekahy/rustbot:latest

    # Restart production stack
    docker-compose -f docker-compose.prod.yml down
    docker-compose -f docker-compose.prod.yml up -d

    print_success "RustBot updated and restarted!"
}

build_image() {
    print_status "Building RustBot Docker image with multi-platform support..."
    check_docker

    # Check if buildx is available
    if ! docker buildx version &> /dev/null; then
        print_error "Docker Buildx is required for multi-platform builds"
        print_error "Please update Docker to a version that includes Buildx"
        exit 1
    fi

    # Prompt for Docker username
    echo "Please enter your Docker Hub username (or press Enter for 'deekahy'):"
    read -r DOCKER_USERNAME
    DOCKER_USERNAME=${DOCKER_USERNAME:-deekahy}

    # Create/use multiplatform builder
    BUILDER_NAME="rustbot-builder"
    if ! docker buildx ls | grep -q "$BUILDER_NAME"; then
        print_status "Creating multiplatform builder..."
        docker buildx create --name "$BUILDER_NAME" --driver docker-container --bootstrap
        docker buildx use "$BUILDER_NAME"
    else
        print_status "Using existing multiplatform builder..."
        docker buildx use "$BUILDER_NAME"
    fi

    # Ask about pushing to Docker Hub
    echo ""
    read -p "Do you want to build and push to Docker Hub? (y/N): " -n 1 -r
    echo

    if [[ $REPLY =~ ^[Yy]$ ]]; then
        # Check if user is logged in
        if ! docker info | grep -q "Username:"; then
            print_status "Logging in to Docker Hub..."
            docker login
        fi

        # Build and push multi-platform image
        print_status "Building and pushing multi-platform image..."
        docker buildx build \
            --platform linux/amd64,linux/arm64 \
            --tag "${DOCKER_USERNAME}/rustbot:latest" \
            --push \
            .
        print_success "Multi-platform image built and pushed successfully!"
        print_status "Image: ${DOCKER_USERNAME}/rustbot:latest"
        print_status "Platforms: linux/amd64, linux/arm64"
    else
        # Build locally for current platform
        print_status "Building for local platform only..."
        docker buildx build \
            --tag "${DOCKER_USERNAME}/rustbot:latest" \
            --load \
            .
        print_success "Docker image built locally!"
        print_status "Image: ${DOCKER_USERNAME}/rustbot:latest"
    fi
}

clean_up() {
    print_status "Cleaning up RustBot containers and images..."

    # Stop all containers
    stop_bot

    # Remove containers
    docker container rm rustbot rustbot-prod rustbot-watchtower 2>/dev/null || true

    # Ask about removing images
    echo ""
    read -p "Do you want to remove Docker images as well? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        docker image rm deekahy/rustbot:latest 2>/dev/null || true
        docker image rm rustbot-test 2>/dev/null || true
        print_success "Containers and images cleaned up!"
    else
        print_success "Containers cleaned up!"
    fi
}

# Main script logic
case "${1:-}" in
    "start")
        start_dev
        ;;
    "prod")
        start_prod
        ;;
    "casaos")
        start_casaos
        ;;
    "stop")
        stop_bot
        ;;
    "restart")
        restart_bot
        ;;
    "logs")
        show_logs
        ;;
    "status")
        show_status
        ;;
    "update")
        update_bot
        ;;
    "build")
        build_image
        ;;
    "clean")
        clean_up
        ;;
    "setup")
        setup_env
        ;;
    "help"|"--help"|"-h")
        show_usage
        ;;
    "")
        show_usage
        ;;
    *)
        print_error "Unknown command: $1"
        echo ""
        show_usage
        exit 1
        ;;
esac
