use anyhow::Result;
use async_openai::{Client, config::OpenAIConfig};
use crate::config::Config;

pub fn create_client(config: &Config) -> Result<Client<OpenAIConfig>> {
    let openai_config = OpenAIConfig::new()
        .with_api_key(&config.openai_api_key);
    
    let client = Client::with_config(openai_config);
    
    tracing::info!("Initialized OpenAI client");
    
    Ok(client)
}
