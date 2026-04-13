use anyhow::Result;
use neo4rs::{Graph, ConfigBuilder, query};
use crate::config::Config;
use crate::models::ExtractedMemory;

pub async fn create_client(config: &Config) -> Result<Graph> {
    let neo4j_config = ConfigBuilder::default()
        .uri(&config.neo4j_uri)
        .user(&config.neo4j_user)
        .password(&config.neo4j_password)
        .build()?;
    
    let graph = Graph::connect(neo4j_config).await?;
    
    tracing::info!("Connected to Neo4j at {}", config.neo4j_uri);
    
    Ok(graph)
}

/// Initialize the graph schema with constraints and indexes
pub async fn initialize_schema(graph: &Graph) -> Result<()> {
    use neo4rs::query;
    
    // Create constraints
    graph.run(
        query("CREATE CONSTRAINT user_id IF NOT EXISTS FOR (u:User) REQUIRE u.id IS UNIQUE")
    ).await?;
    
    graph.run(
        query("CREATE CONSTRAINT person_id IF NOT EXISTS FOR (p:Person) REQUIRE p.id IS UNIQUE")
    ).await?;
    
    graph.run(
        query("CREATE CONSTRAINT concept_name IF NOT EXISTS FOR (c:Concept) REQUIRE c.name IS UNIQUE")
    ).await?;
    
    tracing::info!("Neo4j schema initialized");
    
    Ok(())
}

/// Save extracted memory to Neo4j as a graph structure
pub async fn save_memory_graph(
    graph: &Graph,
    episode_id: &str,
    user_id: &str,
    user_text: &str,
    memory: &ExtractedMemory,
    timestamp: i64,
) -> Result<()> {
    // Create Episode node
    let episode_query = query(
        "MERGE (e:Episode {id: $episode_id})
         SET e.text = $text, e.timestamp = $timestamp, e.user_id = $user_id"
    )
    .param("episode_id", episode_id)
    .param("text", user_text)
    .param("timestamp", timestamp)
    .param("user_id", user_id);
    
    graph.run(episode_query).await?;
    
    // Create Person node and relationship if person is mentioned
    if let Some(person_name) = &memory.person_name {
        let person_query = query(
            "MERGE (p:Person {name: $name})
             WITH p
             MATCH (e:Episode {id: $episode_id})
             MERGE (e)-[:MENTIONS]->(p)"
        )
        .param("name", person_name.as_str())
        .param("episode_id", episode_id);
        
        graph.run(person_query).await?;
    }
    
    // Create emotion relationship
    if !memory.emotion_type.is_empty() {
        let emotion_query = query(
            "MATCH (e:Episode {id: $episode_id})
             MERGE (emotion:Emotion {type: $emotion_type})
             MERGE (e)-[r:FELT]->(emotion)
             SET r.intensity = $intensity, r.reason = $reason"
        )
        .param("episode_id", episode_id)
        .param("emotion_type", memory.emotion_type.as_str())
        .param("intensity", memory.intensity)
        .param("reason", memory.reason.as_str());
        
        graph.run(emotion_query).await?;
    }
    
    // Create Concept nodes and relationships
    for concept in &memory.concepts {
        let concept_query = query(
            "MERGE (c:Concept {name: $name})
             WITH c
             MATCH (e:Episode {id: $episode_id})
             MERGE (e)-[:RELATES_TO]->(c)"
        )
        .param("name", concept.as_str())
        .param("episode_id", episode_id);
        
        graph.run(concept_query).await?;
    }
    
    Ok(())
}
