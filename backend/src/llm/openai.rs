use anyhow::Result;
use async_openai::{Client, config::OpenAIConfig, types::CreateTranscriptionRequestArgs};
use crate::config::Config;
use std::path::PathBuf;

pub fn create_client(config: &Config) -> Result<Client<OpenAIConfig>> {
    let openai_config = OpenAIConfig::new()
        .with_api_key(&config.openai_api_key);
    
    let client = Client::with_config(openai_config);
    
    tracing::info!("Initialized OpenAI client");
    
    Ok(client)
}

/// Transcribe audio file to text using OpenAI Whisper API
pub async fn transcribe_audio(
    client: &Client<OpenAIConfig>,
    audio_path: PathBuf,
) -> Result<String> {
    tracing::info!("Transcribing audio file: {:?}", audio_path);
    
    let request = CreateTranscriptionRequestArgs::default()
        .file(audio_path)
        .model("whisper-1")
        .language("ja") // Japanese language
        .build()?;
    
    let response = client.audio().transcribe(request).await?;
    
    tracing::info!("Transcription completed: {}", response.text);
    
    Ok(response.text)
}
