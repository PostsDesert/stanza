#!/bin/bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Function to handle cleanup on exit
cleanup() {
    echo ""
    echo "Stopping backend and frontend..."
    # Kill all background processes in this process group
    kill $(jobs -p) 2>/dev/null
    exit
}

# Trap Ctrl+C (SIGINT) and exit (SIGTERM)
trap cleanup SIGINT SIGTERM

echo "Starting Stanza Development Environment..."

# Start Backend
echo "Starting Backend..."
(cd "$SCRIPT_DIR/backend" && DATABASE_URL=sqlite:database/stanza.db JWT_SECRET=dev-secret-key cargo run) &

# Start Frontend
echo "Starting Frontend..."
(cd "$SCRIPT_DIR/frontend" && npm run dev) &

# Wait for background processes
wait
