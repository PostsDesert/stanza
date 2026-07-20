# Stanza Deployment Guide

## Environment Configuration

The application requires configuration for both the **frontend** (at build time) and **backend** (at runtime).

### Frontend Configuration (Build Time)

The frontend needs to know the API URL **when it's being built**. This is set via the `VITE_API_URL` environment variable.

**Important:** The API URL must be accessible from the **user's browser**, not from inside the Docker container.

### Backend Configuration (Runtime)

The backend configuration is set at runtime through environment variables:
- `DATABASE_URL`: SQLite database path (default: `sqlite:///app/database/stanza.db`)
- `JWT_SECRET`: Secret key for JWT token generation (⚠️ **MUST** be changed in production!)
- `RUST_LOG`: Logging level (options: `error`, `warn`, `info`, `debug`, `trace`)

## Deployment Scenarios

### Scenario 1: Local Testing with Docker

If you're running the container locally and accessing it from your local browser:

```bash
# Create a .env file
cat > .env << 'EOF'
VITE_API_URL=http://localhost:57283/api
JWT_SECRET=your-secret-key-here
EOF

# Build and run
docker compose -f docker/docker-compose.yml --env-file .env up -d --build
```

Access the app at: `http://localhost:57283`

### Scenario 2: Production Deployment (Single Server)

If deploying on a server with domain `example.com`:

```bash
# Create a .env file
cat > .env << 'EOF'
VITE_API_URL=https://example.com/api
JWT_SECRET=your-long-random-secret-key-min-32-chars
RUST_LOG=info
EOF

# Build and run
docker compose -f docker/docker-compose.yml --env-file .env up -d --build
```

**Note:** You'll need a reverse proxy (nginx, Caddy, Traefik) to handle HTTPS and forward traffic to port 3000.

### Scenario 3: Production with Separate API Domain

If your API is on a different domain (e.g., `api.example.com`):

```bash
# Create a .env file
cat > .env << 'EOF'
VITE_API_URL=https://api.example.com/api
JWT_SECRET=your-long-random-secret-key-min-32-chars
RUST_LOG=info
EOF

# Build and run
docker compose -f docker/docker-compose.yml --env-file .env up -d --build
```

## Manual Docker Build (Without Docker Compose)

If you prefer to use Docker directly:

```bash
# Build the image
docker build \
  --build-arg VITE_API_URL=https://your-domain.com/api \
  -t stanza:latest \
  -f docker/Dockerfile \
  .

# Run the container
docker run -d \
  --name stanza \
  -p 3000:3000 \
  -e DATABASE_URL=sqlite:///app/database/stanza.db \
  -e JWT_SECRET=your-secret-key \
  -e RUST_LOG=info \
  -v stanza-data:/app/database \
  stanza:latest
```

## User Management

The container includes a `manage_users` binary for user management:

```bash
# Exec into the container
docker exec -it stanza /bin/bash

# Add a user
manage_users add email@example.com username 'password123'

# List users
manage_users list

# Remove a user
manage_users remove email@example.com
```

## Security Checklist

- [ ] Change `JWT_SECRET` to a long random string (minimum 32 characters)
- [ ] Use HTTPS in production (set up reverse proxy with SSL)
- [ ] Set `RUST_LOG=info` or `warn` in production (avoid `debug` or `trace`)
- [ ] Regularly backup the `backend/database/stanza.db` file
- [ ] Keep Docker images up to date

## Troubleshooting

### CORS Errors

If you see CORS errors in the browser console:

1. **Check the API URL**: Make sure `VITE_API_URL` points to where your backend is actually accessible from the **browser** (not from inside Docker)
2. **Rebuild**: Changes to `VITE_API_URL` require rebuilding the image:
   ```bash
   docker-compose down
   docker-compose up --build
   ```
3. **Verify**: Check the browser's network tab to see what URL it's trying to reach

### Database Management

The database is stored as a file on the host machine at `backend/database/stanza.db`. This file is mounted into the container.

**Backup:**
Simply copy the `backend/database/stanza.db` file to a safe location.

**Reset Database:**
To reset the database (⚠️ deletes all data), stop the container and delete the file:
```bash
docker-compose down
rm backend/database/stanza.db
docker-compose up -d --build
```

### Logs

View application logs:
```bash
# All logs
docker-compose logs -f

# Just application logs
docker logs -f stanza
```

## Example nginx Configuration

If you're using nginx as a reverse proxy:

```nginx
server {
    listen 80;
    server_name example.com;
    
    # Redirect HTTP to HTTPS
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name example.com;
    
    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;
    
    location / {
        proxy_pass http://localhost:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

Then set `VITE_API_URL=https://example.com/api` when building.
