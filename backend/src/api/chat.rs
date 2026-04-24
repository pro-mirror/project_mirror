use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use axum::extract::Multipart;
use crate::api::AppState;
use crate::models::{ChatRequest, ChatResponse};
use crate::llm::prompts::SYSTEM_PROMPT;
use crate::llm::{extractor, embedding, openai};
use crate::db::{vector, neo4j, neo4j_context, postgres};
use tracing::{info, error};
use uuid::Uuid;
use std::collections::HashSet;
use std::io::Write;

pub async fn send_message(
    State(state): State<AppState>,
    Json(payload): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, StatusCode> {
    info!("Received message from user: {}", payload.user_id);

    // --- ロックを取得して各クライアントを安全に取り出す ---
    let lock = state.inner.read().await;
    
    let openai = lock.openai.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let qdrant = lock.qdrant.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let neo4j_conn = lock.neo4j.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let pg_pool = lock.pg_pool.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    // ----------------------------------------------

    // Step 1 - Create embedding for query
    let user_embedding = embedding::create_embedding(openai, &payload.text)
        .await
        .map_err(|e| {
            error!("Failed to create embedding: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    // Step 2 - Search for similar parent_ids in Qdrant
    let qdrant_results = vector::search_similar_parent_ids(qdrant, user_embedding.clone(), 20)
        .await
        .map_err(|e| {
            error!("Failed to search Qdrant: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    // Filter by similarity threshold 
    const SIMILARITY_THRESHOLD: f32 = 0.3;
    let filtered_results: Vec<(String, f32)> = qdrant_results
        .into_iter()
        .filter(|(_, score)| *score > SIMILARITY_THRESHOLD)
        .collect();
    
    info!("Applied similarity threshold: {} (filtered: {} results)", 
        SIMILARITY_THRESHOLD, filtered_results.len());
    
    // Step 3 - Fetch core values from Neo4j
    let core_values = neo4j::fetch_user_core_values(neo4j_conn, &payload.user_id, 5)
        .await
        .unwrap_or_default();
    
    // Step 4 - Extract entities from user query
    let mut entities: Vec<String> = Vec::new();
    
    match extractor::extract_memory(openai, &payload.text).await {
        Ok(extracted) => {
            entities.extend(extracted.persons);
            entities.extend(extracted.keywords);
            tracing::info!("LLM extracted entities successfully");
        }
        Err(e) => {
            tracing::warn!("LLM extraction failed, falling back to pattern-based: {}", e);
            let person_names = neo4j_context::extract_person_names(&payload.text);
            entities.extend(person_names);
        }
    }
    
    for (value_name, _, _) in &core_values {
        entities.push(value_name.clone());
    }
    
    // Step 5 - Get related parent_ids from Neo4j
    let neo4j_parent_ids = neo4j::fetch_related_parent_ids(neo4j_conn, &payload.user_id, &entities)
        .await
        .unwrap_or_default();
    
    // Step 6 - Get current active session
    let current_session_id = postgres::get_or_create_active_session(pg_pool, &payload.user_id)
        .await
        .ok();
    
    // Step 7 - Deduplicate parent_ids
    let mut unique_parent_ids = HashSet::new();
    for (parent_id_str, _) in &filtered_results {
        if let Ok(uuid) = Uuid::parse_str(parent_id_str) {
            if Some(uuid) != current_session_id {
                unique_parent_ids.insert(uuid);
            }
        }
    }
    for parent_id in neo4j_parent_ids {
        if Some(parent_id) != current_session_id {
            unique_parent_ids.insert(parent_id);
        }
    }
    
    let parent_ids: Vec<Uuid> = unique_parent_ids.into_iter().collect();
    
    // Step 8 - Fetch full content from PostgreSQL
    let session_contents = postgres::fetch_session_content(pg_pool, &parent_ids)
        .await
        .map_err(|e| {
            error!("Failed to fetch session contents: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    // Step 9 - Generate response (内側でロックを取るためStateを渡す)
    let reply = generate_mirror_response(&state, &payload.text, &session_contents, &core_values)
        .await
        .map_err(|e| {
            error!("Failed to generate response: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    // Step 10 - Save memory in background
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
        transcribed_text: None,
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

    // ロック取得
    let lock = state.inner.read().await;
    let openai = lock.openai.as_ref().ok_or_else(|| anyhow::anyhow!("OpenAI not initialized"))?;
    
    let mut context = String::from(SYSTEM_PROMPT);
    
    if !core_values.is_empty() {
        context.push_str("\n\n【現在焦点を当てているコアバリュー】\n");
        for (value_name, weight, ctx) in core_values {
            context.push_str(&format!("- **{}** (重要度: {:.2})\n背景: {}\n\n", value_name, weight, ctx));
        }
    }
    
    if !session_contents.is_empty() {
        context.push_str("\n\n【現在の話題の対象に関する過去の記憶】\n");
        for session in session_contents.iter().take(5) {
            context.push_str(&format!("- セッション（{}ターン）:\n{}\n\n", session.turn_count, session.content));
        }
    }
    
    let messages = vec![
        ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessageArgs::default().content(context).build()?),
        ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessageArgs::default().content(user_text).build()?),
    ];
    
    let request = CreateChatCompletionRequestArgs::default()
        .model("gpt-4o-mini")
        .messages(messages)
        .max_tokens(1000u16) // Explicitly set max tokens to prevent truncation
        .build()?;
    
    let response = openai.chat().create(request).await?;
    
    let choice = response
        .choices
        .first()
        .ok_or_else(|| anyhow::anyhow!("No response from OpenAI"))?;
    
    // Check if response was truncated
    if let Some(finish_reason) = &choice.finish_reason {
        tracing::debug!("OpenAI finish_reason: {:?}", finish_reason);
        let reason_str = format!("{:?}", finish_reason).to_lowercase();
        if reason_str.contains("length") {
            tracing::warn!("OpenAI response was truncated due to max_tokens limit");
        }
    }
    
    let reply = choice.message.content.clone()
        .unwrap_or_else(|| "応答を生成できませんでした。".to_string());
    
    Ok(reply)
}

async fn save_memory(
    state: &AppState,
    user_text: &str,
    user_id: &str,
    reply_text: &str,
    embedding: Vec<f32>,
) -> anyhow::Result<()> {
    // ロック取得
    let lock = state.inner.read().await;
    let openai = lock.openai.as_ref().ok_or_else(|| anyhow::anyhow!("OpenAI not initialized"))?;
    let qdrant = lock.qdrant.as_ref().ok_or_else(|| anyhow::anyhow!("Qdrant not initialized"))?;
    let neo4j_conn = lock.neo4j.as_ref().ok_or_else(|| anyhow::anyhow!("Neo4j not initialized"))?;
    let pg_pool = lock.pg_pool.as_ref().ok_or_else(|| anyhow::anyhow!("Postgres not initialized"))?;

    let parent_id = postgres::get_or_create_active_session(pg_pool, user_id).await?;
    let _sub_chunk_id = postgres::add_turn_to_session(pg_pool, &parent_id, user_id, user_text, reply_text).await?;
    let _point_id = vector::save_sub_chunk(qdrant, embedding, &parent_id.to_string(), user_id).await?;
    let core_values = extractor::extract_core_values(openai, user_text, reply_text).await?;
    
    if !core_values.is_empty() {
        neo4j::save_core_values(neo4j_conn, user_id, &parent_id, &core_values).await?;
    }
    
    Ok(())
}

pub async fn send_voice_message(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<ChatResponse>, StatusCode> {
    let mut user_id_str: Option<String> = None;
    let mut audio_data: Option<Vec<u8>> = None;
    let mut _filename: Option<String> = None;
    
    // 【修正】field の型を明示するか、unwrap のエラーハンドリングを整理
    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "user_id" => user_id_str = Some(field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?),
            "audio" => {
                _filename = field.file_name().map(|s| s.to_string());
                audio_data = Some(field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?.to_vec());
            }
            _ => {}
        }
    }
    
    let user_id = user_id_str.ok_or(StatusCode::BAD_REQUEST)?;
    let audio_data = audio_data.ok_or(StatusCode::BAD_REQUEST)?;
    
    let temp_path = std::env::temp_dir().join(format!("voice_{}.m4a", Uuid::new_v4()));
    std::fs::File::create(&temp_path).and_then(|mut f| f.write_all(&audio_data)).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // OpenAIクライアント取得
    let lock = state.inner.read().await;
    let openai_client = lock.openai.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let transcribed_text = openai::transcribe_audio(openai_client, temp_path.clone())
        .await
        .map_err(|_| {
            let _ = std::fs::remove_file(&temp_path);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    let _ = std::fs::remove_file(&temp_path);
    
    let chat_request = ChatRequest { user_id, text: transcribed_text.clone() };
    let mut response = send_message(State(state.clone()), Json(chat_request)).await?;
    response.0.transcribed_text = Some(transcribed_text);
    
    Ok(response)
}