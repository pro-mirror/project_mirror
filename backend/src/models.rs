use serde::{Deserialize, Serialize};

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

#[derive(Debug, Serialize, Deserialize)]
pub struct EpisodePayload {
    pub chunk_id: String,
    pub timestamp: i64,
    pub speaker: String,
    pub text: String,
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
