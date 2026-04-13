use anyhow::Result;
use qdrant_client::Qdrant;
use qdrant_client::qdrant::{
    PointStruct, UpsertPointsBuilder,
};
use uuid::Uuid;
use std::collections::HashMap;
use crate::models::EpisodePayload;

const COLLECTION_NAME: &str = "mirror_episodes";

pub async fn save_episode(
    client: &Qdrant,
    embedding: Vec<f32>,
    payload: EpisodePayload,
) -> Result<String> {
    let point_id = Uuid::new_v4().to_string();
    
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

pub async fn search_similar(
    client: &Qdrant,
    query_vector: Vec<f32>,
    limit: u64,
) -> Result<Vec<(String, f32, EpisodePayload)>> {
    use qdrant_client::qdrant::SearchPointsBuilder;
    
    let search_result = client
        .search_points(
            SearchPointsBuilder::new(COLLECTION_NAME, query_vector, limit)
                .with_payload(true)
        )
        .await?;

    let mut results = Vec::new();
    for scored_point in search_result.result {
        let id = match scored_point.id {
            Some(point_id) => match point_id.point_id_options {
                Some(options) => match options {
                    qdrant_client::qdrant::point_id::PointIdOptions::Uuid(uuid) => uuid,
                    qdrant_client::qdrant::point_id::PointIdOptions::Num(num) => num.to_string(),
                },
                None => String::new(),
            },
            None => String::new(),
        };
        let score = scored_point.score;
        
        if !scored_point.payload.is_empty() {
            let episode: EpisodePayload = serde_json::from_value(
                serde_json::to_value(&scored_point.payload)?
            )?;
            results.push((id, score, episode));
        }
    }

    Ok(results)
}
