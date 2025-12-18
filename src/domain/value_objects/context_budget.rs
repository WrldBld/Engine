//! Context budget configuration for LLM prompts
//!
//! # Architectural Note (Phase 1: LLM Context Enhancement)
//!
//! Token budgets allow fine-grained control over how much context
//! is provided to the LLM in each category. When a category exceeds
//! its budget, it will be summarized to fit within the allocation.
//!
//! Default values are tuned for ~8K context models (Ollama defaults).
//! Larger models can increase these values accordingly.

use serde::{Deserialize, Serialize};

/// Categories of context that can be included in LLM prompts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContextCategory {
    /// Scene description, location, time, atmosphere
    Scene,
    /// NPC details: personality, motivations, relationships
    Character,
    /// Recent conversation history with the player
    ConversationHistory,
    /// Active challenges that could be triggered
    Challenges,
    /// Active narrative events / story beats
    NarrativeEvents,
    /// GM's directorial notes, tone, pacing guidance
    DirectorialNotes,
    /// Location-specific details (connected locations, NPCs who frequent here)
    LocationContext,
    /// Player character details and stats
    PlayerContext,
}

impl ContextCategory {
    /// Returns all context categories in priority order
    /// Higher priority categories are less likely to be summarized aggressively
    pub fn all_by_priority() -> Vec<Self> {
        vec![
            Self::Character,           // Most important - who they're talking to
            Self::Scene,               // Current situation
            Self::DirectorialNotes,    // GM guidance
            Self::ConversationHistory, // Recent context
            Self::Challenges,          // What could happen
            Self::NarrativeEvents,     // Story beats
            Self::LocationContext,     // Environmental details
            Self::PlayerContext,       // Player info
        ]
    }

    /// Display name for this category
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Scene => "Scene Context",
            Self::Character => "Character Details",
            Self::ConversationHistory => "Conversation History",
            Self::Challenges => "Active Challenges",
            Self::NarrativeEvents => "Narrative Events",
            Self::DirectorialNotes => "Directorial Notes",
            Self::LocationContext => "Location Context",
            Self::PlayerContext => "Player Context",
        }
    }
}

/// Token budget configuration for LLM context building
///
/// Each category has its own token budget. When the total context
/// would exceed these limits, lower-priority categories are summarized
/// or truncated to fit.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextBudgetConfig {
    /// Total token budget for the entire system prompt
    /// Should leave room for user message and response
    pub total_budget_tokens: usize,

    /// Budget for scene context (location, time, atmosphere)
    pub scene_tokens: usize,

    /// Budget for character details (personality, wants, relationships)
    pub character_tokens: usize,

    /// Budget for conversation history
    pub conversation_history_tokens: usize,

    /// Budget for active challenges
    pub challenges_tokens: usize,

    /// Budget for narrative events
    pub narrative_events_tokens: usize,

    /// Budget for directorial notes
    pub directorial_notes_tokens: usize,

    /// Budget for location-specific context
    pub location_context_tokens: usize,

    /// Budget for player character context
    pub player_context_tokens: usize,

    /// Whether to enable automatic summarization when over budget
    pub enable_summarization: bool,

    /// Model to use for summarization (uses main model if None)
    pub summarization_model: Option<String>,
}

impl Default for ContextBudgetConfig {
    fn default() -> Self {
        // Defaults tuned for ~8K context models
        // Total: ~4000 tokens for system prompt, leaving ~4000 for conversation
        Self {
            total_budget_tokens: 4000,
            scene_tokens: 500,
            character_tokens: 800,
            conversation_history_tokens: 1000,
            challenges_tokens: 400,
            narrative_events_tokens: 400,
            directorial_notes_tokens: 300,
            location_context_tokens: 300,
            player_context_tokens: 300,
            enable_summarization: true,
            summarization_model: None,
        }
    }
}

impl ContextBudgetConfig {
    /// Create a configuration for larger context models (32K+)
    pub fn large_context() -> Self {
        Self {
            total_budget_tokens: 12000,
            scene_tokens: 1500,
            character_tokens: 2500,
            conversation_history_tokens: 3000,
            challenges_tokens: 1200,
            narrative_events_tokens: 1200,
            directorial_notes_tokens: 800,
            location_context_tokens: 900,
            player_context_tokens: 900,
            enable_summarization: true,
            summarization_model: None,
        }
    }

    /// Create a minimal configuration for very limited context models
    pub fn minimal() -> Self {
        Self {
            total_budget_tokens: 2000,
            scene_tokens: 250,
            character_tokens: 400,
            conversation_history_tokens: 500,
            challenges_tokens: 200,
            narrative_events_tokens: 200,
            directorial_notes_tokens: 150,
            location_context_tokens: 150,
            player_context_tokens: 150,
            enable_summarization: true,
            summarization_model: None,
        }
    }

    /// Get the budget for a specific category
    pub fn budget_for(&self, category: ContextCategory) -> usize {
        match category {
            ContextCategory::Scene => self.scene_tokens,
            ContextCategory::Character => self.character_tokens,
            ContextCategory::ConversationHistory => self.conversation_history_tokens,
            ContextCategory::Challenges => self.challenges_tokens,
            ContextCategory::NarrativeEvents => self.narrative_events_tokens,
            ContextCategory::DirectorialNotes => self.directorial_notes_tokens,
            ContextCategory::LocationContext => self.location_context_tokens,
            ContextCategory::PlayerContext => self.player_context_tokens,
        }
    }

    /// Set the budget for a specific category
    pub fn set_budget_for(&mut self, category: ContextCategory, tokens: usize) {
        match category {
            ContextCategory::Scene => self.scene_tokens = tokens,
            ContextCategory::Character => self.character_tokens = tokens,
            ContextCategory::ConversationHistory => self.conversation_history_tokens = tokens,
            ContextCategory::Challenges => self.challenges_tokens = tokens,
            ContextCategory::NarrativeEvents => self.narrative_events_tokens = tokens,
            ContextCategory::DirectorialNotes => self.directorial_notes_tokens = tokens,
            ContextCategory::LocationContext => self.location_context_tokens = tokens,
            ContextCategory::PlayerContext => self.player_context_tokens = tokens,
        }
    }

    /// Sum of all category budgets (may exceed total_budget_tokens if overlap is expected)
    pub fn sum_category_budgets(&self) -> usize {
        self.scene_tokens
            + self.character_tokens
            + self.conversation_history_tokens
            + self.challenges_tokens
            + self.narrative_events_tokens
            + self.directorial_notes_tokens
            + self.location_context_tokens
            + self.player_context_tokens
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.total_budget_tokens == 0 {
            return Err("Total budget must be greater than 0".to_string());
        }

        // Individual budgets can exceed total (summarization will handle it)
        // but warn if way over
        let sum = self.sum_category_budgets();
        if sum > self.total_budget_tokens * 3 {
            return Err(format!(
                "Sum of category budgets ({}) is more than 3x total budget ({})",
                sum, self.total_budget_tokens
            ));
        }

        Ok(())
    }
}

/// Result of building context for a category
#[derive(Debug, Clone)]
pub struct CategoryContext {
    /// The category this context is for
    pub category: ContextCategory,
    /// The built context text
    pub content: String,
    /// Estimated token count
    pub token_count: usize,
    /// Whether the content was summarized to fit budget
    pub was_summarized: bool,
    /// Original token count before summarization (if applicable)
    pub original_token_count: Option<usize>,
}

impl CategoryContext {
    /// Create a new category context
    pub fn new(category: ContextCategory, content: String, token_count: usize) -> Self {
        Self {
            category,
            content,
            token_count,
            was_summarized: false,
            original_token_count: None,
        }
    }

    /// Create a summarized category context
    pub fn summarized(
        category: ContextCategory,
        content: String,
        token_count: usize,
        original_token_count: usize,
    ) -> Self {
        Self {
            category,
            content,
            token_count,
            was_summarized: true,
            original_token_count: Some(original_token_count),
        }
    }
}

/// Assembled context ready for prompt building
#[derive(Debug, Clone, Default)]
pub struct AssembledContext {
    /// All category contexts, keyed by category
    pub categories: Vec<CategoryContext>,
    /// Total token count across all categories
    pub total_tokens: usize,
    /// Categories that were summarized
    pub summarized_categories: Vec<ContextCategory>,
    /// Categories that were omitted entirely due to budget
    pub omitted_categories: Vec<ContextCategory>,
}

impl AssembledContext {
    /// Get context for a specific category
    pub fn get(&self, category: ContextCategory) -> Option<&CategoryContext> {
        self.categories.iter().find(|c| c.category == category)
    }

    /// Build the final combined context string
    pub fn build_combined(&self) -> String {
        self.categories
            .iter()
            .map(|c| c.content.as_str())
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

// ============================================================================
// Token Counting Utility
// ============================================================================

/// Token counting configuration
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TokenCountMethod {
    /// Simple character-based approximation: 1 token ≈ 4 characters
    /// Fast but less accurate, good for quick estimates
    CharacterApprox,
    /// Word-based approximation: 1 token ≈ 0.75 words (or ~1.33 tokens per word)
    /// Better for English text
    WordApprox,
    /// Hybrid: uses character count for short text, word count for longer text
    /// Provides best balance of speed and accuracy
    Hybrid,
}

impl Default for TokenCountMethod {
    fn default() -> Self {
        Self::Hybrid
    }
}

/// Token counter for estimating LLM token usage
/// 
/// # Accuracy Notes
/// 
/// These are approximations tuned for typical LLM tokenizers (GPT-style BPE).
/// Actual token counts may vary by ±10-20% depending on:
/// - The specific model's tokenizer
/// - Language (non-English text often tokenizes differently)
/// - Special characters and formatting
/// - Code vs prose
///
/// For Ollama models (Llama, Mistral, etc.), these approximations are
/// generally conservative (slightly overestimate), which is desirable
/// for budget management.
#[derive(Debug, Clone)]
pub struct TokenCounter {
    method: TokenCountMethod,
    /// Characters per token for character-based counting
    chars_per_token: f64,
    /// Tokens per word for word-based counting
    tokens_per_word: f64,
    /// Threshold (in chars) for switching from char to word counting in Hybrid mode
    hybrid_threshold: usize,
}

impl Default for TokenCounter {
    fn default() -> Self {
        Self {
            method: TokenCountMethod::Hybrid,
            chars_per_token: 4.0,      // ~4 characters per token
            tokens_per_word: 1.33,     // ~1.33 tokens per word (GPT-style)
            hybrid_threshold: 100,     // Use char counting for very short text
        }
    }
}

impl TokenCounter {
    /// Create a new token counter with the specified method
    pub fn new(method: TokenCountMethod) -> Self {
        Self {
            method,
            ..Default::default()
        }
    }

    /// Create a token counter tuned for Llama-style models
    /// Llama tokenizers tend to be slightly more efficient
    pub fn llama_tuned() -> Self {
        Self {
            method: TokenCountMethod::Hybrid,
            chars_per_token: 3.8,
            tokens_per_word: 1.25,
            hybrid_threshold: 100,
        }
    }

    /// Count tokens in the given text
    pub fn count(&self, text: &str) -> usize {
        if text.is_empty() {
            return 0;
        }

        match self.method {
            TokenCountMethod::CharacterApprox => self.count_by_chars(text),
            TokenCountMethod::WordApprox => self.count_by_words(text),
            TokenCountMethod::Hybrid => {
                if text.len() < self.hybrid_threshold {
                    self.count_by_chars(text)
                } else {
                    self.count_by_words(text)
                }
            }
        }
    }

    /// Count tokens using character-based approximation
    fn count_by_chars(&self, text: &str) -> usize {
        let chars = text.chars().count();
        ((chars as f64) / self.chars_per_token).ceil() as usize
    }

    /// Count tokens using word-based approximation
    fn count_by_words(&self, text: &str) -> usize {
        let words = text.split_whitespace().count();
        // Also account for punctuation and special tokens
        let special_chars = text.chars().filter(|c| {
            matches!(c, '\n' | '\t' | '"' | '\'' | '(' | ')' | '[' | ']' | '{' | '}')
        }).count();
        
        let base_tokens = (words as f64 * self.tokens_per_word).ceil() as usize;
        // Add ~0.5 tokens per special character group
        let special_tokens = special_chars / 2;
        
        base_tokens + special_tokens
    }

    /// Estimate how many characters fit within a token budget
    pub fn chars_for_budget(&self, tokens: usize) -> usize {
        ((tokens as f64) * self.chars_per_token) as usize
    }

    /// Estimate how many words fit within a token budget
    pub fn words_for_budget(&self, tokens: usize) -> usize {
        ((tokens as f64) / self.tokens_per_word) as usize
    }

    /// Check if text exceeds a token budget
    pub fn exceeds_budget(&self, text: &str, budget: usize) -> bool {
        self.count(text) > budget
    }

    /// Truncate text to fit within a token budget (by words)
    /// Returns the truncated text and whether truncation occurred
    pub fn truncate_to_budget(&self, text: &str, budget: usize) -> (String, bool) {
        let current_tokens = self.count(text);
        if current_tokens <= budget {
            return (text.to_string(), false);
        }

        // Estimate target word count
        let target_words = self.words_for_budget(budget);
        
        // Take words up to target, leaving room for ellipsis
        let words: Vec<&str> = text.split_whitespace().collect();
        let take_words = target_words.saturating_sub(1); // Leave room for "..."
        
        if take_words == 0 {
            return ("...".to_string(), true);
        }

        let truncated: String = words.iter()
            .take(take_words)
            .copied()
            .collect::<Vec<_>>()
            .join(" ");
        
        (format!("{}...", truncated), true)
    }
}

/// Convenience function to count tokens using default settings
pub fn count_tokens(text: &str) -> usize {
    TokenCounter::default().count(text)
}

/// Convenience function to check if text exceeds a token budget
pub fn exceeds_token_budget(text: &str, budget: usize) -> bool {
    TokenCounter::default().exceeds_budget(text, budget)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ContextBudgetConfig::default();
        assert_eq!(config.total_budget_tokens, 4000);
        assert!(config.enable_summarization);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_large_context_config() {
        let config = ContextBudgetConfig::large_context();
        assert_eq!(config.total_budget_tokens, 12000);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_category_budget_access() {
        let mut config = ContextBudgetConfig::default();
        assert_eq!(config.budget_for(ContextCategory::Character), 800);

        config.set_budget_for(ContextCategory::Character, 1000);
        assert_eq!(config.budget_for(ContextCategory::Character), 1000);
    }

    #[test]
    fn test_validation_zero_budget() {
        let mut config = ContextBudgetConfig::default();
        config.total_budget_tokens = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_category_priority_order() {
        let priorities = ContextCategory::all_by_priority();
        assert_eq!(priorities[0], ContextCategory::Character);
        assert_eq!(priorities[1], ContextCategory::Scene);
    }

    #[test]
    fn test_category_context_creation() {
        let ctx = CategoryContext::new(
            ContextCategory::Scene,
            "A dark tavern".to_string(),
            10,
        );
        assert!(!ctx.was_summarized);
        assert!(ctx.original_token_count.is_none());

        let summarized = CategoryContext::summarized(
            ContextCategory::Scene,
            "Dark tavern".to_string(),
            5,
            10,
        );
        assert!(summarized.was_summarized);
        assert_eq!(summarized.original_token_count, Some(10));
    }

    // Token counter tests
    #[test]
    fn test_token_counter_empty_string() {
        let counter = TokenCounter::default();
        assert_eq!(counter.count(""), 0);
    }

    #[test]
    fn test_token_counter_short_text() {
        let counter = TokenCounter::default();
        // "Hello" = 5 chars, ~1.25 tokens -> ceil = 2
        let tokens = counter.count("Hello");
        assert!(tokens >= 1 && tokens <= 3);
    }

    #[test]
    fn test_token_counter_longer_text() {
        let counter = TokenCounter::default();
        // Longer text uses word-based counting in hybrid mode
        let text = "The quick brown fox jumps over the lazy dog. This is a sample sentence to test the token counting functionality.";
        let tokens = counter.count(text);
        // ~20 words * 1.33 = ~27 tokens
        assert!(tokens >= 20 && tokens <= 35);
    }

    #[test]
    fn test_token_counter_char_method() {
        let counter = TokenCounter::new(TokenCountMethod::CharacterApprox);
        // 20 chars / 4 = 5 tokens
        let tokens = counter.count("12345678901234567890");
        assert_eq!(tokens, 5);
    }

    #[test]
    fn test_token_counter_word_method() {
        let counter = TokenCounter::new(TokenCountMethod::WordApprox);
        // 5 words * 1.33 = 6.65 -> ceil = 7 tokens
        let tokens = counter.count("one two three four five");
        assert_eq!(tokens, 7);
    }

    #[test]
    fn test_token_counter_exceeds_budget() {
        let counter = TokenCounter::default();
        let text = "This is a test sentence that should exceed a small budget.";
        assert!(counter.exceeds_budget(text, 5));
        assert!(!counter.exceeds_budget(text, 100));
    }

    #[test]
    fn test_token_counter_truncate() {
        let counter = TokenCounter::default();
        let text = "This is a fairly long sentence that we want to truncate to fit within a smaller token budget.";
        
        let (truncated, was_truncated) = counter.truncate_to_budget(text, 10);
        assert!(was_truncated);
        assert!(truncated.ends_with("..."));
        assert!(counter.count(&truncated) <= 15); // Allow some margin
    }

    #[test]
    fn test_token_counter_no_truncate_needed() {
        let counter = TokenCounter::default();
        let text = "Short text";
        
        let (result, was_truncated) = counter.truncate_to_budget(text, 100);
        assert!(!was_truncated);
        assert_eq!(result, text);
    }

    #[test]
    fn test_convenience_functions() {
        assert_eq!(count_tokens(""), 0);
        assert!(count_tokens("Hello world") > 0);
        assert!(exceeds_token_budget("Hello world", 1));
        assert!(!exceeds_token_budget("Hi", 10));
    }

    #[test]
    fn test_llama_tuned_counter() {
        let counter = TokenCounter::llama_tuned();
        // Llama tuned should give slightly different results
        let text = "This is a test sentence.";
        let default_tokens = TokenCounter::default().count(text);
        let llama_tokens = counter.count(text);
        // Should be in similar range
        assert!(llama_tokens > 0);
        assert!((default_tokens as i32 - llama_tokens as i32).abs() <= 3);
    }
}
