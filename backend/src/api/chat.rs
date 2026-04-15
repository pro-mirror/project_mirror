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
    
    // Step 1 - Create embedding for query
    let user_embedding = embedding::create_embedding(&state.openai, &payload.text)
        .await
        .map_err(|e| {
            error!("Failed to create embedding: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    // Step 2 - Search for similar parent_ids in Qdrant (with similarity threshold)
    let qdrant_results = vector::search_similar_parent_ids(&state.qdrant, user_embedding.clone(), 20)
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
    
    // Step 3 - Fetch core values from Neo4j (needed for entity extraction)
    let core_values = neo4j::fetch_user_core_values(&state.neo4j, &payload.user_id, 5)
        .await
        .unwrap_or_default();
    
    // Step 4 - Extract entities from user query (persons + keywords + core values)
    let mut entities: Vec<String> = Vec::new();
    
    // Try LLM-based extraction first for higher accuracy
    match extractor::extract_memory(&state.openai, &payload.text).await {
        Ok(extracted) => {
            let persons_count = extracted.persons.len();
            let keywords_count = extracted.keywords.len();
            
            // Add all extracted persons
            entities.extend(extracted.persons);
            
            // Add extracted keywords (themes, topics)
            entities.extend(extracted.keywords);
            
            tracing::info!("LLM extracted {} persons and {} keywords", 
                persons_count, keywords_count);
        }
        Err(e) => {
            tracing::warn!("LLM extraction failed, falling back to pattern-based: {}", e);
            
            // Fallback to pattern-based extraction
            let person_names = neo4j_context::extract_person_names(&payload.text);
            entities.extend(person_names);
        }
    }
    
    // Add core value names as potential entities
    for (value_name, _, _) in &core_values {
        entities.push(value_name.clone());
    }
    
    if !entities.is_empty() {
        info!("Extracted {} entities: {:?}", entities.len(), entities);
    }
    
    // Step 5 - Get related parent_ids from Neo4j (based on entities)
    let neo4j_parent_ids = neo4j::fetch_related_parent_ids(&state.neo4j, &payload.user_id, &entities)
        .await
        .unwrap_or_default();
    
    info!("Found {} related parent_ids from Neo4j", neo4j_parent_ids.len());
    
    // Step 6 - Get current active session to exclude from context
    let current_session_id = postgres::get_or_create_active_session(&state.pg_pool, &payload.user_id)
        .await
        .ok(); // Use ok() to convert Result to Option
    
    if let Some(session_id) = current_session_id {
        info!("Current active session: {} (will be excluded from context)", session_id);
    }
    
    // Step 7 - Deduplicate parent_ids using HashSet
    let mut unique_parent_ids = HashSet::new();
    for (parent_id_str, _score) in &filtered_results {
        if let Ok(uuid) = Uuid::parse_str(parent_id_str) {
            // Exclude current active session
            if let Some(current_id) = current_session_id {
                if uuid != current_id {
                    unique_parent_ids.insert(uuid);
                }
            } else {
                unique_parent_ids.insert(uuid);
            }
        }
    }
    for parent_id in neo4j_parent_ids {
        // Exclude current active session
        if let Some(current_id) = current_session_id {
            if parent_id != current_id {
                unique_parent_ids.insert(parent_id);
            }
        } else {
            unique_parent_ids.insert(parent_id);
        }
    }
    
    let parent_ids: Vec<Uuid> = unique_parent_ids.into_iter().collect();
    info!("Total unique parent_ids (excluding current session): {}", parent_ids.len());
    
    // Step 8 - Fetch full content from PostgreSQL
    let session_contents: Vec<postgres::SessionContent> = postgres::fetch_session_content(&state.pg_pool, &parent_ids)
        .await
        .map_err(|e| {
            error!("Failed to fetch session contents: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    info!("Fetched {} session contents from PostgreSQL", session_contents.len());
    
    // Step 9 - Generate response with rich context
    let reply = generate_mirror_response(&state, &payload.text, &session_contents, &core_values)
        .await
        .map_err(|e| {
            error!("Failed to generate response: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    // Step 10 - Save memory in background (PostgreSQL -> Qdrant -> Neo4j)
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
    
    // Build context with dynamic core values injection
    let mut context = String::from(SYSTEM_PROMPT);
    
    // Inject core values dynamically
    if !core_values.is_empty() {
        info!("Injecting {} core values into context", core_values.len());
        context.push_str("\n\n【現在焦点を当てているコアバリュー】\n");
        for (value_name, weight, ctx) in core_values {
            context.push_str(&format!("- **{}** (重要度: {:.2})\n", value_name, weight));
            context.push_str(&format!("  背景: {}\n\n", ctx));
        }
    }
    
    // Add parent episode contexts (rich, full conversations)
    if !session_contents.is_empty() {
        let max_sessions = 5; // Limit to top 5 most relevant sessions
        let sessions_to_inject = session_contents.iter().take(max_sessions);
        info!("Injecting {} session contexts", sessions_to_inject.clone().count());
        
        context.push_str("\n\n【現在の話題の対象に関する過去の記憶】\n");
        for session in sessions_to_inject {
            context.push_str(&format!("- セッション（{}ターン）:\n{}\n\n", session.turn_count, session.content));
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
    reply_text: &str,
    embedding: Vec<f32>,
) -> anyhow::Result<()> {
    info!("Saving memory with session-based architecture");
    
    // Step 1: Get or create active session
    let parent_id: Uuid = postgres::get_or_create_active_session(&state.pg_pool, user_id).await?;
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

pub async fn send_voice_message(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<ChatResponse>, StatusCode> {
    info!("Received voice message");
    
    let mut user_id: Option<String> = None;
    let mut audio_data: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;
    
    // Parse multipart form data
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        error!("Failed to get next field: {}", e);
        StatusCode::BAD_REQUEST
    })? {
        let name = field.name().unwrap_or("").to_string();
        
        match name.as_str() {
            "user_id" => {
                user_id = Some(field.text().await.map_err(|e| {
                    error!("Failed to read user_id: {}", e);
                    StatusCode::BAD_REQUEST
                })?);
            }
            "audio" => {
                filename = field.file_name().map(|s| s.to_string());
                audio_data = Some(field.bytes().await.map_err(|e| {
                    error!("Failed to read audio data: {}", e);
                    StatusCode::BAD_REQUEST
                })?.to_vec());
            }
            _ => {
                info!("Unknown field: {}", name);
            }
        }
    }
    
    let user_id = user_id.ok_or_else(|| {
        error!("Missing user_id");
        StatusCode::BAD_REQUEST
    })?;
    
    let audio_data = audio_data.ok_or_else(|| {
        error!("Missing audio data");
        StatusCode::BAD_REQUEST
    })?;
    
    info!("Received audio from user: {} (size: {} bytes)", user_id, audio_data.len());
    
    // Save audio to temporary file
    let temp_dir = std::env::temp_dir();
    let file_ext = filename
        .and_then(|f| f.split('.').last().map(|s| s.to_string()))
        .unwrap_or_else(|| "m4a".to_string());
    let temp_path = temp_dir.join(format!("voice_{}.{}", Uuid::new_v4(), file_ext));
    
    {
        let mut file = std::fs::File::create(&temp_path).map_err(|e| {
            error!("Failed to create temp file: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        
        file.write_all(&audio_data).map_err(|e| {
            error!("Failed to write audio data: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    }
    
    info!("Saved audio to temp file: {:?}", temp_path);
    
    // Transcribe audio using OpenAI Whisper
    let transcribed_text = openai::transcribe_audio(&state.openai, temp_path.clone())
        .await
        .map_err(|e| {
            error!("Failed to transcribe audio: {}", e);
            // Clean up temp file
            let _ = std::fs::remove_file(&temp_path);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    info!("Transcribed text: {}", transcribed_text);
    
    // Clean up temp file
    if let Err(e) = std::fs::remove_file(&temp_path) {
        error!("Failed to remove temp file: {}", e);
    }
    
    // Create ChatRequest from transcribed text
    let chat_request = ChatRequest {
        user_id,
        text: transcribed_text.clone(),
    };
    
    // Use existing send_message logic
    let mut response = send_message(State(state), Json(chat_request)).await?;
    
    // Add transcribed text to response
    response.0.transcribed_text = Some(transcribed_text);
    
    Ok(response)
}
