use sqlx::{migrate::MigrateDatabase, sqlite::SqlitePoolOptions, Pool, Sqlite};
use thiserror::Error;

use crate::models::{Message, SearchQuery, User};

#[derive(Debug, Error)]
pub enum DbError {
    #[error("Database error: {0}")]
    SqlxError(#[from] sqlx::Error),
    #[error("User not found")]
    UserNotFound,
    #[error("Message not found")]
    MessageNotFound,
    #[error("Email already exists")]
    EmailAlreadyExists,
}

pub type DbPool = Pool<Sqlite>;

/// Initialize the database connection pool
pub async fn init_pool(database_url: &str) -> Result<DbPool, DbError> {
    // Create database if it doesn't exist
    if !Sqlite::database_exists(database_url).await.unwrap_or(false) {
        Sqlite::create_database(database_url).await?;
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;

    // Run schema initialization
    init_schema(&pool).await?;

    Ok(pool)
}

/// Initialize the database schema
async fn init_schema(pool: &DbPool) -> Result<(), DbError> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            email TEXT UNIQUE NOT NULL,
            username TEXT NOT NULL,
            password_hash TEXT NOT NULL,
            salt TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_users_email ON users(email)
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_users_username ON users(username)
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_messages_user_id ON messages(user_id)
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_messages_created_at ON messages(created_at DESC)
        "#,
    )
    .execute(pool)
    .await?;

    // Enable WAL mode
    sqlx::query("PRAGMA journal_mode = WAL")
        .execute(pool)
        .await?;

    // Create FTS5 virtual table for full-text search on messages
    sqlx::query(
        r#"
        CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
            id UNINDEXED,
            user_id UNINDEXED,
            content,
            created_at UNINDEXED,
            content='messages',
            content_rowid='rowid'
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create triggers to keep FTS index in sync with messages table
    // We use IF NOT EXISTS to avoid errors on restart
    sqlx::query(
        r#"
        CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages BEGIN
            INSERT INTO messages_fts(rowid, id, user_id, content, created_at)
            VALUES (NEW.rowid, NEW.id, NEW.user_id, NEW.content, NEW.created_at);
        END
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TRIGGER IF NOT EXISTS messages_ad AFTER DELETE ON messages BEGIN
            INSERT INTO messages_fts(messages_fts, rowid, id, user_id, content, created_at)
            VALUES ('delete', OLD.rowid, OLD.id, OLD.user_id, OLD.content, OLD.created_at);
        END
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TRIGGER IF NOT EXISTS messages_au AFTER UPDATE ON messages BEGIN
            INSERT INTO messages_fts(messages_fts, rowid, id, user_id, content, created_at)
            VALUES ('delete', OLD.rowid, OLD.id, OLD.user_id, OLD.content, OLD.created_at);
            INSERT INTO messages_fts(rowid, id, user_id, content, created_at)
            VALUES (NEW.rowid, NEW.id, NEW.user_id, NEW.content, NEW.created_at);
        END
        "#,
    )
    .execute(pool)
    .await?;

    // Always rebuild the FTS index on startup to ensure it's perfectly in sync.
    // This fixes issues where 'COUNT(*)' on external content tables might be misleading,
    // and ensures any missed triggers (e.g. during migrations) are corrected.
    // For <100k messages, this is very fast (sub-second).
    sqlx::query("INSERT INTO messages_fts(messages_fts) VALUES('rebuild')")
        .execute(pool)
        .await?;

    Ok(())
}

// ============ User Operations ============

/// Find a user by email
pub async fn find_user_by_email(pool: &DbPool, email: &str) -> Result<Option<User>, DbError> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ?")
        .bind(email)
        .fetch_optional(pool)
        .await?;

    Ok(user)
}

/// Find a user by ID
pub async fn find_user_by_id(pool: &DbPool, id: &str) -> Result<Option<User>, DbError> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;

    Ok(user)
}

/// Create a new user
#[allow(dead_code)]
pub async fn create_user(pool: &DbPool, user: &User) -> Result<(), DbError> {
    // Check if email already exists
    if find_user_by_email(pool, &user.email).await?.is_some() {
        return Err(DbError::EmailAlreadyExists);
    }

    sqlx::query(
        r#"
        INSERT INTO users (id, email, username, password_hash, salt, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&user.id)
    .bind(&user.email)
    .bind(&user.username)
    .bind(&user.password_hash)
    .bind(&user.salt)
    .bind(&user.created_at)
    .bind(&user.updated_at)
    .execute(pool)
    .await?;

    Ok(())
}

/// List all users
#[allow(dead_code)]
pub async fn list_users(pool: &DbPool) -> Result<Vec<User>, DbError> {
    let users = sqlx::query_as::<_, User>("SELECT * FROM users")
        .fetch_all(pool)
        .await?;
    Ok(users)
}

/// Delete a user by email
#[allow(dead_code)]
pub async fn delete_user_by_email(pool: &DbPool, email: &str) -> Result<(), DbError> {
    let result = sqlx::query("DELETE FROM users WHERE email = ?")
        .bind(email)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::UserNotFound);
    }
    Ok(())
}

/// Update user email
pub async fn update_user_email(pool: &DbPool, user_id: &str, email: &str) -> Result<(), DbError> {
    // Check if email already exists (and it's not the user's current email)
    if let Some(existing_user) = find_user_by_email(pool, email).await? {
        if existing_user.id != user_id {
            return Err(DbError::EmailAlreadyExists);
        }
    }

    let updated_at = chrono::Utc::now().to_rfc3339();

    let result = sqlx::query(
        r#"
        UPDATE users SET email = ?, updated_at = ? WHERE id = ?
        "#,
    )
    .bind(email)
    .bind(&updated_at)
    .bind(user_id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::UserNotFound);
    }

    Ok(())
}

/// Update user username
pub async fn update_user_username(
    pool: &DbPool,
    user_id: &str,
    username: &str,
) -> Result<(), DbError> {
    let updated_at = chrono::Utc::now().to_rfc3339();

    let result = sqlx::query(
        r#"
        UPDATE users SET username = ?, updated_at = ? WHERE id = ?
        "#,
    )
    .bind(username)
    .bind(&updated_at)
    .bind(user_id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::UserNotFound);
    }

    Ok(())
}

/// Update user password
pub async fn update_user_password(
    pool: &DbPool,
    user_id: &str,
    password_hash: &str,
    salt: &str,
) -> Result<(), DbError> {
    let updated_at = chrono::Utc::now().to_rfc3339();

    let result = sqlx::query(
        r#"
        UPDATE users SET password_hash = ?, salt = ?, updated_at = ? WHERE id = ?
        "#,
    )
    .bind(password_hash)
    .bind(salt)
    .bind(&updated_at)
    .bind(user_id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::UserNotFound);
    }

    Ok(())
}

// ============ Message Operations ============

/// Get all messages for a user, optionally filtered by timestamp
pub async fn get_messages_for_user(
    pool: &DbPool,
    user_id: &str,
    since: Option<&str>,
) -> Result<Vec<Message>, DbError> {
    let messages = if let Some(since_timestamp) = since {
        sqlx::query_as::<_, Message>(
            r#"
            SELECT * FROM messages 
            WHERE user_id = ? AND (created_at > ? OR updated_at > ?)
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .bind(since_timestamp)
        .bind(since_timestamp)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, Message>(
            r#"
            SELECT * FROM messages 
            WHERE user_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?
    };

    Ok(messages)
}

/// Create a new message
pub async fn create_message(pool: &DbPool, message: &Message) -> Result<Message, DbError> {
    sqlx::query(
        r#"
        INSERT INTO messages (id, user_id, content, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(&message.id)
    .bind(&message.user_id)
    .bind(&message.content)
    .bind(&message.created_at)
    .bind(&message.updated_at)
    .execute(pool)
    .await?;

    Ok(message.clone())
}

/// Get a message by ID
pub async fn get_message_by_id(pool: &DbPool, id: &str) -> Result<Option<Message>, DbError> {
    let message = sqlx::query_as::<_, Message>("SELECT * FROM messages WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;

    Ok(message)
}

/// Update a message
pub async fn update_message(
    pool: &DbPool,
    id: &str,
    user_id: &str,
    content: &str,
) -> Result<Message, DbError> {
    let updated_at = chrono::Utc::now().to_rfc3339();

    let result = sqlx::query(
        r#"
        UPDATE messages SET content = ?, updated_at = ? WHERE id = ? AND user_id = ?
        "#,
    )
    .bind(content)
    .bind(&updated_at)
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::MessageNotFound);
    }

    // Fetch and return updated message
    get_message_by_id(pool, id)
        .await?
        .ok_or(DbError::MessageNotFound)
}

/// Delete a message
pub async fn delete_message(pool: &DbPool, id: &str, user_id: &str) -> Result<(), DbError> {
    let result = sqlx::query(
        r#"
        DELETE FROM messages WHERE id = ? AND user_id = ?
        "#,
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::MessageNotFound);
    }

    Ok(())
}

/// Search messages using full-text search with optional filters
pub async fn search_messages(
    pool: &DbPool,
    user_id: &str,
    query: &SearchQuery,
) -> Result<Vec<Message>, DbError> {
    // Build the query dynamically based on provided filters
    let mut sql = String::from(
        r#"
        SELECT m.id, m.user_id, m.content, m.created_at, m.updated_at
        FROM messages m
        WHERE m.user_id = ?
        "#,
    );
    let mut has_fts = false;

    // Add FTS condition if query text is provided
    if let Some(ref q) = query.q {
        if !q.trim().is_empty() {
            sql.push_str(
                r#"
                AND m.rowid IN (
                    SELECT rowid FROM messages_fts WHERE messages_fts MATCH ?
                )
                "#,
            );
            has_fts = true;
        }
    }

    // Add date range conditions
    if query.from.is_some() {
        sql.push_str(" AND m.created_at >= ?");
    }
    if query.to.is_some() {
        sql.push_str(" AND m.created_at <= ?");
    }

    // Add hashtag conditions
    // Tags are searched as exact matches with # prefix in content
    if let Some(ref tags) = query.tags {
        let tag_list: Vec<&str> = tags.split(',').map(|t| t.trim()).filter(|t| !t.is_empty()).collect();
        for _ in &tag_list {
            sql.push_str(" AND m.content LIKE ?");
        }
    }

    sql.push_str(" ORDER BY m.created_at DESC LIMIT 100");

    // Execute query with dynamic bindings
    let mut query_builder = sqlx::query_as::<_, Message>(&sql).bind(user_id);

    // Bind FTS query if present
    if has_fts {
        if let Some(ref q) = query.q {
            // Escape special FTS5 characters and format for prefix matching
            let fts_query = format_fts_query(q);
            query_builder = query_builder.bind(fts_query);
        }
    }

    // Bind date filters
    if let Some(ref from) = query.from {
        query_builder = query_builder.bind(from);
    }
    if let Some(ref to) = query.to {
        query_builder = query_builder.bind(to);
    }

    // Bind tag filters
    if let Some(ref tags) = query.tags {
        let tag_list: Vec<&str> = tags.split(',').map(|t| t.trim()).filter(|t| !t.is_empty()).collect();
        for tag in tag_list {
            // Match hashtag anywhere in content (case-insensitive via LIKE)
            let pattern = format!("%#{}%", tag);
            query_builder = query_builder.bind(pattern);
        }
    }

    let messages = query_builder.fetch_all(pool).await?;
    Ok(messages)
}

/// Format a user query for FTS5 MATCH
/// Handles prefix matching and escapes special characters
fn format_fts_query(query: &str) -> String {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    // Split into terms and add prefix matching for each
    let terms: Vec<String> = trimmed
        .split_whitespace()
        .map(|term| {
            // Escape double quotes
            let escaped = term.replace('"', "\"\"");
            // Add prefix matching with * for partial matches
            format!("\"{}\"*", escaped)
        })
        .collect();

    terms.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::hash_password;

    async fn setup_test_db() -> DbPool {
        // Use in-memory SQLite database for tests
        init_pool("sqlite::memory:").await.unwrap()
    }

    fn create_test_user(email: &str) -> User {
        let (hash, salt) = hash_password("password123").unwrap();
        User::new(
            email.to_string(),
            "testuser".to_string(),
            hash,
            salt,
        )
    }

    #[tokio::test]
    async fn test_init_pool_creates_tables() {
        let pool = setup_test_db().await;

        // Tables should exist
        let result = sqlx::query("SELECT name FROM sqlite_master WHERE type='table' AND name='users'")
            .fetch_optional(&pool)
            .await
            .unwrap();
        assert!(result.is_some());

        let result = sqlx::query("SELECT name FROM sqlite_master WHERE type='table' AND name='messages'")
            .fetch_optional(&pool)
            .await
            .unwrap();
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_create_user_success() {
        let pool = setup_test_db().await;
        let user = create_test_user("test@example.com");

        let result = create_user(&pool, &user).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_user_duplicate_email_fails() {
        let pool = setup_test_db().await;
        let user1 = create_test_user("duplicate@example.com");
        let user2 = create_test_user("duplicate@example.com");

        create_user(&pool, &user1).await.unwrap();
        let result = create_user(&pool, &user2).await;

        assert!(matches!(result, Err(DbError::EmailAlreadyExists)));
    }

    #[tokio::test]
    async fn test_find_user_by_email_exists() {
        let pool = setup_test_db().await;
        let user = create_test_user("find@example.com");
        create_user(&pool, &user).await.unwrap();

        let found = find_user_by_email(&pool, "find@example.com").await.unwrap();

        assert!(found.is_some());
        assert_eq!(found.unwrap().email, "find@example.com");
    }

    #[tokio::test]
    async fn test_find_user_by_email_not_exists() {
        let pool = setup_test_db().await;

        let found = find_user_by_email(&pool, "nonexistent@example.com").await.unwrap();

        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_find_user_by_id() {
        let pool = setup_test_db().await;
        let user = create_test_user("byid@example.com");
        let user_id = user.id.clone();
        create_user(&pool, &user).await.unwrap();

        let found = find_user_by_id(&pool, &user_id).await.unwrap();

        assert!(found.is_some());
        assert_eq!(found.unwrap().id, user_id);
    }

    #[tokio::test]
    async fn test_update_user_email() {
        let pool = setup_test_db().await;
        let user = create_test_user("old@example.com");
        let user_id = user.id.clone();
        create_user(&pool, &user).await.unwrap();

        update_user_email(&pool, &user_id, "new@example.com").await.unwrap();

        let found = find_user_by_id(&pool, &user_id).await.unwrap().unwrap();
        assert_eq!(found.email, "new@example.com");
    }

    #[tokio::test]
    async fn test_update_user_username() {
        let pool = setup_test_db().await;
        let user = create_test_user("username@example.com");
        let user_id = user.id.clone();
        create_user(&pool, &user).await.unwrap();

        update_user_username(&pool, &user_id, "newusername").await.unwrap();

        let found = find_user_by_id(&pool, &user_id).await.unwrap().unwrap();
        assert_eq!(found.username, "newusername");
    }

    #[tokio::test]
    async fn test_update_user_password() {
        let pool = setup_test_db().await;
        let user = create_test_user("password@example.com");
        let user_id = user.id.clone();
        let old_hash = user.password_hash.clone();
        create_user(&pool, &user).await.unwrap();

        let (new_hash, new_salt) = hash_password("newpassword").unwrap();
        update_user_password(&pool, &user_id, &new_hash, &new_salt).await.unwrap();

        let found = find_user_by_id(&pool, &user_id).await.unwrap().unwrap();
        assert_ne!(found.password_hash, old_hash);
    }

    #[tokio::test]
    async fn test_create_message() {
        let pool = setup_test_db().await;
        let user = create_test_user("msg@example.com");
        create_user(&pool, &user).await.unwrap();

        let message = Message::new(user.id.clone(), "Hello, world!".to_string());
        let created = create_message(&pool, &message).await.unwrap();

        assert_eq!(created.content, "Hello, world!");
        assert_eq!(created.user_id, user.id);
    }

    #[tokio::test]
    async fn test_get_messages_for_user() {
        let pool = setup_test_db().await;
        let user = create_test_user("getmsgs@example.com");
        create_user(&pool, &user).await.unwrap();

        let msg1 = Message::new(user.id.clone(), "Message 1".to_string());
        let msg2 = Message::new(user.id.clone(), "Message 2".to_string());
        create_message(&pool, &msg1).await.unwrap();
        create_message(&pool, &msg2).await.unwrap();

        let messages = get_messages_for_user(&pool, &user.id, None).await.unwrap();

        assert_eq!(messages.len(), 2);
    }

    #[tokio::test]
    async fn test_get_messages_for_user_filters_by_since() {
        let pool = setup_test_db().await;
        let user = create_test_user("since@example.com");
        create_user(&pool, &user).await.unwrap();

        let msg1 = Message::new(user.id.clone(), "Old message".to_string());
        create_message(&pool, &msg1).await.unwrap();

        // Wait a moment and create another message
        let future_timestamp = chrono::Utc::now().to_rfc3339();

        let messages = get_messages_for_user(&pool, &user.id, Some(&future_timestamp))
            .await
            .unwrap();

        // No messages should be newer than the future timestamp
        assert_eq!(messages.len(), 0);
    }

    #[tokio::test]
    async fn test_get_message_by_id() {
        let pool = setup_test_db().await;
        let user = create_test_user("getbyid@example.com");
        create_user(&pool, &user).await.unwrap();

        let message = Message::new(user.id.clone(), "Find me!".to_string());
        let msg_id = message.id.clone();
        create_message(&pool, &message).await.unwrap();

        let found = get_message_by_id(&pool, &msg_id).await.unwrap();

        assert!(found.is_some());
        assert_eq!(found.unwrap().content, "Find me!");
    }

    #[tokio::test]
    async fn test_update_message() {
        let pool = setup_test_db().await;
        let user = create_test_user("update@example.com");
        create_user(&pool, &user).await.unwrap();

        let message = Message::new(user.id.clone(), "Original content".to_string());
        let msg_id = message.id.clone();
        create_message(&pool, &message).await.unwrap();

        let updated = update_message(&pool, &msg_id, &user.id, "Updated content")
            .await
            .unwrap();

        assert_eq!(updated.content, "Updated content");
    }

    #[tokio::test]
    async fn test_update_message_wrong_user_fails() {
        let pool = setup_test_db().await;
        let user = create_test_user("owner@example.com");
        create_user(&pool, &user).await.unwrap();

        let message = Message::new(user.id.clone(), "My message".to_string());
        let msg_id = message.id.clone();
        create_message(&pool, &message).await.unwrap();

        let result = update_message(&pool, &msg_id, "wrong-user-id", "Hacked!")
            .await;

        assert!(matches!(result, Err(DbError::MessageNotFound)));
    }

    #[tokio::test]
    async fn test_delete_message() {
        let pool = setup_test_db().await;
        let user = create_test_user("delete@example.com");
        create_user(&pool, &user).await.unwrap();

        let message = Message::new(user.id.clone(), "Delete me".to_string());
        let msg_id = message.id.clone();
        create_message(&pool, &message).await.unwrap();

        delete_message(&pool, &msg_id, &user.id).await.unwrap();

        let found = get_message_by_id(&pool, &msg_id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_delete_message_wrong_user_fails() {
        let pool = setup_test_db().await;
        let user = create_test_user("nodelete@example.com");
        create_user(&pool, &user).await.unwrap();

        let message = Message::new(user.id.clone(), "Protected".to_string());
        let msg_id = message.id.clone();
        create_message(&pool, &message).await.unwrap();

        let result = delete_message(&pool, &msg_id, "wrong-user-id").await;

        assert!(matches!(result, Err(DbError::MessageNotFound)));
    }

    #[tokio::test]
    async fn test_user_isolation_messages() {
        let pool = setup_test_db().await;
        let user1 = create_test_user("user1@example.com");
        let user2 = create_test_user("user2@example.com");
        create_user(&pool, &user1).await.unwrap();
        create_user(&pool, &user2).await.unwrap();

        let msg1 = Message::new(user1.id.clone(), "User 1's message".to_string());
        let msg2 = Message::new(user2.id.clone(), "User 2's message".to_string());
        create_message(&pool, &msg1).await.unwrap();
        create_message(&pool, &msg2).await.unwrap();

        let user1_messages = get_messages_for_user(&pool, &user1.id, None).await.unwrap();
        let user2_messages = get_messages_for_user(&pool, &user2.id, None).await.unwrap();

        assert_eq!(user1_messages.len(), 1);
        assert_eq!(user2_messages.len(), 1);
        assert_eq!(user1_messages[0].content, "User 1's message");
        assert_eq!(user2_messages[0].content, "User 2's message");
    }

    #[tokio::test]
    async fn test_search_messages_fts_rebuild() {
        let pool = setup_test_db().await;
        let user = create_test_user("rebuild@example.com");
        create_user(&pool, &user).await.unwrap();

        let msg1 = Message::new(user.id.clone(), "I will mount the TV".to_string());
        let msg2 = Message::new(user.id.clone(), "Mount Everest is tall".to_string());
        create_message(&pool, &msg1).await.unwrap();
        create_message(&pool, &msg2).await.unwrap();

        // Simulate broken index by clearing it (triggers are active on create_message, so index is populated)
        // We manually clear it to test rebuild
        sqlx::query("DELETE FROM messages_fts")
            .execute(&pool)
            .await
            .unwrap();

        // Verify search broken
        let query_broken = SearchQuery { q: Some("mount".to_string()), ..Default::default() };
        let results_broken = search_messages(&pool, &user.id, &query_broken).await.unwrap();
        assert_eq!(results_broken.len(), 0);

        // Run rebuild
        sqlx::query("INSERT INTO messages_fts(messages_fts) VALUES('rebuild')")
            .execute(&pool)
            .await
            .unwrap();

        // Verify search fixed
        let query_fixed = SearchQuery { q: Some("mount".to_string()), ..Default::default() };
        let results_fixed = search_messages(&pool, &user.id, &query_fixed).await.unwrap();
        assert_eq!(results_fixed.len(), 2);
    }
}
