// use anyhow::Result;
// use axum::{
//     routing::{get, post},
//     Router,
// };
// use std::net::SocketAddr;
// use tower_http::cors::CorsLayer;
// use tracing_subscriber;

// mod api;
// mod db;
// mod llm;
// mod models;
// mod config;

// use config::Config;

// #[tokio::main]
// async fn main() -> Result<()> {
//     // Initialize tracing
//     tracing_subscriber::fmt::init();
    
//     // Load environment variables
//     dotenv::dotenv().ok();
    
//     // Load configuration
//     let config = Config::from_env()?;
    
//     tracing::info!("Starting Project Mirror Backend...");
//     tracing::info!("Server will listen on {}:{}", config.host, config.port);
    
//     // Initialize database connections
//     let neo4j_client = db::neo4j::create_client(&config).await?;
//     let qdrant_client = db::qdrant::create_client(&config).await?;
//     let pg_pool = db::postgres::create_pool(&config.database_public_url).await?;
    
//     // Initialize Neo4j schema
//     db::neo4j::initialize_schema(&neo4j_client).await?;
    
//     // Initialize Qdrant collection
//     db::qdrant::initialize_collection(&qdrant_client).await?;
    
//     // Initialize PostgreSQL schema
//     db::postgres::initialize_schema(&pg_pool).await?;
    
//     // Initialize OpenAI client
//     let openai_client = llm::openai::create_client(&config)?;
    
//     // Create application state
//     let app_state = api::AppState {
//         neo4j: neo4j_client,
//         qdrant: qdrant_client,
//         openai: openai_client,
//         pg_pool,
//     };
    
//     // Build router
//     let app = Router::new()
//         .route("/health", get(api::health::health_check))
//         .route("/api/v1/chat/message", post(api::chat::send_message))
//         .route("/api/v1/chat/voice", post(api::chat::send_voice_message))
//         .route("/api/v1/insights/core-value-graph", get(api::insights::get_core_value_graph))
//         .route("/api/v1/insights/core-values/:name", get(api::insights::get_core_value_detail))
//         .route("/api/v1/episodes", get(api::episodes::get_episodes))
//         .route("/api/v1/episodes/:id", get(api::episodes::get_episode_by_id))
//         .route("/api/v1/episodes/parent/:parent_id", get(api::episodes::get_episode_by_parent_id))
//         .layer(CorsLayer::permissive())
//         .with_state(app_state);
    
//     // Start server
//     let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
//     tracing::info!("Server listening on {}", addr);
    
//     let listener = tokio::net::TcpListener::bind(addr).await?;
//     axum::serve(listener, app).await?;
    
//     Ok(())
// }

use anyhow::Result;                                                                                     
use axum::{                                                                                             
    routing::{get, post},                                                                               
    Router,                                                                                             
};                                                                                                      
use std::net::SocketAddr;                                                                               
use std::sync::Arc;                                                                                     
use tokio::sync::RwLock;                                                                                
use tower_http::cors::CorsLayer;                                                                        
use tracing_subscriber;                                                                                 
                                                                                                        
mod api;                                                                                                
mod db;                                                                                                 
mod llm;                                                                                                
mod models;                                                                                             
mod config;                                                                                             
                                                                                                        
use config::Config;                                                                                     
                                                                                                        
#[tokio::main]                                                                                          
async fn main() -> Result<()> {                                                                         
    // Initialize tracing                                                                               
    tracing_subscriber::fmt::init();                                                                    
                                                                                                        
    // Load environment variables                                                                       
    dotenv::dotenv().ok();                                                                              
                                                                                                        
    // Load configuration                                                                               
    let config = Config::from_env()?;                                                                   
                                                                                                        
    tracing::info!("Starting Project Mirror Backend...");                                               
    tracing::info!("Server will listen on {}:{}", config.host, config.port);                            
                                                                                                        
    // Create shared state for initialization status                                                    
    let init_state = Arc::new(RwLock::new(api::InitState::default()));                                  
    let init_state_clone = init_state.clone();                                                          
    let config_clone = config.clone();                                                                  
                                                                                                        
    // Spawn background task for database initialization                                                
    tokio::spawn(async move {                                                                           
        match initialize_databases(&config_clone, &init_state_clone).await {                            
            Ok(_) => {                                                                                  
                tracing::info!("Database initialization completed successfully");                       
            }                                                                                           
            Err(e) => {                                                                                 
                tracing::error!("Database initialization failed: {}", e);                               
            }                                                                                           
        }                                                                                               
    });                                                                                                 
                                                                                                        
    // Build router with lazy-initialized state                                                         
    let app = Router::new()                                                                             
        .route("/health", get(api::health::health_check))                                               
        .route("/api/v1/chat/message", post(api::chat::send_message))                                   
        .route("/api/v1/chat/voice", post(api::chat::send_voice_message))                               
        .route("/api/v1/insights/core-value-graph", get(api::insights::get_core_value_graph))           
        .route("/api/v1/insights/core-values/:name", get(api::insights::get_core_value_detail))         
        .route("/api/v1/episodes", get(api::episodes::get_episodes))                                    
        .route("/api/v1/episodes/:id", get(api::episodes::get_episode_by_id))                           
        .route("/api/v1/episodes/parent/:parent_id", get(api::episodes::get_episode_by_parent_id))      
        .layer(CorsLayer::permissive())                                                                 
        .with_state((init_state, config));                                                              
                                                                                                        
    // Start server immediately                                                                         
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));                                           
    tracing::info!("Server listening on {}", addr);                                                     
                                                                                                        
    let listener = tokio::net::TcpListener::bind(addr).await?;                                          
    axum::serve(listener, app).await?;                                                                  
                                                                                                        
    Ok(())                                                                                              
}                                                                                                       
                                                                                                        
async fn initialize_databases(config: &Config, init_state: &Arc<RwLock<api::InitState>>) -> Result<()> {
    // Initialize database connections with timeout                                                     
    let neo4j_result = tokio::time::timeout(                                                            
        std::time::Duration::from_secs(30),                                                             
        db::neo4j::create_client(config)                                                                
    ).await;                                                                                            
                                                                                                        
    let qdrant_result = tokio::time::timeout(                                                           
        std::time::Duration::from_secs(30),                                                             
        db::qdrant::create_client(config)                                                               
    ).await;                                                                                            
                                                                                                        
    let pg_result = tokio::time::timeout(                                                               
        std::time::Duration::from_secs(30),                                                             
        db::postgres::create_pool(&config.database_public_url)                                          
    ).await;                                                                                            
                                                                                                        
    let openai_result = llm::openai::create_client(config);                                             
                                                                                                        
    // Handle results                                                                                   
    let neo4j_client = neo4j_result??;                                                                  
    let qdrant_client = qdrant_result??;                                                                
    let pg_pool = pg_result??;                                                                          
    let openai_client = openai_result?;                                                                 
                                                                                                        
    // Initialize schemas                                                                               
    db::neo4j::initialize_schema(&neo4j_client).await?;                                                 
    db::qdrant::initialize_collection(&qdrant_client).await?;                                           
    db::postgres::initialize_schema(&pg_pool).await?;                                                   
                                                                                                        
    // Update initialization state                                                                      
    let mut state = init_state.write().await;                                                           
    state.neo4j = Some(neo4j_client);                                                                   
    state.qdrant = Some(qdrant_client);                                                                 
    state.pg_pool = Some(pg_pool);                                                                      
    state.openai = Some(openai_client);                                                                 
    state.initialized = true;                                                                           
                                                                                                        
    Ok(())                                                                                              
}       