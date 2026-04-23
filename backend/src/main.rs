use anyhow::Result;
use axum::{routing::{get, post}, Router};
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
use crate::api::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    dotenv::dotenv().ok();
    
    tracing::info!("Initializing Project Mirror Backend...");
    let config = Config::from_env()?;
    tracing::info!("Configuration loaded successfully");
    tracing::info!("Server will listen on port: {}", config.port);

    let init_state = Arc::new(RwLock::new(api::InitState::default()));

    // Spawn database initialization as a background task so the HTTP server
    // can bind and pass health checks immediately without waiting for DB setup.
    {
        let config_bg = config.clone();
        let init_state_bg = Arc::clone(&init_state);
        tokio::spawn(async move {
            tracing::info!("Background DB initialization started");
            match initialize_databases(&config_bg, &init_state_bg).await {
                Ok(()) => tracing::info!("Background DB initialization completed successfully"),
                Err(e) => tracing::error!("Background DB initialization failed: {:#}", e),
            }
        });
    }

    let app_state = AppState {
        inner: init_state,
        config: config.clone(),
    };

    let app = Router::new()
        .route("/health", get(api::health::health_check))
        .route("/api/v1/chat/message", post(api::chat::send_message))
        .route("/api/v1/chat/voice", post(api::chat::send_voice_message))
        .route("/api/v1/insights/core-value-graph", get(api::insights::get_core_value_graph))
        .route("/api/v1/insights/core-values/:name", get(api::insights::get_core_value_detail))
        .route("/api/v1/episodes", get(api::episodes::get_episodes))
        .route("/api/v1/episodes/:id", get(api::episodes::get_episode_by_id))
        .route("/api/v1/episodes/parent/:parent_id", get(api::episodes::get_episode_by_parent_id))
        .route("/api/v1/maintenance/cleanup", post(api::maintenance::cleanup_old_data))
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("Attempting to bind to address: {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("Successfully bound to {}", addr);
    tracing::info!("Project Mirror Backend is now running and accepting connections");
    tracing::info!("Health check available at: http://{}/health", addr);
    axum::serve(listener, app).await?;
    Ok(())
}

async fn initialize_databases(config: &Config, init_state: &Arc<RwLock<api::InitState>>) -> Result<()> {
    tracing::info!("Starting database initialization...");
    
    tracing::info!("Connecting to Neo4j...");
    let neo4j_client = db::neo4j::create_client(config).await?;
    tracing::info!("Neo4j client created successfully");
    
    tracing::info!("Connecting to Qdrant...");
    let qdrant_client = db::qdrant::create_client(config).await?;
    tracing::info!("Qdrant client created successfully");
    
    tracing::info!("Connecting to PostgreSQL...");
    let pg_pool = db::postgres::create_pool(&config.database_public_url).await?;
    tracing::info!("PostgreSQL pool created successfully");
    
    tracing::info!("Creating OpenAI client...");
    let openai_client = llm::openai::create_client(config)?;
    tracing::info!("OpenAI client created successfully");

    tracing::info!("Initializing Neo4j schema...");
    db::neo4j::initialize_schema(&neo4j_client).await?;
    tracing::info!("Neo4j schema initialized");
    
    tracing::info!("Initializing Qdrant collection...");
    db::qdrant::initialize_collection(&qdrant_client).await?;
    tracing::info!("Qdrant collection initialized");
    
    tracing::info!("Initializing PostgreSQL schema...");
    db::postgres::initialize_schema(&pg_pool).await?;
    tracing::info!("PostgreSQL schema initialized");

    let mut state = init_state.write().await;
    state.neo4j = Some(neo4j_client);
    state.qdrant = Some(qdrant_client); // Qdrant型で一致
    state.pg_pool = Some(pg_pool);
    state.openai = Some(openai_client);
    state.initialized = true;
    
    tracing::info!("All databases initialized successfully");
    Ok(())
}