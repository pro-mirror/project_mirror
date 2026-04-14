use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::api::AppState;
use crate::db::postgres;

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
    .fetch_all(&state.pg_pool)
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
            tracing::error!("Failed to fetch episodes: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn get_episode_by_id(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<EpisodeResponse>, StatusCode> {
    info!("Fetching episode with id: {}", id);
    
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
    .fetch_optional(&state.pg_pool)
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
            tracing::error!("Failed to fetch episode: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
