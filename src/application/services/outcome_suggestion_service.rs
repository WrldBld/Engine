//! Outcome Suggestion Service (P3.3)
//!
//! Provides LLM-powered suggestions for challenge outcome descriptions.
//! Used when DM requests alternative outcome text.

use std::sync::Arc;

use crate::application::dto::OutcomeSuggestionRequest;
use crate::application::ports::outbound::LlmPort;

/// Error type for outcome suggestion operations
#[derive(Debug, thiserror::Error)]
pub enum SuggestionError {
    #[error("LLM error: {0}")]
    LlmError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
}

/// Service for generating LLM-powered outcome suggestions
pub struct OutcomeSuggestionService<L: LlmPort> {
    llm: Arc<L>,
}

impl<L: LlmPort> OutcomeSuggestionService<L> {
    /// Create a new outcome suggestion service
    pub fn new(llm: Arc<L>) -> Self {
        Self { llm }
    }

    /// Generate alternative outcome descriptions
    ///
    /// Returns 3 alternative descriptions for the given outcome tier.
    pub async fn generate_suggestions(
        &self,
        request: &OutcomeSuggestionRequest,
    ) -> Result<Vec<String>, SuggestionError> {
        let system_prompt = self.build_system_prompt();
        let user_prompt = self.build_user_prompt(request);

        use crate::application::ports::outbound::{ChatMessage, LlmRequest};

        let messages = vec![ChatMessage::user(user_prompt)];

        let llm_request = LlmRequest::new(messages)
            .with_system_prompt(system_prompt)
            .with_temperature(0.8)  // Higher temperature for creativity
            .with_max_tokens(Some(500));

        let response = self
            .llm
            .generate(llm_request)
            .await
            .map_err(|e| SuggestionError::LlmError(format!("{:?}", e)))?;

        // Parse suggestions from response
        let suggestions = self.parse_suggestions(&response.content)?;

        Ok(suggestions)
    }

    /// Build the system prompt for outcome generation
    fn build_system_prompt(&self) -> String {
        r#"You are a creative TTRPG game master assistant specializing in vivid challenge outcomes.

Your task is to generate engaging outcome descriptions for skill challenges. Each description should:
- Be 2-3 sentences of evocative narrative
- Match the outcome tier (critical success, success, failure, critical failure)
- Describe what happens as a result of the roll
- Be written in second person ("You...")
- Add sensory details and dramatic tension

Format: Return exactly 3 suggestions, each on its own line. Do not number them or add prefixes."#.to_string()
    }

    /// Build the user prompt for a specific request
    fn build_user_prompt(&self, request: &OutcomeSuggestionRequest) -> String {
        let mut prompt = format!(
            "Generate 3 alternative {} outcome descriptions for:\n\n\
            Challenge: {}\n\
            Description: {}\n\
            Skill: {}\n\
            Roll Context: {}",
            request.outcome_type,
            request.challenge_name,
            request.challenge_description,
            request.skill_name,
            request.roll_context,
        );

        if let Some(guidance) = &request.guidance {
            prompt.push_str(&format!("\n\nDM Guidance: {}", guidance));
        }

        if let Some(context) = &request.narrative_context {
            prompt.push_str(&format!("\n\nNarrative Context: {}", context));
        }

        prompt.push_str("\n\nProvide 3 distinct suggestions, one per line:");

        prompt
    }

    /// Parse suggestions from LLM response
    fn parse_suggestions(&self, content: &str) -> Result<Vec<String>, SuggestionError> {
        let suggestions: Vec<String> = content
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            // Filter out numbered prefixes like "1." or "1)"
            .map(|line| {
                let trimmed = line.trim_start_matches(|c: char| c.is_numeric() || c == '.' || c == ')' || c == ' ');
                trimmed.trim().to_string()
            })
            .filter(|line| !line.is_empty())
            .take(3)
            .collect();

        if suggestions.is_empty() {
            return Err(SuggestionError::ParseError(
                "No valid suggestions in LLM response".to_string(),
            ));
        }

        Ok(suggestions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_suggestions_simple() {
        let service = OutcomeSuggestionService {
            llm: Arc::new(MockLlm),
        };

        let content = "You succeed with flying colors!\nThe guard barely notices you slip past.\nYour skills prove more than adequate.";
        let result = service.parse_suggestions(content).unwrap();

        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "You succeed with flying colors!");
    }

    #[test]
    fn test_parse_suggestions_numbered() {
        let service = OutcomeSuggestionService {
            llm: Arc::new(MockLlm),
        };

        let content = "1. You succeed with flying colors!\n2. The guard barely notices you slip past.\n3. Your skills prove more than adequate.";
        let result = service.parse_suggestions(content).unwrap();

        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "You succeed with flying colors!");
    }

    struct MockLlm;

    #[async_trait::async_trait]
    impl LlmPort for MockLlm {
        type Error = String;

        async fn generate(
            &self,
            _request: crate::application::ports::outbound::LlmRequest,
        ) -> Result<crate::application::ports::outbound::LlmResponse, Self::Error> {
            Ok(crate::application::ports::outbound::LlmResponse {
                content: "Mock response".to_string(),
                tool_calls: None,
                finish_reason: Some("stop".to_string()),
            })
        }

        async fn health_check(&self) -> Result<(), Self::Error> {
            Ok(())
        }
    }
}
