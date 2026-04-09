use anyhow::{Context, Result};
use reqwest::Client;

use crate::config::LlmConfig;

// ── Prompts (faithful to mem0) ───────────────────────────────────────

const FACT_EXTRACTION_PROMPT: &str = r#"You are a Personal Information Organizer, specialized in accurately storing facts, user memories, and preferences. Your job is to extract distinct facts from the conversation.

Extract these types of information:
1. Personal Preferences (likes, dislikes: food, products, activities, entertainment)
2. Important Personal Details (names, relationships, important dates)
3. Plans and Intentions (upcoming events, trips, goals)
4. Activity/Service Preferences (dining, travel, hobbies)
5. Health/Wellness (dietary restrictions, fitness routines)
6. Professional Details (job titles, work habits, career goals)
7. Miscellaneous (favorite books, movies, brands)

IMPORTANT: Extract from BOTH user and assistant messages. Information the assistant provided to the user (recommendations, answers, facts shared) is also worth storing. Do NOT include greetings or generic statements.

Return a JSON array of strings, each being one distinct fact.
If no facts can be extracted, return: []

Examples:
- "Hi." → []
- "There are branches in trees." → []
- "I am looking for a restaurant in San Francisco." → ["Looking for a restaurant in San Francisco"]
- "My name is John. I am a software engineer." → ["Name is John", "Is a software engineer"]
- "Yesterday I had a meeting with John at 3pm about the new project." → ["Had a meeting with John at 3pm", "Discussed the new project with John"]

Respond with ONLY the JSON array."#;

const DEDUP_PROMPT: &str = r#"You are a smart memory manager which controls the memory of a system.

You will be given existing memories and new facts. For each new fact, decide what to do:

1. ADD: New fact not covered by any existing memory. Use a new integer ID.
2. UPDATE: Existing memory should be updated with richer/more specific info. Keep the same ID, provide new text.
   - "Likes cheese pizza" + "Loves cheese pizza" → NO update (same semantic meaning)
   - "Likes cricket" + "Loves playing cricket with friends" → UPDATE (more specific)
3. DELETE: New fact CONTRADICTS an existing memory. Remove the old memory.
   - "Loves cheese pizza" + "Dislikes cheese pizza" → DELETE the old one
4. NONE: Fact is already fully covered. No action needed.

Existing memories:
{existing}

New facts:
{new_facts}

Respond with ONLY a JSON object:
{"memory": [{"id": "<integer_id_or_new>", "text": "<content>", "event": "ADD|UPDATE|DELETE|NONE", "old_memory": "<only for UPDATE>"}]}"#;

const ENTITY_EXTRACTION_PROMPT: &str = r#"You are a smart assistant who understands entities and their types.
If the text contains self-references such as 'I', 'me', 'my', 'myself', use "{user_id}" as the entity name.
Extract all entities from the text with their types.

Return a JSON array: [{"entity": "name", "entity_type": "type"}]
Types: person, place, organization, product, event, concept, other

Respond with ONLY the JSON array."#;

const RELATION_EXTRACTION_PROMPT: &str = r#"You are a smart assistant who extracts relationships between entities.
Given a list of entities and the source text, extract relationships as triples.
If the text uses 'I', 'me', 'my', replace with "{user_id}".

Entities: {entities}
Text: {text}

Return a JSON array: [{"source": "entity1", "relation": "relationship", "destination": "entity2"}]
Use simple lowercase relation names like: lives_in, works_at, likes, knows, born_in, married_to, etc.

Respond with ONLY the JSON array."#;

// ── Fact extraction ──────────────────────────────────────────────────

pub async fn extract_facts(config: &LlmConfig, text: &str) -> Result<Vec<String>> {
    let response = llm_call(config, FACT_EXTRACTION_PROMPT, text).await?;
    parse_json_array(&response)
}

// ── Deduplication ────────────────────────────────────────────────────

#[derive(Debug)]
pub struct DeduplicatedFact {
    pub fact: String,
    pub action: FactAction,
    pub existing_id: Option<String>,
}

#[derive(Debug)]
pub enum FactAction {
    Add,
    Update,
    Delete,
    None,
}

pub async fn deduplicate(
    config: &LlmConfig,
    existing: &[(String, String)], // (integer_id, text)
    new_facts: &[String],
) -> Result<Vec<DeduplicatedFact>> {
    if new_facts.is_empty() {
        return Ok(Vec::new());
    }

    if existing.is_empty() {
        return Ok(new_facts
            .iter()
            .map(|f| DeduplicatedFact {
                fact: f.clone(),
                action: FactAction::Add,
                existing_id: None,
            })
            .collect());
    }

    let existing_str = existing
        .iter()
        .map(|(id, text)| format!("[{id}] {text}"))
        .collect::<Vec<_>>()
        .join("\n");

    let new_str = new_facts
        .iter()
        .enumerate()
        .map(|(i, f)| format!("{}. {f}", i + 1))
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = DEDUP_PROMPT
        .replace("{existing}", &existing_str)
        .replace("{new_facts}", &new_str);

    let response = llm_call(config, &prompt, "Deduplicate these memories.").await?;

    // Parse the {"memory": [...]} response
    let parsed: serde_json::Value = serde_json::from_str(&response)
        .or_else(|_| {
            // Try to find JSON in response
            if let Some(start) = response.find('{') {
                if let Some(end) = response.rfind('}') {
                    return serde_json::from_str(&response[start..=end]);
                }
            }
            Ok(serde_json::json!({"memory": []}))
        })
        .unwrap_or(serde_json::json!({"memory": []}));

    let memory_arr = parsed
        .get("memory")
        .and_then(|m| m.as_array())
        .cloned()
        .unwrap_or_default();

    let mut results = Vec::new();
    for item in memory_arr {
        let fact = item
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let event = item
            .get("event")
            .and_then(|v| v.as_str())
            .unwrap_or("ADD")
            .to_uppercase();
        let id = item
            .get("id")
            .and_then(|v| {
                v.as_str()
                    .map(String::from)
                    .or_else(|| v.as_u64().map(|n| n.to_string()))
            });

        let action = match event.as_str() {
            "UPDATE" => FactAction::Update,
            "DELETE" => FactAction::Delete,
            "NONE" => FactAction::None,
            _ => FactAction::Add,
        };

        if !fact.is_empty() || matches!(action, FactAction::Delete) {
            results.push(DeduplicatedFact {
                fact,
                action,
                existing_id: id,
            });
        }
    }

    // Fallback: if parsing failed entirely, treat all as ADD
    if results.is_empty() && !new_facts.is_empty() {
        results = new_facts
            .iter()
            .map(|f| DeduplicatedFact {
                fact: f.clone(),
                action: FactAction::Add,
                existing_id: None,
            })
            .collect();
    }

    Ok(results)
}

// ── Entity extraction (for graph) ────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Entity {
    pub name: String,
    pub entity_type: String,
}

pub async fn extract_entities(
    config: &LlmConfig,
    text: &str,
    user_id: &str,
) -> Result<Vec<Entity>> {
    let prompt = ENTITY_EXTRACTION_PROMPT.replace("{user_id}", user_id);
    let response = llm_call(config, &prompt, text).await?;

    let parsed: Vec<serde_json::Value> = parse_json_value_array(&response)?;

    let entities: Vec<Entity> = parsed
        .into_iter()
        .filter_map(|item| {
            let name = item.get("entity")?.as_str()?.to_lowercase();
            let etype = item
                .get("entity_type")
                .and_then(|v| v.as_str())
                .unwrap_or("other")
                .to_lowercase();

            // Self-reference resolution: replace I/me/my with user_id
            let resolved = match name.as_str() {
                "i" | "me" | "my" | "myself" | "我" | "我的" => user_id.to_lowercase(),
                _ => name,
            };

            Some(Entity {
                name: resolved,
                entity_type: etype,
            })
        })
        .collect();

    Ok(entities)
}

// ── Relation extraction (for graph) ──────────────────────────────────

#[derive(Debug, Clone)]
pub struct ExtractedRelation {
    pub source: String,
    pub relation: String,
    pub destination: String,
}

pub async fn extract_relations(
    config: &LlmConfig,
    text: &str,
    entities: &[Entity],
) -> Result<Vec<ExtractedRelation>> {
    let entity_names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();

    let prompt = RELATION_EXTRACTION_PROMPT
        .replace("{entities}", &format!("{:?}", entity_names))
        .replace("{text}", text);

    let response = llm_call(config, &prompt, "Extract relations.").await?;

    let parsed: Vec<serde_json::Value> = parse_json_value_array(&response)?;

    let relations: Vec<ExtractedRelation> = parsed
        .into_iter()
        .filter_map(|item| {
            let source = item.get("source")?.as_str()?.to_lowercase();
            let relation = item
                .get("relation")?
                .as_str()?
                .to_lowercase()
                .replace(' ', "_");
            let destination = item.get("destination")?.as_str()?.to_lowercase();

            if source.is_empty() || destination.is_empty() || relation.is_empty() {
                return None;
            }

            Some(ExtractedRelation {
                source,
                relation,
                destination,
            })
        })
        .collect();

    Ok(relations)
}

// ── LLM call helper ──────────────────────────────────────────────────

async fn llm_call(config: &LlmConfig, system: &str, user: &str) -> Result<String> {
    let client = Client::new();

    let is_anthropic = config.provider.as_str() == "anthropic";

    let base = if config.base_url.is_empty() {
        match config.provider.as_str() {
            "openai" => "https://api.openai.com",
            "anthropic" => "https://api.anthropic.com",
            _ => "http://127.0.0.1:11434",
        }
    } else {
        config.base_url.trim_end_matches('/')
    };

    let model = if config.model.is_empty() {
        if is_anthropic { "claude-sonnet-4-6" } else { "qwen2.5:32b" }
    } else {
        &config.model
    };

    if is_anthropic {
        // Anthropic Messages API (native)
        let url = format!("{base}/v1/messages");
        let body = serde_json::json!({
            "model": model,
            "max_tokens": 4096,
            "system": system,
            "messages": [
                {"role": "user", "content": user}
            ],
            "temperature": 0.1,
        });

        let mut req = client.post(&url)
            .header("content-type", "application/json")
            .header("anthropic-version", "2023-06-01");
        if !config.api_key.is_empty() {
            req = req.header("x-api-key", &config.api_key);
        }

        let resp = req.json(&body).send().await.context("Anthropic API request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Anthropic API error {status}: {text}");
        }

        let data: serde_json::Value = resp.json().await?;
        let content = data
            .get("content")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();

        Ok(content)
    } else {
        // OpenAI-compatible API (OpenAI, Ollama, etc.)
        let url = format!("{base}/v1/chat/completions");
        let body = serde_json::json!({
            "model": model,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": user}
            ],
            "temperature": 0.1,
        });

        let mut req = client.post(&url).header("content-type", "application/json");
        if !config.api_key.is_empty() {
            req = req.header("authorization", format!("Bearer {}", config.api_key));
        }

        let resp = req.json(&body).send().await.context("LLM request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("LLM API error {status}: {text}");
        }

        let data: serde_json::Value = resp.json().await?;
        let content = data
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();

        Ok(content)
    }
}

// ── JSON parsing helpers ─────────────────────────────────────────────

fn parse_json_array(s: &str) -> Result<Vec<String>> {
    // Try direct parse
    if let Ok(arr) = serde_json::from_str::<Vec<String>>(s) {
        return Ok(arr);
    }
    // Try to find array in response
    if let Some(start) = s.find('[') {
        if let Some(end) = s.rfind(']') {
            if let Ok(arr) = serde_json::from_str::<Vec<String>>(&s[start..=end]) {
                return Ok(arr);
            }
        }
    }
    // Try {"facts": [...]} format
    if let Ok(obj) = serde_json::from_str::<serde_json::Value>(s) {
        if let Some(facts) = obj.get("facts").and_then(|f| f.as_array()) {
            return Ok(facts
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect());
        }
    }
    Ok(Vec::new())
}

fn parse_json_value_array(s: &str) -> Result<Vec<serde_json::Value>> {
    if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(s) {
        return Ok(arr);
    }
    if let Some(start) = s.find('[') {
        if let Some(end) = s.rfind(']') {
            if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&s[start..=end]) {
                return Ok(arr);
            }
        }
    }
    Ok(Vec::new())
}
