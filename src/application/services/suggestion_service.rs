//! Suggestion Service - LLM-powered content suggestions for world-building
//!
//! This service provides AI-generated suggestions for:
//! - Character names, descriptions, wants, fears, backstories
//! - Location names, descriptions, atmosphere, features
//! - Plot hooks and story connections

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::application::ports::outbound::{ChatMessage, LlmPort, LlmRequest, MessageRole};

/// Service for generating content suggestions
pub struct SuggestionService<L: LlmPort> {
    llm: L,
}

impl<L: LlmPort> SuggestionService<L> {
    /// Create a new suggestion service
    pub fn new(llm: L) -> Self {
        Self { llm }
    }

    /// Generate character name suggestions
    pub async fn suggest_character_names(
        &self,
        context: &SuggestionContext,
    ) -> Result<Vec<String>> {
        let prompt = format!(
            "Generate 5 unique character names for a {} character in a {} setting. \
            The character archetype is: {}. \
            Return only the names, one per line, no numbering or explanations.",
            context.entity_type.as_deref().unwrap_or("fantasy"),
            context.world_setting.as_deref().unwrap_or("fantasy"),
            context.hints.as_deref().unwrap_or("heroic adventurer")
        );

        self.generate_list(&prompt, 5).await
    }

    /// Generate character description suggestions
    pub async fn suggest_character_description(
        &self,
        context: &SuggestionContext,
    ) -> Result<Vec<String>> {
        let name = context.entity_name.as_deref().unwrap_or("this character");
        let prompt = format!(
            "Generate 3 different physical descriptions for {}. \
            Setting: {}. Archetype: {}. \
            Each description should be 2-3 sentences covering appearance, mannerisms, and voice. \
            Return each description on its own line, separated by blank lines.",
            name,
            context.world_setting.as_deref().unwrap_or("fantasy"),
            context.hints.as_deref().unwrap_or("mysterious stranger")
        );

        self.generate_list(&prompt, 3).await
    }

    /// Generate character wants/desires suggestions
    pub async fn suggest_character_wants(&self, context: &SuggestionContext) -> Result<Vec<String>> {
        let name = context.entity_name.as_deref().unwrap_or("this character");
        let prompt = format!(
            "Generate 4 different character motivations/wants for {}. \
            Archetype: {}. Description: {}. \
            Each want should be a single sentence describing what the character desires. \
            Return each want on its own line.",
            name,
            context.hints.as_deref().unwrap_or("adventurer"),
            context.additional_context.as_deref().unwrap_or("a mysterious character")
        );

        self.generate_list(&prompt, 4).await
    }

    /// Generate character fears suggestions
    pub async fn suggest_character_fears(&self, context: &SuggestionContext) -> Result<Vec<String>> {
        let name = context.entity_name.as_deref().unwrap_or("this character");
        let prompt = format!(
            "Generate 4 different fears for {}. \
            Archetype: {}. Wants: {}. \
            Each fear should be a single sentence. Consider fears that create interesting dramatic tension. \
            Return each fear on its own line.",
            name,
            context.hints.as_deref().unwrap_or("adventurer"),
            context.additional_context.as_deref().unwrap_or("power and glory")
        );

        self.generate_list(&prompt, 4).await
    }

    /// Generate character backstory suggestions
    pub async fn suggest_character_backstory(
        &self,
        context: &SuggestionContext,
    ) -> Result<Vec<String>> {
        let name = context.entity_name.as_deref().unwrap_or("this character");
        let prompt = format!(
            "Generate 2 different backstory options for {}. \
            Archetype: {}. Wants: {}. Fears: {}. \
            Each backstory should be 3-4 sentences covering origin, key events, and how they became who they are. \
            Return each backstory separated by a blank line.",
            name,
            context.hints.as_deref().unwrap_or("adventurer"),
            context.additional_context.as_deref().unwrap_or("success"),
            context.world_setting.as_deref().unwrap_or("failure")
        );

        self.generate_list(&prompt, 2).await
    }

    /// Generate location name suggestions
    pub async fn suggest_location_names(&self, context: &SuggestionContext) -> Result<Vec<String>> {
        let loc_type = context.entity_type.as_deref().unwrap_or("tavern");
        let prompt = format!(
            "Generate 5 unique names for a {} in a {} setting. \
            Names should be evocative and memorable. \
            Return only the names, one per line.",
            loc_type,
            context.world_setting.as_deref().unwrap_or("fantasy")
        );

        self.generate_list(&prompt, 5).await
    }

    /// Generate location description suggestions
    pub async fn suggest_location_description(
        &self,
        context: &SuggestionContext,
    ) -> Result<Vec<String>> {
        let name = context.entity_name.as_deref().unwrap_or("this location");
        let loc_type = context.entity_type.as_deref().unwrap_or("location");
        let prompt = format!(
            "Generate 3 different descriptions for {} (a {}). \
            Setting: {}. \
            Each description should be 2-3 sentences covering what stands out visually, sounds, and smells. \
            Return each description separated by a blank line.",
            name,
            loc_type,
            context.world_setting.as_deref().unwrap_or("fantasy")
        );

        self.generate_list(&prompt, 3).await
    }

    /// Generate location atmosphere suggestions
    pub async fn suggest_location_atmosphere(
        &self,
        context: &SuggestionContext,
    ) -> Result<Vec<String>> {
        let name = context.entity_name.as_deref().unwrap_or("this location");
        let prompt = format!(
            "Generate 4 different atmosphere/mood options for {}. \
            Location type: {}. Description: {}. \
            Each should be a short phrase (2-5 words) capturing the feel. \
            Examples: 'Tense and watchful', 'Cozy but cluttered', 'Eerily silent'. \
            Return each atmosphere on its own line.",
            name,
            context.entity_type.as_deref().unwrap_or("location"),
            context.additional_context.as_deref().unwrap_or("a mysterious place")
        );

        self.generate_list(&prompt, 4).await
    }

    /// Generate location notable features
    pub async fn suggest_location_features(
        &self,
        context: &SuggestionContext,
    ) -> Result<Vec<String>> {
        let name = context.entity_name.as_deref().unwrap_or("this location");
        let prompt = format!(
            "Generate 5 notable features or points of interest for {}. \
            Location type: {}. Atmosphere: {}. \
            Each should be a single sentence describing something players might interact with or notice. \
            Return each feature on its own line.",
            name,
            context.entity_type.as_deref().unwrap_or("location"),
            context.hints.as_deref().unwrap_or("mysterious")
        );

        self.generate_list(&prompt, 5).await
    }

    /// Generate location hidden secrets
    pub async fn suggest_location_secrets(
        &self,
        context: &SuggestionContext,
    ) -> Result<Vec<String>> {
        let name = context.entity_name.as_deref().unwrap_or("this location");
        let prompt = format!(
            "Generate 3 hidden secrets that could be discovered in {}. \
            Location type: {}. Features: {}. \
            Each secret should be something players might find with investigation, 1-2 sentences. \
            Return each secret separated by a blank line.",
            name,
            context.entity_type.as_deref().unwrap_or("location"),
            context.additional_context.as_deref().unwrap_or("various objects")
        );

        self.generate_list(&prompt, 3).await
    }

    /// Internal method to generate a list of suggestions
    async fn generate_list(&self, prompt: &str, expected_count: usize) -> Result<Vec<String>> {
        let request = LlmRequest::new(vec![ChatMessage {
            role: MessageRole::User,
            content: prompt.to_string(),
        }])
        .with_temperature(0.9) // High creativity for suggestions
        .with_max_tokens(Some(1000));

        let response = self
            .llm
            .generate(request)
            .await
            .map_err(|e| anyhow::anyhow!("LLM error: {}", e))?;

        // Parse the response into individual suggestions
        let suggestions: Vec<String> = response
            .content
            .split('\n')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| {
                // Remove common list prefixes
                let s = s
                    .trim_start_matches(|c: char| c.is_numeric() || c == '.' || c == ')' || c == '-');
                s.trim().to_string()
            })
            .filter(|s| !s.is_empty())
            .take(expected_count)
            .collect();

        Ok(suggestions)
    }
}

/// Context for generating suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionContext {
    /// Type of entity (e.g., "character", "location", "tavern", "forest")
    pub entity_type: Option<String>,
    /// Name of the entity (if already set)
    pub entity_name: Option<String>,
    /// World/setting name or type
    pub world_setting: Option<String>,
    /// Hints or keywords to guide generation
    pub hints: Option<String>,
    /// Additional context from other fields
    pub additional_context: Option<String>,
}

impl Default for SuggestionContext {
    fn default() -> Self {
        Self {
            entity_type: None,
            entity_name: None,
            world_setting: Some("fantasy".to_string()),
            hints: None,
            additional_context: None,
        }
    }
}

/// Request for a suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionRequest {
    /// Type of suggestion to generate
    pub suggestion_type: SuggestionType,
    /// Context for the suggestion
    pub context: SuggestionContext,
}

/// Types of suggestions available
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionType {
    CharacterName,
    CharacterDescription,
    CharacterWants,
    CharacterFears,
    CharacterBackstory,
    LocationName,
    LocationDescription,
    LocationAtmosphere,
    LocationFeatures,
    LocationSecrets,
}

impl SuggestionType {
    /// Convert to the field type string used in LLM queue routing
    pub fn to_field_type(&self) -> &'static str {
        match self {
            SuggestionType::CharacterName => "character_name",
            SuggestionType::CharacterDescription => "character_description",
            SuggestionType::CharacterWants => "character_wants",
            SuggestionType::CharacterFears => "character_fears",
            SuggestionType::CharacterBackstory => "character_backstory",
            SuggestionType::LocationName => "location_name",
            SuggestionType::LocationDescription => "location_description",
            SuggestionType::LocationAtmosphere => "location_atmosphere",
            SuggestionType::LocationFeatures => "location_features",
            SuggestionType::LocationSecrets => "location_secrets",
        }
    }
}

/// Response containing suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionResponse {
    /// The type of suggestion that was generated
    pub suggestion_type: SuggestionType,
    /// The generated suggestions
    pub suggestions: Vec<String>,
}
