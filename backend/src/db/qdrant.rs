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
