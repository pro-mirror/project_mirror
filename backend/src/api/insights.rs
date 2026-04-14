use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use neo4rs::query;
use crate::api::AppState;
use crate::models::{GraphResponse, GraphNode, GraphEdge};
use tracing::{info, error};

pub async fn get_graph(
    State(state): State<AppState>,
) -> Result<Json<GraphResponse>, StatusCode> {
    info!("Fetching graph data from Neo4j");
    
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    
    // Get all Person nodes
    let person_query = query("MATCH (p:Person) RETURN p.name as name, id(p) as id");
    
    match state.neo4j.execute(person_query).await {
        Ok(mut result) => {
            while let Ok(Some(row)) = result.next().await {
                if let (Ok(name), Ok(id)) = (row.get::<String>("name"), row.get::<i64>("id")) {
                    nodes.push(GraphNode {
                        id: format!("person_{}", id),
                        label: name,
                        node_type: "Person".to_string(),
                    });
                }
            }
        }
        Err(e) => {
            error!("Failed to fetch Person nodes: {}", e);
        }
    }
    
    // Get all Emotion nodes
    let emotion_query = query("MATCH (e:Emotion) RETURN e.type as type, id(e) as id");
    
    match state.neo4j.execute(emotion_query).await {
        Ok(mut result) => {
            while let Ok(Some(row)) = result.next().await {
                if let (Ok(emotion_type), Ok(id)) = (row.get::<String>("type"), row.get::<i64>("id")) {
                    nodes.push(GraphNode {
                        id: format!("emotion_{}", id),
                        label: emotion_type,
                        node_type: "Emotion".to_string(),
                    });
                }
            }
        }
        Err(e) => {
            error!("Failed to fetch Emotion nodes: {}", e);
        }
    }
    
    // Get Person relationships (MENTIONS)
    let person_rel_query = query(
        "MATCH (e:Episode)-[r:MENTIONS]->(p:Person) 
         RETURN id(p) as target_id, count(r) as weight"
    );
    
    match state.neo4j.execute(person_rel_query).await {
        Ok(mut result) => {
            while let Ok(Some(row)) = result.next().await {
                if let (Ok(target_id), Ok(weight)) = (row.get::<i64>("target_id"), row.get::<i64>("weight")) {
                    edges.push(GraphEdge {
                        source: "user".to_string(),
                        target: format!("person_{}", target_id),
                        relation: "MENTIONS".to_string(),
                        weight: (weight as f32) / 10.0,
                    });
                }
            }
        }
        Err(e) => {
            error!("Failed to fetch Person relationships: {}", e);
        }
    }
    
    // Get Emotion relationships (FELT)
    let emotion_rel_query = query(
        "MATCH (e:Episode)-[r:FELT]->(em:Emotion) 
         RETURN id(em) as target_id, avg(r.intensity) as avg_intensity, count(r) as weight"
    );
    
    match state.neo4j.execute(emotion_rel_query).await {
        Ok(mut result) => {
            while let Ok(Some(row)) = result.next().await {
                if let (Ok(target_id), Ok(weight)) = (row.get::<i64>("target_id"), row.get::<i64>("weight")) {
                    edges.push(GraphEdge {
                        source: "user".to_string(),
                        target: format!("emotion_{}", target_id),
                        relation: "FELT".to_string(),
                        weight: (weight as f32) / 10.0,
                    });
                }
            }
        }
        Err(e) => {
            error!("Failed to fetch Emotion relationships: {}", e);
        }
    }
    
    // Get Concept relationships (RELATES_TO)
    // Only include concepts with weight >= 2 to reduce clutter
    let concept_rel_query = query(
        "MATCH (e:Episode)-[r:RELATES_TO]->(c:Concept) 
         WITH c, count(r) as weight
         WHERE weight >= 2
         RETURN id(c) as target_id, c.name as name, weight
         ORDER BY weight DESC
         LIMIT 10"
    );
    
    match state.neo4j.execute(concept_rel_query).await {
        Ok(mut result) => {
            while let Ok(Some(row)) = result.next().await {
                if let (Ok(target_id), Ok(name), Ok(weight)) = (
                    row.get::<i64>("target_id"), 
                    row.get::<String>("name"),
                    row.get::<i64>("weight")
                ) {
                    // Add concept node if not already added
                    let concept_id = format!("concept_{}", target_id);
                    if !nodes.iter().any(|n| n.id == concept_id) {
                        nodes.push(GraphNode {
                            id: concept_id.clone(),
                            label: name,
                            node_type: "Concept".to_string(),
                        });
                    }
                    
                    edges.push(GraphEdge {
                        source: "user".to_string(),
                        target: concept_id,
                        relation: "RELATES_TO".to_string(),
                        weight: (weight as f32) / 10.0,
                    });
                }
            }
        }
        Err(e) => {
            error!("Failed to fetch Concept relationships: {}", e);
        }
    }
    
    // Get Person-Concept relationships (people associated with concepts)
    let person_concept_query = query(
        "MATCH (p:Person)<-[:MENTIONS]-(e:Episode)-[:RELATES_TO]->(c:Concept)
         WITH p, c, count(*) as weight
         WHERE weight >= 2
         RETURN id(p) as person_id, id(c) as concept_id, weight
         ORDER BY weight DESC
         LIMIT 15"
    );
    
    match state.neo4j.execute(person_concept_query).await {
        Ok(mut result) => {
            while let Ok(Some(row)) = result.next().await {
                if let (Ok(person_id), Ok(concept_id), Ok(weight)) = (
                    row.get::<i64>("person_id"),
                    row.get::<i64>("concept_id"),
                    row.get::<i64>("weight")
                ) {
                    edges.push(GraphEdge {
                        source: format!("person_{}", person_id),
                        target: format!("concept_{}", concept_id),
                        relation: "RELATES_TO".to_string(),
                        weight: (weight as f32) / 10.0,
                    });
                }
            }
        }
        Err(e) => {
            error!("Failed to fetch Person-Concept relationships: {}", e);
        }
    }
    
    // Get Person-Person relationships (people appearing together)
    let person_person_query = query(
        "MATCH (p1:Person)<-[:MENTIONS]-(e:Episode)-[:MENTIONS]->(p2:Person)
         WHERE id(p1) < id(p2)
         RETURN id(p1) as person1_id, id(p2) as person2_id, count(*) as weight
         ORDER BY weight DESC
         LIMIT 10"
    );
    
    match state.neo4j.execute(person_person_query).await {
        Ok(mut result) => {
            while let Ok(Some(row)) = result.next().await {
                if let (Ok(person1_id), Ok(person2_id), Ok(weight)) = (
                    row.get::<i64>("person1_id"),
                    row.get::<i64>("person2_id"),
                    row.get::<i64>("weight")
                ) {
                    edges.push(GraphEdge {
                        source: format!("person_{}", person1_id),
                        target: format!("person_{}", person2_id),
                        relation: "CO_OCCURS".to_string(),
                        weight: (weight as f32) / 5.0,
                    });
                }
            }
        }
        Err(e) => {
            error!("Failed to fetch Person-Person relationships: {}", e);
        }
    }
    
    // Add user node in the center
    if !nodes.is_empty() {
        nodes.insert(0, GraphNode {
            id: "user".to_string(),
            label: "あなた".to_string(),
            node_type: "User".to_string(),
        });
    }
    
    info!("Found {} nodes and {} edges", nodes.len(), edges.len());
    
    Ok(Json(GraphResponse { nodes, edges }))
}
