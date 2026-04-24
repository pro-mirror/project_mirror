use anyhow::{Result, Context};
use sqlx::PgPool;
use uuid::Uuid;
use std::time::Duration;

const MAX_RETRIES: u32 = 10;
const INITIAL_RETRY_DELAY_MS: u64 = 500;

/// Create PostgreSQL connection pool with retry logic
pub async fn create_pool(database_public_url: &str) -> Result<PgPool> {
    let mut retry_count = 0;
    let mut delay = Duration::from_millis(INITIAL_RETRY_DELAY_MS);
    
    loop {
        tracing::info!("Attempting to connect to PostgreSQL (attempt {}/{})", 
            retry_count + 1, MAX_RETRIES);
        
        // Configure connection pool with timeouts (recreate each attempt)
        let pool_options = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(10))
            .idle_timeout(Duration::from_secs(600));
        
        match pool_options.connect(database_public_url).await {
            Ok(pool) => {
                tracing::info!("✓ Successfully connected to PostgreSQL");
                return Ok(pool);
            }
            Err(e) if retry_count < MAX_RETRIES => {
                retry_count += 1;
                tracing::warn!(
                    "Failed to connect to PostgreSQL (attempt {}/{}): {}. Retrying in {:?}...", 
                    retry_count, MAX_RETRIES, e, delay
                );
                tokio::time::sleep(delay).await;
                // Exponential backoff with max 8 seconds
                delay = std::cmp::min(delay * 2, Duration::from_secs(8));
            }
            Err(e) => {
                return Err(e).context(format!(
                    "Failed to connect to PostgreSQL after {} attempts. Check: 1) Database URL is correct, 2) Credentials are valid, 3) Network connectivity, 4) PostgreSQL service is running", 
                    MAX_RETRIES
                ));
            }
        }
    }
}

/// Initialize database schema (create tables if not exist)
pub async fn initialize_schema(pool: &PgPool) -> Result<()> {
    // Create parent_episodes table (conversation sessions)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS parent_episodes (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id TEXT NOT NULL,
            summary TEXT,
            turn_count INT NOT NULL DEFAULT 0,
            is_active BOOLEAN NOT NULL DEFAULT TRUE,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            last_updated TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#
    )
    .execute(pool)
    .await?;

    // Create sub_chunks table (individual turns)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS sub_chunks (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            parent_id UUID NOT NULL REFERENCES parent_episodes(id) ON DELETE CASCADE,
            user_id TEXT NOT NULL,
            user_text TEXT NOT NULL,
            reply_text TEXT NOT NULL,
            turn_index INT NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#
    )
    .execute(pool)
    .await?;

    // Create indexes
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_parent_episodes_user_id 
        ON parent_episodes(user_id, is_active, last_updated DESC)
        "#
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_sub_chunks_parent_id 
        ON sub_chunks(parent_id, turn_index)
        "#
    )
    .execute(pool)
    .await?;

    tracing::info!("PostgreSQL schema initialized");
    Ok(())
}

/// Get or create an active session for the user
/// A session is considered active if updated within the last 10 minutes
pub async fn get_or_create_active_session(
    pool: &PgPool,
    user_id: &str,
) -> Result<Uuid> {
    // Try to find an active session updated within last 10 minutes
    let session: Option<(Uuid,)> = sqlx::query_as(
        r#"
        SELECT id
        FROM parent_episodes
        WHERE user_id = $1 
          AND is_active = TRUE 
          AND last_updated > NOW() - INTERVAL '10 minutes'
        ORDER BY last_updated DESC
        LIMIT 1
        "#
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    if let Some((session_id,)) = session {
        tracing::debug!("Found active session: {}", session_id);
        return Ok(session_id);
    }

    // Create new session
    let row: (Uuid,) = sqlx::query_as(
        r#"
        INSERT INTO parent_episodes (user_id, is_active)
        VALUES ($1, TRUE)
        RETURNING id
        "#
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    tracing::info!("Created new session: {}", row.0);
    Ok(row.0)
}

/// Add a turn to an active session
pub async fn add_turn_to_session(
    pool: &PgPool,
    parent_id: &Uuid,
    user_id: &str,
    user_text: &str,
    reply_text: &str,
) -> Result<Uuid> {
    // Get current turn count
    let turn_count: (i32,) = sqlx::query_as(
        "SELECT turn_count FROM parent_episodes WHERE id = $1"
    )
    .bind(parent_id)
    .fetch_one(pool)
    .await?;

    // Insert sub chunk
    let sub_chunk_id: (Uuid,) = sqlx::query_as(
        r#"
        INSERT INTO sub_chunks (parent_id, user_id, user_text, reply_text, turn_index)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id
        "#
    )
    .bind(parent_id)
    .bind(user_id)
    .bind(user_text)
    .bind(reply_text)
    .bind(turn_count.0)
    .fetch_one(pool)
    .await?;

    // Update parent episode
    sqlx::query(
        r#"
        UPDATE parent_episodes
        SET turn_count = turn_count + 1,
            last_updated = CURRENT_TIMESTAMP
        WHERE id = $1
        "#
    )
    .bind(parent_id)
    .execute(pool)
    .await?;

    tracing::debug!("Added turn {} to session {}", turn_count.0, parent_id);
    Ok(sub_chunk_id.0)
}

/// Fetch full session content (all turns combined)
pub async fn fetch_session_content(
    pool: &PgPool,
    parent_ids: &[Uuid],
) -> Result<Vec<SessionContent>> {
    if parent_ids.is_empty() {
        return Ok(Vec::new());
    }

    let sessions: Vec<SessionContent> = sqlx::query_as(
        r#"
        SELECT
            pe.turn_count,
            COALESCE(
                STRING_AGG(
                    '[' || TO_CHAR(sc.created_at, 'YYYY-MM-DD HH24:MI:SS') || '] ' ||
                    'ユーザー: ' || sc.user_text || E'\n' ||
                    'Mirror: ' || sc.reply_text,
                    E'\n\n'
                    ORDER BY sc.turn_index
                ),
                ''
            ) as content
        FROM parent_episodes pe
        LEFT JOIN sub_chunks sc ON pe.id = sc.parent_id
        WHERE pe.id = ANY($1)
        GROUP BY pe.id, pe.turn_count, pe.created_at
        ORDER BY pe.created_at DESC
        "#
    )
    .bind(parent_ids)
    .fetch_all(pool)
    .await?;

    tracing::debug!("Fetched {} session contents", sessions.len());
    Ok(sessions)
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SessionContent {
    pub turn_count: i32,
    pub content: String,
}

/// Delete episodes by parent_ids (for cleanup)
/// Returns the number of parent_episodes deleted
pub async fn delete_episodes_by_parent_ids(
    pool: &PgPool,
    parent_ids: &[Uuid],
) -> Result<u64> {
    if parent_ids.is_empty() {
        return Ok(0);
    }

    // Delete parent_episodes (sub_chunks will be cascade deleted)
    let deleted = sqlx::query(
        "DELETE FROM parent_episodes WHERE id = ANY($1)"
    )
    .bind(parent_ids)
    .execute(pool)
    .await?
    .rows_affected();
    
    tracing::info!("Deleted {} parent_episodes from PostgreSQL", deleted);
    Ok(deleted)
}

