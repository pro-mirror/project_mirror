use serde::{Deserialize, Serialize};

// API Models
#[derive(Debug, Serialize, Deserialize)]
pub struct ChatRequest {
    pub user_id: String,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatResponse {
    pub reply_text: String,
    pub emotion_detected: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcribed_text: Option<String>,
}

// Graph Models
#[derive(Debug, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub node_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,  // For Episode nodes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,  // For Episode nodes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_weight: Option<f32>,  // For CoreValue nodes
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub relation: String,
    pub weight: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphResponse {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

// CoreValue Detail
#[derive(Debug, Serialize, Deserialize)]
pub struct CoreValueContext {
    pub episode_parent_id: String,
    pub context: String,
    pub weight: f32,
    pub timestamp: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CoreValueDetail {
    pub value_name: String,
    pub total_weight: f32,
    pub contexts: Vec<CoreValueContext>,
}

// Episode Detail
#[derive(Debug, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub role: String,  // "user" or "assistant"
    pub content: String,
    pub timestamp: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EpisodeDetail {
    pub parent_id: String,
    pub timestamp: i64,
    pub core_values: Vec<String>,
    pub persons: Vec<String>,
    pub messages: Vec<ConversationMessage>,
}

// Qdrant Payload (simplified for parent-document retrieval)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SubChunkPayload {
    pub parent_id: String,
    pub user_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExtractedMemory {
    pub persons: Vec<String>,  // Mentioned persons (names, relationships, pronouns)
    pub keywords: Vec<String>,  // Related keywords, themes, or topics
    pub emotion_type: String,
    pub intensity: f32,
    pub reason: String,
}

// New architecture: Core Value extraction
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CoreValueExtraction {
    pub value_name: String,  // Abstract core value (e.g., "家族との絆", "誠実さ")
    pub weight: f32,         // Importance (0.0 - 1.0)
    pub context: String,     // LLM's interpretation of what user felt
    pub related_person: Option<String>,  // Person related to this value
}
