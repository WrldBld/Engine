//! LLM Service - AI-assisted game directing
//!
//! This service provides an interface for generating NPC responses and
//! game narrative content using Large Language Models. It handles:
//!
//! - Building context-aware prompts from game state
//! - Generating NPC dialogue with personality
//! - Parsing tool calls for game mechanics
//! - Providing internal reasoning for the DM to review

use serde::{Deserialize, Serialize};

use crate::application::ports::outbound::{
    ChatMessage, LlmPort, LlmRequest, MessageRole, ToolDefinition,
};

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
        let system_prompt =
            self.build_system_prompt(&request.scene_context, &request.responding_character);
        let user_message = self.build_user_message(&request);

        let mut messages = self.build_conversation_history(&request.conversation_history);
        messages.push(ChatMessage {
            role: MessageRole::User,
            content: user_message,
        });

        let llm_request = LlmRequest::new(messages)
            .with_system_prompt(system_prompt)
            .with_temperature(0.8); // Slightly creative for roleplay

        let tools = self.get_game_tool_definitions();

        let response = self
            .ollama
            .generate_with_tools(llm_request, tools)
            .await
            .map_err(|e| LLMServiceError::LlmError(e.to_string()))?;

        self.parse_response(&response.content, &response.tool_calls)
    }

    /// Build the system prompt that establishes the NPC's personality and context
    pub fn build_system_prompt(
        &self,
        context: &SceneContext,
        character: &CharacterContext,
    ) -> String {
        let mut prompt = String::new();

        // Role establishment
        prompt.push_str(&format!(
            "You are roleplaying as {}, a {}.\n\n",
            character.name, character.archetype
        ));

        // Scene context
        prompt.push_str(&format!("CURRENT SCENE: {}\n", context.scene_name));
        prompt.push_str(&format!("LOCATION: {}\n", context.location_name));
        prompt.push_str(&format!("TIME: {}\n", context.time_context));

        if !context.present_characters.is_empty() {
            prompt.push_str(&format!(
                "OTHERS PRESENT: {}\n",
                context.present_characters.join(", ")
            ));
        }
        prompt.push('\n');

        // Character details
        if let Some(mood) = &character.current_mood {
            prompt.push_str(&format!("YOUR CURRENT MOOD: {}\n", mood));
        }

        if !character.wants.is_empty() {
            prompt.push_str("YOUR MOTIVATIONS:\n");
            for want in &character.wants {
                prompt.push_str(&format!("- {}\n", want));
            }
        }

        if let Some(relationship) = &character.relationship_to_player {
            prompt.push_str(&format!(
                "\nYOUR RELATIONSHIP TO THE PLAYER: {}\n",
                relationship
            ));
        }

        // Response format instructions
        prompt.push_str(r#"

RESPONSE FORMAT:
You must respond in the following format:

<reasoning>
Your internal thoughts about how to respond. Consider:
- What does your character know?
- How does your character feel about this situation?
- What are your character's goals in this conversation?
- Should any game mechanics be triggered?
This section is hidden from the player but shown to the Game Master.
</reasoning>

<dialogue>
Your character's spoken response. Stay in character.
Write naturally as the character would speak.
</dialogue>

<suggested_beats>
Optional narrative suggestions for the Game Master, one per line.
These help shape the story direction.
</suggested_beats>

AVAILABLE TOOLS:
You may propose tool calls to affect game state. Available tools:
- give_item: Give an item to the player (item_name: string, description: string)
- reveal_info: Reveal plot-relevant information (info_type: string, content: string, importance: "minor"|"major"|"critical")
- change_relationship: Modify relationship with player (change: "improve"|"worsen", amount: "slight"|"moderate"|"significant", reason: string)
- trigger_event: Trigger a game event (event_type: string, description: string)

Only propose tool calls when dramatically appropriate. The Game Master will approve or reject them.
"#);

        prompt
    }

    /// Build the user message containing the player's action and directorial notes
    fn build_user_message(&self, request: &GamePromptRequest) -> String {
        let mut message = String::new();

        // Directorial notes (for the AI, not visible to player)
        if !request.directorial_notes.is_empty() {
            message.push_str(&format!(
                "[DIRECTOR'S NOTES: {}]\n\n",
                request.directorial_notes
            ));
        }

        // Player action
        match request.player_action.action_type.as_str() {
            "speak" => {
                if let Some(dialogue) = &request.player_action.dialogue {
                    if let Some(target) = &request.player_action.target {
                        message.push_str(&format!(
                            "The player says to {}: \"{}\"\n",
                            target, dialogue
                        ));
                    } else {
                        message.push_str(&format!("The player says: \"{}\"\n", dialogue));
                    }
                }
            }
            "examine" => {
                if let Some(target) = &request.player_action.target {
                    message.push_str(&format!("The player examines {}.\n", target));
                }
            }
            "use_item" => {
                if let Some(target) = &request.player_action.target {
                    message.push_str(&format!("The player uses an item on {}.\n", target));
                }
            }
            other => {
                message.push_str(&format!("The player performs action: {}\n", other));
                if let Some(target) = &request.player_action.target {
                    message.push_str(&format!("Target: {}\n", target));
                }
            }
        }

        message.push_str(&format!(
            "\nRespond as {}.",
            request.responding_character.name
        ));

        message
    }

    /// Convert conversation history to ChatMessage format
    fn build_conversation_history(&self, history: &[ConversationTurn]) -> Vec<ChatMessage> {
        history
            .iter()
            .map(|turn| {
                // Determine role based on speaker name
                // If it matches the player, it's a user message; otherwise assistant
                let role = if turn.speaker.to_lowercase() == "player" {
                    MessageRole::User
                } else {
                    MessageRole::Assistant
                };

                ChatMessage {
                    role,
                    content: format!("{}: {}", turn.speaker, turn.text),
                }
            })
            .collect()
    }

    /// Get the tool definitions for game mechanics
    fn get_game_tool_definitions(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "give_item".to_string(),
                description: "Give an item to the player character".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "item_name": {
                            "type": "string",
                            "description": "Name of the item to give"
                        },
                        "description": {
                            "type": "string",
                            "description": "Description of the item"
                        }
                    },
                    "required": ["item_name", "description"]
                }),
            },
            ToolDefinition {
                name: "reveal_info".to_string(),
                description: "Reveal plot-relevant information to the player".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "info_type": {
                            "type": "string",
                            "description": "Category of information (lore, quest, character, location)"
                        },
                        "content": {
                            "type": "string",
                            "description": "The information being revealed"
                        },
                        "importance": {
                            "type": "string",
                            "enum": ["minor", "major", "critical"],
                            "description": "How important this information is to the plot"
                        }
                    },
                    "required": ["info_type", "content", "importance"]
                }),
            },
            ToolDefinition {
                name: "change_relationship".to_string(),
                description: "Modify the NPC's relationship with the player".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "change": {
                            "type": "string",
                            "enum": ["improve", "worsen"],
                            "description": "Direction of relationship change"
                        },
                        "amount": {
                            "type": "string",
                            "enum": ["slight", "moderate", "significant"],
                            "description": "Magnitude of the change"
                        },
                        "reason": {
                            "type": "string",
                            "description": "Why the relationship changed"
                        }
                    },
                    "required": ["change", "amount", "reason"]
                }),
            },
            ToolDefinition {
                name: "trigger_event".to_string(),
                description: "Trigger a game event or narrative beat".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "event_type": {
                            "type": "string",
                            "description": "Type of event (combat, discovery, social, environmental)"
                        },
                        "description": {
                            "type": "string",
                            "description": "Description of what happens"
                        }
                    },
                    "required": ["event_type", "description"]
                }),
            },
        ]
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

        let proposed_tool_calls = self.parse_tool_calls_from_response(tool_calls);

        Ok(LLMGameResponse {
            npc_dialogue: dialogue.trim().to_string(),
            internal_reasoning: reasoning.trim().to_string(),
            proposed_tool_calls,
            suggested_beats,
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

    /// Parse tool calls from the LLM response into ProposedToolCall format
    pub fn parse_tool_calls(&self, response: &str) -> Vec<ProposedToolCall> {
        // Try to parse tool calls from JSON in the response
        // This handles cases where the model returns tool calls in the text
        let mut calls = Vec::new();

        // Look for JSON objects that might be tool calls
        if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                let potential_json = &response[start..=end];
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(potential_json) {
                    if let Some(tool_name) = value.get("tool").and_then(|v| v.as_str()) {
                        calls.push(ProposedToolCall {
                            tool_name: tool_name.to_string(),
                            arguments: value
                                .get("arguments")
                                .cloned()
                                .unwrap_or(serde_json::Value::Null),
                            description: value
                                .get("description")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                        });
                    }
                }
            }
        }

        calls
    }

    /// Convert LLM ToolCall format to ProposedToolCall
    fn parse_tool_calls_from_response(
        &self,
        tool_calls: &[crate::application::ports::outbound::ToolCall],
    ) -> Vec<ProposedToolCall> {
        tool_calls
            .iter()
            .map(|tc| {
                let description = self.generate_tool_description(&tc.name, &tc.arguments);
                ProposedToolCall {
                    tool_name: tc.name.clone(),
                    arguments: tc.arguments.clone(),
                    description,
                }
            })
            .collect()
    }

    /// Generate a human-readable description of a tool call
    fn generate_tool_description(&self, name: &str, arguments: &serde_json::Value) -> String {
        match name {
            "give_item" => {
                let item = arguments
                    .get("item_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown item");
                format!("Give '{}' to the player", item)
            }
            "reveal_info" => {
                let info_type = arguments
                    .get("info_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("information");
                let importance = arguments
                    .get("importance")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                format!("Reveal {} {} to the player", importance, info_type)
            }
            "change_relationship" => {
                let change = arguments
                    .get("change")
                    .and_then(|v| v.as_str())
                    .unwrap_or("change");
                let amount = arguments
                    .get("amount")
                    .and_then(|v| v.as_str())
                    .unwrap_or("slightly");
                format!("{} relationship {} with player", change, amount)
            }
            "trigger_event" => {
                let event_type = arguments
                    .get("event_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("event");
                format!("Trigger {} event", event_type)
            }
            _ => format!("Call {} with provided arguments", name),
        }
    }
}

/// Request for generating an NPC response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamePromptRequest {
    /// The player's action that triggered this response
    pub player_action: PlayerActionContext,
    /// Current scene information
    pub scene_context: SceneContext,
    /// Director's notes for guiding the AI response
    pub directorial_notes: String,
    /// Previous conversation turns for context
    pub conversation_history: Vec<ConversationTurn>,
    /// The NPC who is responding
    pub responding_character: CharacterContext,
}

/// Context about the player's action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerActionContext {
    /// Type of action: "speak", "examine", "use_item", etc.
    pub action_type: String,
    /// Target of the action (NPC name, object, etc.)
    pub target: Option<String>,
    /// Dialogue content if the action is speech
    pub dialogue: Option<String>,
}

/// Context about the current scene
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneContext {
    /// Name of the current scene
    pub scene_name: String,
    /// Name of the location
    pub location_name: String,
    /// Time of day / narrative time context
    pub time_context: String,
    /// Names of characters present in the scene
    pub present_characters: Vec<String>,
}

/// Context about the responding character
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterContext {
    /// Character's name
    pub name: String,
    /// Character archetype / personality summary
    pub archetype: String,
    /// Current emotional state
    pub current_mood: Option<String>,
    /// Character's motivations and desires
    pub wants: Vec<String>,
    /// How this character relates to the player
    pub relationship_to_player: Option<String>,
}

/// A single turn in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    /// Name of the speaker
    pub speaker: String,
    /// What was said
    pub text: String,
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

    #[test]
    fn test_extract_tag_content() {
        struct MockLlm;

        #[async_trait::async_trait]
        impl LlmPort for MockLlm {
            type Error = std::io::Error;

            async fn generate(
                &self,
                _request: LlmRequest,
            ) -> Result<crate::application::ports::outbound::LlmResponse, Self::Error> {
                unimplemented!()
            }

            async fn generate_with_tools(
                &self,
                _request: LlmRequest,
                _tools: Vec<ToolDefinition>,
            ) -> Result<crate::application::ports::outbound::LlmResponse, Self::Error> {
                unimplemented!()
            }
        }

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

    #[test]
    fn test_build_system_prompt() {
        struct MockLlm;

        #[async_trait::async_trait]
        impl LlmPort for MockLlm {
            type Error = std::io::Error;

            async fn generate(
                &self,
                _request: LlmRequest,
            ) -> Result<crate::application::ports::outbound::LlmResponse, Self::Error> {
                unimplemented!()
            }

            async fn generate_with_tools(
                &self,
                _request: LlmRequest,
                _tools: Vec<ToolDefinition>,
            ) -> Result<crate::application::ports::outbound::LlmResponse, Self::Error> {
                unimplemented!()
            }
        }

        let service = LLMService::new(MockLlm);

        let context = SceneContext {
            scene_name: "The Rusty Anchor".to_string(),
            location_name: "Port Valdris".to_string(),
            time_context: "Late evening".to_string(),
            present_characters: vec!["Bartender".to_string()],
        };

        let character = CharacterContext {
            name: "Gorm".to_string(),
            archetype: "Gruff tavern keeper".to_string(),
            current_mood: Some("Suspicious".to_string()),
            wants: vec!["Protect his tavern".to_string()],
            relationship_to_player: Some("Acquaintance".to_string()),
        };

        let prompt = service.build_system_prompt(&context, &character);

        assert!(prompt.contains("Gorm"));
        assert!(prompt.contains("Gruff tavern keeper"));
        assert!(prompt.contains("The Rusty Anchor"));
        assert!(prompt.contains("Suspicious"));
        assert!(prompt.contains("Protect his tavern"));
        assert!(prompt.contains("<reasoning>"));
        assert!(prompt.contains("<dialogue>"));
    }

    #[test]
    fn test_parse_tool_calls() {
        struct MockLlm;

        #[async_trait::async_trait]
        impl LlmPort for MockLlm {
            type Error = std::io::Error;

            async fn generate(
                &self,
                _request: LlmRequest,
            ) -> Result<crate::application::ports::outbound::LlmResponse, Self::Error> {
                unimplemented!()
            }

            async fn generate_with_tools(
                &self,
                _request: LlmRequest,
                _tools: Vec<ToolDefinition>,
            ) -> Result<crate::application::ports::outbound::LlmResponse, Self::Error> {
                unimplemented!()
            }
        }

        let service = LLMService::new(MockLlm);

        let response = r#"Some text {"tool": "give_item", "arguments": {"item_name": "key"}, "description": "Give key"} more text"#;

        let calls = service.parse_tool_calls(response);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].tool_name, "give_item");
    }
}
