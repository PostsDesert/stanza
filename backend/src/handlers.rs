use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

use crate::{
    auth::{create_token, AuthError},
    db::{self, DbError, DbPool},
    models::*,
    utils::{hash_password, verify_password},
};

/// Application state shared across handlers
pub struct AppState {
    pub pool: DbPool,
    pub jwt_secret: String,
}

pub type SharedState = Arc<AppState>;

/// Error response type
#[derive(Debug, serde::Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

impl ErrorResponse {
    pub fn new(message: impl Into<String>) -> Json<ErrorResponse> {
        Json(ErrorResponse {
            error: message.into(),
        })
    }
}

/// Convert DbError to HTTP response
impl IntoResponse for DbError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            DbError::UserNotFound => (StatusCode::NOT_FOUND, "User not found"),
            DbError::MessageNotFound => (StatusCode::NOT_FOUND, "Message not found"),
            DbError::EmailAlreadyExists => (StatusCode::CONFLICT, "Email already exists"),
            DbError::SqlxError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Database error"),
        };

        (status, ErrorResponse::new(message)).into_response()
    }
}

/// Convert AuthError to HTTP response
impl IntoResponse for AuthError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            AuthError::TokenExpired => (StatusCode::UNAUTHORIZED, "Token expired"),
            AuthError::InvalidToken(_) => (StatusCode::UNAUTHORIZED, "Invalid token"),
            AuthError::MissingAuthHeader => (StatusCode::UNAUTHORIZED, "Missing authorization"),
            AuthError::InvalidAuthHeader => (StatusCode::UNAUTHORIZED, "Invalid authorization header"),
            AuthError::TokenCreationError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create token")
            }
        };

        (status, ErrorResponse::new(message)).into_response()
    }
}

// ============ Authentication Handlers ============

/// POST /api/login
/// Authenticate user and return JWT token
pub async fn login(
    State(state): State<SharedState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Find user by email
    let user = db::find_user_by_email(&state.pool, &payload.email)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorResponse::new("Database error"),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                ErrorResponse::new("Invalid email or password"),
            )
        })?;

    // Verify password
    let is_valid = verify_password(&payload.password, &user.password_hash).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            ErrorResponse::new("Password verification error"),
        )
    })?;

    if !is_valid {
        return Err((
            StatusCode::UNAUTHORIZED,
            ErrorResponse::new("Invalid email or password"),
        ));
    }

    // Create JWT token
    let token = create_token(&user.id, &state.jwt_secret).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            ErrorResponse::new("Failed to create token"),
        )
    })?;

    Ok(Json(LoginResponse {
        token,
        user: user.to_public(),
    }))
}

// ============ Message Handlers ============

/// GET /api/messages
/// Get all messages for authenticated user
pub async fn get_messages(
    State(state): State<SharedState>,
    user_id: String,
    Query(query): Query<MessagesQuery>,
) -> Result<Json<MessagesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let messages =
        db::get_messages_for_user(&state.pool, &user_id, query.since.as_deref())
            .await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ErrorResponse::new("Database error"),
                )
            })?;

    let message_responses: Vec<MessageResponse> =
        messages.iter().map(|m| m.to_response()).collect();

    Ok(Json(MessagesResponse {
        messages: message_responses,
    }))
}

/// POST /api/messages
/// Create a new message
pub async fn create_message(
    State(state): State<SharedState>,
    user_id: String,
    Json(payload): Json<CreateMessageRequest>,
) -> Result<(StatusCode, Json<MessageResponse>), (StatusCode, Json<ErrorResponse>)> {
    // Validate content
    if payload.content.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            ErrorResponse::new("Content cannot be empty"),
        ));
    }

    // Create message (with optional client-provided ID)
    let message = if let Some(id) = payload.id {
        Message::with_id(id, user_id, payload.content)
    } else {
        Message::new(user_id, payload.content)
    };

    let created = db::create_message(&state.pool, &message).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            ErrorResponse::new("Failed to create message"),
        )
    })?;

    Ok((StatusCode::CREATED, Json(created.to_response())))
}

/// PUT /api/messages/:id
/// Update a message
pub async fn update_message(
    State(state): State<SharedState>,
    user_id: String,
    Path(message_id): Path<String>,
    Json(payload): Json<UpdateMessageRequest>,
) -> Result<Json<MessageResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Validate content
    if payload.content.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            ErrorResponse::new("Content cannot be empty"),
        ));
    }

    let updated = db::update_message(&state.pool, &message_id, &user_id, &payload.content)
        .await
        .map_err(|e| match e {
            DbError::MessageNotFound => (StatusCode::NOT_FOUND, ErrorResponse::new("Message not found")),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorResponse::new("Failed to update message"),
            ),
        })?;

    Ok(Json(updated.to_response()))
}

/// DELETE /api/messages/:id
/// Delete a message
pub async fn delete_message(
    State(state): State<SharedState>,
    user_id: String,
    Path(message_id): Path<String>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    db::delete_message(&state.pool, &message_id, &user_id)
        .await
        .map_err(|e| match e {
            DbError::MessageNotFound => (StatusCode::NOT_FOUND, ErrorResponse::new("Message not found")),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorResponse::new("Failed to delete message"),
            ),
        })?;

    Ok(Json(SuccessResponse::new()))
}

/// GET /api/messages/search
/// Search messages with full-text search and filters
pub async fn search_messages(
    State(state): State<SharedState>,
    user_id: String,
    Query(query): Query<SearchQuery>,
) -> Result<Json<SearchResponse>, (StatusCode, Json<ErrorResponse>)> {
    let messages = db::search_messages(&state.pool, &user_id, &query)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorResponse::new("Search failed"),
            )
        })?;

    let total = messages.len();
    let message_responses: Vec<MessageResponse> = messages.iter().map(|m| m.to_response()).collect();

    Ok(Json(SearchResponse {
        messages: message_responses,
        total,
    }))
}

// ============ User Management Handlers ============

/// PUT /api/user/email
/// Update user email
pub async fn update_email(
    State(state): State<SharedState>,
    user_id: String,
    Json(payload): Json<UpdateEmailRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Validate email format
    if !payload.email.contains('@') {
        return Err((
            StatusCode::BAD_REQUEST,
            ErrorResponse::new("Invalid email format"),
        ));
    }

    db::update_user_email(&state.pool, &user_id, &payload.email)
        .await
        .map_err(|e| match e {
            DbError::EmailAlreadyExists => {
                (StatusCode::CONFLICT, ErrorResponse::new("Email already exists"))
            }
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorResponse::new("Failed to update email"),
            ),
        })?;

    Ok(Json(SuccessResponse::new()))
}

/// PUT /api/user/username
/// Update user username
pub async fn update_username(
    State(state): State<SharedState>,
    user_id: String,
    Json(payload): Json<UpdateUsernameRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Validate username
    if payload.username.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            ErrorResponse::new("Username cannot be empty"),
        ));
    }

    db::update_user_username(&state.pool, &user_id, &payload.username)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorResponse::new("Failed to update username"),
            )
        })?;

    Ok(Json(SuccessResponse::new()))
}

/// PUT /api/user/password
/// Update user password
pub async fn update_password(
    State(state): State<SharedState>,
    user_id: String,
    Json(payload): Json<UpdatePasswordRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Get current user
    let user = db::find_user_by_id(&state.pool, &user_id)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorResponse::new("Database error"),
            )
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, ErrorResponse::new("User not found")))?;

    // Verify current password
    let is_valid = verify_password(&payload.current_password, &user.password_hash).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            ErrorResponse::new("Password verification error"),
        )
    })?;

    if !is_valid {
        return Err((
            StatusCode::UNAUTHORIZED,
            ErrorResponse::new("Invalid current password"),
        ));
    }

    // Validate new password
    if payload.new_password.len() < 8 {
        return Err((
            StatusCode::BAD_REQUEST,
            ErrorResponse::new("Password must be at least 8 characters"),
        ));
    }

    // Hash new password
    let (new_hash, new_salt) = hash_password(&payload.new_password).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            ErrorResponse::new("Failed to hash password"),
        )
    })?;

    // Update password
    db::update_user_password(&state.pool, &user_id, &new_hash, &new_salt)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorResponse::new("Failed to update password"),
            )
        })?;

    Ok(Json(SuccessResponse::new()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::hash_password;

    async fn setup_test_state() -> SharedState {
        let pool = db::init_pool("sqlite::memory:").await.unwrap();
        Arc::new(AppState {
            pool,
            jwt_secret: "test-secret".to_string(),
        })
    }

    async fn create_test_user(state: &SharedState, email: &str, password: &str) -> User {
        let (hash, salt) = hash_password(password).unwrap();
        let user = User::new(email.to_string(), "testuser".to_string(), hash, salt);
        db::create_user(&state.pool, &user).await.unwrap();
        user
    }

    #[tokio::test]
    async fn test_login_success() {
        let state = setup_test_state().await;
        create_test_user(&state, "login@example.com", "password123").await;

        let request = LoginRequest {
            email: "login@example.com".to_string(),
            password: "password123".to_string(),
        };

        let result = login(State(state), Json(request)).await;

        assert!(result.is_ok());
        let response = result.unwrap().0;
        assert!(!response.token.is_empty());
        assert_eq!(response.user.email, "login@example.com");
    }

    #[tokio::test]
    async fn test_login_wrong_email() {
        let state = setup_test_state().await;

        let request = LoginRequest {
            email: "nonexistent@example.com".to_string(),
            password: "password123".to_string(),
        };

        let result = login(State(state), Json(request)).await;

        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_login_wrong_password() {
        let state = setup_test_state().await;
        create_test_user(&state, "wrongpw@example.com", "password123").await;

        let request = LoginRequest {
            email: "wrongpw@example.com".to_string(),
            password: "wrongpassword".to_string(),
        };

        let result = login(State(state), Json(request)).await;

        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_get_messages_empty() {
        let state = setup_test_state().await;
        let user = create_test_user(&state, "empty@example.com", "password123").await;

        let result = get_messages(
            State(state),
            user.id,
            Query(MessagesQuery::default()),
        )
        .await;

        assert!(result.is_ok());
        assert!(result.unwrap().0.messages.is_empty());
    }

    #[tokio::test]
    async fn test_create_message_success() {
        let state = setup_test_state().await;
        let user = create_test_user(&state, "create@example.com", "password123").await;

        let request = CreateMessageRequest {
            content: "Hello, world!".to_string(),
            id: None,
        };

        let result = create_message(State(state), user.id.clone(), Json(request)).await;

        assert!(result.is_ok());
        let (status, response) = result.unwrap();
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(response.0.content, "Hello, world!");
    }

    #[tokio::test]
    async fn test_create_message_with_client_id() {
        let state = setup_test_state().await;
        let user = create_test_user(&state, "clientid@example.com", "password123").await;

        let client_id = "custom-uuid-123".to_string();
        let request = CreateMessageRequest {
            content: "Message with custom ID".to_string(),
            id: Some(client_id.clone()),
        };

        let result = create_message(State(state), user.id, Json(request)).await;

        assert!(result.is_ok());
        let (_, response) = result.unwrap();
        assert_eq!(response.0.id, client_id);
    }

    #[tokio::test]
    async fn test_create_message_empty_content_fails() {
        let state = setup_test_state().await;
        let user = create_test_user(&state, "emptymsg@example.com", "password123").await;

        let request = CreateMessageRequest {
            content: "   ".to_string(),
            id: None,
        };

        let result = create_message(State(state), user.id, Json(request)).await;

        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_update_message_success() {
        let state = setup_test_state().await;
        let user = create_test_user(&state, "updatemsg@example.com", "password123").await;

        // Create a message first
        let message = Message::new(user.id.clone(), "Original".to_string());
        db::create_message(&state.pool, &message).await.unwrap();

        let request = UpdateMessageRequest {
            content: "Updated content".to_string(),
        };

        let result = update_message(
            State(state),
            user.id,
            Path(message.id),
            Json(request),
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().0.content, "Updated content");
    }

    #[tokio::test]
    async fn test_update_message_not_found() {
        let state = setup_test_state().await;
        let user = create_test_user(&state, "notfound@example.com", "password123").await;

        let request = UpdateMessageRequest {
            content: "Update non-existent".to_string(),
        };

        let result = update_message(
            State(state),
            user.id,
            Path("non-existent-id".to_string()),
            Json(request),
        )
        .await;

        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_delete_message_success() {
        let state = setup_test_state().await;
        let user = create_test_user(&state, "deletemsg@example.com", "password123").await;

        let message = Message::new(user.id.clone(), "Delete me".to_string());
        db::create_message(&state.pool, &message).await.unwrap();

        let result = delete_message(
            State(state.clone()),
            user.id.clone(),
            Path(message.id.clone()),
        )
        .await;

        assert!(result.is_ok());

        // Verify message is gone
        let deleted = db::get_message_by_id(&state.pool, &message.id).await.unwrap();
        assert!(deleted.is_none());
    }

    #[tokio::test]
    async fn test_update_email_success() {
        let state = setup_test_state().await;
        let user = create_test_user(&state, "oldemail@example.com", "password123").await;

        let request = UpdateEmailRequest {
            email: "newemail@example.com".to_string(),
        };

        let result = update_email(State(state.clone()), user.id.clone(), Json(request)).await;

        assert!(result.is_ok());

        // Verify email changed
        let updated = db::find_user_by_id(&state.pool, &user.id).await.unwrap().unwrap();
        assert_eq!(updated.email, "newemail@example.com");
    }

    #[tokio::test]
    async fn test_update_email_invalid_format() {
        let state = setup_test_state().await;
        let user = create_test_user(&state, "invalid@example.com", "password123").await;

        let request = UpdateEmailRequest {
            email: "not-an-email".to_string(),
        };

        let result = update_email(State(state), user.id, Json(request)).await;

        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_update_username_success() {
        let state = setup_test_state().await;
        let user = create_test_user(&state, "username@example.com", "password123").await;

        let request = UpdateUsernameRequest {
            username: "newusername".to_string(),
        };

        let result = update_username(State(state.clone()), user.id.clone(), Json(request)).await;

        assert!(result.is_ok());

        let updated = db::find_user_by_id(&state.pool, &user.id).await.unwrap().unwrap();
        assert_eq!(updated.username, "newusername");
    }

    #[tokio::test]
    async fn test_update_password_success() {
        let state = setup_test_state().await;
        let user = create_test_user(&state, "changepw@example.com", "oldpassword123").await;

        let request = UpdatePasswordRequest {
            current_password: "oldpassword123".to_string(),
            new_password: "newpassword456".to_string(),
        };

        let result = update_password(State(state.clone()), user.id.clone(), Json(request)).await;

        assert!(result.is_ok());

        // Verify new password works
        let updated = db::find_user_by_id(&state.pool, &user.id).await.unwrap().unwrap();
        assert!(verify_password("newpassword456", &updated.password_hash).unwrap());
    }

    #[tokio::test]
    async fn test_update_password_wrong_current() {
        let state = setup_test_state().await;
        let user = create_test_user(&state, "wrongcurrent@example.com", "password123").await;

        let request = UpdatePasswordRequest {
            current_password: "wrongpassword".to_string(),
            new_password: "newpassword456".to_string(),
        };

        let result = update_password(State(state), user.id, Json(request)).await;

        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_update_password_too_short() {
        let state = setup_test_state().await;
        let user = create_test_user(&state, "shortpw@example.com", "password123").await;

        let request = UpdatePasswordRequest {
            current_password: "password123".to_string(),
            new_password: "short".to_string(),
        };

        let result = update_password(State(state), user.id, Json(request)).await;

        assert!(result.is_err());
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
}
