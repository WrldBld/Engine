use serde::{Deserialize, Serialize};

use crate::domain::entities::{
    CharacterSheetTemplate, FieldType, SectionLayout, SheetSection,
};

/// Response for a sheet template.
#[derive(Debug, Serialize)]
pub struct SheetTemplateResponseDto {
    pub id: String,
    pub world_id: String,
    pub name: String,
    pub description: String,
    pub variant: String,
    pub sections: Vec<SheetSection>,
    pub is_default: bool,
}

impl From<CharacterSheetTemplate> for SheetTemplateResponseDto {
    fn from(template: CharacterSheetTemplate) -> Self {
        Self {
            id: template.id.0,
            world_id: template.world_id.to_string(),
            name: template.name,
            description: template.description,
            variant: format!("{:?}", template.variant),
            sections: template.sections,
            is_default: template.is_default,
        }
    }
}

/// Summary response (without sections).
#[derive(Debug, Serialize)]
pub struct SheetTemplateSummaryDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub is_default: bool,
    pub section_count: usize,
    pub field_count: usize,
}

impl From<CharacterSheetTemplate> for SheetTemplateSummaryDto {
    fn from(template: CharacterSheetTemplate) -> Self {
        let field_count: usize = template.sections.iter().map(|s| s.fields.len()).sum();
        Self {
            id: template.id.0,
            name: template.name,
            description: template.description,
            is_default: template.is_default,
            section_count: template.sections.len(),
            field_count,
        }
    }
}

/// Request to create a custom section.
#[derive(Debug, Deserialize)]
pub struct CreateSectionRequestDto {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub layout: Option<SectionLayout>,
    #[serde(default)]
    pub collapsible: bool,
    #[serde(default)]
    pub collapsed_by_default: bool,
}

/// Request to create a custom field.
#[derive(Debug, Deserialize)]
pub struct CreateFieldRequestDto {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub field_type: FieldType,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub read_only: bool,
}

