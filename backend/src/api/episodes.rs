use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use tracing::{info, error};

use crate::api::AppState;
use crate::models::EpisodeDetail;

#[derive(Debug, Serialize, Deserialize)]
pub struct EpisodeResponse {
    pub id: String,
    pub timestamp: i64,
    pub text: String,
    pub reply_text: Option<String>,
    pub emotion_type: Option<String>,
    pub score: Option<f32>,
}

pub async fn get_episodes(
    State(state): State<AppState>,
) -> Result<Json<Vec<EpisodeResponse>>, StatusCode> {
    info!("Fetching all episodes from PostgreSQL");
    
    // --- ロック取得とクライアント抽出 ---
    let lock = state.inner.read().await;
    let pg_pool = lock.pg_pool.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    // ----------------------------------

    // Get all sub-chunks from PostgreSQL
    match sqlx::query_as::<_, (String, String, String, i64)>(
        r#"
        SELECT 
            sc.id::text,
            sc.user_text,
            sc.reply_text,
            EXTRACT(EPOCH FROM sc.created_at)::bigint as timestamp
        FROM sub_chunks sc
        ORDER BY sc.created_at DESC
        LIMIT 100
        "#
    )
    .fetch_all(pg_pool) // &state.pg_pool から pg_pool へ変更
    .await
    {
        Ok(results) => {
            let episodes: Vec<EpisodeResponse> = results
                .into_iter()
                .map(|(id, user_text, reply_text, timestamp)| EpisodeResponse {
                    id,
                    timestamp,
                    text: user_text,
                    reply_text: Some(reply_text),
                    emotion_type: Some("neutral".to_string()),
                    score: None,
                })
                .collect();
            
            info!("Found {} episodes", episodes.len());
            Ok(Json(episodes))
        }
        Err(e) => {
            error!("Failed to fetch episodes: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn get_episode_by_id(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<EpisodeResponse>, StatusCode> {
    info!("Fetching episode with id: {}", id);
    
    // --- ロック取得 ---
    let lock = state.inner.read().await;
    let pg_pool = lock.pg_pool.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    match sqlx::query_as::<_, (String, String, String, i64)>(
        r#"
        SELECT 
            sc.id::text,
            sc.user_text,
            sc.reply_text,
            EXTRACT(EPOCH FROM sc.created_at)::bigint as timestamp
        FROM sub_chunks sc
        WHERE sc.id::text = $1
        "#
    )
    .bind(&id)
    .fetch_optional(pg_pool)
    .await
    {
        Ok(Some((id, user_text, reply_text, timestamp))) => {
            Ok(Json(EpisodeResponse {
                id,
                timestamp,
                text: user_text,
                reply_text: Some(reply_text),
                emotion_type: Some("neutral".to_string()),
                score: None,
            }))
        }
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            error!("Failed to fetch episode: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get episode detail by parent_id with full conversation history
pub async fn get_episode_by_parent_id(
    State(state): State<AppState>,
    axum::extract::Path(parent_id): axum::extract::Path<String>,
) -> Result<Json<EpisodeDetail>, StatusCode> {
    info!("Fetching episode detail for parent_id: {}", parent_id);
    
    // --- ロック取得 ---
    let lock = state.inner.read().await;
    let pg_pool = lock.pg_pool.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let neo4j = lock.neo4j.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    // Parse parent_id as UUID
    let parent_uuid = match uuid::Uuid::parse_str(&parent_id) {
        Ok(uuid) => uuid,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };
    
    // Get all sub-chunks with this parent_id from PostgreSQL
    match sqlx::query_as::<_, (String, String, i64)>(
        r#"
        SELECT 
            sc.user_text,
            sc.reply_text,
            EXTRACT(EPOCH FROM sc.created_at)::bigint as timestamp
        FROM sub_chunks sc
        WHERE sc.parent_id = $1
        ORDER BY sc.created_at ASC
        "#
    )
    .bind(parent_uuid)
    .fetch_all(pg_pool)
    .await
    {
        Ok(results) => {
            if results.is_empty() {
                return Err(StatusCode::NOT_FOUND);
            }
            
            let mut messages = Vec::new();
            let first_timestamp = results.first().map(|r| r.2).unwrap_or(0);
            
            for (user_text, reply_text, timestamp) in results {
                messages.push(crate::models::ConversationMessage {
                    role: "user".to_string(),
                    content: user_text,
                    timestamp,
                });
                messages.push(crate::models::ConversationMessage {
                    role: "assistant".to_string(),
                    content: reply_text,
                    timestamp,
                });
            }
            
            let mut core_values = Vec::new();
            let mut persons = Vec::new();
            
            let neo4j_query = neo4rs::query(
                "MATCH (e:Episode {parent_id: $parent_id})
                 OPTIONAL MATCH (e)-[:HOLDS]->(cv:CoreValue)
                 OPTIONAL MATCH (p:Person)-[:RELATED_TO]->(e)
                 RETURN collect(DISTINCT cv.name) as core_values,
                        collect(DISTINCT p.name) as persons"
            ).param("parent_id", parent_id.clone());
            
            if let Ok(mut result) = neo4j.execute(neo4j_query).await {
                if let Ok(Some(row)) = result.next().await {
                    // 型を明示的に指定して抽出
                    let cvs: Vec<String> = row.get("core_values").unwrap_or_default();
                    let ps: Vec<String> = row.get("persons").unwrap_or_default();

                    core_values = cvs.into_iter().filter(|s| !s.is_empty()).collect();
                    persons = ps.into_iter().filter(|s| !s.is_empty()).collect();
                }
            }
            
            Ok(Json(EpisodeDetail {
                parent_id,
                timestamp: first_timestamp,
                core_values,
                persons,
                messages,
            }))
        }
        Err(e) => {
            error!("Failed to fetch episode detail: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
