#!/usr/bin/env bash
# Quick deployment script for Stanza
# This script helps you deploy with the correct configuration

set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMPOSE_FILE=""
for candidate in "$SCRIPT_DIR/docker-compose.yml" "$SCRIPT_DIR/docker/docker-compose.yml"; do
    if [ -f "$candidate" ]; then
        COMPOSE_FILE="$candidate"
        break
    fi
done

if [ -z "$COMPOSE_FILE" ]; then
    echo "Could not find a docker-compose file."
    echo "Checked:"
    echo "  - $SCRIPT_DIR/docker-compose.yml"
    echo "  - $SCRIPT_DIR/docker/docker-compose.yml"
    exit 1
fi

echo -e "${GREEN}Stanza Deployment Helper${NC}"
echo ""

# Check if .env exists
if [ ! -f "$SCRIPT_DIR/.env" ]; then
    echo -e "${YELLOW}No .env file found. Creating one...${NC}"
    
    # Prompt for API URL
    echo ""
    echo "Enter the API URL (where the backend will be accessible from browsers):"
    echo "Examples:"
    echo "  - Local testing: http://localhost:57283/api"
    echo "  - Production: https://yourdomain.com/api"
    read -p "API URL: " api_url
    
    # Prompt for JWT secret
    echo ""
    echo "Enter a JWT secret (minimum 32 characters, random string):"
    echo "You can generate one with: openssl rand -base64 32"
    read -p "JWT Secret: " jwt_secret
    
    # Create .env file
    cat > "$SCRIPT_DIR/.env" << EOF
# Frontend Build Configuration
VITE_API_URL=$api_url

# Backend Runtime Configuration
JWT_SECRET=$jwt_secret
RUST_LOG=info
EOF
    
    echo -e "${GREEN}Created .env file${NC}"
else
    echo -e "${GREEN}.env file already exists${NC}"
fi

echo ""
echo -e "${YELLOW}Current configuration:${NC}"
grep -v "^#" "$SCRIPT_DIR/.env" | grep -v "^$"

echo ""
read -p "Build and deploy with this configuration? (y/n) " -n 1 -r
echo

if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo -e "${GREEN}Building and deploying...${NC}"
    docker compose -f "$COMPOSE_FILE" --env-file "$SCRIPT_DIR/.env" up --build -d
    
    echo ""
    echo -e "${GREEN}Deployment complete!${NC}"
    echo ""
    echo "View logs with: docker compose -f $COMPOSE_FILE logs -f"
    echo "Stop with: docker compose -f $COMPOSE_FILE down"
    echo ""
    echo "To manage users, exec into the container:"
    echo "  docker exec -it stanza /bin/bash"
    echo "  Then run: manage_users add email@example.com username 'password'"
else
    echo "Deployment cancelled"
fi
