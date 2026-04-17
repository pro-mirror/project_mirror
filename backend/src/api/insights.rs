use axum::{
    extract::{State, Path},
    http::StatusCode,
    Json,
};
use neo4rs::query;
use crate::api::AppState;
use crate::models::{GraphResponse, GraphNode, GraphEdge, CoreValueDetail, CoreValueContext};
use tracing::{info, error};

/// Get CoreValue-centric graph for visualization
/// Returns CoreValue nodes with their connected Episode nodes
pub async fn get_core_value_graph(
    State(state): State<AppState>,
) -> Result<Json<GraphResponse>, StatusCode> {
    info!("Fetching CoreValue-centric graph from Neo4j");
    
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    
    // Get all CoreValue nodes with total weight
    let cv_query = query(
        "MATCH (cv:CoreValue)
         RETURN cv.name as name, cv.total_weight as total_weight"
    );
    
    match state.neo4j.execute(cv_query).await {
        Ok(mut result) => {
            while let Ok(Some(row)) = result.next().await {
                if let (Ok(name), Ok(total_weight)) = (
                    row.get::<String>("name"),
                    row.get::<f64>("total_weight")
                ) {
                    nodes.push(GraphNode {
                        id: format!("cv_{}", name),
                        label: name,
                        node_type: "CoreValue".to_string(),
                        parent_id: None,
                        timestamp: None,
                        total_weight: Some(total_weight as f32),
                    });
                }
            }
        }
        Err(e) => {
            error!("Failed to fetch CoreValue nodes: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
    
    // Get all Episodes connected to CoreValues
    let episode_query = query(
        "MATCH (e:Episode)-[r:HOLDS]->(cv:CoreValue)
         RETURN e.parent_id as parent_id, 
                e.created_at as created_at,
                cv.name as cv_name,
                r.weight as weight"
    );
    
    match state.neo4j.execute(episode_query).await {
        Ok(mut result) => {
            while let Ok(Some(row)) = result.next().await {
                if let (Ok(parent_id), Ok(cv_name), Ok(weight)) = (
                    row.get::<String>("parent_id"),
                    row.get::<String>("cv_name"),
                    row.get::<f64>("weight")
                ) {
                    let created_at = row.get::<i64>("created_at").ok();
                    let episode_id = format!("ep_{}", parent_id);
                    
                    // Add episode node if not exists
                    if !nodes.iter().any(|n| n.id == episode_id) {
                        nodes.push(GraphNode {
                            id: episode_id.clone(),
                            label: format!("エピソード"),
                            node_type: "Episode".to_string(),
                            parent_id: Some(parent_id.clone()),
                            timestamp: created_at,
                            total_weight: None,
                        });
                    }
                    
                    // Add edge from Episode to CoreValue
                    edges.push(GraphEdge {
                        source: episode_id,
                        target: format!("cv_{}", cv_name),
                        relation: "HOLDS".to_string(),
                        weight: weight as f32,
                    });
                }
            }
        }
        Err(e) => {
            error!("Failed to fetch Episodes: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
    
    info!("Found {} nodes and {} edges", nodes.len(), edges.len());
    
    Ok(Json(GraphResponse { nodes, edges }))
}

/// Get detailed contexts for a specific CoreValue
pub async fn get_core_value_detail(
    State(state): State<AppState>,
    Path(value_name): Path<String>,
) -> Result<Json<CoreValueDetail>, StatusCode> {
    info!("Fetching details for CoreValue: {}", value_name);
    
    // Get total weight
    let weight_query = query(
        "MATCH (cv:CoreValue {name: $name})
         RETURN cv.total_weight as total_weight"
    ).param("name", value_name.as_str());
    
    let total_weight = match state.neo4j.execute(weight_query).await {
        Ok(mut result) => {
            if let Ok(Some(row)) = result.next().await {
                row.get::<f64>("total_weight").unwrap_or(0.0) as f32
            } else {
                return Err(StatusCode::NOT_FOUND);
            }
        }
        Err(e) => {
            error!("Failed to fetch CoreValue weight: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    
    // Get all contexts (episodes that hold this value)
    let contexts_query = query(
        "MATCH (e:Episode)-[r:HOLDS]->(cv:CoreValue {name: $name})
         RETURN e.parent_id as parent_id,
                COALESCE(r.latest_context, r.context) as context,
                r.weight as weight,
                COALESCE(e.created_at.epochSeconds, 0) as created_at
         ORDER BY e.created_at DESC"
    ).param("name", value_name.as_str());
    
    let mut contexts = Vec::new();
    
    match state.neo4j.execute(contexts_query).await {
        Ok(mut result) => {
            while let Ok(Some(row)) = result.next().await {
                if let (Ok(parent_id), Ok(weight), Ok(created_at)) = (
                    row.get::<String>("parent_id"),
                    row.get::<f64>("weight"),
                    row.get::<i64>("created_at")
                ) {
                    let context = row.get::<String>("context").unwrap_or_else(|_| value_name.clone());
                    
                    contexts.push(CoreValueContext {
                        episode_parent_id: parent_id,
                        context,
                        weight: weight as f32,
                        timestamp: created_at,
                    });
                }
            }
        }
        Err(e) => {
            error!("Failed to fetch CoreValue contexts: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
    
    Ok(Json(CoreValueDetail {
        value_name,
        total_weight,
        contexts,
    }))
}
