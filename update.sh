#!/usr/bin/env bash
# Update script for Stanza
# Pulls latest changes and rebuilds the Docker containers

set -e

# Colors for output
GREEN='\033[0;32m'
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

echo -e "${GREEN}Pulling latest changes from git...${NC}"
git -C "$SCRIPT_DIR" pull

echo -e "${GREEN}Rebuilding and restarting containers...${NC}"
# Use the same command structure as deploy.sh/DEPLOYMENT.md
docker compose -f "$COMPOSE_FILE" --env-file "$SCRIPT_DIR/.env" up -d --build

echo -e "${GREEN}Update complete!${NC}"
echo "Showing recent logs (Ctrl+C to exit logs):"
docker compose -f "$COMPOSE_FILE" logs -f --tail=20
