use std::sync::Arc;
use tokio::sync::RwLock;
// 修正：QdrantClient ではなく Qdrant をインポート
pub use qdrant_client::Qdrant;

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

pub mod health;
pub mod chat;
pub mod insights;
pub mod episodes;
pub mod maintenance;

#[derive(Clone, Default)]
pub struct InitState {
    pub neo4j: Option<neo4rs::Graph>,
    // 修正：ジェネリクス不要の Qdrant 型にする
    pub qdrant: Option<Qdrant>,
    pub pg_pool: Option<sqlx::PgPool>,
    pub openai: Option<async_openai::Client<async_openai::config::OpenAIConfig>>,
    pub initialized: bool,
}

#[derive(Clone)]
pub struct AppState {
    pub inner: Arc<RwLock<InitState>>,
    pub config: crate::config::Config,
}

/// Middleware that rejects non-health requests with 503 until DB initialization completes.
pub async fn require_initialized(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let initialized = state.inner.read().await.initialized;
    if !initialized {
        tracing::warn!(
            "Request to {} rejected: database initialization not yet complete",
            request.uri().path()
        );
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "error": "Service is starting up, please retry shortly",
                "status": "initializing"
            })),
        )
            .into_response();
    }
    next.run(request).await
}