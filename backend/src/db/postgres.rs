use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

/// Create PostgreSQL connection pool
pub async fn create_pool(database_public_url: &str) -> Result<PgPool> {
    let pool = PgPool::connect(database_public_url).await?;
    tracing::info!("Connected to PostgreSQL");
    Ok(pool)
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
            pe.id,
            pe.user_id,
            pe.turn_count,
            pe.created_at,
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
        GROUP BY pe.id, pe.user_id, pe.turn_count, pe.created_at
        ORDER BY pe.created_at DESC
        "#
    )
    .bind(parent_ids)
    .fetch_all(pool)
    .await?;

    tracing::debug!("Fetched {} session contents", sessions.len());
    Ok(sessions)
}

/// Fetch sub chunks by parent_id
pub async fn fetch_sub_chunks(
    pool: &PgPool,
    parent_id: &Uuid,
) -> Result<Vec<SubChunk>> {
    let chunks: Vec<SubChunk> = sqlx::query_as(
        r#"
        SELECT id, parent_id, user_id, user_text, reply_text, turn_index, created_at
        FROM sub_chunks
        WHERE parent_id = $1
        ORDER BY turn_index ASC
        "#
    )
    .bind(parent_id)
    .fetch_all(pool)
    .await?;

    Ok(chunks)
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SubChunk {
    pub id: Uuid,
    pub parent_id: Uuid,
    pub user_id: String,
    pub user_text: String,
    pub reply_text: String,
    pub turn_index: i32,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SessionContent {
    pub id: Uuid,
    pub user_id: String,
    pub turn_count: i32,
    pub created_at: chrono::NaiveDateTime,
    pub content: String,
}

