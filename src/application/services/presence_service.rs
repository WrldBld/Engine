//! NPC Presence Query Service (Phase 23C)
//!
//! Determines which NPCs are present in a region based on:
//! - NPC-Region relationships (works_at, frequents, home, avoids)
//! - Current game time (time of day)
//! - Story context
//!
//! Uses LLM reasoning to make natural decisions about NPC presence,
//! with caching based on game time to avoid redundant queries.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::application::ports::outbound::{ChatMessage, LlmPort, LlmRequest, RegionRepositoryPort};
use crate::domain::entities::Character;
use crate::domain::value_objects::{
    GameTime, RegionId, RegionRelationshipType, TimeOfDay,
};

/// Result of an NPC presence query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcPresenceResult {
    /// The NPC's character ID
    pub character_id: String,
    /// The NPC's name
    pub name: String,
    /// Whether the NPC is present in the region
    pub is_present: bool,
    /// The LLM's reasoning for the decision
    pub reasoning: String,
    /// The NPC's sprite asset (if present)
    pub sprite_asset: Option<String>,
}

/// Cache entry for presence query results
#[derive(Debug, Clone)]
struct PresenceCacheEntry {
    /// The presence results
    pub results: Vec<NpcPresenceResult>,
    /// Game time when this was cached
    pub cached_at_game_time: DateTime<Utc>,
    /// TTL in game hours
    pub ttl_game_hours: u32,
}

/// Configuration for the presence service
#[derive(Debug, Clone)]
pub struct PresenceServiceConfig {
    /// Default cache TTL in game hours
    pub default_ttl_hours: u32,
    /// Temperature for LLM queries (lower = more deterministic)
    pub llm_temperature: f32,
    /// Whether to use LLM for presence decisions (false = use simple rules)
    pub use_llm: bool,
}

impl Default for PresenceServiceConfig {
    fn default() -> Self {
        Self {
            default_ttl_hours: 1,
            llm_temperature: 0.3,
            use_llm: true,
        }
    }
}

/// Service for querying NPC presence in regions
pub struct PresenceService<L: LlmPort, R: RegionRepositoryPort> {
    region_repository: Arc<R>,
    llm_port: Arc<L>,
    config: PresenceServiceConfig,
    /// Cache: region_id -> cache entry
    cache: RwLock<HashMap<RegionId, PresenceCacheEntry>>,
}

impl<L: LlmPort, R: RegionRepositoryPort> PresenceService<L, R> {
    pub fn new(region_repository: Arc<R>, llm_port: Arc<L>) -> Self {
        Self {
            region_repository,
            llm_port,
            config: PresenceServiceConfig::default(),
            cache: RwLock::new(HashMap::new()),
        }
    }

    pub fn with_config(mut self, config: PresenceServiceConfig) -> Self {
        self.config = config;
        self
    }

    /// Query which NPCs are present in a region at the given game time
    pub async fn query_presence(
        &self,
        region_id: RegionId,
        game_time: &GameTime,
    ) -> Result<Vec<NpcPresenceResult>> {
        // Check cache first
        if let Some(cached) = self.get_cached(&region_id, game_time).await {
            tracing::debug!("Using cached presence for region {}", region_id);
            return Ok(cached);
        }

        // Get region info
        let region = self
            .region_repository
            .get(region_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Region not found: {}", region_id))?;

        // Get all NPCs with relationships to this region
        let npc_relationships = self
            .region_repository
            .get_npcs_related_to_region(region_id)
            .await?;

        if npc_relationships.is_empty() {
            // No NPCs have relationships to this region
            return Ok(vec![]);
        }

        let time_of_day = game_time.time_of_day();

        // Determine presence for each NPC
        let results = if self.config.use_llm {
            self.determine_presence_with_llm(&region.name, &npc_relationships, time_of_day)
                .await?
        } else {
            self.determine_presence_simple(&npc_relationships, time_of_day)
        };

        // Cache the results
        self.cache_results(region_id, &results, game_time).await;

        Ok(results)
    }

    /// Simple rule-based presence determination (no LLM)
    /// 
    /// Uses the canonical presence rules from RegionRelationshipType.
    fn determine_presence_simple(
        &self,
        npc_relationships: &[(Character, RegionRelationshipType)],
        time_of_day: TimeOfDay,
    ) -> Vec<NpcPresenceResult> {
        npc_relationships
            .iter()
            .map(|(character, rel_type)| {
                // Use canonical domain logic
                let is_present = rel_type.is_npc_present(time_of_day);
                let reasoning = rel_type.presence_reasoning(time_of_day);

                NpcPresenceResult {
                    character_id: character.id.to_string(),
                    name: character.name.clone(),
                    is_present,
                    reasoning,
                    sprite_asset: character.sprite_asset.clone(),
                }
            })
            .collect()
    }

    /// LLM-based presence determination
    async fn determine_presence_with_llm(
        &self,
        region_name: &str,
        npc_relationships: &[(Character, RegionRelationshipType)],
        time_of_day: TimeOfDay,
    ) -> Result<Vec<NpcPresenceResult>> {
        // Build the prompt
        let system_prompt = self.build_presence_system_prompt();
        let user_prompt = self.build_presence_user_prompt(region_name, npc_relationships, time_of_day);

        let request = LlmRequest::new(vec![ChatMessage::user(user_prompt)])
            .with_system_prompt(system_prompt)
            .with_temperature(self.config.llm_temperature);

        // Query the LLM
        let response = self.llm_port.generate(request).await
            .map_err(|e| anyhow::anyhow!("LLM query failed: {}", e))?;

        // Parse the response
        self.parse_presence_response(&response.content, npc_relationships)
    }

    fn build_presence_system_prompt(&self) -> String {
        r#"You are a game master assistant helping determine which NPCs are present in a location.

Given information about NPCs and their relationships to a location, decide who would realistically be there at the current time.

Consider:
- Work schedules (day shift, night shift, always present)
- Home locations (people are usually home at night)
- Frequenting patterns (regulars vs occasional visitors)
- Avoidance (NPCs who avoid a location won't be there)
- Time of day context

Respond in JSON format with an array of objects:
[
  {
    "name": "NPC Name",
    "is_present": true/false,
    "reasoning": "Brief explanation"
  }
]

Be realistic and consistent. Don't have everyone present at once unless it makes sense."#.to_string()
    }

    fn build_presence_user_prompt(
        &self,
        region_name: &str,
        npc_relationships: &[(Character, RegionRelationshipType)],
        time_of_day: TimeOfDay,
    ) -> String {
        let mut prompt = format!(
            "Location: {}\nTime of Day: {} ({})\n\nNPCs with connections to this location:\n\n",
            region_name,
            time_of_day.display_name(),
            match time_of_day {
                TimeOfDay::Morning => "6 AM - 12 PM",
                TimeOfDay::Afternoon => "12 PM - 6 PM",
                TimeOfDay::Evening => "6 PM - 10 PM",
                TimeOfDay::Night => "10 PM - 6 AM",
            }
        );

        for (character, rel_type) in npc_relationships {
            let relationship_desc = match rel_type {
                RegionRelationshipType::Home => "Lives here".to_string(),
                RegionRelationshipType::WorksAt { shift } => format!("Works here ({:?} shift)", shift),
                RegionRelationshipType::Frequents { frequency } => format!("Frequents here ({:?})", frequency),
                RegionRelationshipType::Avoids { reason } => format!("Avoids this place: {}", reason),
            };

            prompt.push_str(&format!(
                "- {} ({}): {}\n",
                character.name,
                character.description,
                relationship_desc
            ));
        }

        prompt.push_str("\nWho is present at this location right now? Respond in JSON format.");
        prompt
    }

    fn parse_presence_response(
        &self,
        response: &str,
        npc_relationships: &[(Character, RegionRelationshipType)],
    ) -> Result<Vec<NpcPresenceResult>> {
        // Try to extract JSON from the response
        let json_str = extract_json_array(response)
            .ok_or_else(|| anyhow::anyhow!("Could not parse LLM response as JSON"))?;

        #[derive(Deserialize)]
        struct LlmPresenceResult {
            name: String,
            is_present: bool,
            reasoning: String,
        }

        let llm_results: Vec<LlmPresenceResult> = serde_json::from_str(&json_str)?;

        // Map LLM results back to full NpcPresenceResult with character data
        let mut results = Vec::new();
        for (character, _rel_type) in npc_relationships {
            let llm_result = llm_results
                .iter()
                .find(|r| r.name.to_lowercase() == character.name.to_lowercase());

            let (is_present, reasoning) = if let Some(r) = llm_result {
                (r.is_present, r.reasoning.clone())
            } else {
                // Default to not present if LLM didn't mention this NPC
                (false, "Not mentioned in scene".to_string())
            };

            results.push(NpcPresenceResult {
                character_id: character.id.to_string(),
                name: character.name.clone(),
                is_present,
                reasoning,
                sprite_asset: character.sprite_asset.clone(),
            });
        }

        Ok(results)
    }

    /// Check if we have a valid cached result
    async fn get_cached(
        &self,
        region_id: &RegionId,
        game_time: &GameTime,
    ) -> Option<Vec<NpcPresenceResult>> {
        let cache = self.cache.read().await;
        if let Some(entry) = cache.get(region_id) {
            // Check if cache is still valid based on game time
            let hours_elapsed = game_time.hours_since(&entry.cached_at_game_time);
            if hours_elapsed < entry.ttl_game_hours as f64 {
                return Some(entry.results.clone());
            }
        }
        None
    }

    /// Cache presence results
    async fn cache_results(
        &self,
        region_id: RegionId,
        results: &[NpcPresenceResult],
        game_time: &GameTime,
    ) {
        let entry = PresenceCacheEntry {
            results: results.to_vec(),
            cached_at_game_time: game_time.current(),
            ttl_game_hours: self.config.default_ttl_hours,
        };
        let mut cache = self.cache.write().await;
        cache.insert(region_id, entry);
    }

    /// Invalidate cache for a region (e.g., when time advances significantly)
    pub async fn invalidate_cache(&self, region_id: &RegionId) {
        let mut cache = self.cache.write().await;
        cache.remove(region_id);
    }

    /// Invalidate all cached presence data
    pub async fn invalidate_all_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Force an NPC to be present (for DM events like approach)
    /// Returns updated presence results
    pub async fn force_npc_present(
        &self,
        region_id: RegionId,
        character: &Character,
        game_time: &GameTime,
    ) -> Result<Vec<NpcPresenceResult>> {
        let mut cache = self.cache.write().await;
        
        if let Some(entry) = cache.get_mut(&region_id) {
            // Check if NPC is already in the results
            let existing = entry.results.iter_mut().find(|r| r.character_id == character.id.to_string());
            
            if let Some(result) = existing {
                result.is_present = true;
                result.reasoning = "Arrived via DM event".to_string();
            } else {
                // Add the NPC to the results
                entry.results.push(NpcPresenceResult {
                    character_id: character.id.to_string(),
                    name: character.name.clone(),
                    is_present: true,
                    reasoning: "Arrived via DM event".to_string(),
                    sprite_asset: character.sprite_asset.clone(),
                });
            }
            
            return Ok(entry.results.clone());
        }
        
        // No cache entry exists, create one with just this NPC
        let results = vec![NpcPresenceResult {
            character_id: character.id.to_string(),
            name: character.name.clone(),
            is_present: true,
            reasoning: "Arrived via DM event".to_string(),
            sprite_asset: character.sprite_asset.clone(),
        }];
        
        let entry = PresenceCacheEntry {
            results: results.clone(),
            cached_at_game_time: game_time.current(),
            ttl_game_hours: self.config.default_ttl_hours,
        };
        cache.insert(region_id, entry);
        
        Ok(results)
    }
}

/// Extract a JSON array from a potentially mixed response
fn extract_json_array(text: &str) -> Option<String> {
    // Try to find JSON array in the response
    let start = text.find('[')?;
    let end = text.rfind(']')?;
    if end > start {
        Some(text[start..=end].to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_array() {
        let response = r#"Here are the results:
[
  {"name": "Bob", "is_present": true, "reasoning": "Works here"}
]
That's all!"#;
        
        let json = extract_json_array(response).unwrap();
        assert!(json.starts_with('['));
        assert!(json.ends_with(']'));
        
        // Verify it's valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_array());
    }

    #[test]
    fn test_extract_json_array_no_match() {
        let response = "No JSON here";
        assert!(extract_json_array(response).is_none());
    }
}
