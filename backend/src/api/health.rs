use axum::{extract::State, http::StatusCode, Json};
use serde_json::{json, Value};

pub async fn health_check(State(app_state): State<crate::api::AppState>) -> (StatusCode, Json<Value>) {
    tracing::info!("Health check endpoint called");

    // Check whether background DB initialization has completed
    let init_state = app_state.inner.read().await;

    if !init_state.initialized {
        tracing::warn!("Health check called but databases not yet initialized");
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "initializing",
                "service": "Project Mirror Backend",
                "version": "0.1.0"
            })),
        );
    }

    // All databases initialized successfully
    (
        StatusCode::OK,
        Json(json!({
            "status": "healthy",
            "service": "Project Mirror Backend",
            "version": "0.1.0"
        })),
    )
}        