mod auth;
mod db;
mod exports;
mod handlers;
mod middleware;
mod models;
pub mod utils;

use std::sync::Arc;

use axum::{
    extract::{FromRequestParts, Path, Query, State},
    http::{request::Parts, StatusCode},
    middleware::from_fn_with_state,
    routing::{delete, get, post, put},
    Json, Router,
};
use handlers::{AppState, ErrorResponse, SharedState};
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Authenticated user extractor
pub struct AuthUser(pub String);

#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<ErrorResponse>);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<String>()
            .cloned()
            .map(AuthUser)
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    ErrorResponse::new("Not authenticated"),
                )
            })
    }
}

/// Create the application router
fn create_router(state: SharedState) -> Router {
    // Public routes (no auth required)
    let public_routes = Router::new().route("/api/login", post(handlers::login));

    // Protected routes (auth required)
    let protected_routes = Router::new()
        // Messages
        .route("/api/messages/search", get(search_messages_handler))
        .route("/api/messages", get(get_messages_handler))
        .route("/api/messages", post(create_message_handler))
        .route("/api/messages/:id", put(update_message_handler))
        .route("/api/messages/:id", delete(delete_message_handler))
        // User management
        .route("/api/user/email", put(update_email_handler))
        .route("/api/user/username", put(update_username_handler))
        .route("/api/user/password", put(update_password_handler))
        // Exports
        .route("/api/export/json", get(export_json_handler))
        .route("/api/export/markdown", get(export_markdown_handler))
        .layer(from_fn_with_state(state.clone(), middleware::auth_middleware));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .fallback_service(ServeDir::new("dist"))
        .layer(middleware::cors_layer())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

// ============ Handler Wrappers ============
// These extract user_id from AuthUser and pass to actual handlers

async fn get_messages_handler(
    State(state): State<SharedState>,
    AuthUser(user_id): AuthUser,
    Query(query): Query<models::MessagesQuery>,
) -> Result<Json<models::MessagesResponse>, (StatusCode, Json<ErrorResponse>)> {
    handlers::get_messages(State(state), user_id, Query(query)).await
}

async fn create_message_handler(
    State(state): State<SharedState>,
    AuthUser(user_id): AuthUser,
    Json(payload): Json<models::CreateMessageRequest>,
) -> Result<(StatusCode, Json<models::MessageResponse>), (StatusCode, Json<ErrorResponse>)> {
    handlers::create_message(State(state), user_id, Json(payload)).await
}

async fn update_message_handler(
    State(state): State<SharedState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<String>,
    Json(payload): Json<models::UpdateMessageRequest>,
) -> Result<Json<models::MessageResponse>, (StatusCode, Json<ErrorResponse>)> {
    handlers::update_message(State(state), user_id, Path(id), Json(payload)).await
}

async fn delete_message_handler(
    State(state): State<SharedState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<String>,
) -> Result<Json<models::SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    handlers::delete_message(State(state), user_id, Path(id)).await
}

async fn search_messages_handler(
    State(state): State<SharedState>,
    AuthUser(user_id): AuthUser,
    Query(query): Query<models::SearchQuery>,
) -> Result<Json<models::SearchResponse>, (StatusCode, Json<ErrorResponse>)> {
    handlers::search_messages(State(state), user_id, Query(query)).await
}

async fn update_email_handler(
    State(state): State<SharedState>,
    AuthUser(user_id): AuthUser,
    Json(payload): Json<models::UpdateEmailRequest>,
) -> Result<Json<models::SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    handlers::update_email(State(state), user_id, Json(payload)).await
}

async fn update_username_handler(
    State(state): State<SharedState>,
    AuthUser(user_id): AuthUser,
    Json(payload): Json<models::UpdateUsernameRequest>,
) -> Result<Json<models::SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    handlers::update_username(State(state), user_id, Json(payload)).await
}

async fn update_password_handler(
    State(state): State<SharedState>,
    AuthUser(user_id): AuthUser,
    Json(payload): Json<models::UpdatePasswordRequest>,
) -> Result<Json<models::SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    handlers::update_password(State(state), user_id, Json(payload)).await
}

async fn export_json_handler(
    State(state): State<SharedState>,
    AuthUser(user_id): AuthUser,
) -> Result<axum::response::Response, (StatusCode, Json<ErrorResponse>)> {
    exports::export_json(State(state), user_id).await
}

async fn export_markdown_handler(
    State(state): State<SharedState>,
    AuthUser(user_id): AuthUser,
) -> Result<axum::response::Response, (StatusCode, Json<ErrorResponse>)> {
    exports::export_markdown(State(state), user_id).await
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "dissipate_backend=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load environment variables
    dotenvy::dotenv().ok();

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:dissipate.db".to_string());
    let jwt_secret =
        std::env::var("JWT_SECRET").expect("JWT_SECRET environment variable must be set");

    // Initialize database
    let pool = db::init_pool(&database_url).await?;

    let state = Arc::new(AppState { pool, jwt_secret });

    let app = create_router(state);

    let addr = "0.0.0.0:3000";
    tracing::info!("Starting server at http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{header, Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use serde_json::json;
    use tower::ServiceExt;

    async fn setup_test_app() -> (Router, SharedState) {
        let pool = db::init_pool("sqlite::memory:").await.unwrap();
        let state = Arc::new(AppState {
            pool,
            jwt_secret: "test-secret".to_string(),
        });
        let app = create_router(state.clone());
        (app, state)
    }

    async fn create_test_user_and_login(state: &SharedState) -> (String, String) {
        let (hash, salt) = utils::hash_password("password123").unwrap();
        let user = models::User::new(
            "test@example.com".to_string(),
            "testuser".to_string(),
            hash,
            salt,
        );
        let user_id = user.id.clone();
        db::create_user(&state.pool, &user).await.unwrap();

        let token = auth::create_token(&user_id, &state.jwt_secret).unwrap();
        (user_id, token)
    }

    #[tokio::test]
    async fn test_login_endpoint() {
        let (app, state) = setup_test_app().await;

        // Create a user
        let (hash, salt) = utils::hash_password("password123").unwrap();
        let user = models::User::new(
            "login@example.com".to_string(),
            "loginuser".to_string(),
            hash,
            salt,
        );
        db::create_user(&state.pool, &user).await.unwrap();

        let request = Request::builder()
            .method("POST")
            .uri("/api/login")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "email": "login@example.com",
                    "password": "password123"
                })
                .to_string(),
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json.get("token").is_some());
        assert_eq!(json["user"]["email"], "login@example.com");
    }

    #[tokio::test]
    async fn test_get_messages_requires_auth() {
        let (app, _) = setup_test_app().await;

        let request = Request::builder()
            .method("GET")
            .uri("/api/messages")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_get_messages_with_auth() {
        let (app, state) = setup_test_app().await;
        let (_, token) = create_test_user_and_login(&state).await;

        let request = Request::builder()
            .method("GET")
            .uri("/api/messages")
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["messages"].is_array());
    }

    #[tokio::test]
    async fn test_create_message() {
        let (app, state) = setup_test_app().await;
        let (_, token) = create_test_user_and_login(&state).await;

        let request = Request::builder()
            .method("POST")
            .uri("/api/messages")
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(json!({"content": "Hello, world!"}).to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["content"], "Hello, world!");
    }

    #[tokio::test]
    async fn test_update_message() {
        let (app, state) = setup_test_app().await;
        let (user_id, token) = create_test_user_and_login(&state).await;

        // Create a message first
        let msg = models::Message::new(user_id, "Original".to_string());
        let msg_id = msg.id.clone();
        db::create_message(&state.pool, &msg).await.unwrap();

        let request = Request::builder()
            .method("PUT")
            .uri(format!("/api/messages/{}", msg_id))
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(json!({"content": "Updated"}).to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["content"], "Updated");
    }

    #[tokio::test]
    async fn test_delete_message() {
        let (app, state) = setup_test_app().await;
        let (user_id, token) = create_test_user_and_login(&state).await;

        let msg = models::Message::new(user_id, "Delete me".to_string());
        let msg_id = msg.id.clone();
        db::create_message(&state.pool, &msg).await.unwrap();

        let request = Request::builder()
            .method("DELETE")
            .uri(format!("/api/messages/{}", msg_id))
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Verify deleted
        let deleted = db::get_message_by_id(&state.pool, &msg_id).await.unwrap();
        assert!(deleted.is_none());
    }

    #[tokio::test]
    async fn test_update_email() {
        let (app, state) = setup_test_app().await;
        let (user_id, token) = create_test_user_and_login(&state).await;

        let request = Request::builder()
            .method("PUT")
            .uri("/api/user/email")
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(json!({"email": "new@example.com"}).to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let user = db::find_user_by_id(&state.pool, &user_id).await.unwrap().unwrap();
        assert_eq!(user.email, "new@example.com");
    }

    #[tokio::test]
    async fn test_update_username() {
        let (app, state) = setup_test_app().await;
        let (user_id, token) = create_test_user_and_login(&state).await;

        let request = Request::builder()
            .method("PUT")
            .uri("/api/user/username")
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(json!({"username": "newname"}).to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let user = db::find_user_by_id(&state.pool, &user_id).await.unwrap().unwrap();
        assert_eq!(user.username, "newname");
    }

    #[tokio::test]
    async fn test_update_password() {
        let (app, state) = setup_test_app().await;
        let (_, token) = create_test_user_and_login(&state).await;

        let request = Request::builder()
            .method("PUT")
            .uri("/api/user/password")
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "current_password": "password123",
                    "new_password": "newpassword456"
                })
                .to_string(),
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_export_json() {
        let (app, state) = setup_test_app().await;
        let (_, token) = create_test_user_and_login(&state).await;

        let request = Request::builder()
            .method("GET")
            .uri("/api/export/json")
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let content_type = response.headers().get(header::CONTENT_TYPE).unwrap();
        assert_eq!(content_type, "application/json");
    }

    #[tokio::test]
    async fn test_export_markdown() {
        let (app, state) = setup_test_app().await;
        let (_, token) = create_test_user_and_login(&state).await;

        let request = Request::builder()
            .method("GET")
            .uri("/api/export/markdown")
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let content_type = response.headers().get(header::CONTENT_TYPE).unwrap();
        assert!(content_type.to_str().unwrap().contains("text/markdown"));
    }

    #[tokio::test]
    async fn test_cors_headers() {
        let (app, _) = setup_test_app().await;

        let request = Request::builder()
            .method("OPTIONS")
            .uri("/api/login")
            .header(header::ORIGIN, "http://localhost:5173")
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        
        // CORS should allow the request
        assert!(response.headers().contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN));
    }

    #[tokio::test]
    async fn test_user_isolation() {
        let (app, state) = setup_test_app().await;

        // Create two users
        let (hash1, salt1) = utils::hash_password("password123").unwrap();
        let user1 = models::User::new(
            "user1@example.com".to_string(),
            "user1".to_string(),
            hash1,
            salt1,
        );
        db::create_user(&state.pool, &user1).await.unwrap();
        let token1 = auth::create_token(&user1.id, &state.jwt_secret).unwrap();

        let (hash2, salt2) = utils::hash_password("password123").unwrap();
        let user2 = models::User::new(
            "user2@example.com".to_string(),
            "user2".to_string(),
            hash2,
            salt2,
        );
        db::create_user(&state.pool, &user2).await.unwrap();
        let token2 = auth::create_token(&user2.id, &state.jwt_secret).unwrap();

        // User1 creates a message
        let msg = models::Message::new(user1.id.clone(), "User 1's secret".to_string());
        db::create_message(&state.pool, &msg).await.unwrap();

        // User2 should not see User1's messages
        let request = Request::builder()
            .method("GET")
            .uri("/api/messages")
            .header(header::AUTHORIZATION, format!("Bearer {}", token2))
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["messages"].as_array().unwrap().len(), 0);

        // User1 should see their own messages
        let request = Request::builder()
            .method("GET")
            .uri("/api/messages")
            .header(header::AUTHORIZATION, format!("Bearer {}", token1))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["messages"].as_array().unwrap().len(), 1);
    }
}
