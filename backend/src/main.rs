use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
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
    
    // Initialize database connections
    let neo4j_client = db::neo4j::create_client(&config).await?;
    let qdrant_client = db::qdrant::create_client(&config).await?;
    
    // Initialize Qdrant collection
    db::qdrant::initialize_collection(&qdrant_client).await?;
    
    // Initialize OpenAI client
    let openai_client = llm::openai::create_client(&config)?;
    
    // Create application state
    let app_state = api::AppState {
        neo4j: neo4j_client,
        qdrant: qdrant_client,
        openai: openai_client,
    };
    
    // Build router
    let app = Router::new()
        .route("/health", get(api::health::health_check))
        .route("/api/v1/chat/message", post(api::chat::send_message))
        .route("/api/v1/insights/graph", get(api::insights::get_graph))
        .layer(CorsLayer::permissive())
        .with_state(app_state);
    
    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("Server listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}
