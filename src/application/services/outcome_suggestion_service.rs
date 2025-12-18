//! Outcome Suggestion Service (P3.3)
//!
//! Provides LLM-powered suggestions for challenge outcome descriptions.
//! Used when DM requests alternative outcome text.

use std::sync::Arc;
use uuid::Uuid;

use crate::application::dto::{OutcomeBranchDto, OutcomeSuggestionRequest};
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

    /// Generate outcome branches with structured data
    ///
    /// Returns N branches based on the configured branch count, each with
    /// a unique ID, title, and description for DM selection.
    ///
    /// # Arguments
    /// * `request` - The suggestion request context
    /// * `branch_count` - Number of branches to generate
    /// * `tokens_per_branch` - Max tokens per branch for LLM (from settings, default 200)
    pub async fn generate_branches(
        &self,
        request: &OutcomeSuggestionRequest,
        branch_count: usize,
        tokens_per_branch: u32,
    ) -> Result<Vec<OutcomeBranchDto>, SuggestionError> {
        let system_prompt = self.build_branch_system_prompt(branch_count);
        let user_prompt = self.build_branch_user_prompt(request, branch_count);

        use crate::application::ports::outbound::{ChatMessage, LlmRequest};

        let messages = vec![ChatMessage::user(user_prompt)];

        // Calculate max tokens based on branch count and configurable tokens per branch
        let max_tokens = tokens_per_branch * branch_count as u32;

        let llm_request = LlmRequest::new(messages)
            .with_system_prompt(system_prompt)
            .with_temperature(0.8)
            .with_max_tokens(Some(max_tokens));

        let response = self
            .llm
            .generate(llm_request)
            .await
            .map_err(|e| SuggestionError::LlmError(format!("{:?}", e)))?;

        // Parse branches from response
        let branches = self.parse_branches(&response.content, branch_count)?;

        Ok(branches)
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

    /// Build the system prompt for branch generation
    fn build_branch_system_prompt(&self, branch_count: usize) -> String {
        format!(
            r#"You are a creative TTRPG game master assistant specializing in vivid challenge outcomes.

Your task is to generate {} distinct outcome branches for skill challenges. Each branch should offer a different narrative direction while staying consistent with the outcome tier.

For each branch, provide:
1. A SHORT TITLE (3-5 words) summarizing the outcome
2. A DESCRIPTION (2-3 sentences) of evocative narrative in second person ("You...")

The branches should offer meaningfully different narrative paths, not just rephrased versions of the same outcome.

FORMAT: Use this exact format for each branch, separated by blank lines:
TITLE: [short title]
DESCRIPTION: [narrative description]

Do not number the branches or add any other formatting."#,
            branch_count
        )
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

    /// Build the user prompt for branch generation
    fn build_branch_user_prompt(&self, request: &OutcomeSuggestionRequest, branch_count: usize) -> String {
        let mut prompt = format!(
            "Generate {} distinct {} outcome branches for:\n\n\
            Challenge: {}\n\
            Description: {}\n\
            Skill: {}\n\
            Roll Context: {}",
            branch_count,
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

        prompt.push_str(&format!(
            "\n\nProvide {} branches using the TITLE:/DESCRIPTION: format:",
            branch_count
        ));

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

    /// Parse branches from LLM response
    fn parse_branches(&self, content: &str, expected_count: usize) -> Result<Vec<OutcomeBranchDto>, SuggestionError> {
        let mut branches = Vec::new();
        let mut current_title: Option<String> = None;
        let mut current_description: Option<String> = None;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                // End of a branch block - save if we have both parts
                if let (Some(title), Some(desc)) = (current_title.take(), current_description.take()) {
                    branches.push(OutcomeBranchDto::new(
                        Uuid::new_v4().to_string(),
                        title,
                        desc,
                    ));
                }
                continue;
            }

            // Check for TITLE: prefix (case insensitive)
            if let Some(title) = line.strip_prefix("TITLE:").or_else(|| line.strip_prefix("Title:")) {
                current_title = Some(title.trim().to_string());
            }
            // Check for DESCRIPTION: prefix (case insensitive)
            else if let Some(desc) = line.strip_prefix("DESCRIPTION:").or_else(|| line.strip_prefix("Description:")) {
                current_description = Some(desc.trim().to_string());
            }
            // If we have a title but no description, and this line doesn't have a prefix,
            // treat it as a continuation of the description
            else if current_title.is_some() && current_description.is_none() {
                // Might be a description without prefix
                current_description = Some(line.to_string());
            }
            // Continuation of description
            else if current_description.is_some() {
                if let Some(ref mut desc) = current_description {
                    desc.push(' ');
                    desc.push_str(line);
                }
            }
        }

        // Don't forget the last branch if content doesn't end with blank line
        if let (Some(title), Some(desc)) = (current_title.take(), current_description.take()) {
            branches.push(OutcomeBranchDto::new(
                Uuid::new_v4().to_string(),
                title,
                desc,
            ));
        }

        // If parsing failed, fall back to line-by-line and create generic titles
        if branches.is_empty() {
            let lines: Vec<&str> = content
                .lines()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty())
                .collect();

            for (i, line) in lines.into_iter().take(expected_count).enumerate() {
                // Strip any leading numbering
                let clean = line
                    .trim_start_matches(|c: char| c.is_numeric() || c == '.' || c == ')' || c == ' ')
                    .trim();
                
                // Generate a title from first few words
                let words: Vec<&str> = clean.split_whitespace().take(5).collect();
                let title = words.join(" ");
                
                branches.push(OutcomeBranchDto::new(
                    Uuid::new_v4().to_string(),
                    format!("Option {}: {}", i + 1, title),
                    clean.to_string(),
                ));
            }
        }

        if branches.is_empty() {
            return Err(SuggestionError::ParseError(
                "No valid branches in LLM response".to_string(),
            ));
        }

        Ok(branches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::outbound::{FinishReason, ToolDefinition};

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

    #[test]
    fn test_parse_branches_structured() {
        let service = OutcomeSuggestionService {
            llm: Arc::new(MockLlm),
        };

        let content = "TITLE: Swift Success\nDESCRIPTION: You move with precision and grace.\n\nTITLE: Lucky Break\nDESCRIPTION: Fortune favors you today.";
        let result = service.parse_branches(content, 2).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].title, "Swift Success");
        assert_eq!(result[0].description, "You move with precision and grace.");
        assert_eq!(result[1].title, "Lucky Break");
    }

    #[test]
    fn test_parse_branches_fallback() {
        let service = OutcomeSuggestionService {
            llm: Arc::new(MockLlm),
        };

        // Unstructured input - should fall back to line-by-line
        let content = "You succeed brilliantly!\nThe task is completed with ease.";
        let result = service.parse_branches(content, 2).unwrap();

        assert_eq!(result.len(), 2);
        // Should have generated titles from content
        assert!(result[0].title.starts_with("Option 1:"));
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
                tool_calls: Vec::new(),
                finish_reason: FinishReason::Stop,
                usage: None,
            })
        }

        async fn generate_with_tools(
            &self,
            _request: crate::application::ports::outbound::LlmRequest,
            _tools: Vec<ToolDefinition>,
        ) -> Result<crate::application::ports::outbound::LlmResponse, Self::Error> {
            Ok(crate::application::ports::outbound::LlmResponse {
                content: "Mock response".to_string(),
                tool_calls: Vec::new(),
                finish_reason: FinishReason::Stop,
                usage: None,
            })
        }
    }
}
