use anyhow::Result;
use async_openai::{
    Client,
    types::{CreateEmbeddingRequestArgs, EmbeddingInput},
};

pub async fn create_embedding(
    client: &Client<async_openai::config::OpenAIConfig>,
    text: &str,
) -> Result<Vec<f32>> {
    let request = CreateEmbeddingRequestArgs::default()
        .model("text-embedding-3-small")
        .input(EmbeddingInput::String(text.to_string()))
        .build()?;

    let response = client.embeddings().create(request).await?;
    
    let embedding = response
        .data
        .first()
        .ok_or_else(|| anyhow::anyhow!("No embedding returned"))?
        .embedding
        .clone();

    Ok(embedding)
}
