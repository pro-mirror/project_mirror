use anyhow::Result;
use qdrant_client::Qdrant;
use qdrant_client::qdrant::{
    PointStruct, UpsertPointsBuilder,
};
use uuid::Uuid;
use std::collections::HashMap;
use crate::models::SubChunkPayload;

const COLLECTION_NAME: &str = "mirror_episodes";

/// Save a sub-chunk with parent_id reference (new architecture)
pub async fn save_sub_chunk(
    client: &Qdrant,
    embedding: Vec<f32>,
    parent_id: &str,
    user_id: &str,
) -> Result<String> {
    let point_id = Uuid::new_v4().to_string();
    
    let payload = SubChunkPayload {
        parent_id: parent_id.to_string(),
        user_id: user_id.to_string(),
    };
    
    let payload_map: HashMap<String, serde_json::Value> = serde_json::from_value(
        serde_json::to_value(&payload)?
    )?;
    
    let point = PointStruct::new(
        point_id.clone(),
        embedding,
        payload_map,
    );

    let points = vec![point];
    
    client
        .upsert_points(UpsertPointsBuilder::new(COLLECTION_NAME, points))
        .await?;

    Ok(point_id)
}

/// Search similar sub-chunks and return parent_ids
pub async fn search_similar_parent_ids(
    client: &Qdrant,
    query_vector: Vec<f32>,
    limit: u64,
) -> Result<Vec<(String, f32)>> {
    use qdrant_client::qdrant::SearchPointsBuilder;
    
    let search_result = client
        .search_points(
            SearchPointsBuilder::new(COLLECTION_NAME, query_vector, limit)
                .with_payload(true)
        )
        .await?;

    let mut results = Vec::new();
    for scored_point in search_result.result {
        let score = scored_point.score;
        
        if !scored_point.payload.is_empty() {
            let payload: SubChunkPayload = serde_json::from_value(
                serde_json::to_value(&scored_point.payload)?
            )?;
            results.push((payload.parent_id, score));
        }
    }

    Ok(results)
}
