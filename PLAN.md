# Stanza - Twitter Clone Implementation Plan

## Project Overview
A personal Twitter-like microblogging platform where users only see their own messages. Built with SolidJS (frontend) and Rust/Axum (backend) with SQLite database.

## Tech Stack

### Frontend
- **SolidJS** - Reactive UI framework
- **Vite** - Build tool and dev server
- **Solid Router** - Client-side routing
- **vite-plugin-pwa** - PWA capabilities
- **idb-keyval** - IndexedDB wrapper for offline storage
- **date-fns** - Date formatting and manipulation

### Backend
- **Rust** - Programming language
- **Axum** - Web framework
- **Sqlx** - Database toolkit with compile-time checked queries
- **SQLite (WAL mode)** - Embedded database
- **Argon2** - Password hashing
- **UUID** - Unique identifiers
- **Tokio** - Async runtime
- **Serde** - Serialization

---

## Backend Structure

### Project Layout
```
backend/
├── Cargo.toml
├── sqlx-data.json
├── database/
│   └── schema.sql
├── scripts/
│   └── seed.sh
└── src/
    ├── main.rs
    ├── db.rs           # Database connection, queries
    ├── models.rs       # Data structures (User, Message, etc.)
    ├── handlers.rs     # API request handlers
    ├── auth.rs         # Authentication logic, JWT
    ├── middleware.rs   # Auth middleware, CORS
    ├── exports.rs      # Export handlers (JSON/Markdown)
    └── utils.rs        # Password hashing, utilities
```

### Dependencies (Cargo.toml)
```toml
[dependencies]
axum = "0.7"
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.7", features = ["runtime-tokio", "sqlite", "uuid", "chrono"] }
uuid = { version = "1", features = ["serde", "v4"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
argon2 = "0.5"
jsonwebtoken = "9"
chrono = { version = "0.4", features = ["serde"] }
tower-http = { version = "0.5", features = ["cors", "trace"] }
dotenv = "0.15"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
```

### Database Schema

```sql
-- Users table
CREATE TABLE users (
    id TEXT PRIMARY KEY,  -- UUID
    email TEXT UNIQUE NOT NULL,
    username TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    salt TEXT NOT NULL,
    created_at TEXT NOT NULL,  -- ISO 8601 datetime
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_username ON users(username);

-- Messages table
CREATE TABLE messages (
    id TEXT PRIMARY KEY,  -- UUID
    user_id TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_messages_user_id ON messages(user_id);
CREATE INDEX idx_messages_created_at ON messages(created_at DESC);
```

### API Endpoints

#### Authentication
- `POST /api/login`
  - Request: `{ email, password }`
  - Response: `{ token, user: { id, email, username } }`
  - JWT expires in 15 days

#### Messages
- `GET /api/messages?since={timestamp}`
  - Returns user's messages created/updated since timestamp
  - Response: `{ messages: [Message] }`
  - Used for periodic polling

- `POST /api/messages`
  - Request: `{ content }` (or include client-generated `id` for offline sync)
  - Response: `Message`

- `PUT /api/messages/:id`
  - Request: `{ content }`
  - Response: `Message`

- `DELETE /api/messages/:id`
  - Response: `{ success: true }`

#### User Management
- `PUT /api/user/email`
  - Request: `{ email }`
  - Response: `{ success: true }`

- `PUT /api/user/username`
  - Request: `{ username }`
  - Response: `{ success: true }`

- `PUT /api/user/password`
  - Request: `{ current_password, new_password }`
  - Response: `{ success: true }`

#### Export
- `GET /api/export/json`
  - Returns downloadable JSON file of user's messages
  - Response: JSON array of messages

- `GET /api/export/markdown`
  - Returns downloadable Markdown file of user's messages
  - Format: `# Messages\n\n## {timestamp}\n\n{content}\n`

### Data Models

```rust
// User
struct User {
    id: Uuid,
    email: String,
    username: String,
    password_hash: String,
    salt: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

// Message
struct Message {
    id: Uuid,
    user_id: Uuid,
    content: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

// Auth Claims
struct Claims {
    user_id: Uuid,
    exp: usize,  // Expiration timestamp
}
```

### Password Hashing
- Use Argon2id algorithm
- Generate random salt per password
- Salt stored alongside hash in database
- Verify: `argon2::verify_encoded(&hash, password.as_bytes())`

### JWT Configuration
- Secret: Environment variable `JWT_SECRET`
- Algorithm: HS256
- Expiration: 15 days (1296000 seconds)
- Store in HTTP-only cookie in production, localStorage for MVP

### Seeding Script
**Location:** `backend/scripts/seed.sh`

Creates 2 test users:
- User 1: `test1@example.com` / `password123` / `Test User 1`
- User 2: `test2@example.com` / `password123` / `Test User 2`

Each user gets 5 sample messages.

Script uses `argon2` CLI to generate password hashes:
```bash
argon2 "$PASSWORD" -id -t 3 -m 16 -p 4
```

---

## Frontend Structure

### Project Layout
```
frontend/
├── package.json
├── vite.config.ts
├── tsconfig.json
├── pwa-manifest.json
├── sw.js
└── src/
    ├── main.tsx
    ├── App.tsx
    ├── components/
    │   ├── LoginForm.tsx
    │   ├── MessageCard.tsx
    │   ├── MessageView.tsx
    │   ├── MessageInput.tsx
    │   ├── SettingsForm.tsx
    │   ├── ThemeToggle.tsx
    │   ├── Toast.tsx
    │   └── LoadingSpinner.tsx
    ├── pages/
    │   ├── Login.tsx
    │   ├── Feed.tsx
    │   └── Settings.tsx
    ├── stores/
    │   ├── authStore.ts     # Auth state, token
    │   ├── messagesStore.ts # Messages, sync status
    │   └── uiStore.ts       # Theme, UI state
    ├── services/
    │   ├── api.ts           # HTTP client
    │   ├── offlineQueue.ts  # IndexedDB, pending operations
    │   ├── sync.ts          # Sync logic, polling
    │   └── exports.ts       # Export handlers
    ├── hooks/
    │   ├── useAuth.ts
    │   ├── useMessages.ts
    │   ├── useOffline.ts
    │   └── useTheme.ts
    ├── utils/
    │   ├── date.ts          # Date formatting
    │   └── validation.ts    # Form validation
    └── types/
        └── index.ts
```

### Dependencies (package.json)
```json
{
  "dependencies": {
    "solid-js": "^1.8",
    "@solidjs/router": "^0.10",
    "idb-keyval": "^6.2",
    "date-fns": "^3.0"
  },
  "devDependencies": {
    "vite": "^5.0",
    "vite-plugin-pwa": "^0.17",
    "typescript": "^5.3"
  }
}
```

### Key Components

#### ThemeToggle.tsx
- Three states: Auto (default), Light, Dark
- Auto: Uses `prefers-color-scheme` media query
- Persists user preference in localStorage
- Updates CSS variables or class on `document.documentElement`

#### MessageInput.tsx
- Fixed at bottom of screen (ChatGPT style)
- Textarea that auto-expands as user types
- Focuses when clicked
- Submit button (disabled if empty)
- Character count indicator
- Handles Enter key to submit (Shift+Enter for new line)

#### MessageCard.tsx
- Displays message preview (first 200 chars)
- Shows truncated indicator if longer
- Click to view full message
- Shows timestamp in user's local time
- Edit and delete buttons
- Loading states during sync

#### MessageView.tsx
- Full message content
- Back button or modal close
- Edit mode
- Delete confirmation

#### LoginForm.tsx
- Email and password fields
- "Create Account" button (disabled)
- Disabled button shows tooltip: "Account creation coming soon"
- Error toasts
- Loading state during login

#### SettingsForm.tsx
- Change email
- Change username
- Change password (current, new, confirm)
- Export JSON button
- Export Markdown button
- Form validation
- Success/error toasts

### Stores

#### authStore.ts
```typescript
{
  token: string | null,
  user: User | null,
  isAuthenticated: boolean,
  login(email, password): Promise<void>,
  logout(): void,
  updateProfile(updates): Promise<void>
}
```

#### messagesStore.ts
```typescript
{
  messages: Message[],
  isSyncing: boolean,
  lastSync: Date | null,
  addMessage(content): Promise<void>,
  updateMessage(id, content): Promise<void>,
  deleteMessage(id): Promise<void>,
  fetchMessages(): Promise<void>,
  pollMessages(): void
}
```

#### uiStore.ts
```typescript
{
  theme: 'auto' | 'light' | 'dark',
  isOnline: boolean,
  setTheme(theme): void,
  toast(message, type): void
}
```

### Offline Storage (IndexedDB)

Using `idb-keyval` for simplicity:

```typescript
// Stores
'messages'         - All user messages
'pending-ops'      - Queue of pending operations
'last-sync'        - Timestamp of last successful sync
'draft'            - Current message draft
```

**Pending Operation Structure:**
```typescript
type PendingOp = {
  id: string,        // UUID
  type: 'create' | 'update' | 'delete',
  data: any,         // Message data for create/update, message ID for delete
  timestamp: Date,
  retries: number
}
```

### Sync Strategy

#### Online Mode
1. User performs action (create/edit/delete)
2. Optimistic update UI immediately
3. Save to IndexedDB
4. Send API request
5. On success: Remove from pending queue, update lastSync
6. On failure: Add to pending queue, show error toast

#### Offline Mode
1. User performs action
2. Save to IndexedDB
3. Add to pending queue
4. Update UI optimistically
5. Queue persists across page reloads

#### Reconnection Sync
1. Network detected (window.addEventListener('online'))
2. Process pending queue in order (FIFO)
3. For each operation:
   - Send API request
   - On success: Remove from queue
   - On failure: Increment retry counter, keep in queue
   - Max retries: 5, then mark as failed
4. After queue processed: `GET /api/messages?since={lastSync}`
5. Merge new messages with local store

#### Conflict Resolution (Last-Write-Wins)
- Compare timestamps: `server.updated_at` vs `client.updated_at`
- Server timestamp wins
- Client accepts server response
- Update local store with server data

### Polling Configuration
- Interval: 30 seconds
- Only poll when online
- Include `since` parameter to minimize data
- Pause polling when page is hidden (Visibility API)
- Resume polling when page becomes visible

### PWA Configuration

#### pwa-manifest.json
```json
{
  "name": "Stanza",
  "short_name": "Stanza",
  "description": "Personal microblogging platform",
  "start_url": "/",
  "display": "standalone",
  "background_color": "#ffffff",
  "theme_color": "#000000",
  "orientation": "portrait-primary",
  "icons": [
    {
      "src": "/icon-192.png",
      "sizes": "192x192",
      "type": "image/png"
    },
    {
      "src": "/icon-512.png",
      "sizes": "512x512",
      "type": "image/png"
    }
  ]
}
```

#### Service Worker (sw.js)
- Cache-first strategy for static assets
- Network-first strategy for API calls
- Offline fallback page
- Cache versions for updates

#### Vite Config
```typescript
import { VitePWA } from 'vite-plugin-pwa'

export default defineConfig({
  plugins: [
    VitePWA({
      manifest: './pwa-manifest.json',
      workbox: {
        runtimeCaching: [
          {
            urlPattern: /^https:\/\/api\.stanza\.com\/.*/i,
            handler: 'NetworkFirst',
            options: {
              cacheName: 'api-cache',
              expiration: {
                maxEntries: 100,
                maxAgeSeconds: 86400 // 24 hours
              }
            }
          }
        ]
      }
    })
  ]
})
```

### Themes

#### CSS Variables
```css
:root {
  --bg-primary: #ffffff;
  --bg-secondary: #f5f5f5;
  --text-primary: #000000;
  --text-secondary: #666666;
  --border-color: #e0e0e0;
  --accent: #1da1f2;
  --error: #e0245e;
  --success: #17bf63;
}

@media (prefers-color-scheme: dark) {
  :root {
    --bg-primary: #15202b;
    --bg-secondary: #192734;
    --text-primary: #ffffff;
    --text-secondary: #8899a6;
    --border-color: #38444d;
    --accent: #1da1f2;
  }
}

[data-theme="light"] {
  --bg-primary: #ffffff;
  /* ... */
}

[data-theme="dark"] {
  --bg-primary: #15202b;
  /* ... */
}
```

### UI/UX Details

#### Feed Page
- Message list scrolls
- Message input fixed at bottom
- Empty state: "No messages yet. Start typing below!"
- Pull-to-refresh (web API)
- Scroll to top button

#### Message Display
- Preview: First 200 chars
- Truncation indicator: "..." (with character count remaining)
- Timestamp: "Jan 4, 2026 5:30 PM" (local time)
- Edit/delete icons visible on hover/tap

#### Mobile Experience
- Full-height viewport (`100dvh`)
- Touch-friendly targets (44px min)
- Safe area insets for notched devices
- Keyboard doesn't overlap input
- Smooth transitions

#### Loading States
- Skeleton screens for message list
- Spinner for API calls
- Optimistic updates for immediate feedback

#### Error Handling
- Toast notifications for errors
- Retry buttons for failed operations
- Offline indicator when disconnected
- Graceful degradation

### Export Formats

#### JSON Export
```json
[
  {
    "id": "uuid",
    "content": "message text",
    "created_at": "2026-01-04T17:30:00Z",
    "updated_at": "2026-01-04T17:30:00Z"
  }
]
```

#### Markdown Export
```markdown
# Messages Export

Exported: January 4, 2026

---

## January 4, 2026 at 5:30 PM

This is my first message. It can be as long as I want!

---

## January 3, 2026 at 2:15 PM

Another message...

---
```

---

## Implementation Order

### Phase 1: Backend Foundation
1. Set up Rust project with Cargo
2. Configure dependencies
3. Create database schema and migrations
4. Implement password hashing utilities
5. Create user and message models
6. Set up database connection and queries
7. Implement authentication handlers (login)
8. Implement message CRUD endpoints
9. Add user management endpoints
10. Implement export endpoints
11. Create seeding script

### Phase 2: Frontend Foundation
1. Set up SolidJS project with Vite
2. Configure TypeScript and ESLint
3. Set up routing with Solid Router
4. Create basic layout and pages
5. Implement API client
6. Build authentication flow (login)
7. Create auth store
8. Build message list component
9. Implement message input (fixed bottom)
10. Create message card component
11. Add create/update/delete handlers

### Phase 3: Advanced Features
1. Set up IndexedDB with idb-keyval
2. Implement offline queue service
3. Add network detection
4. Build sync service
5. Implement periodic polling
6. Add optimistic updates
7. Handle sync conflicts (LWW)
8. Create settings page
9. Implement theme toggle
10. Add export functionality

### Phase 4: PWA
1. Configure vite-plugin-pwa
2. Create manifest.json
3. Set up service worker
4. Configure caching strategies
5. Test offline functionality
6. Add PWA install prompt

### Phase 5: Polish
1. Theme styling (light/dark/auto)
2. Mobile optimizations
3. Loading states and transitions
4. Error handling and toasts
5. Accessibility improvements
6. Performance optimization
7. Testing (unit, integration)
8. Documentation

---

## Development Workflow

### Backend
```bash
cd backend
cargo run                # Start dev server
cargo test               # Run tests
cargo clippy             # Lint
```

### Frontend
```bash
cd frontend
npm run dev              # Start dev server
npm run build            # Build for production
npm run preview          # Preview production build
npm run lint             # Lint
npm run typecheck        # Type check
```

### Database
```bash
cd backend
sqlite3 stanza.db     # Open database
./scripts/seed.sh        # Seed test data
```

---

## Environment Variables

### Backend (.env)
```
DATABASE_URL=sqlite:stanza.db
JWT_SECRET=your-secret-key
RUST_LOG=debug
```

### Frontend (.env)
```
VITE_API_URL=http://localhost:3000/api
```

---

## Security Considerations

1. **Passwords**
   - Argon2id hashing
   - Minimum 8 characters
   - Server-side validation

2. **Authentication**
   - JWT tokens
   - HttpOnly cookies (production)
   - Secure flag (HTTPS only)

3. **API Security**
   - CORS configured
   - Rate limiting (future)
   - Input validation

4. **Data Privacy**
   - User data isolation (only own messages)
   - No data sharing
   - Export only includes own data

5. **HTTPS**
   - Required in production
   - Protects passwords and tokens

---

## Performance Optimization

1. **Database**
   - SQLite WAL mode for concurrency
   - Indexed columns (user_id, created_at)
   - Connection pooling

2. **Frontend**
   - Virtual scrolling for large message lists (future)
   - Debounced API calls
   - Lazy loading images (future)
   - Code splitting with Vite

3. **Sync**
   - Incremental sync with `since` parameter
   - Batch API calls when possible
   - Compress responses (gzip)

4. **Caching**
   - Service worker caching
   - IndexedDB for fast local reads
   - In-memory caching for frequently accessed data

---

## Future Enhancements (Out of Scope)

1. **Authentication**
   - Email verification
   - Password reset
   - OAuth providers

2. **Messages**
   - Rich text / Markdown support
   - Attachments (images, files)
   - Tags and categories
   - Search and filtering

3. **Real-time**
   - WebSocket support
   - Push notifications

4. **Collaboration**
   - Share messages (read-only)
   - Public profiles

5. **Analytics**
   - Message statistics
   - Writing streaks

6. **Integrations**
   - Import from other platforms
   - Calendar exports
   - RSS feeds

---

## Notes

- All timestamps in UTC, displayed in user's local time
- Message IDs are UUIDs (v4)
- No email verification for MVP
- Create account disabled for now
- Last-Write-Wins conflict resolution
- Offline-first architecture
- PWA installable on mobile
- Theme: Auto (default), Light, Dark
- No quick entry - user clicks input at bottom
- Input expands as user types (ChatGPT style)
- Message preview truncated at 200 characters
