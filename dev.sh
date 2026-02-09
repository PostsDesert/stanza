#!/bin/bash

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

echo "Starting Dissipate Development Environment..."

# Start Backend
echo "Starting Backend..."
(cd backend && DATABASE_URL=sqlite:database/dissipate.db JWT_SECRET=dev-secret-key cargo run) &

# Start Frontend
echo "Starting Frontend..."
(cd frontend && npm run dev) &

# Wait for background processes
wait
