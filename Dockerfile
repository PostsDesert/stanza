# --- Stage 1: Build Frontend ---
FROM node:20-slim AS frontend-builder

# Accept API URL as build argument - this must be set at build time
# Example: docker build --build-arg VITE_API_URL=https://api.example.com/api ...
ARG VITE_API_URL
ENV VITE_API_URL=${VITE_API_URL}

WORKDIR /app/frontend
COPY frontend/package*.json ./
RUN npm install
COPY frontend/ ./
RUN npm run build

# --- Stage 2: Build Backend ---
FROM rust:1.92.0-slim-bookworm AS backend-builder
WORKDIR /app
# Install build dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Copy backend source
COPY backend/ ./

# Copy frontend build results so they are available for include_str! or ServeDir
COPY --from=frontend-builder /app/frontend/dist ./dist

# Build the backend and management utility
RUN cargo build --release

# --- Stage 3: Final Runtime Image ---
FROM debian:bookworm-slim
WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    sqlite3 \
    && rm -rf /var/lib/apt/lists/*

# Copy the backend binary and the management utility
COPY --from=backend-builder /app/target/release/dissipate-backend /app/
COPY --from=backend-builder /app/target/release/manage_users /app/

# Copy the frontend static files
COPY --from=frontend-builder /app/frontend/dist /app/dist

# Environment variables
ENV DATABASE_URL="sqlite:///app/database/dissipate.db"
ENV JWT_SECRET="change_me_in_production"
ENV RUST_LOG="info"
ENV PATH="/app:${PATH}"

# Persistence
RUN mkdir -p /app/database
VOLUME /app/database

# Expose the backend port
EXPOSE 3000

# Start the backend
CMD ["./dissipate-backend"]
