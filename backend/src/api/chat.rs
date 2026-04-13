use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use crate::api::AppState;
use crate::models::{ChatRequest, ChatResponse, EpisodePayload};
use crate::llm::prompts::SYSTEM_PROMPT;
use crate::llm::{extractor, embedding};
use crate::db::{vector, neo4j};
use tracing::{info, error};
use chrono::Utc;
use uuid::Uuid;

pub async fn send_message(
    State(state): State<AppState>,
    Json(payload): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, StatusCode> {
    info!("Received message from user: {}", payload.user_id);
    
    // Step 1 - Retrieve similar episodes from Qdrant
    let user_embedding = embedding::create_embedding(&state.openai, &payload.text)
        .await
        .map_err(|e| {
            error!("Failed to create embedding: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    let similar_episodes = vector::search_similar(&state.qdrant, user_embedding.clone(), 3)
        .await
        .unwrap_or_default();
    
    info!("Found {} similar episodes", similar_episodes.len());
    
    // Step 2 - Generate response using OpenAI (with context)
    let reply = generate_mirror_response(&state, &payload.text, &similar_episodes)
        .await
        .map_err(|e| {
            error!("Failed to generate response: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    // Step 3 - Extract memory and save in background
    let state_clone = state.clone();
    let user_text = payload.text.clone();
    let user_id = payload.user_id.clone();
    
    tokio::spawn(async move {
        if let Err(e) = save_memory(&state_clone, &user_text, &user_id, user_embedding).await {
            error!("Failed to save memory: {}", e);
        }
    });
    
    Ok(Json(ChatResponse {
        reply_text: reply,
        emotion_detected: "neutral".to_string(),
    }))
}

async fn generate_mirror_response(
    state: &AppState,
    user_text: &str,
    similar_episodes: &[(String, f32, EpisodePayload)],
) -> anyhow::Result<String> {
    use async_openai::types::{
        ChatCompletionRequestMessage,
        ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    };
    
    // 過去の記憶をコンテキストに追加
    let mut context = String::from(SYSTEM_PROMPT);
    if !similar_episodes.is_empty() {
        context.push_str("\n\n【過去の記憶】\n");
        for (_, _score, episode) in similar_episodes.iter().take(2) {
            context.push_str(&format!("- {}\n", episode.text));
        }
    }
    
    let messages = vec![
        ChatCompletionRequestMessage::System(
            ChatCompletionRequestSystemMessageArgs::default()
                .content(context)
                .build()?
        ),
        ChatCompletionRequestMessage::User(
            ChatCompletionRequestUserMessageArgs::default()
                .content(user_text)
                .build()?
        ),
    ];
    
    let request = CreateChatCompletionRequestArgs::default()
        .model("gpt-4o-mini")
        .messages(messages)
        .build()?;
    
    let response = state.openai.chat().create(request).await?;
    
    let reply = response
        .choices
        .first()
        .and_then(|choice| choice.message.content.clone())
        .unwrap_or_else(|| "申し訳ありません。応答を生成できませんでした。".to_string());
    
    Ok(reply)
}

async fn save_memory(
    state: &AppState,
    user_text: &str,
    user_id: &str,
    embedding: Vec<f32>,
) -> anyhow::Result<()> {
    info!("Extracting memory from text");
    
    // Extract structured memory
    let extracted = extractor::extract_memory(&state.openai, user_text).await?;
    
    info!("Extracted memory: {:?}", extracted);
    
    let episode_id = Uuid::new_v4().to_string();
    let timestamp = Utc::now().timestamp();
    
    // Create payload
    let payload = EpisodePayload {
        chunk_id: episode_id.clone(),
        timestamp,
        speaker: "user".to_string(),
        text: user_text.to_string(),
        emotion_type: extracted.emotion_type.clone(),
        linked_node_ids: Vec::new(),
    };
    
    // Save to Qdrant
    let point_id = vector::save_episode(&state.qdrant, embedding, payload).await?;
    
    info!("Saved episode to Qdrant with ID: {}", point_id);
    
    // Save to Neo4j
    neo4j::save_memory_graph(
        &state.neo4j,
        &episode_id,
        user_id,
        user_text,
        &extracted,
        timestamp,
    ).await?;
    
    info!("Saved episode to Neo4j graph");
    
    Ok(())
}
