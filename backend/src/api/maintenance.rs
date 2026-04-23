use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use anyhow::Result;
use crate::api::AppState;

#[derive(Debug, Deserialize)]
pub struct CleanupRequest {
    pub user_id: String,
    #[serde(default = "default_days_threshold")]
    pub days_threshold: i64,
    #[serde(default = "default_min_deletion_score")]
    pub min_deletion_score: f64,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_days_threshold() -> i64 { 180 }  // 9 months
fn default_min_deletion_score() -> f64 { 30.0 }
fn default_limit() -> i64 { 150 }

#[derive(Debug, Serialize)]
pub struct CleanupResponse {
    pub success: bool,
    pub deleted_count: usize,
    pub message: String,
}

/// Cleanup old episodes across all databases
/// 
/// Execution order:
/// 1. Neo4j: Find episodes to delete based on deletion score
/// 2. Qdrant: Delete vectors by parent_ids
/// 3. PostgreSQL: Delete episodes by parent_ids
/// 4. Neo4j: Delete episodes and clean up orphaned nodes
pub async fn cleanup_old_data(
    State(state): State<AppState>,
    Json(req): Json<CleanupRequest>,
) -> Result<Json<CleanupResponse>, (StatusCode, String)> {
    let init = state.inner.read().await;
    
    if !init.initialized {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "Database not initialized".to_string(),
        ));
    }

    let neo4j = init.neo4j.as_ref().unwrap();
    let qdrant = init.qdrant.as_ref().unwrap();
    let pg_pool = init.pg_pool.as_ref().unwrap();

    // Step 1 & 4: Delete from Neo4j (returns parent_ids to delete)
    let parent_ids = crate::db::neo4j::cleanup_old_episodes(
        neo4j,
        &req.user_id,
        req.days_threshold,
        req.min_deletion_score,
        req.limit,
    )
    .await
    .map_err(|e| {
        tracing::error!("Neo4j cleanup failed: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Neo4j cleanup failed: {}", e))
    })?;

    if parent_ids.is_empty() {
        return Ok(Json(CleanupResponse {
            success: true,
            deleted_count: 0,
            message: "No episodes found for deletion".to_string(),
        }));
    }

    // Step 2: Delete from Qdrant
    crate::db::qdrant::delete_vectors_by_parent_ids(qdrant, &parent_ids)
        .await
        .map_err(|e| {
            tracing::error!("Qdrant cleanup failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Qdrant cleanup failed: {}", e))
        })?;

    // Step 3: Delete from PostgreSQL
    crate::db::postgres::delete_episodes_by_parent_ids(pg_pool, &parent_ids)
        .await
        .map_err(|e| {
            tracing::error!("PostgreSQL cleanup failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("PostgreSQL cleanup failed: {}", e))
        })?;

    Ok(Json(CleanupResponse {
        success: true,
        deleted_count: parent_ids.len(),
        message: format!("Successfully deleted {} episodes", parent_ids.len()),
    }))
}
