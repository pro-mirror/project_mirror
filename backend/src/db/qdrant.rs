use anyhow::{Result, Context};
use qdrant_client::Qdrant;
use qdrant_client::qdrant::Distance;
use crate::config::Config;
use std::time::Duration;

const COLLECTION_NAME: &str = "mirror_episodes";
const VECTOR_SIZE: u64 = 1536; // OpenAI text-embedding-3-small
const MAX_RETRIES: u32 = 10;
const INITIAL_RETRY_DELAY_MS: u64 = 500;

pub async fn create_client(config: &Config) -> Result<Qdrant> {
    let mut retry_count = 0;
    let mut delay = Duration::from_millis(INITIAL_RETRY_DELAY_MS);
    
    loop {
        tracing::info!("Attempting to connect to Qdrant at {} (attempt {}/{})", 
            config.qdrant_url, retry_count + 1, MAX_RETRIES);
        
        match Qdrant::from_url(&config.qdrant_url)
            .api_key(config.qdrant_api_key.clone())
            .build() 
        {
            Ok(client) => {
                tracing::info!("✓ Successfully connected to Qdrant at {}", config.qdrant_url);
                return Ok(client);
            }
            Err(e) if retry_count < MAX_RETRIES => {
                retry_count += 1;
                tracing::warn!(
                    "Failed to connect to Qdrant (attempt {}/{}): {}. Retrying in {:?}...", 
                    retry_count, MAX_RETRIES, e, delay
                );
                tokio::time::sleep(delay).await;
                // Exponential backoff with max 8 seconds
                delay = std::cmp::min(delay * 2, Duration::from_secs(8));
            }
            Err(e) => {
                return Err(e).context(format!(
                    "Failed to connect to Qdrant after {} attempts. Check: 1) Qdrant URL is correct, 2) API key is valid, 3) Network connectivity, 4) Qdrant service is running", 
                    MAX_RETRIES
                ));
            }
        }
    }
}

/// Initialize the Qdrant collection if it doesn't exist
pub async fn initialize_collection(client: &Qdrant) -> Result<()> {
    use qdrant_client::qdrant::{FieldType, CreateFieldIndexCollectionBuilder};
    
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
        
        // Create payload index for parent_id field (required for filtering)
        let index_request = CreateFieldIndexCollectionBuilder::new(
            COLLECTION_NAME,
            "parent_id",
            FieldType::Keyword,
        );
        
        client.create_field_index(index_request).await?;
        
        tracing::info!("Created payload index for parent_id field");
    } else {
        tracing::info!("Qdrant collection already exists: {}", COLLECTION_NAME);
        
        // Ensure index exists even if collection was created before
        // This is idempotent - won't fail if index already exists
        let index_request = CreateFieldIndexCollectionBuilder::new(
            COLLECTION_NAME,
            "parent_id",
            FieldType::Keyword,
        );
        
        match client.create_field_index(index_request).await {
            Ok(_) => tracing::info!("Ensured payload index exists for parent_id field"),
            Err(e) => tracing::warn!("Index creation skipped (may already exist): {}", e),
        }
    }
    
    Ok(())
}

/// Delete and recreate the Qdrant collection (use with caution)
/// Development utility: not used in production
#[allow(dead_code)]
pub async fn recreate_collection(client: &Qdrant) -> Result<()> {
    use qdrant_client::qdrant::{FieldType, CreateFieldIndexCollectionBuilder};
    
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
    
    // Create payload index for parent_id field
    let index_request = CreateFieldIndexCollectionBuilder::new(
        COLLECTION_NAME,
        "parent_id",
        FieldType::Keyword,
    );
    
    client.create_field_index(index_request).await?;
    
    tracing::info!("Created payload index for parent_id field");
    
    Ok(())
}

/// Delete vectors by parent_ids (for cleanup)
/// Returns the number of points deleted (approximate)
pub async fn delete_vectors_by_parent_ids(
    client: &Qdrant,
    parent_ids: &[uuid::Uuid],
) -> Result<usize> {
    use qdrant_client::qdrant::{Condition, Filter, DeletePoints, PointsSelector, points_selector};
    
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

    // Delete points matching the filter using low-level API
    let delete_request = DeletePoints {
        collection_name: COLLECTION_NAME.to_string(),
        wait: Some(true),
        points: Some(PointsSelector {
            points_selector_one_of: Some(
                points_selector::PointsSelectorOneOf::Filter(filter)
            )
        }),
        ordering: None,
        shard_key_selector: None, timeout: None,
    };
    
    client.delete_points(delete_request).await?;

    let deleted_count = parent_ids.len();
    tracing::info!("Deleted ~{} vectors from Qdrant (parent_ids: {})", deleted_count, parent_ids.len());
    
    Ok(deleted_count)
}
