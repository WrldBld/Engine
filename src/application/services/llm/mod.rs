//! LLM Service - AI-assisted game directing
//!
//! This service provides an interface for generating NPC responses and
//! game narrative content using Large Language Models. It handles:
//!
//! - Building context-aware prompts from game state
//! - Generating NPC dialogue with personality
//! - Parsing tool calls for game mechanics
//! - Providing internal reasoning for the DM to review

mod prompt_builder;
mod tool_definitions;
mod tool_parser;

// Re-export public types and functions

use serde::{Deserialize, Serialize};

use crate::application::ports::outbound::{LlmPort, LlmRequest};
use crate::domain::value_objects::{DirectorialNotes, GamePromptRequest};

use prompt_builder::{build_conversation_history, build_system_prompt_with_notes, build_user_message};
use tool_definitions::get_game_tool_definitions;
use tool_parser::parse_tool_calls_from_response;

/// Service for generating AI-powered game responses
///
/// # Example
///
/// ```ignore
/// use wrldbldr_engine::application::services::LLMService;
/// use wrldbldr_engine::infrastructure::ollama::OllamaClient;
///
/// let client = OllamaClient::new("http://localhost:11434/v1", "llama3.2");
/// let service = LLMService::new(client);
///
/// let request = GamePromptRequest {
///     player_action: PlayerActionContext {
///         action_type: "speak".to_string(),
///         target: Some("Bartender".to_string()),
///         dialogue: Some("What news from the capital?".to_string()),
///     },
///     scene_context: SceneContext {
///         scene_name: "The Rusty Anchor".to_string(),
///         location_name: "Port Valdris".to_string(),
///         time_context: "Late evening".to_string(),
///         present_characters: vec!["Bartender".to_string(), "Mysterious Stranger".to_string()],
///     },
///     directorial_notes: "Build tension about the rebellion".to_string(),
///     conversation_history: vec![],
///     responding_character: CharacterContext {
///         name: "Gorm the Bartender".to_string(),
///         archetype: "Gruff but kind-hearted tavern keeper".to_string(),
///         current_mood: Some("Cautious".to_string()),
///         wants: vec!["Protect his establishment".to_string()],
///         relationship_to_player: Some("Acquaintance".to_string()),
///     },
/// };
///
/// let response = service.generate_npc_response(request).await?;
/// ```
pub struct LLMService<L: LlmPort> {
    ollama: L,
}

impl<L: LlmPort> LLMService<L> {
    /// Create a new LLM service with the provided client
    pub fn new(ollama: L) -> Self {
        Self { ollama }
    }

    /// Generate an NPC response to a player action
    ///
    /// This method builds a comprehensive prompt from the game context,
    /// sends it to the LLM, and parses the response into a structured format
    /// that includes dialogue, reasoning, and any proposed tool calls.
    pub async fn generate_npc_response(
        &self,
        request: GamePromptRequest,
    ) -> Result<LLMGameResponse, LLMServiceError> {
        // Use the enhanced version with no directorial notes
        self.generate_npc_response_with_direction(request, None).await
    }

    /// Generate an NPC response with comprehensive directorial guidance
    ///
    /// This is the enhanced version that integrates DirectorialNotes for fuller
    /// scene context and more tailored LLM responses. Recommended for complex
    /// scene interactions where pacing, tone, and narrative beats are important.
    ///
    /// # Arguments
    ///
    /// * `request` - The core interaction request
    /// * `directorial_notes` - Optional structured guidance for the LLM about
    ///                         tone, pacing, and narrative direction
    ///
    /// # Example
    ///
    /// ```ignore
    /// use wrldbldr_engine::domain::value_objects::{DirectorialNotes, ToneGuidance, PacingGuidance};
    ///
    /// let notes = DirectorialNotes::new()
    ///     .with_tone(ToneGuidance::Mysterious)
    ///     .with_pacing(PacingGuidance::Slow)
    ///     .with_general_notes("Build suspicion about the stranger");
    ///
    /// let response = service.generate_npc_response_with_direction(request, Some(&notes)).await?;
    /// ```
    pub async fn generate_npc_response_with_direction(
        &self,
        request: GamePromptRequest,
        directorial_notes: Option<&DirectorialNotes>,
    ) -> Result<LLMGameResponse, LLMServiceError> {
        let system_prompt = build_system_prompt_with_notes(
            &request.scene_context,
            &request.responding_character,
            directorial_notes,
            &request.active_challenges,
            &request.active_narrative_events,
        );
        let user_message = build_user_message(&request);

        let mut messages = build_conversation_history(&request.conversation_history);
        messages.push(crate::application::ports::outbound::ChatMessage {
            role: crate::application::ports::outbound::MessageRole::User,
            content: user_message,
        });

        let llm_request = LlmRequest::new(messages)
            .with_system_prompt(system_prompt)
            .with_temperature(0.8); // Slightly creative for roleplay

        let tools = get_game_tool_definitions();

        let response = self
            .ollama
            .generate_with_tools(llm_request, tools)
            .await
            .map_err(|e| LLMServiceError::LlmError(e.to_string()))?;

        self.parse_response(&response.content, &response.tool_calls)
    }

    /// Parse the LLM response into structured components
    fn parse_response(
        &self,
        content: &str,
        tool_calls: &[crate::application::ports::outbound::ToolCall],
    ) -> Result<LLMGameResponse, LLMServiceError> {
        let reasoning = self
            .extract_tag_content(content, "reasoning")
            .unwrap_or_else(|| "No internal reasoning provided.".to_string());

        let dialogue = self
            .extract_tag_content(content, "dialogue")
            .unwrap_or_else(|| {
                // Fallback: if no tags, treat the whole content as dialogue
                content.to_string()
            });

        let suggested_beats = self
            .extract_tag_content(content, "suggested_beats")
            .map(|beats| {
                beats
                    .lines()
                    .map(|line| line.trim())
                    .filter(|line| !line.is_empty())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        let proposed_tool_calls = parse_tool_calls_from_response(tool_calls);

        // Parse challenge suggestion if present
        let challenge_suggestion = self
            .extract_tag_content(content, "challenge_suggestion")
            .and_then(|suggestion_text| {
                serde_json::from_str::<ChallengeSuggestion>(&suggestion_text).ok()
            });

        // Parse narrative event suggestion if present
        let narrative_event_suggestion = self
            .extract_tag_content(content, "narrative_event_suggestion")
            .and_then(|suggestion_text| {
                serde_json::from_str::<NarrativeEventSuggestion>(&suggestion_text).ok()
            });

        Ok(LLMGameResponse {
            npc_dialogue: dialogue.trim().to_string(),
            internal_reasoning: reasoning.trim().to_string(),
            proposed_tool_calls,
            suggested_beats,
            challenge_suggestion,
            narrative_event_suggestion,
        })
    }

    /// Extract content between XML-style tags
    fn extract_tag_content(&self, text: &str, tag: &str) -> Option<String> {
        let open_tag = format!("<{}>", tag);
        let close_tag = format!("</{}>", tag);

        let start = text.find(&open_tag)?;
        let end = text.find(&close_tag)?;

        if start >= end {
            return None;
        }

        let content_start = start + open_tag.len();
        Some(text[content_start..end].to_string())
    }
}

/// Suggested challenge from the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeSuggestion {
    /// ID of the suggested challenge
    pub challenge_id: String,
    /// Confidence level of the suggestion
    pub confidence: SuggestionConfidence,
    /// Why the LLM suggests this challenge
    pub reasoning: String,
}

/// Suggested narrative event trigger from the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeEventSuggestion {
    /// ID of the suggested narrative event
    pub event_id: String,
    /// Confidence level of the suggestion
    pub confidence: SuggestionConfidence,
    /// Why the LLM suggests triggering this event
    pub reasoning: String,
    /// Which triggers matched based on context
    pub matched_triggers: Vec<String>,
}

/// Confidence level for a challenge suggestion
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SuggestionConfidence {
    High,
    Medium,
    Low,
}

/// Response from the LLM service for a game prompt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMGameResponse {
    /// The NPC's dialogue to show to the player
    pub npc_dialogue: String,
    /// Internal reasoning (shown to DM, hidden from player)
    pub internal_reasoning: String,
    /// Proposed game mechanic changes (require DM approval)
    pub proposed_tool_calls: Vec<ProposedToolCall>,
    /// Narrative suggestions for the DM
    pub suggested_beats: Vec<String>,
    /// Optional suggested challenge from the LLM
    pub challenge_suggestion: Option<ChallengeSuggestion>,
    /// Optional suggested narrative event trigger from the LLM
    pub narrative_event_suggestion: Option<NarrativeEventSuggestion>,
}

/// A proposed tool call that requires DM approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedToolCall {
    /// Name of the tool to call
    pub tool_name: String,
    /// Arguments for the tool call
    pub arguments: serde_json::Value,
    /// Human-readable description of what this will do
    pub description: String,
}

/// Errors that can occur in the LLM service
#[derive(Debug, thiserror::Error)]
pub enum LLMServiceError {
    /// Error from the underlying LLM client
    #[error("LLM error: {0}")]
    LlmError(String),
    /// Error parsing the LLM response
    #[error("Parse error: {0}")]
    ParseError(String),
    /// Invalid request
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::outbound::{LlmResponse, ToolDefinition};
    use crate::domain::value_objects::{CharacterContext, SceneContext};

    /// Shared mock LLM for tests that don't need actual LLM calls
    struct MockLlm;

    #[async_trait::async_trait]
    impl LlmPort for MockLlm {
        type Error = std::io::Error;

        async fn generate(
            &self,
            _request: LlmRequest,
        ) -> Result<LlmResponse, Self::Error> {
            Ok(LlmResponse {
                content: String::new(),
                tool_calls: Vec::new(),
                finish_reason: crate::application::ports::outbound::FinishReason::Stop,
                usage: None,
            })
        }

        async fn generate_with_tools(
            &self,
            _request: LlmRequest,
            _tools: Vec<ToolDefinition>,
        ) -> Result<LlmResponse, Self::Error> {
            self.generate(_request).await
        }
    }

    #[test]
    fn test_extract_tag_content() {
        let service = LLMService::new(MockLlm);

        let text = r#"
<reasoning>
This is the reasoning section.
It has multiple lines.
</reasoning>

<dialogue>
Hello, traveler! What brings you here?
</dialogue>
"#;

        let reasoning = service.extract_tag_content(text, "reasoning");
        assert!(reasoning.is_some());
        assert!(reasoning.unwrap().contains("This is the reasoning section"));

        let dialogue = service.extract_tag_content(text, "dialogue");
        assert!(dialogue.is_some());
        assert!(dialogue.unwrap().contains("Hello, traveler"));

        let missing = service.extract_tag_content(text, "missing");
        assert!(missing.is_none());
    }
}
