use anyhow::Result;
use neo4rs::{Graph, query};
use serde::{Serialize, Deserialize};

/// Future use: Person detail view
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct PersonContext {
    pub name: String,
    pub mention_count: i64,
    pub related_core_values: Vec<String>,
    pub emotions: Vec<String>,
}

/// Get context about a specific person from Neo4j
/// Future use: Person detail API endpoint
#[allow(dead_code)]
pub async fn get_person_context(graph: &Graph, person_name: &str) -> Result<Option<PersonContext>> {
    // Find person and their related data through Episodes
    let query_str = query(
        "MATCH (p:Person {name: $name})
         OPTIONAL MATCH (p)-[:RELATED_TO]->(e:Episode)
         OPTIONAL MATCH (e)-[:HOLDS]->(cv:CoreValue)
         OPTIONAL MATCH (e)-[:FELT]->(em:Emotion)
         RETURN p.name as name, 
                count(DISTINCT e) as mention_count,
                collect(DISTINCT cv.name) as core_values,
                collect(DISTINCT em.type) as emotions"
    ).param("name", person_name);
    
    let mut result = graph.execute(query_str).await?;
    
    if let Ok(Some(row)) = result.next().await {
        if let (Ok(name), Ok(count)) = (
            row.get::<String>("name"),
            row.get::<i64>("mention_count")
        ) {
            let core_values = row.get::<Vec<String>>("core_values")
                .unwrap_or_default()
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect();
            
            let emotions = row.get::<Vec<String>>("emotions")
                .unwrap_or_default()
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect();
            
            return Ok(Some(PersonContext {
                name,
                mention_count: count,
                related_core_values: core_values,
                emotions,
            }));
        }
    }
    
    Ok(None)
}

/// Extract person names from user text
/// Note: This is a simple pattern-based extraction.
/// In production, use the LLM's entity extraction capability.
pub fn extract_person_names(text: &str) -> Vec<String> {
    let mut persons = Vec::new();
    
    // Common Japanese name patterns
    let name_patterns = ["さん", "くん", "ちゃん", "先生", "社長"];
    for word in text.split_whitespace() {
        for pattern in &name_patterns {
            if word.contains(pattern) {
                // Avoid duplicates
                if !persons.contains(&word.to_string()) {
                    persons.push(word.to_string());
                }
            }
        }
    }
    
    persons
}
