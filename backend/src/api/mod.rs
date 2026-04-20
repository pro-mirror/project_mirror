use std::sync::Arc;
use tokio::sync::RwLock;
// 修正：QdrantClient ではなく Qdrant をインポート
pub use qdrant_client::Qdrant; 

pub mod health;
pub mod chat;
pub mod insights;
pub mod episodes;

#[derive(Clone, Default)]
pub struct InitState {
    pub neo4j: Option<neo4rs::Graph>,
    // 修正：ジェネリクス不要の Qdrant 型にする
    pub qdrant: Option<Qdrant>, 
    pub pg_pool: Option<sqlx::PgPool>,
    pub openai: Option<async_openai::Client<async_openai::config::OpenAIConfig>>,
    pub initialized: bool,
}

#[derive(Clone)]
pub struct AppState {
    pub inner: Arc<RwLock<InitState>>,
    pub config: crate::config::Config,
}