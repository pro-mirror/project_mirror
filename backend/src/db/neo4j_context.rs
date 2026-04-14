use anyhow::Result;
use neo4rs::{Graph, query};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct PersonContext {
    pub name: String,
    pub mention_count: i64,
    pub related_concepts: Vec<String>,
    pub emotions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConceptContext {
    pub name: String,
    pub frequency: i64,
    pub related_emotions: Vec<String>,
}

/// Get context about a specific person from Neo4j
pub async fn get_person_context(graph: &Graph, person_name: &str) -> Result<Option<PersonContext>> {
    // Find person and their related data
    let query_str = query(
        "MATCH (p:Person {name: $name})
         OPTIONAL MATCH (e:Episode)-[:MENTIONS]->(p)
         OPTIONAL MATCH (e)-[:RELATES_TO]->(c:Concept)
         OPTIONAL MATCH (e)-[:FELT]->(em:Emotion)
         RETURN p.name as name, 
                count(DISTINCT e) as mention_count,
                collect(DISTINCT c.name) as concepts,
                collect(DISTINCT em.type) as emotions"
    ).param("name", person_name);
    
    let mut result = graph.execute(query_str).await?;
    
    if let Ok(Some(row)) = result.next().await {
        if let (Ok(name), Ok(count)) = (
            row.get::<String>("name"),
            row.get::<i64>("mention_count")
        ) {
            let concepts = row.get::<Vec<String>>("concepts")
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
                related_concepts: concepts,
                emotions,
            }));
        }
    }
    
    Ok(None)
}

/// Get context about a specific concept from Neo4j
pub async fn get_concept_context(graph: &Graph, concept_name: &str) -> Result<Option<ConceptContext>> {
    let query_str = query(
        "MATCH (c:Concept {name: $name})
         OPTIONAL MATCH (e:Episode)-[:RELATES_TO]->(c)
         OPTIONAL MATCH (e)-[:FELT]->(em:Emotion)
         RETURN c.name as name,
                count(DISTINCT e) as frequency,
                collect(DISTINCT em.type) as emotions"
    ).param("name", concept_name);
    
    let mut result = graph.execute(query_str).await?;
    
    if let Ok(Some(row)) = result.next().await {
        if let (Ok(name), Ok(freq)) = (
            row.get::<String>("name"),
            row.get::<i64>("frequency")
        ) {
            let emotions = row.get::<Vec<String>>("emotions")
                .unwrap_or_default()
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect();
            
            return Ok(Some(ConceptContext {
                name,
                frequency: freq,
                related_emotions: emotions,
            }));
        }
    }
    
    Ok(None)
}

/// Extract person names and concepts from user text
pub fn extract_entities(text: &str) -> (Vec<String>, Vec<String>) {
    // Simple extraction - look for common patterns
    // In production, use NLP or the LLM's entity extraction
    let mut persons = Vec::new();
    let mut concepts = Vec::new();
    
    // Common Japanese name patterns
    let name_patterns = ["さん", "くん", "ちゃん", "先生", "社長"];
    for word in text.split_whitespace() {
        for pattern in &name_patterns {
            if word.contains(pattern) {
                persons.push(word.to_string());
            }
        }
    }
    
    // Common concept keywords
    let concept_keywords = [
        "仕事", "家族", "友達", "趣味", "散歩", "公園", 
        "犬", "猫", "旅行", "食事", "料理", "映画"
    ];
    for keyword in &concept_keywords {
        if text.contains(keyword) {
            concepts.push(keyword.to_string());
        }
    }
    
    (persons, concepts)
}
