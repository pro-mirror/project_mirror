use anyhow::Result;
use qdrant_client::Qdrant;
use qdrant_client::qdrant::Distance;
use crate::config::Config;

const COLLECTION_NAME: &str = "mirror_episodes";
const VECTOR_SIZE: u64 = 1536; // OpenAI text-embedding-3-small

pub async fn create_client(config: &Config) -> Result<Qdrant> {
    let client = Qdrant::from_url(&config.qdrant_url)
        .api_key(config.qdrant_api_key.clone())
        .build()?;
    
    tracing::info!("Connected to Qdrant at {}", config.qdrant_url);
    
    Ok(client)
}

/// Initialize the Qdrant collection if it doesn't exist
pub async fn initialize_collection(client: &Qdrant) -> Result<()> {
    // Check if collection exists
    let collections = client.list_collections().await?;
    let exists = collections.collections.iter()
        .any(|c| c.name == COLLECTION_NAME);
    
    if !exists {
        client.create_collection(
            qdrant_client::qdrant::CreateCollectionBuilder::new(COLLECTION_NAME)
                .vectors_config(
                    qdrant_client::qdrant::VectorParamsBuilder::new(VECTOR_SIZE, Distance::Cosine)
                )
        ).await?;
        
        tracing::info!("Created Qdrant collection: {}", COLLECTION_NAME);
    } else {
        tracing::info!("Qdrant collection already exists: {}", COLLECTION_NAME);
    }
    
    Ok(())
}

/// Delete and recreate the Qdrant collection (use with caution)
/// Development utility: not used in production
#[allow(dead_code)]
pub async fn recreate_collection(client: &Qdrant) -> Result<()> {
    // Delete if exists
    let collections = client.list_collections().await?;
    let exists = collections.collections.iter()
        .any(|c| c.name == COLLECTION_NAME);
    
    if exists {
        client.delete_collection(COLLECTION_NAME).await?;
        tracing::info!("Deleted Qdrant collection: {}", COLLECTION_NAME);
    }
    
    // Recreate
    client.create_collection(
        qdrant_client::qdrant::CreateCollectionBuilder::new(COLLECTION_NAME)
            .vectors_config(
                qdrant_client::qdrant::VectorParamsBuilder::new(VECTOR_SIZE, Distance::Cosine)
            )
    ).await?;
    
    tracing::info!("Recreated Qdrant collection: {}", COLLECTION_NAME);
    
    Ok(())
}

/// Delete vectors by parent_ids (for cleanup)
/// Returns the number of points deleted (approximate)
pub async fn delete_vectors_by_parent_ids(
    client: &Qdrant,
    parent_ids: &[uuid::Uuid],
) -> Result<usize> {
    use qdrant_client::qdrant::{Condition, Filter, PointsSelector, DeletePointsBuilder};
    
    if parent_ids.is_empty() {
        return Ok(0);
    }

    let parent_id_strs: Vec<String> = parent_ids.iter()
        .map(|id| id.to_string())
        .collect();

    // Create filter for parent_ids
    let filter = Filter::must([
        Condition::matches("parent_id", parent_id_strs)
    ]);

    // Delete points matching the filter
    let delete_result = client
        .delete_points(
            DeletePointsBuilder::new(COLLECTION_NAME)
                .points(PointsSelector::Filter(filter))
        )
        .await?;

    let deleted_count = parent_ids.len();
    tracing::info!("Deleted ~{} vectors from Qdrant (parent_ids: {})", deleted_count, parent_ids.len());
    
    Ok(deleted_count)
}
