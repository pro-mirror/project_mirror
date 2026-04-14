use anyhow::{Context, Result};
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub openai_api_key: String,
    pub qdrant_url: String,
    pub qdrant_api_key: String,
    pub neo4j_uri: String,
    pub neo4j_user: String,
    pub neo4j_password: String,
    pub neo4j_database: String,
    pub database_public_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .context("Failed to parse PORT")?,
            openai_api_key: env::var("OPENAI_API_KEY")
                .context("OPENAI_API_KEY must be set")?,
            qdrant_url: env::var("QDRANT_URL")
                .context("QDRANT_URL must be set")?,
            qdrant_api_key: env::var("QDRANT_API_KEY")
                .context("QDRANT_API_KEY must be set")?,
            neo4j_uri: env::var("NEO4J_URI")
                .context("NEO4J_URI must be set")?,
            neo4j_user: env::var("NEO4J_USER")
                .context("NEO4J_USER must be set")?,
            neo4j_password: env::var("NEO4J_PASSWORD")
                .context("NEO4J_PASSWORD must be set")?,
            neo4j_database: env::var("NEO4J_DATABASE")
                .context("NEO4J_DATABASE must be set")?,
            database_public_url: env::var("DATABASE_PUBLIC_URL")
                .context("DATABASE_PUBLIC_URL must be set")?
        })
    }
}
