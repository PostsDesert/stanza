# Stanza - Personal Microblogging Platform

A Twitter-like microblogging platform where users only see their own messages.

## Project Structure

```
stanza/
├── backend/          # Rust/Axum backend
├── frontend/         # SolidJS frontend
├── backend/database/ # SQLite database file (stanza.db)
└── PLAN.md           # Implementation plan
```

## Development

### Quick Start

You can start both the backend and frontend simultaneously using the provided wrapper script in the root directory:

```bash
./dev.sh
```

### Database Setup

The database schema is automatically initialized when you first run the backend. To seed the database with test data:

1. **Start the backend once** to initialize the schema:
   ```bash
   cd backend
   cargo run
   ```
   Press Ctrl+C to stop after it starts.

2. **Run the seed script** to add test users and sample messages:
   ```bash
   cd backend/scripts
   ./seed.sh
   ```
   
   Test credentials:
   - User 1: `test1@example.com` / `password123`
   - User 2: `test2@example.com` / `password123`

### Backend Only

```bash
cd backend
cargo run
```

### Frontend Only

```bash
cd frontend
npm install
npm run dev
```


## Environment Variables

Copy `.env.example` to `.env` and configure:

**Backend (.env):**
- `DATABASE_URL` - SQLite database path
- `JWT_SECRET` - JWT signing secret
- `RUST_LOG` - Log level

**Frontend (.env):**
- `VITE_API_URL` - Backend API URL
