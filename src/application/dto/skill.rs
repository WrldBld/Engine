use serde::{Deserialize, Serialize};

use crate::domain::entities::{Skill, SkillCategory};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SkillCategoryDto {
    Physical,
    Mental,
    Social,
    Interpersonal,
    Investigation,
    Academic,
    Practical,
    Combat,
    Approach,
    Aspect,
    Other,
    Custom,
}

impl From<SkillCategoryDto> for SkillCategory {
    fn from(value: SkillCategoryDto) -> Self {
        match value {
            SkillCategoryDto::Physical => SkillCategory::Physical,
            SkillCategoryDto::Mental => SkillCategory::Mental,
            SkillCategoryDto::Social => SkillCategory::Social,
            SkillCategoryDto::Interpersonal => SkillCategory::Interpersonal,
            SkillCategoryDto::Investigation => SkillCategory::Investigation,
            SkillCategoryDto::Academic => SkillCategory::Academic,
            SkillCategoryDto::Practical => SkillCategory::Practical,
            SkillCategoryDto::Combat => SkillCategory::Combat,
            SkillCategoryDto::Approach => SkillCategory::Approach,
            SkillCategoryDto::Aspect => SkillCategory::Aspect,
            SkillCategoryDto::Other => SkillCategory::Other,
            SkillCategoryDto::Custom => SkillCategory::Custom,
        }
    }
}

impl From<SkillCategory> for SkillCategoryDto {
    fn from(value: SkillCategory) -> Self {
        match value {
            SkillCategory::Physical => SkillCategoryDto::Physical,
            SkillCategory::Mental => SkillCategoryDto::Mental,
            SkillCategory::Social => SkillCategoryDto::Social,
            SkillCategory::Interpersonal => SkillCategoryDto::Interpersonal,
            SkillCategory::Investigation => SkillCategoryDto::Investigation,
            SkillCategory::Academic => SkillCategoryDto::Academic,
            SkillCategory::Practical => SkillCategoryDto::Practical,
            SkillCategory::Combat => SkillCategoryDto::Combat,
            SkillCategory::Approach => SkillCategoryDto::Approach,
            SkillCategory::Aspect => SkillCategoryDto::Aspect,
            SkillCategory::Other => SkillCategoryDto::Other,
            SkillCategory::Custom => SkillCategoryDto::Custom,
        }
    }
}

/// Request to create a custom skill.
#[derive(Debug, Deserialize)]
pub struct CreateSkillRequestDto {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub category: SkillCategoryDto,
    pub base_attribute: Option<String>,
}

/// Request to update a skill.
#[derive(Debug, Deserialize)]
pub struct UpdateSkillRequestDto {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub category: Option<SkillCategoryDto>,
    #[serde(default)]
    pub base_attribute: Option<String>,
    #[serde(default)]
    pub is_hidden: Option<bool>,
    #[serde(default)]
    pub order: Option<u32>,
}

/// Skill response.
#[derive(Debug, Serialize)]
pub struct SkillResponseDto {
    pub id: String,
    pub world_id: String,
    pub name: String,
    pub description: String,
    pub category: SkillCategoryDto,
    pub base_attribute: Option<String>,
    pub is_custom: bool,
    pub is_hidden: bool,
    pub order: u32,
}

impl From<Skill> for SkillResponseDto {
    fn from(skill: Skill) -> Self {
        Self {
            id: skill.id.to_string(),
            world_id: skill.world_id.to_string(),
            name: skill.name,
            description: skill.description,
            category: skill.category.into(),
            base_attribute: skill.base_attribute,
            is_custom: skill.is_custom,
            is_hidden: skill.is_hidden,
            order: skill.order,
        }
    }
}

