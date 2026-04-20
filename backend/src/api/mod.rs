// pub mod chat;
// pub mod health;
// pub mod insights;
// pub mod episodes;

// use neo4rs::Graph;
// use qdrant_client::Qdrant;
// use async_openai::Client;
// use sqlx::PgPool;

// #[derive(Clone)]
// pub struct AppState {
//     pub neo4j: Graph,
//     pub qdrant: Qdrant,
//     pub openai: Client<async_openai::config::OpenAIConfig>,
//     pub pg_pool: PgPool,
// }


use std::sync::Arc;                                                              
use tokio::sync::RwLock;                                                         
                                                                                 
pub mod health;                                                                  
pub mod chat;                                                                    
pub mod insights;                                                                
pub mod episodes;                                                                
                                                                                 
#[derive(Clone, Default)]                                                        
pub struct InitState {                                                           
    pub neo4j: Option<neo4rs::Graph>,                                            
    pub qdrant: Option<qdrant_client::client::QdrantClient>,                     
    pub pg_pool: Option<sqlx::PgPool>,                                           
    pub openai: Option<async_openai::Client<async_openai::config::OpenAIConfig>>,
    pub initialized: bool,                                                       
}                                                                                
                                                                                 
pub type AppState = (Arc<RwLock<InitState>>, crate::config::Config);   