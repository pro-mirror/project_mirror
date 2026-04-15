use anyhow::Result;
use neo4rs::{Graph, ConfigBuilder, query};
use crate::config::Config;
use crate::models::CoreValueExtraction;
use uuid::Uuid;

pub async fn create_client(config: &Config) -> Result<Graph> {
    let neo4j_config = ConfigBuilder::default()
        .uri(&config.neo4j_uri)
        .user(&config.neo4j_user)
        .password(&config.neo4j_password)
        .db(config.neo4j_database.as_str())
        .build()?;
    
    let graph = Graph::connect(neo4j_config).await?;
    
    tracing::info!("Connected to Neo4j at {} (database: {})", config.neo4j_uri, config.neo4j_database);
    
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
        query("CREATE CONSTRAINT episode_parent_id IF NOT EXISTS FOR (e:Episode) REQUIRE e.parent_id IS UNIQUE")
    ).await?;
    
    graph.run(
        query("CREATE CONSTRAINT person_name IF NOT EXISTS FOR (p:Person) REQUIRE p.name IS UNIQUE")
    ).await?;
    
    graph.run(
        query("CREATE CONSTRAINT core_value_name IF NOT EXISTS FOR (cv:CoreValue) REQUIRE cv.name IS UNIQUE")
    ).await?;
    
    tracing::info!("Neo4j schema initialized");
    
    Ok(())
}

/// New Architecture: Save core values to graph
/// Creates Episode-centric relationships:
/// User -[:HAS]-> Episode -[:HOLDS]-> CoreValue
/// Person -[:RELATED_TO]-> Episode (if related_person exists)
pub async fn save_core_values(
    graph: &Graph,
    user_id: &str,
    parent_id: &Uuid,
    values: &[CoreValueExtraction],
) -> Result<()> {
    // Ensure User node exists
    let user_query = query(
        "MERGE (u:User {id: $user_id})"
    )
    .param("user_id", user_id);
    graph.run(user_query).await?;
    
    // Create Episode node
    let episode_query = query(
        "MERGE (e:Episode {parent_id: $parent_id})
         ON CREATE SET e.created_at = datetime({timezone: 'Asia/Tokyo'})
         WITH e
         MATCH (u:User {id: $user_id})
         MERGE (u)-[r:HAS]->(e)
         ON CREATE SET r.created_at = datetime({timezone: 'Asia/Tokyo'})"
    )
    .param("parent_id", parent_id.to_string())
    .param("user_id", user_id);
    graph.run(episode_query).await?;
    
    for value in values {
        // Create CoreValue node with metadata
        let value_query = query(
            "MATCH (e:Episode {parent_id: $parent_id})
             MERGE (cv:CoreValue {name: $value_name})
             ON CREATE SET cv.first_discovered = datetime({timezone: 'Asia/Tokyo'}), cv.total_weight = 0.0
             MERGE (e)-[r:HOLDS]->(cv)
             ON CREATE SET r.weight = $weight, 
                           r.context = $context, 
                           r.created_at = datetime({timezone: 'Asia/Tokyo'})
             ON MATCH SET r.weight = r.weight + $weight, 
                          r.latest_context = $context, 
                          r.updated_at = datetime({timezone: 'Asia/Tokyo'})
             SET cv.total_weight = cv.total_weight + $weight,
                 cv.last_mentioned = datetime({timezone: 'Asia/Tokyo'})"
        )
        .param("parent_id", parent_id.to_string())
        .param("value_name", value.value_name.as_str())
        .param("weight", value.weight)
        .param("context", value.context.as_str());
        
        graph.run(value_query).await?;
        
        // If there's a related person, link them to the Episode
        if let Some(person_name) = &value.related_person {
            let person_query = query(
                "MATCH (e:Episode {parent_id: $parent_id})
                 MERGE (p:Person {name: $person_name})
                 ON CREATE SET p.first_mentioned = datetime({timezone: 'Asia/Tokyo'})
                 MERGE (p)-[r:RELATED_TO]->(e)
                 ON CREATE SET r.relationship_context = $context,
                               r.first_mentioned_at = datetime({timezone: 'Asia/Tokyo'}),
                               r.mention_count = 1
                 ON MATCH SET r.mention_count = r.mention_count + 1,
                              r.last_mentioned_at = datetime({timezone: 'Asia/Tokyo'})
                 SET p.last_mentioned = datetime({timezone: 'Asia/Tokyo'})"
            )
            .param("parent_id", parent_id.to_string())
            .param("person_name", person_name.as_str())
            .param("context", value.context.as_str());
            
            graph.run(person_query).await?;
        }
    }
    
    Ok(())
}

/// Fetch core values for dynamic prompt injection
/// Traverses: User -[:HAS]-> Episode -[:HOLDS]-> CoreValue
pub async fn fetch_user_core_values(
    graph: &Graph,
    user_id: &str,
    limit: i64,
) -> Result<Vec<(String, f64, String)>> {
    let query_str = query(
        "MATCH (u:User {id: $user_id})-[:HAS]->(e:Episode)-[r:HOLDS]->(cv:CoreValue)
         WITH cv, SUM(r.weight) as total_weight, r.latest_context as context
         RETURN cv.name as value_name, 
                cv.total_weight as total_weight, 
                COALESCE(context, cv.name) as context
         ORDER BY total_weight DESC
         LIMIT $limit"
    )
    .param("user_id", user_id)
    .param("limit", limit);
    
    let mut result = graph.execute(query_str).await?;
    let mut values = Vec::new();
    
    while let Some(row) = result.next().await? {
        let value_name: String = row.get("value_name").unwrap_or_default();
        let weight: f64 = row.get("total_weight").unwrap_or(0.0);
        let context: String = row.get("context").unwrap_or_default();
        values.push((value_name, weight, context));
    }
    
    Ok(values)
}

/// Fetch parent_ids related to specific entities (persons or core values)
/// Searches: Person -[:RELATED_TO]-> Episode OR Episode -[:HOLDS]-> CoreValue
pub async fn fetch_related_parent_ids(
    graph: &Graph,
    user_id: &str,
    entities: &[String],
) -> Result<Vec<Uuid>> {
    if entities.is_empty() {
        return Ok(Vec::new());
    }
    
    // Search for episodes related to persons or core values
    let query_str = query(
        "MATCH (u:User {id: $user_id})-[:HAS]->(e:Episode)
         WHERE EXISTS {
             MATCH (p:Person)-[:RELATED_TO]->(e)
             WHERE p.name IN $entities
         } OR EXISTS {
             MATCH (e)-[:HOLDS]->(cv:CoreValue)
             WHERE cv.name IN $entities
         }
         RETURN DISTINCT e.parent_id as parent_id"
    )
    .param("user_id", user_id)
    .param("entities", entities);
    
    let mut result = graph.execute(query_str).await?;
    let mut parent_ids = Vec::new();
    
    while let Some(row) = result.next().await? {
        if let Ok(parent_id_str) = row.get::<String>("parent_id") {
            if let Ok(parent_id) = Uuid::parse_str(&parent_id_str) {
                parent_ids.push(parent_id);
            }
        }
    }
    
    Ok(parent_ids)
}
