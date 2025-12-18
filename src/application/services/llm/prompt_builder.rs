//! Prompt building functions for LLM requests
//!
//! # Phase 1: LLM Context Enhancement
//!
//! This module provides functions for building LLM prompts. It now includes
//! enhanced functions that can integrate with the `LLMContextService` for
//! graph-based context building with token budget management.

use crate::application::ports::outbound::{ChatMessage, MessageRole};
use crate::domain::value_objects::{
    ActiveChallengeContext, ActiveNarrativeEventContext, AssembledContext, CategoryContext,
    CharacterContext, ContextCategory, ConversationTurn, DirectorialNotes, GamePromptRequest,
    SceneContext, TokenCounter,
};

/// Build the system prompt that establishes the NPC's personality and context
pub fn build_system_prompt(
    context: &SceneContext,
    character: &CharacterContext,
) -> String {
    build_system_prompt_with_notes(context, character, None, &[], &[])
}

/// Build system prompt with optional directorial notes
///
/// This enhanced version integrates DirectorialNotes for better LLM guidance
/// on tone, pacing, and scene-specific guidance.
pub fn build_system_prompt_with_notes(
    context: &SceneContext,
    character: &CharacterContext,
    directorial_notes: Option<&DirectorialNotes>,
    active_challenges: &[ActiveChallengeContext],
    active_narrative_events: &[ActiveNarrativeEventContext],
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
    prompt.push_str("\n");

    // Directorial notes - tone and pacing guidance
    if let Some(notes) = directorial_notes {
        prompt.push_str("=== DIRECTOR'S SCENE GUIDANCE ===\n");
        prompt.push_str(&format!("Tone: {}\n", notes.tone.description()));
        prompt.push_str(&format!("Pacing: {}\n", notes.pacing.description()));

        if !notes.general_notes.is_empty() {
            prompt.push_str(&format!("General Notes: {}\n", notes.general_notes));
        }

        if !notes.forbidden_topics.is_empty() {
            prompt.push_str(&format!(
                "Avoid discussing: {}\n",
                notes.forbidden_topics.join(", ")
            ));
        }

        if !notes.suggested_beats.is_empty() {
            prompt.push_str("Suggested narrative beats to work toward:\n");
            for beat in &notes.suggested_beats {
                prompt.push_str(&format!("  - {}\n", beat));
            }
        }
        prompt.push_str("\n");
    }

    // Character details
    if let Some(mood) = &character.current_mood {
        prompt.push_str(&format!("YOUR CURRENT MOOD: {}\n", mood));
    }

    if !character.wants.is_empty() {
        prompt.push_str("YOUR MOTIVATIONS AND DESIRES:\n");
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

    // Active challenges - potential things that might be triggered
    if !active_challenges.is_empty() {
        prompt.push_str("## Active Challenges\n");
        prompt.push_str("The following challenges may be triggered based on player actions:\n\n");
        for (idx, challenge) in active_challenges.iter().enumerate() {
            prompt.push_str(&format!(
                "{}. \"{}\" ({} {})\n",
                idx + 1,
                challenge.name,
                challenge.skill_name,
                challenge.difficulty_display
            ));
            prompt.push_str(&format!(
                "   Triggers: {}\n",
                challenge.trigger_hints.join(", ")
            ));
            prompt.push_str(&format!(
                "   Description: {}\n\n",
                challenge.description
            ));
        }

        prompt.push_str("If a player's action matches a trigger condition, include a challenge suggestion in your response using:\n");
        prompt.push_str("<challenge_suggestion>\n");
        prompt.push_str("{\"challenge_id\": \"...\", \"confidence\": \"high|medium|low\", \"reasoning\": \"...\"}\n");
        prompt.push_str("</challenge_suggestion>\n\n");
    }

    // Active narrative events - DM-designed story beats that can be triggered
    if !active_narrative_events.is_empty() {
        prompt.push_str("## Active Narrative Events\n");
        prompt.push_str("The following story events may be triggered based on player actions or conversation:\n\n");
        for (idx, event) in active_narrative_events.iter().enumerate() {
            prompt.push_str(&format!(
                "{}. \"{}\" (Priority: {})\n",
                idx + 1,
                event.name,
                event.priority
            ));
            prompt.push_str(&format!(
                "   Description: {}\n",
                event.description
            ));
            if !event.trigger_hints.is_empty() {
                prompt.push_str(&format!(
                    "   Triggers when: {}\n",
                    event.trigger_hints.join(", ")
                ));
            }
            if !event.featured_npc_names.is_empty() {
                prompt.push_str(&format!(
                    "   Featured NPCs: {}\n",
                    event.featured_npc_names.join(", ")
                ));
            }
            prompt.push_str("\n");
        }

        prompt.push_str("If a player's action or dialogue matches a narrative event trigger, suggest triggering it using:\n");
        prompt.push_str("<narrative_event_suggestion>\n");
        prompt.push_str("{\"event_id\": \"...\", \"confidence\": \"high|medium|low\", \"reasoning\": \"...\", \"matched_triggers\": [\"...\"]}\n");
        prompt.push_str("</narrative_event_suggestion>\n\n");
    }

    // Response format instructions
    prompt.push_str(r#"

RESPONSE FORMAT:
You must respond in the following format:

<reasoning>
Your internal thoughts about how to respond. Consider:
- What does your character know about the situation?
- How does your character feel about this moment?
- What are your character's immediate goals in this conversation?
- Are any game mechanics or tool calls dramatically appropriate?
- How do the directorial notes influence your response?
- Could the player's action trigger any of the active challenges?
- Could the player's action or dialogue trigger any narrative events?
This section is hidden from the player but shown to the Game Master for review.
</reasoning>

<dialogue>
Your character's spoken response. Stay in character.
Write naturally as the character would speak. Use appropriate dialect or speech patterns.
Keep responses concise but meaningful (1-3 sentences typically).
</dialogue>

<suggested_beats>
Optional narrative suggestions for the Game Master, one per line.
These help shape the story direction and are only suggestions.
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
pub fn build_user_message(request: &GamePromptRequest) -> String {
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
pub fn build_conversation_history(history: &[ConversationTurn]) -> Vec<ChatMessage> {
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

// ============================================================================
// Phase 1: Enhanced Prompt Building with Graph-Based Context
// ============================================================================

/// Build a system prompt from assembled graph-based context
///
/// This function takes pre-assembled context from `LLMContextService` and
/// builds a comprehensive system prompt. It handles cases where some
/// categories may have been summarized or omitted due to token budgets.
pub fn build_system_prompt_from_assembled(
    assembled: &AssembledContext,
    character: &CharacterContext,
    directorial_notes: Option<&DirectorialNotes>,
) -> String {
    let mut prompt = String::new();

    // Role establishment (always included, not budget-constrained)
    prompt.push_str(&format!(
        "You are roleplaying as {}, a {}.\n\n",
        character.name, character.archetype
    ));

    // Include assembled context categories in order
    for ctx in &assembled.categories {
        if !ctx.content.is_empty() {
            prompt.push_str(&ctx.content);
            prompt.push_str("\n\n");
        }
    }

    // Directorial notes - tone and pacing guidance (from DirectorialNotes struct)
    if let Some(notes) = directorial_notes {
        prompt.push_str("=== DIRECTOR'S SCENE GUIDANCE ===\n");
        prompt.push_str(&format!("Tone: {}\n", notes.tone.description()));
        prompt.push_str(&format!("Pacing: {}\n", notes.pacing.description()));

        if !notes.general_notes.is_empty() {
            prompt.push_str(&format!("General Notes: {}\n", notes.general_notes));
        }

        if !notes.forbidden_topics.is_empty() {
            prompt.push_str(&format!(
                "Avoid discussing: {}\n",
                notes.forbidden_topics.join(", ")
            ));
        }

        if !notes.suggested_beats.is_empty() {
            prompt.push_str("Suggested narrative beats to work toward:\n");
            for beat in &notes.suggested_beats {
                prompt.push_str(&format!("  - {}\n", beat));
            }
        }
        prompt.push_str("\n");
    }

    // Character details from CharacterContext (which may include graph-fetched wants)
    if let Some(mood) = &character.current_mood {
        prompt.push_str(&format!("YOUR CURRENT MOOD: {}\n", mood));
    }

    if !character.wants.is_empty() {
        prompt.push_str("YOUR MOTIVATIONS AND DESIRES:\n");
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

    // If challenges context was included, add suggestion format
    if assembled.get(ContextCategory::Challenges).is_some() {
        prompt.push_str("\nIf a player's action matches a challenge trigger, include:\n");
        prompt.push_str("<challenge_suggestion>\n");
        prompt.push_str("{\"challenge_id\": \"...\", \"confidence\": \"high|medium|low\", \"reasoning\": \"...\"}\n");
        prompt.push_str("</challenge_suggestion>\n");
    }

    // If narrative events context was included, add suggestion format
    if assembled.get(ContextCategory::NarrativeEvents).is_some() {
        prompt.push_str("\nIf a player's action matches a narrative event trigger, include:\n");
        prompt.push_str("<narrative_event_suggestion>\n");
        prompt.push_str("{\"event_id\": \"...\", \"confidence\": \"high|medium|low\", \"reasoning\": \"...\", \"matched_triggers\": [\"...\"]}\n");
        prompt.push_str("</narrative_event_suggestion>\n");
    }

    // Response format instructions
    prompt.push_str(RESPONSE_FORMAT_INSTRUCTIONS);

    prompt
}

/// Merge additional context into an assembled context
///
/// This is useful when you have graph-based assembled context but need to
/// add additional context that wasn't fetched from the graph (e.g., conversation
/// history from the session).
pub fn merge_conversation_history(
    assembled: &mut AssembledContext,
    history: &[ConversationTurn],
    budget: usize,
) {
    if history.is_empty() {
        return;
    }

    let token_counter = TokenCounter::default();
    
    let mut content = String::new();
    content.push_str("RECENT CONVERSATION:\n");
    
    for turn in history {
        content.push_str(&format!("{}: {}\n", turn.speaker, turn.text));
    }

    let token_count = token_counter.count(&content);
    
    let (final_content, was_truncated) = if token_count > budget {
        token_counter.truncate_to_budget(&content, budget)
    } else {
        (content, false)
    };

    let final_token_count = token_counter.count(&final_content);

    let ctx = if was_truncated {
        CategoryContext::summarized(
            ContextCategory::ConversationHistory,
            final_content,
            final_token_count,
            token_count,
        )
    } else {
        CategoryContext::new(ContextCategory::ConversationHistory, final_content, final_token_count)
    };

    assembled.total_tokens += ctx.token_count;
    if ctx.was_summarized {
        assembled.summarized_categories.push(ContextCategory::ConversationHistory);
    }
    assembled.categories.push(ctx);
}

/// Calculate the token count for a prompt
pub fn count_prompt_tokens(prompt: &str) -> usize {
    TokenCounter::default().count(prompt)
}

/// Standard response format instructions (shared between prompt builders)
const RESPONSE_FORMAT_INSTRUCTIONS: &str = r#"

RESPONSE FORMAT:
You must respond in the following format:

<reasoning>
Your internal thoughts about how to respond. Consider:
- What does your character know about the situation?
- How does your character feel about this moment?
- What are your character's immediate goals in this conversation?
- Are any game mechanics or tool calls dramatically appropriate?
- How do the directorial notes influence your response?
- Could the player's action trigger any of the active challenges?
- Could the player's action or dialogue trigger any narrative events?
This section is hidden from the player but shown to the Game Master for review.
</reasoning>

<dialogue>
Your character's spoken response. Stay in character.
Write naturally as the character would speak. Use appropriate dialect or speech patterns.
Keep responses concise but meaningful (1-3 sentences typically).
</dialogue>

<suggested_beats>
Optional narrative suggestions for the Game Master, one per line.
These help shape the story direction and are only suggestions.
</suggested_beats>

AVAILABLE TOOLS:
You may propose tool calls to affect game state. Available tools:
- give_item: Give an item to the player (item_name: string, description: string)
- reveal_info: Reveal plot-relevant information (info_type: string, content: string, importance: "minor"|"major"|"critical")
- change_relationship: Modify relationship with player (change: "improve"|"worsen", amount: "slight"|"moderate"|"significant", reason: string)
- trigger_event: Trigger a game event (event_type: string, description: string)

Only propose tool calls when dramatically appropriate. The Game Master will approve or reject them.
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_system_prompt() {
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

        let prompt = build_system_prompt(&context, &character);

        assert!(prompt.contains("Gorm"));
        assert!(prompt.contains("Gruff tavern keeper"));
        assert!(prompt.contains("The Rusty Anchor"));
        assert!(prompt.contains("Suspicious"));
        assert!(prompt.contains("Protect his tavern"));
        assert!(prompt.contains("<reasoning>"));
        assert!(prompt.contains("<dialogue>"));
    }
}
