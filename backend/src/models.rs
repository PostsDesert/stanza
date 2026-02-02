use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// User database model
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub username: String,
    pub password_hash: String,
    pub salt: String,
    pub created_at: String,
    pub updated_at: String,
}

impl User {
    /// Create a new user with generated UUID and timestamps
    #[allow(dead_code)]
    pub fn new(email: String, username: String, password_hash: String, salt: String) -> Self {
        let now = Utc::now().to_rfc3339();
        Self {
            id: Uuid::new_v4().to_string(),
            email,
            username,
            password_hash,
            salt,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// Convert to public user response (without sensitive fields)
    pub fn to_public(&self) -> UserResponse {
        UserResponse {
            id: self.id.clone(),
            email: self.email.clone(),
            username: self.username.clone(),
        }
    }
}

/// Public user response (excludes sensitive fields)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub username: String,
}

/// Message database model
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub user_id: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
}

impl Message {
    /// Create a new message with generated UUID and timestamps
    pub fn new(user_id: String, content: String) -> Self {
        let now = Utc::now().to_rfc3339();
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            content,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// Create a new message with a client-provided ID (for offline sync)
    pub fn with_id(id: String, user_id: String, content: String) -> Self {
        let now = Utc::now().to_rfc3339();
        Self {
            id,
            user_id,
            content,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// Convert to API response format
    pub fn to_response(&self) -> MessageResponse {
        MessageResponse {
            id: self.id.clone(),
            content: self.content.clone(),
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
        }
    }
}

/// Message response for API
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MessageResponse {
    pub id: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
}

/// JWT Claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub user_id: String,
    pub exp: usize,
}

// ============ Request DTOs ============

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateMessageRequest {
    pub content: String,
    #[serde(default)]
    pub id: Option<String>, // Optional client-generated ID for offline sync
}

#[derive(Debug, Deserialize)]
pub struct UpdateMessageRequest {
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEmailRequest {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUsernameRequest {
    pub username: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

// ============ Response DTOs ============

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserResponse,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessagesResponse {
    pub messages: Vec<MessageResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SuccessResponse {
    pub success: bool,
}

impl SuccessResponse {
    pub fn new() -> Self {
        Self { success: true }
    }
}

impl Default for SuccessResponse {
    fn default() -> Self {
        Self::new()
    }
}

// ============ Query Parameters ============

#[derive(Debug, Deserialize, Default)]
pub struct MessagesQuery {
    pub since: Option<String>,
}

/// Search query parameters
#[derive(Debug, Deserialize, Default)]
pub struct SearchQuery {
    /// Full-text search query
    pub q: Option<String>,
    /// Filter by start date (ISO 8601 format)
    pub from: Option<String>,
    /// Filter by end date (ISO 8601 format)
    pub to: Option<String>,
    /// Filter by hashtags (comma-separated, without # prefix)
    pub tags: Option<String>,
}

/// Search response with results
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    pub messages: Vec<MessageResponse>,
    pub total: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_new_creates_valid_user() {
        let user = User::new(
            "test@example.com".to_string(),
            "testuser".to_string(),
            "hash123".to_string(),
            "salt123".to_string(),
        );

        assert!(!user.id.is_empty());
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.username, "testuser");
        assert_eq!(user.password_hash, "hash123");
        assert_eq!(user.salt, "salt123");
        assert!(!user.created_at.is_empty());
        assert_eq!(user.created_at, user.updated_at);

        // Verify UUID format
        Uuid::parse_str(&user.id).expect("User ID should be valid UUID");
    }

    #[test]
    fn test_user_to_public_excludes_sensitive_data() {
        let user = User::new(
            "test@example.com".to_string(),
            "testuser".to_string(),
            "hash123".to_string(),
            "salt123".to_string(),
        );

        let public = user.to_public();

        assert_eq!(public.id, user.id);
        assert_eq!(public.email, user.email);
        assert_eq!(public.username, user.username);
    }

    #[test]
    fn test_message_new_creates_valid_message() {
        let user_id = Uuid::new_v4().to_string();
        let message = Message::new(user_id.clone(), "Hello, world!".to_string());

        assert!(!message.id.is_empty());
        assert_eq!(message.user_id, user_id);
        assert_eq!(message.content, "Hello, world!");
        assert!(!message.created_at.is_empty());
        assert_eq!(message.created_at, message.updated_at);

        // Verify UUID format
        Uuid::parse_str(&message.id).expect("Message ID should be valid UUID");
    }

    #[test]
    fn test_message_with_id_uses_provided_id() {
        let custom_id = Uuid::new_v4().to_string();
        let user_id = Uuid::new_v4().to_string();
        let message = Message::with_id(
            custom_id.clone(),
            user_id.clone(),
            "Test content".to_string(),
        );

        assert_eq!(message.id, custom_id);
        assert_eq!(message.user_id, user_id);
        assert_eq!(message.content, "Test content");
    }

    #[test]
    fn test_message_to_response() {
        let message = Message::new(Uuid::new_v4().to_string(), "Test message".to_string());

        let response = message.to_response();

        assert_eq!(response.id, message.id);
        assert_eq!(response.content, message.content);
        assert_eq!(response.created_at, message.created_at);
        assert_eq!(response.updated_at, message.updated_at);
    }

    #[test]
    fn test_claims_serialization() {
        let claims = Claims {
            user_id: "user-123".to_string(),
            exp: 1704067200,
        };

        let json = serde_json::to_string(&claims).unwrap();
        let deserialized: Claims = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.user_id, claims.user_id);
        assert_eq!(deserialized.exp, claims.exp);
    }

    #[test]
    fn test_login_request_deserialization() {
        let json = r#"{"email": "test@example.com", "password": "secret123"}"#;
        let request: LoginRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.email, "test@example.com");
        assert_eq!(request.password, "secret123");
    }

    #[test]
    fn test_create_message_request_with_optional_id() {
        // Without id
        let json1 = r#"{"content": "Hello"}"#;
        let request1: CreateMessageRequest = serde_json::from_str(json1).unwrap();
        assert_eq!(request1.content, "Hello");
        assert!(request1.id.is_none());

        // With id
        let json2 = r#"{"content": "Hello", "id": "custom-id"}"#;
        let request2: CreateMessageRequest = serde_json::from_str(json2).unwrap();
        assert_eq!(request2.content, "Hello");
        assert_eq!(request2.id, Some("custom-id".to_string()));
    }

    #[test]
    fn test_success_response_default() {
        let response = SuccessResponse::default();
        assert!(response.success);
    }
}
