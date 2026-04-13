use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use crate::api::AppState;
use crate::models::{GraphResponse, GraphNode, GraphEdge};
use tracing::info;

pub async fn get_graph(
    State(_state): State<AppState>,
) -> Result<Json<GraphResponse>, StatusCode> {
    info!("Fetching graph data");
    
    // TODO: Query Neo4j for nodes and relationships
    
    // Mock data for now
    let nodes = vec![
        GraphNode {
            id: "user".to_string(),
            label: "あなた".to_string(),
            node_type: "User".to_string(),
        },
        GraphNode {
            id: "person_1".to_string(),
            label: "奥様".to_string(),
            node_type: "Person".to_string(),
        },
        GraphNode {
            id: "concept_1".to_string(),
            label: "感謝".to_string(),
            node_type: "Concept".to_string(),
        },
    ];
    
    let edges = vec![
        GraphEdge {
            source: "user".to_string(),
            target: "person_1".to_string(),
            relation: "FELT_GRATITUDE".to_string(),
            weight: 0.9,
        },
        GraphEdge {
            source: "user".to_string(),
            target: "concept_1".to_string(),
            relation: "VALUES".to_string(),
            weight: 1.0,
        },
    ];
    
    Ok(Json(GraphResponse { nodes, edges }))
}
