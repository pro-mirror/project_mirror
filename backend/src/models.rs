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
}

// Graph Models
#[derive(Debug, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub node_type: String,
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

// Qdrant Payload (simplified for parent-document retrieval)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SubChunkPayload {
    pub parent_id: String,
    pub user_id: String,
}

// Legacy payload for backward compatibility (will be phased out)
#[derive(Debug, Serialize, Deserialize)]
pub struct EpisodePayload {
    pub chunk_id: String,
    pub timestamp: i64,
    pub speaker: String,
    pub text: String,
    pub reply_text: Option<String>,
    pub emotion_type: String,
    pub linked_node_ids: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExtractedMemory {
    pub person_name: Option<String>,
    pub emotion_type: String,
    pub intensity: f32,
    pub reason: String,
    pub concepts: Vec<String>,
}

// New architecture: Core Value extraction
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CoreValueExtraction {
    pub value_name: String,  // Abstract core value (e.g., "家族との絆", "誠実さ")
    pub weight: f32,         // Importance (0.0 - 1.0)
    pub context: String,     // LLM's interpretation of what user felt
    pub related_person: Option<String>,  // Person related to this value
}
