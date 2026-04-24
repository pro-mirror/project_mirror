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
    // Use eprintln! first - it doesn't depend on tracing initialization
    eprintln!("=== APPLICATION STARTING ===");
    eprintln!("Checking environment variables:");
    eprintln!("  PORT: {:?}", std::env::var("PORT"));
    eprintln!("  HOST: {:?}", std::env::var("HOST"));
    eprintln!("  DATABASE_PUBLIC_URL exists: {}", std::env::var("DATABASE_PUBLIC_URL").is_ok());
    eprintln!("  NEO4J_URI exists: {}", std::env::var("NEO4J_URI").is_ok());
    eprintln!("  QDRANT_URL exists: {}", std::env::var("QDRANT_URL").is_ok());
    eprintln!("  OPENAI_API_KEY exists: {}", std::env::var("OPENAI_API_KEY").is_ok());
    
    // Load .env file first
    eprintln!("Loading .env file...");
    dotenv::dotenv().ok();
    eprintln!(".env loaded");
    
    // Initialize tracing
    eprintln!("Initializing tracing subscriber...");
    tracing_subscriber::fmt::init();
    eprintln!("Tracing initialized successfully");
    
    tracing::info!("Initializing Project Mirror Backend...");
    eprintln!("Creating configuration from environment...");
    let config = Config::from_env()?;
    eprintln!("Configuration loaded successfully");
    tracing::info!("Configuration loaded successfully");
    tracing::info!("Server will listen on port: {}", config.port);

    eprintln!("Creating initialization state...");
    let init_state = Arc::new(RwLock::new(api::InitState::default()));
    eprintln!("Initialization state created");

    // Spawn database initialization as a background task so the HTTP server
    // can bind and pass health checks immediately without waiting for DB setup.
    {
        eprintln!("Spawning background database initialization task...");
        let config_bg = config.clone();
        let init_state_bg = Arc::clone(&init_state);
        tokio::spawn(async move {
            eprintln!("Background DB initialization task started");
            tracing::info!("Background DB initialization started");
            match initialize_databases(&config_bg, &init_state_bg).await {
                Ok(()) => {
                    eprintln!("Background DB initialization completed successfully");
                    tracing::info!("Background DB initialization completed successfully");
                },
                Err(e) => {
                    eprintln!("Background DB initialization failed: {:#}", e);
                    tracing::error!("Background DB initialization failed: {:#}", e);
                }
            }
        });
        eprintln!("Background task spawned");
    }

    eprintln!("Creating application state...");
    let app_state = AppState {
        inner: init_state,
        config: config.clone(),
    };
    eprintln!("Application state created");

    eprintln!("Building HTTP router...");
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
    eprintln!("HTTP router built successfully");

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    eprintln!("Attempting to bind to address: {}", addr);
    tracing::info!("Attempting to bind to address: {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    eprintln!("Successfully bound to {}", addr);
    tracing::info!("Successfully bound to {}", addr);
    tracing::info!("Project Mirror Backend is now running and accepting connections");
    tracing::info!("Health check available at: http://{}/health", addr);
    eprintln!("Starting axum server...");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn initialize_databases(config: &Config, init_state: &Arc<RwLock<api::InitState>>) -> Result<()> {
    tracing::info!("=== Starting database initialization ===");
    
    tracing::info!("[1/4] Connecting to Neo4j...");
    let neo4j_client = match db::neo4j::create_client(config).await {
        Ok(client) => {
            tracing::info!("[1/4] ✓ Neo4j connection established");
            client
        }
        Err(e) => {
            tracing::error!("[1/4] ✗ Neo4j connection failed: {:#}", e);
            return Err(e);
        }
    };
    
    tracing::info!("[2/4] Connecting to Qdrant...");
    let qdrant_client = match db::qdrant::create_client(config).await {
        Ok(client) => {
            tracing::info!("[2/4] ✓ Qdrant connection established");
            client
        }
        Err(e) => {
            tracing::error!("[2/4] ✗ Qdrant connection failed: {:#}", e);
            return Err(e);
        }
    };
    
    tracing::info!("[3/4] Connecting to PostgreSQL...");
    let pg_pool = match db::postgres::create_pool(&config.database_public_url).await {
        Ok(pool) => {
            tracing::info!("[3/4] ✓ PostgreSQL connection established");
            pool
        }
        Err(e) => {
            tracing::error!("[3/4] ✗ PostgreSQL connection failed: {:#}", e);
            return Err(e);
        }
    };
    
    tracing::info!("[4/4] Creating OpenAI client...");
    let openai_client = match llm::openai::create_client(config) {
        Ok(client) => {
            tracing::info!("[4/4] ✓ OpenAI client created");
            client
        }
        Err(e) => {
            tracing::error!("[4/4] ✗ OpenAI client creation failed: {:#}", e);
            return Err(e);
        }
    };

    tracing::info!("=== Initializing database schemas ===");
    
    tracing::info!("Initializing Neo4j schema...");
    if let Err(e) = db::neo4j::initialize_schema(&neo4j_client).await {
        tracing::error!("Neo4j schema initialization failed: {:#}", e);
        return Err(e);
    }
    tracing::info!("✓ Neo4j schema initialized");
    
    tracing::info!("Initializing Qdrant collection...");
    if let Err(e) = db::qdrant::initialize_collection(&qdrant_client).await {
        tracing::error!("Qdrant collection initialization failed: {:#}", e);
        return Err(e);
    }
    tracing::info!("✓ Qdrant collection initialized");
    
    tracing::info!("Initializing PostgreSQL schema...");
    if let Err(e) = db::postgres::initialize_schema(&pg_pool).await {
        tracing::error!("PostgreSQL schema initialization failed: {:#}", e);
        return Err(e);
    }
    tracing::info!("✓ PostgreSQL schema initialized");

    let mut state = init_state.write().await;
    state.neo4j = Some(neo4j_client);
    state.qdrant = Some(qdrant_client);
    state.pg_pool = Some(pg_pool);
    state.openai = Some(openai_client);
    state.initialized = true;
    
    tracing::info!("=== ✓ All databases initialized successfully ===");
    Ok(())
}