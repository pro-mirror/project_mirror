pub mod chat;
pub mod health;
pub mod insights;

use neo4rs::Graph;
use qdrant_client::Qdrant;
use async_openai::Client;

#[derive(Clone)]
pub struct AppState {
    pub neo4j: Graph,
    pub qdrant: Qdrant,
    pub openai: Client<async_openai::config::OpenAIConfig>,
}
