pub mod chat;
pub mod health;
pub mod insights;
pub mod episodes;

use neo4rs::Graph;
use qdrant_client::Qdrant;
use async_openai::Client;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub neo4j: Graph,
    pub qdrant: Qdrant,
    pub openai: Client<async_openai::config::OpenAIConfig>,
    pub pg_pool: PgPool,
}
