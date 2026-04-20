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
    let config = Config::from_env()?;

    tracing::info!("Starting Project Mirror Backend...");

    let init_state = Arc::new(RwLock::new(api::InitState::default()));
    let init_state_clone = init_state.clone();
    let config_clone = config.clone();

    tokio::spawn(async move {
        if let Err(e) = initialize_databases(&config_clone, &init_state_clone).await {
            tracing::error!("Database initialization failed: {}", e);
        }
    });

    // 修正：AppState構造体として初期化
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
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn initialize_databases(config: &Config, init_state: &Arc<RwLock<api::InitState>>) -> Result<()> {
    let neo4j_client = db::neo4j::create_client(config).await?;
    let qdrant_client = db::qdrant::create_client(config).await?;
    let pg_pool = db::postgres::create_pool(&config.database_public_url).await?;
    let openai_client = llm::openai::create_client(config)?;

    db::neo4j::initialize_schema(&neo4j_client).await?;
    db::qdrant::initialize_collection(&qdrant_client).await?;
    db::postgres::initialize_schema(&pg_pool).await?;

    let mut state = init_state.write().await;
    state.neo4j = Some(neo4j_client);
    state.qdrant = Some(qdrant_client); // Qdrant型で一致
    state.pg_pool = Some(pg_pool);
    state.openai = Some(openai_client);
    state.initialized = true;
    Ok(())
}