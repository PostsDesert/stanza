#!/bin/bash

# Seed script for Dissipate backend
# Creates 2 test users with 5 sample messages each

set -e

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Default database path relative to the backend directory
DB_PATH="${1:-$SCRIPT_DIR/../dissipate.db}"

echo "Seeding database: $DB_PATH"

# Verify database exists and has schema
if ! sqlite3 "$DB_PATH" "SELECT name FROM sqlite_master WHERE type='table' AND name='users';" 2>/dev/null | grep -q users; then
    echo ""
    echo "Error: Database schema not initialized."
    echo "Please run the backend once to initialize the schema:"
    echo "  cd backend && cargo run"
    echo ""
    echo "Then re-run this seed script."
    exit 1
fi

# Check if sqlite3 is available
if ! command -v sqlite3 &> /dev/null; then
    echo "Error: sqlite3 is required but not installed."
    exit 1
fi

# Generate UUIDs (compatible with macOS and Linux)
generate_uuid() {
    if command -v uuidgen &> /dev/null; then
        uuidgen | tr '[:upper:]' '[:lower:]'
    else
        cat /proc/sys/kernel/random/uuid 2>/dev/null || python3 -c "import uuid; print(uuid.uuid4())"
    fi
}

# Get current timestamp in ISO 8601 format
get_timestamp() {
    date -u +"%Y-%m-%dT%H:%M:%SZ"
}

NOW=$(get_timestamp)

# Argon2 hash for "password123" generated using the backend's hash_password function
# Generated with: cargo run --bin hash_password
HASH='$argon2id$v=19$m=19456,t=2,p=1$K7rg5DN2QRVVHrEpLCb/MA$OAPud13UtPoxXEAO5DS15iiwx/nSrxkru08B0eNDuuY'
SALT='K7rg5DN2QRVVHrEpLCb/MA'

USER1_ID=$(generate_uuid)
USER2_ID=$(generate_uuid)

echo "Creating users..."

sqlite3 "$DB_PATH" <<EOF
-- Insert test users
INSERT OR IGNORE INTO users (id, email, username, password_hash, salt, created_at, updated_at)
VALUES 
    ('$USER1_ID', 'test1@example.com', 'Test User 1', '$HASH', '$SALT', '$NOW', '$NOW'),
    ('$USER2_ID', 'test2@example.com', 'Test User 2', '$HASH', '$SALT', '$NOW', '$NOW');

-- Insert sample messages for User 1
INSERT OR IGNORE INTO messages (id, user_id, content, created_at, updated_at)
VALUES
    ('$(generate_uuid)', '$USER1_ID', 'Just setting up my Dissipate account! 🎉', '$NOW', '$NOW'),
    ('$(generate_uuid)', '$USER1_ID', 'This is a great way to jot down my thoughts without worrying about likes or followers.', '$NOW', '$NOW'),
    ('$(generate_uuid)', '$USER1_ID', 'Reminder: Buy groceries tomorrow. Need milk, eggs, and bread.', '$NOW', '$NOW'),
    ('$(generate_uuid)', '$USER1_ID', 'Had an amazing idea for a new project today. Need to sketch it out later.', '$NOW', '$NOW'),
    ('$(generate_uuid)', '$USER1_ID', 'Sometimes the best thoughts come when you least expect them. ✨', '$NOW', '$NOW');

-- Insert sample messages for User 2
INSERT OR IGNORE INTO messages (id, user_id, content, created_at, updated_at)
VALUES
    ('$(generate_uuid)', '$USER2_ID', 'Hello, Dissipate! Ready to start my personal microblog.', '$NOW', '$NOW'),
    ('$(generate_uuid)', '$USER2_ID', 'Today I learned something new about Rust generics. The type system is fascinating!', '$NOW', '$NOW'),
    ('$(generate_uuid)', '$USER2_ID', 'Meeting notes: Discussed the Q1 roadmap. Key focus areas are performance and UX.', '$NOW', '$NOW'),
    ('$(generate_uuid)', '$USER2_ID', 'Random thought: Coffee tastes better when you have a deadline. ☕', '$NOW', '$NOW'),
    ('$(generate_uuid)', '$USER2_ID', 'End of day reflection: Productive day overall. Need to sleep earlier though.', '$NOW', '$NOW');
EOF

echo "Seed complete!"
echo ""
echo "Test credentials:"
echo "  User 1: test1@example.com / password123"
echo "  User 2: test2@example.com / password123"
echo ""
