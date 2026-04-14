use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use crate::api::AppState;
use crate::models::{ChatRequest, ChatResponse};
use crate::llm::prompts::SYSTEM_PROMPT;
use crate::llm::{extractor, embedding};
use crate::db::{vector, neo4j, postgres};
use tracing::{info, error};
use chrono::Utc;
use uuid::Uuid;
use std::collections::HashSet;

pub async fn send_message(
    State(state): State<AppState>,
    Json(payload): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, StatusCode> {
    info!("Received message from user: {}", payload.user_id);
    
    // Step 1 - Create embedding for query
    let user_embedding = embedding::create_embedding(&state.openai, &payload.text)
        .await
        .map_err(|e| {
            error!("Failed to create embedding: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    // Step 2 - Search for similar parent_ids in Qdrant
    let qdrant_results = vector::search_similar_parent_ids(&state.qdrant, user_embedding.clone(), 20)
        .await
        .map_err(|e| {
            error!("Failed to search Qdrant: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    info!("Found {} similar parent_ids from Qdrant", qdrant_results.len());
    
    // Step 3 - Get related parent_ids from Neo4j (based on conversation content)
    let entities: Vec<String> = Vec::new(); // Extract entities from user text if needed
    let neo4j_parent_ids = neo4j::fetch_related_parent_ids(&state.neo4j, &payload.user_id, &entities)
        .await
        .unwrap_or_default();
    
    info!("Found {} related parent_ids from Neo4j", neo4j_parent_ids.len());
    
    // Step 4 - Deduplicate parent_ids using HashSet
    let mut unique_parent_ids = HashSet::new();
    for (parent_id_str, _score) in &qdrant_results {
        if let Ok(uuid) = Uuid::parse_str(parent_id_str) {
            unique_parent_ids.insert(uuid);
        }
    }
    for parent_id in neo4j_parent_ids {
        unique_parent_ids.insert(parent_id);
    }
    
    let parent_ids: Vec<Uuid> = unique_parent_ids.into_iter().collect();
    info!("Total unique parent_ids: {}", parent_ids.len());
    
    // Step 5 - Fetch full content from PostgreSQL
    let session_contents = postgres::fetch_session_content(&state.pg_pool, &parent_ids)
        .await
        .map_err(|e| {
            error!("Failed to fetch session contents: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    info!("Fetched {} session contents from PostgreSQL", session_contents.len());
    
    // Step 6 - Fetch core values from Neo4j for dynamic prompt
    let core_values = neo4j::fetch_user_core_values(&state.neo4j, &payload.user_id, 5)
        .await
        .unwrap_or_default();
    
    // Step 7 - Generate response with rich context
    let reply = generate_mirror_response(&state, &payload.text, &session_contents, &core_values)
        .await
        .map_err(|e| {
            error!("Failed to generate response: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    // Step 8 - Save memory in background (PostgreSQL -> Qdrant -> Neo4j)
    let state_clone = state.clone();
    let user_text = payload.text.clone();
    let user_id = payload.user_id.clone();
    let reply_clone = reply.clone();
    
    tokio::spawn(async move {
        if let Err(e) = save_memory(&state_clone, &user_text, &user_id, &reply_clone, user_embedding).await {
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
    session_contents: &[postgres::SessionContent],
    core_values: &[(String, f64, String)],
) -> anyhow::Result<String> {
    use async_openai::types::{
        ChatCompletionRequestMessage,
        ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    };
    
    // Build context with dynamic core values injection
    let mut context = String::from(SYSTEM_PROMPT);
    
    // Inject core values dynamically
    if !core_values.is_empty() {
        context.push_str("\n\n【現在の私たちの軸（コアバリュー）】\n");
        context.push_str("※以下の価値観は、これまでの対話から理解した、相手が誰であっても変わらない私たちのブレない軸です。\n\n");
        for (value_name, weight, ctx) in core_values {
            context.push_str(&format!("- **{}** (重要度: {:.2})\n", value_name, weight));
            context.push_str(&format!("  背景: {}\n\n", ctx));
        }
    }
    
    // Add parent episode contexts (rich, full conversations)
    if !session_contents.is_empty() {
        context.push_str("\n\n【過去の記憶（関連する会話セッション）】\n");
        for session in session_contents.iter().take(10) {
            context.push_str(&format!("- セッション（{}ターン）:\n{}\n\n", session.turn_count, session.content));
        }
    }
    
    // Log the full context being sent to LLM
    let context_preview: String = context.chars().take(500).collect();
    tracing::info!("=== LLM Context (preview) ===\n{}", context_preview);
    tracing::info!("=== User Query ===\n{}", user_text);
    
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
    reply_text: &str,
    embedding: Vec<f32>,
) -> anyhow::Result<()> {
    info!("Saving memory with session-based architecture");
    
    // Step 1: Get or create active session
    let parent_id = postgres::get_or_create_active_session(&state.pg_pool, user_id).await?;
    info!("Using session: {}", parent_id);
    
    // Step 2: Add turn to session
    let sub_chunk_id = postgres::add_turn_to_session(
        &state.pg_pool,
        &parent_id,
        user_id,
        user_text,
        reply_text,
    ).await?;
    info!("Added turn (sub_chunk: {}) to session {}", sub_chunk_id, parent_id);
    
    // Step 3: Save sub-chunk embedding to Qdrant
    let point_id = vector::save_sub_chunk(&state.qdrant, embedding, &parent_id.to_string(), user_id).await?;
    info!("Saved sub-chunk embedding to Qdrant: {}", point_id);
    
    // Step 4: Extract core values using LLM
    let core_values = extractor::extract_core_values(&state.openai, user_text, reply_text).await?;
    info!("Extracted {} core values", core_values.len());
    
    // Step 5: Save core values to Neo4j graph (episode = session)
    if !core_values.is_empty() {
        neo4j::save_core_values(&state.neo4j, user_id, &parent_id, &core_values).await?;
        info!("Saved core values to Neo4j");
    }
    
    Ok(())
}
