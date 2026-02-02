#!/usr/bin/env bash
# Update script for Dissipate
# Pulls latest changes and rebuilds the Docker containers

set -e

# Colors for output
GREEN='\033[0;32m'
NC='\033[0m' # No Color

echo -e "${GREEN}Pulling latest changes from git...${NC}"
git pull

echo -e "${GREEN}Rebuilding and restarting containers...${NC}"
# Use the same command structure as deploy.sh/DEPLOYMENT.md
docker compose -f docker/docker-compose.yml --env-file .env up -d --build

echo -e "${GREEN}Update complete!${NC}"
echo "Showing recent logs (Ctrl+C to exit logs):"
docker compose -f docker/docker-compose.yml logs -f --tail=20
