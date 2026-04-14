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
