//! LLM Context Service - Builds rich context from graph relationships
//!
//! # Phase 1: LLM Context Enhancement
//!
//! This service builds comprehensive context for LLM prompts by traversing
//! the Neo4j graph rather than relying on embedded JSON fields. It respects
//! token budgets and can summarize content when over budget.

use std::sync::Arc;

use crate::application::ports::outbound::{
    ChallengeRepositoryPort, CharacterRepositoryPort, LocationRepositoryPort,
    NarrativeEventRepositoryPort, SceneRepositoryPort,
};
use crate::domain::value_objects::{
    ActiveChallengeContext, ActiveNarrativeEventContext, AssembledContext, CategoryContext,
    CharacterContext, ContextBudgetConfig, ContextCategory, SceneContext, TokenCounter,
    CharacterId, SceneId,
};

/// Service for building LLM context from graph relationships
pub struct LLMContextService {
    character_repo: Arc<dyn CharacterRepositoryPort + Send + Sync>,
    location_repo: Arc<dyn LocationRepositoryPort + Send + Sync>,
    scene_repo: Arc<dyn SceneRepositoryPort + Send + Sync>,
    challenge_repo: Arc<dyn ChallengeRepositoryPort + Send + Sync>,
    narrative_event_repo: Arc<dyn NarrativeEventRepositoryPort + Send + Sync>,
    token_counter: TokenCounter,
}

impl LLMContextService {
    /// Create a new LLM context service
    pub fn new(
        character_repo: Arc<dyn CharacterRepositoryPort + Send + Sync>,
        location_repo: Arc<dyn LocationRepositoryPort + Send + Sync>,
        scene_repo: Arc<dyn SceneRepositoryPort + Send + Sync>,
        challenge_repo: Arc<dyn ChallengeRepositoryPort + Send + Sync>,
        narrative_event_repo: Arc<dyn NarrativeEventRepositoryPort + Send + Sync>,
    ) -> Self {
        Self {
            character_repo,
            location_repo,
            scene_repo,
            challenge_repo,
            narrative_event_repo,
            token_counter: TokenCounter::llama_tuned(),
        }
    }

    /// Build complete context for an LLM prompt
    ///
    /// This method assembles context from multiple graph queries, respecting
    /// the configured token budgets for each category.
    pub async fn build_context(
        &self,
        scene_id: SceneId,
        responding_character_id: CharacterId,
        budget: &ContextBudgetConfig,
    ) -> Result<AssembledContext, LLMContextError> {
        let mut assembled = AssembledContext::default();

        // Build each category in priority order
        for category in ContextCategory::all_by_priority() {
            let category_budget = budget.budget_for(category);
            
            let category_ctx = match category {
                ContextCategory::Scene => {
                    self.build_scene_context(scene_id, category_budget).await?
                }
                ContextCategory::Character => {
                    self.build_character_context(responding_character_id, category_budget).await?
                }
                ContextCategory::LocationContext => {
                    self.build_location_context_from_scene(scene_id, category_budget).await?
                }
                ContextCategory::Challenges => {
                    self.build_challenges_context(scene_id, category_budget).await?
                }
                ContextCategory::NarrativeEvents => {
                    self.build_narrative_events_context(scene_id, category_budget).await?
                }
                // These categories are built elsewhere (from conversation history, directorial notes)
                ContextCategory::ConversationHistory => None,
                ContextCategory::DirectorialNotes => None,
                ContextCategory::PlayerContext => None,
            };

            if let Some(ctx) = category_ctx {
                assembled.total_tokens += ctx.token_count;
                if ctx.was_summarized {
                    assembled.summarized_categories.push(category);
                }
                assembled.categories.push(ctx);
            }
        }

        Ok(assembled)
    }

    /// Build scene context from graph
    async fn build_scene_context(
        &self,
        scene_id: SceneId,
        budget: usize,
    ) -> Result<Option<CategoryContext>, LLMContextError> {
        let scene = self.scene_repo
            .get(scene_id)
            .await
            .map_err(|e| LLMContextError::RepositoryError(e.to_string()))?;

        let Some(scene) = scene else {
            return Ok(None);
        };

        // Get location via edge (or fall back to deprecated field)
        let location_id = self.scene_repo
            .get_location(scene_id)
            .await
            .map_err(|e| LLMContextError::RepositoryError(e.to_string()))?
            .unwrap_or(scene.location_id);

        let location_name = self.location_repo
            .get(location_id)
            .await
            .map_err(|e| LLMContextError::RepositoryError(e.to_string()))?
            .map(|l| l.name)
            .unwrap_or_else(|| "Unknown Location".to_string());

        // Get featured characters via edges
        let featured_chars = self.scene_repo
            .get_featured_characters(scene_id)
            .await
            .map_err(|e| LLMContextError::RepositoryError(e.to_string()))?;

        let mut present_characters = Vec::new();
        for (char_id, _scene_char) in featured_chars {
            if let Ok(Some(character)) = self.character_repo.get(char_id).await {
                present_characters.push(character.name);
            }
        }

        // Build context string
        let mut content = String::new();
        content.push_str(&format!("CURRENT SCENE: {}\n", scene.name));
        content.push_str(&format!("LOCATION: {}\n", location_name));
        content.push_str(&format!("TIME: {:?}\n", scene.time_context));
        
        if !present_characters.is_empty() {
            content.push_str(&format!("OTHERS PRESENT: {}\n", present_characters.join(", ")));
        }

        let token_count = self.token_counter.count(&content);
        
        // Truncate if over budget
        let (final_content, was_truncated) = if token_count > budget {
            self.token_counter.truncate_to_budget(&content, budget)
        } else {
            (content, false)
        };

        let final_token_count = self.token_counter.count(&final_content);

        Ok(Some(if was_truncated {
            CategoryContext::summarized(
                ContextCategory::Scene,
                final_content,
                final_token_count,
                token_count,
            )
        } else {
            CategoryContext::new(ContextCategory::Scene, final_content, final_token_count)
        }))
    }

    /// Build character context from graph (including wants via edges)
    async fn build_character_context(
        &self,
        character_id: CharacterId,
        budget: usize,
    ) -> Result<Option<CategoryContext>, LLMContextError> {
        let character = self.character_repo
            .get(character_id)
            .await
            .map_err(|e| LLMContextError::RepositoryError(e.to_string()))?;

        let Some(character) = character else {
            return Ok(None);
        };

        // Get wants via HAS_WANT edges
        let wants = self.character_repo
            .get_wants(character_id)
            .await
            .map_err(|e| LLMContextError::RepositoryError(e.to_string()))?;

        // Build context string
        let mut content = String::new();
        
        content.push_str(&format!(
            "CHARACTER: {} ({})\n",
            character.name,
            character.current_archetype.description()
        ));

        if !character.description.is_empty() {
            content.push_str(&format!("DESCRIPTION: {}\n", character.description));
        }

        if !wants.is_empty() {
            content.push_str("MOTIVATIONS AND DESIRES:\n");
            for want in &wants {
                content.push_str(&format!("- {} (priority: {})\n", want.want.description, want.priority));
            }
        }

        let token_count = self.token_counter.count(&content);
        
        let (final_content, was_truncated) = if token_count > budget {
            self.token_counter.truncate_to_budget(&content, budget)
        } else {
            (content, false)
        };

        let final_token_count = self.token_counter.count(&final_content);

        Ok(Some(if was_truncated {
            CategoryContext::summarized(
                ContextCategory::Character,
                final_content,
                final_token_count,
                token_count,
            )
        } else {
            CategoryContext::new(ContextCategory::Character, final_content, final_token_count)
        }))
    }

    /// Build location context from scene's location
    async fn build_location_context_from_scene(
        &self,
        scene_id: SceneId,
        budget: usize,
    ) -> Result<Option<CategoryContext>, LLMContextError> {
        // Get location from scene
        let scene = self.scene_repo
            .get(scene_id)
            .await
            .map_err(|e| LLMContextError::RepositoryError(e.to_string()))?;

        let Some(scene) = scene else {
            return Ok(None);
        };

        let location_id = self.scene_repo
            .get_location(scene_id)
            .await
            .map_err(|e| LLMContextError::RepositoryError(e.to_string()))?
            .unwrap_or(scene.location_id);

        let location = self.location_repo
            .get(location_id)
            .await
            .map_err(|e| LLMContextError::RepositoryError(e.to_string()))?;

        let Some(location) = location else {
            return Ok(None);
        };

        // Get connected locations via edges
        let connections = self.location_repo
            .get_connections(location_id)
            .await
            .map_err(|e| LLMContextError::RepositoryError(e.to_string()))?;

        let mut connected_names = Vec::new();
        for conn in &connections {
            if let Ok(Some(conn_loc)) = self.location_repo.get(conn.to_location).await {
                connected_names.push(conn_loc.name);
            }
        }

        let mut content = String::new();
        
        if !location.description.is_empty() {
            content.push_str(&format!("LOCATION DETAILS: {}\n", location.description));
        }

        if !connected_names.is_empty() {
            content.push_str(&format!("\nNEARBY AREAS: {}\n", connected_names.join(", ")));
        }

        if content.is_empty() {
            return Ok(None);
        }

        let token_count = self.token_counter.count(&content);
        
        let (final_content, was_truncated) = if token_count > budget {
            self.token_counter.truncate_to_budget(&content, budget)
        } else {
            (content, false)
        };

        let final_token_count = self.token_counter.count(&final_content);

        Ok(Some(if was_truncated {
            CategoryContext::summarized(
                ContextCategory::LocationContext,
                final_content,
                final_token_count,
                token_count,
            )
        } else {
            CategoryContext::new(ContextCategory::LocationContext, final_content, final_token_count)
        }))
    }

    /// Build challenges context for the scene
    async fn build_challenges_context(
        &self,
        scene_id: SceneId,
        budget: usize,
    ) -> Result<Option<CategoryContext>, LLMContextError> {
        // Get challenges tied to this scene
        let challenges = self.challenge_repo
            .list_by_scene(scene_id)
            .await
            .map_err(|e| LLMContextError::RepositoryError(e.to_string()))?;

        if challenges.is_empty() {
            return Ok(None);
        }

        let mut content = String::new();
        content.push_str("## Active Challenges\n");
        content.push_str("The following challenges may be triggered based on player actions:\n\n");

        for (idx, challenge) in challenges.iter().enumerate() {
            content.push_str(&format!(
                "{}. \"{}\" ({})\n",
                idx + 1,
                challenge.name,
                challenge.difficulty.display()
            ));

            if !challenge.description.is_empty() {
                content.push_str(&format!("   Description: {}\n\n", challenge.description));
            }
        }

        let token_count = self.token_counter.count(&content);
        
        let (final_content, was_truncated) = if token_count > budget {
            self.token_counter.truncate_to_budget(&content, budget)
        } else {
            (content, false)
        };

        let final_token_count = self.token_counter.count(&final_content);

        Ok(Some(if was_truncated {
            CategoryContext::summarized(
                ContextCategory::Challenges,
                final_content,
                final_token_count,
                token_count,
            )
        } else {
            CategoryContext::new(ContextCategory::Challenges, final_content, final_token_count)
        }))
    }

    /// Build narrative events context for the scene
    async fn build_narrative_events_context(
        &self,
        scene_id: SceneId,
        budget: usize,
    ) -> Result<Option<CategoryContext>, LLMContextError> {
        // Get narrative events tied to this scene
        let events = self.narrative_event_repo
            .list_by_scene(scene_id)
            .await
            .map_err(|e| LLMContextError::RepositoryError(e.to_string()))?;

        if events.is_empty() {
            return Ok(None);
        }

        let mut content = String::new();
        content.push_str("## Active Narrative Events\n");
        content.push_str("The following story events may be triggered based on player actions:\n\n");

        for (idx, event) in events.iter().enumerate() {
            content.push_str(&format!(
                "{}. \"{}\" (Priority: {})\n",
                idx + 1,
                event.name,
                event.priority
            ));

            if !event.description.is_empty() {
                content.push_str(&format!("   Description: {}\n", event.description));
            }

            // Get featured NPCs via edge
            if let Ok(featured_npcs) = self.narrative_event_repo.get_featured_npcs(event.id).await {
                let mut npc_names = Vec::new();
                for featured in featured_npcs {
                    if let Ok(Some(character)) = self.character_repo.get(featured.character_id).await {
                        npc_names.push(character.name);
                    }
                }
                if !npc_names.is_empty() {
                    content.push_str(&format!("   Featured NPCs: {}\n", npc_names.join(", ")));
                }
            }

            content.push_str("\n");
        }

        let token_count = self.token_counter.count(&content);
        
        let (final_content, was_truncated) = if token_count > budget {
            self.token_counter.truncate_to_budget(&content, budget)
        } else {
            (content, false)
        };

        let final_token_count = self.token_counter.count(&final_content);

        Ok(Some(if was_truncated {
            CategoryContext::summarized(
                ContextCategory::NarrativeEvents,
                final_content,
                final_token_count,
                token_count,
            )
        } else {
            CategoryContext::new(ContextCategory::NarrativeEvents, final_content, final_token_count)
        }))
    }

    // =========================================================================
    // Helper methods for building specific context types
    // =========================================================================

    /// Build a CharacterContext value object from graph data
    pub async fn build_character_context_vo(
        &self,
        character_id: CharacterId,
    ) -> Result<CharacterContext, LLMContextError> {
        let character = self.character_repo
            .get(character_id)
            .await
            .map_err(|e| LLMContextError::RepositoryError(e.to_string()))?
            .ok_or_else(|| LLMContextError::NotFound(format!("Character {}", character_id)))?;

        // Get wants via edges
        let wants = self.character_repo
            .get_wants(character_id)
            .await
            .map_err(|e| LLMContextError::RepositoryError(e.to_string()))?;

        let want_descriptions: Vec<String> = wants.iter()
            .map(|w| w.want.description.clone())
            .collect();

        Ok(CharacterContext {
            name: character.name,
            archetype: character.current_archetype.description().to_string(),
            current_mood: None, // Character entity doesn't have mood - would need session state
            wants: want_descriptions,
            relationship_to_player: None, // Would need player context to determine
        })
    }

    /// Build a SceneContext value object from graph data
    pub async fn build_scene_context_vo(
        &self,
        scene_id: SceneId,
    ) -> Result<SceneContext, LLMContextError> {
        let scene = self.scene_repo
            .get(scene_id)
            .await
            .map_err(|e| LLMContextError::RepositoryError(e.to_string()))?
            .ok_or_else(|| LLMContextError::NotFound(format!("Scene {}", scene_id)))?;

        // Get location name
        let location_id = self.scene_repo
            .get_location(scene_id)
            .await
            .ok()
            .flatten()
            .unwrap_or(scene.location_id);

        let location_name = self.location_repo
            .get(location_id)
            .await
            .ok()
            .flatten()
            .map(|l| l.name)
            .unwrap_or_else(|| "Unknown".to_string());

        // Get featured characters
        let featured = self.scene_repo
            .get_featured_characters(scene_id)
            .await
            .map_err(|e| LLMContextError::RepositoryError(e.to_string()))?;

        let mut present_characters = Vec::new();
        for (char_id, _) in featured {
            if let Ok(Some(character)) = self.character_repo.get(char_id).await {
                present_characters.push(character.name);
            }
        }

        Ok(SceneContext {
            scene_name: scene.name,
            location_name,
            time_context: format!("{:?}", scene.time_context),
            present_characters,
        })
    }

    /// Build ActiveChallengeContext list for a scene
    pub async fn build_active_challenges(
        &self,
        scene_id: SceneId,
    ) -> Result<Vec<ActiveChallengeContext>, LLMContextError> {
        let challenges = self.challenge_repo
            .list_by_scene(scene_id)
            .await
            .map_err(|e| LLMContextError::RepositoryError(e.to_string()))?;

        let result: Vec<ActiveChallengeContext> = challenges.iter().map(|challenge| {
            ActiveChallengeContext {
                id: challenge.id.to_string(),
                name: challenge.name.clone(),
                description: challenge.description.clone(),
                skill_name: "General".to_string(), // Would need skill repo
                difficulty_display: challenge.difficulty.display(),
                trigger_hints: challenge.trigger_conditions.iter().map(|t| t.description.clone()).collect(),
            }
        }).collect();

        Ok(result)
    }

    /// Build ActiveNarrativeEventContext list for a scene
    pub async fn build_active_narrative_events(
        &self,
        scene_id: SceneId,
    ) -> Result<Vec<ActiveNarrativeEventContext>, LLMContextError> {
        let events = self.narrative_event_repo
            .list_by_scene(scene_id)
            .await
            .map_err(|e| LLMContextError::RepositoryError(e.to_string()))?;

        let mut result = Vec::new();
        for event in events {
            // Get featured NPC names
            let npc_names = if let Ok(featured_npcs) = self.narrative_event_repo.get_featured_npcs(event.id).await {
                let mut names = Vec::new();
                for featured in featured_npcs {
                    if let Ok(Some(character)) = self.character_repo.get(featured.character_id).await {
                        names.push(character.name);
                    }
                }
                names
            } else {
                Vec::new()
            };

            result.push(ActiveNarrativeEventContext {
                id: event.id.to_string(),
                name: event.name,
                description: event.description,
                scene_direction: event.scene_direction,
                trigger_hints: event.trigger_conditions.iter().map(|t| t.description.clone()).collect(),
                featured_npc_names: npc_names,
                priority: event.priority,
            });
        }

        Ok(result)
    }
}

// ============================================================================
// Context Summarization
// ============================================================================

/// Prompt templates for summarizing different context categories
pub struct SummarizationPrompts;

impl SummarizationPrompts {
    /// Get the summarization prompt for a given category
    pub fn for_category(category: ContextCategory, content: &str, target_tokens: usize) -> String {
        let category_guidance = match category {
            ContextCategory::Scene => "scene context including location, time, and atmosphere",
            ContextCategory::Character => "character details including personality, motivations, and relationships",
            ContextCategory::ConversationHistory => "conversation history, preserving key exchanges and context",
            ContextCategory::Challenges => "available challenges, keeping names and key triggers",
            ContextCategory::NarrativeEvents => "narrative events, preserving event names and trigger conditions",
            ContextCategory::DirectorialNotes => "director's notes on tone, pacing, and guidance",
            ContextCategory::LocationContext => "location details including nearby areas and atmosphere",
            ContextCategory::PlayerContext => "player character details and current status",
        };

        format!(
            r#"Summarize the following {} into approximately {} words while preserving the most important information for roleplay context.

Keep:
- Names of characters, locations, and items
- Key relationships and motivations
- Important triggers or conditions
- Emotional tone and atmosphere

Remove:
- Redundant descriptions
- Minor details that won't affect roleplay
- Verbose explanations

Original content:
{}

Provide only the summarized content, no explanations."#,
            category_guidance,
            target_tokens / 2,
            content
        )
    }

    /// Get a system prompt for the summarizer
    pub fn system_prompt() -> &'static str {
        "You are a context summarizer for a tabletop roleplaying game. Your task is to condense game context while preserving essential information for character roleplay. Be concise but preserve all names, key relationships, and important details."
    }
}

/// Result of a summarization operation
#[derive(Debug, Clone)]
pub struct SummarizationResult {
    pub content: String,
    pub original_tokens: usize,
    pub summarized_tokens: usize,
    pub category: ContextCategory,
}

/// A request to summarize context
#[derive(Debug, Clone)]
pub struct SummarizationRequest {
    pub category: ContextCategory,
    pub content: String,
    pub target_tokens: usize,
    pub original_tokens: usize,
}

impl SummarizationRequest {
    pub fn new(category: ContextCategory, content: String, target_tokens: usize, original_tokens: usize) -> Self {
        Self { category, content, target_tokens, original_tokens }
    }

    pub fn build_prompt(&self) -> String {
        SummarizationPrompts::for_category(self.category, &self.content, self.target_tokens)
    }

    pub fn system_prompt() -> &'static str {
        SummarizationPrompts::system_prompt()
    }
}

/// Utility to determine which categories need summarization
pub struct SummarizationPlanner;

impl SummarizationPlanner {
    /// Plan summarization for assembled context that exceeds budget
    pub fn plan_summarization(
        assembled: &AssembledContext,
        budget: &ContextBudgetConfig,
    ) -> Vec<SummarizationRequest> {
        let mut requests = Vec::new();

        let overage = assembled.total_tokens.saturating_sub(budget.total_budget_tokens);
        if overage == 0 {
            return requests;
        }

        let priorities: Vec<ContextCategory> = ContextCategory::all_by_priority()
            .into_iter()
            .rev()
            .collect();

        let mut tokens_to_save = overage;

        for category in &priorities {
            if tokens_to_save == 0 {
                break;
            }

            if let Some(ctx) = assembled.get(*category) {
                let category_budget = budget.budget_for(*category);
                
                if ctx.token_count > category_budget {
                    let save_amount = ctx.token_count - category_budget;
                    
                    requests.push(SummarizationRequest::new(
                        *category,
                        ctx.content.clone(),
                        category_budget,
                        ctx.token_count,
                    ));

                    tokens_to_save = tokens_to_save.saturating_sub(save_amount);
                }
            }
        }

        if tokens_to_save > 0 {
            for category in &priorities {
                if tokens_to_save == 0 {
                    break;
                }

                if requests.iter().any(|r| r.category == *category) {
                    continue;
                }

                if let Some(ctx) = assembled.get(*category) {
                    let new_target = ctx.token_count / 2;
                    let save_amount = ctx.token_count - new_target;

                    if save_amount > 0 && new_target >= 50 {
                        requests.push(SummarizationRequest::new(
                            *category,
                            ctx.content.clone(),
                            new_target,
                            ctx.token_count,
                        ));

                        tokens_to_save = tokens_to_save.saturating_sub(save_amount);
                    }
                }
            }
        }

        requests
    }

    pub fn needs_summarization(assembled: &AssembledContext, budget: &ContextBudgetConfig) -> bool {
        budget.enable_summarization && assembled.total_tokens > budget.total_budget_tokens
    }
}

/// Errors that can occur in the LLM context service
#[derive(Debug, thiserror::Error)]
pub enum LLMContextError {
    #[error("Repository error: {0}")]
    RepositoryError(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Token budget exceeded: {0} tokens (budget: {1})")]
    BudgetExceeded(usize, usize),

    #[error("Summarization failed: {0}")]
    SummarizationFailed(String),
}
