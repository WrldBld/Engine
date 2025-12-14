use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::entities::{
    CharacterSheetTemplate, FieldType, ItemListType, SectionLayout, SelectOption, SheetField,
    SheetSection,
};
use crate::domain::value_objects::{RuleSystemVariant, WorldId};

/// Response for a sheet template.
#[derive(Debug, Serialize)]
pub struct SheetTemplateResponseDto {
    pub id: String,
    pub world_id: String,
    pub name: String,
    pub description: String,
    pub variant: String,
    pub sections: Vec<SheetSectionDto>,
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
            sections: template.sections.into_iter().map(Into::into).collect(),
            is_default: template.is_default,
        }
    }
}

// ============================================================================
// Persistence DTO (Neo4j stores template JSON)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SheetTemplateStorageDto {
    pub id: String,
    pub world_id: String,
    pub name: String,
    pub description: String,
    pub variant: serde_json::Value,
    pub sections: Vec<SheetSectionDto>,
    pub is_default: bool,
}

impl SheetTemplateStorageDto {
    fn variant_to_json(variant: &RuleSystemVariant) -> serde_json::Value {
        match variant {
            RuleSystemVariant::Dnd5e => serde_json::Value::String("Dnd5e".to_string()),
            RuleSystemVariant::Pathfinder2e => serde_json::Value::String("Pathfinder2e".to_string()),
            RuleSystemVariant::GenericD20 => serde_json::Value::String("GenericD20".to_string()),
            RuleSystemVariant::CallOfCthulhu7e => {
                serde_json::Value::String("CallOfCthulhu7e".to_string())
            }
            RuleSystemVariant::RuneQuest => serde_json::Value::String("RuneQuest".to_string()),
            RuleSystemVariant::GenericD100 => serde_json::Value::String("GenericD100".to_string()),
            RuleSystemVariant::KidsOnBikes => serde_json::Value::String("KidsOnBikes".to_string()),
            RuleSystemVariant::FateCore => serde_json::Value::String("FateCore".to_string()),
            RuleSystemVariant::PoweredByApocalypse => {
                serde_json::Value::String("PoweredByApocalypse".to_string())
            }
            RuleSystemVariant::Custom(name) => serde_json::json!({ "Custom": name }),
        }
    }

    fn variant_from_json(value: serde_json::Value) -> RuleSystemVariant {
        match value {
            serde_json::Value::String(s) => match s.as_str() {
                "Dnd5e" => RuleSystemVariant::Dnd5e,
                "Pathfinder2e" => RuleSystemVariant::Pathfinder2e,
                "GenericD20" => RuleSystemVariant::GenericD20,
                "CallOfCthulhu7e" => RuleSystemVariant::CallOfCthulhu7e,
                "RuneQuest" => RuleSystemVariant::RuneQuest,
                "GenericD100" => RuleSystemVariant::GenericD100,
                "KidsOnBikes" => RuleSystemVariant::KidsOnBikes,
                "FateCore" => RuleSystemVariant::FateCore,
                "PoweredByApocalypse" => RuleSystemVariant::PoweredByApocalypse,
                other => RuleSystemVariant::Custom(other.to_string()),
            },
            serde_json::Value::Object(map) => {
                if let Some(serde_json::Value::String(name)) = map.get("Custom") {
                    RuleSystemVariant::Custom(name.clone())
                } else {
                    RuleSystemVariant::Custom(serde_json::Value::Object(map).to_string())
                }
            }
            other => RuleSystemVariant::Custom(other.to_string()),
        }
    }
}

impl From<&CharacterSheetTemplate> for SheetTemplateStorageDto {
    fn from(value: &CharacterSheetTemplate) -> Self {
        Self {
            id: value.id.0.clone(),
            world_id: value.world_id.to_string(),
            name: value.name.clone(),
            description: value.description.clone(),
            variant: Self::variant_to_json(&value.variant),
            sections: value.sections.clone().into_iter().map(Into::into).collect(),
            is_default: value.is_default,
        }
    }
}

impl TryFrom<SheetTemplateStorageDto> for CharacterSheetTemplate {
    type Error = anyhow::Error;

    fn try_from(value: SheetTemplateStorageDto) -> anyhow::Result<Self> {
        let world_uuid = Uuid::parse_str(&value.world_id)?;
        let world_id = WorldId::from_uuid(world_uuid);

        Ok(Self {
            id: crate::domain::entities::SheetTemplateId::from_string(value.id),
            world_id,
            name: value.name,
            description: value.description,
            variant: SheetTemplateStorageDto::variant_from_json(value.variant),
            sections: value.sections.into_iter().map(Into::into).collect(),
            is_default: value.is_default,
        })
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
    pub layout: Option<SectionLayoutDto>,
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
    pub field_type: FieldTypeDto,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub read_only: bool,
}

// ============================================================================
// Nested DTOs (so we can remove serde from domain)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SheetSectionDto {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub fields: Vec<SheetFieldDto>,
    pub layout: SectionLayoutDto,
    pub collapsible: bool,
    pub collapsed_by_default: bool,
    pub order: u32,
}

impl From<SheetSection> for SheetSectionDto {
    fn from(value: SheetSection) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
            fields: value.fields.into_iter().map(Into::into).collect(),
            layout: value.layout.into(),
            collapsible: value.collapsible,
            collapsed_by_default: value.collapsed_by_default,
            order: value.order,
        }
    }
}

impl From<SheetSectionDto> for SheetSection {
    fn from(value: SheetSectionDto) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
            fields: value.fields.into_iter().map(Into::into).collect(),
            layout: value.layout.into(),
            collapsible: value.collapsible,
            collapsed_by_default: value.collapsed_by_default,
            order: value.order,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SheetFieldDto {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub field_type: FieldTypeDto,
    pub required: bool,
    pub read_only: bool,
    pub order: u32,
}

impl From<SheetField> for SheetFieldDto {
    fn from(value: SheetField) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
            field_type: value.field_type.into(),
            required: value.required,
            read_only: value.read_only,
            order: value.order,
        }
    }
}

impl From<SheetFieldDto> for SheetField {
    fn from(value: SheetFieldDto) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
            field_type: value.field_type.into(),
            required: value.required,
            read_only: value.read_only,
            order: value.order,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SectionLayoutDto {
    Vertical,
    Grid { columns: u8 },
    Flow,
    TwoColumn,
}

impl From<SectionLayout> for SectionLayoutDto {
    fn from(value: SectionLayout) -> Self {
        match value {
            SectionLayout::Vertical => Self::Vertical,
            SectionLayout::Grid { columns } => Self::Grid { columns },
            SectionLayout::Flow => Self::Flow,
            SectionLayout::TwoColumn => Self::TwoColumn,
        }
    }
}

impl From<SectionLayoutDto> for SectionLayout {
    fn from(value: SectionLayoutDto) -> Self {
        match value {
            SectionLayoutDto::Vertical => Self::Vertical,
            SectionLayoutDto::Grid { columns } => Self::Grid { columns },
            SectionLayoutDto::Flow => Self::Flow,
            SectionLayoutDto::TwoColumn => Self::TwoColumn,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectOptionDto {
    pub value: String,
    pub label: String,
    pub description: Option<String>,
}

impl From<SelectOption> for SelectOptionDto {
    fn from(value: SelectOption) -> Self {
        Self {
            value: value.value,
            label: value.label,
            description: value.description,
        }
    }
}

impl From<SelectOptionDto> for SelectOption {
    fn from(value: SelectOptionDto) -> Self {
        Self {
            value: value.value,
            label: value.label,
            description: value.description,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemListTypeDto {
    Inventory,
    Features,
    Spells,
    Notes,
}

impl From<ItemListType> for ItemListTypeDto {
    fn from(value: ItemListType) -> Self {
        match value {
            ItemListType::Inventory => Self::Inventory,
            ItemListType::Features => Self::Features,
            ItemListType::Spells => Self::Spells,
            ItemListType::Notes => Self::Notes,
        }
    }
}

impl From<ItemListTypeDto> for ItemListType {
    fn from(value: ItemListTypeDto) -> Self {
        match value {
            ItemListTypeDto::Inventory => Self::Inventory,
            ItemListTypeDto::Features => Self::Features,
            ItemListTypeDto::Spells => Self::Spells,
            ItemListTypeDto::Notes => Self::Notes,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FieldTypeDto {
    Number {
        min: Option<i32>,
        max: Option<i32>,
        default: Option<i32>,
    },
    Text {
        multiline: bool,
        max_length: Option<usize>,
    },
    Checkbox {
        default: bool,
    },
    Select {
        options: Vec<SelectOptionDto>,
    },
    SkillReference {
        categories: Option<Vec<String>>,
        show_attribute: bool,
    },
    Derived {
        formula: String,
        depends_on: Vec<String>,
    },
    Resource {
        max_field: Option<String>,
        default_max: Option<i32>,
    },
    ItemList {
        item_type: ItemListTypeDto,
        max_items: Option<usize>,
    },
    SkillList {
        show_modifier: bool,
        show_proficiency: bool,
    },
}

impl From<FieldType> for FieldTypeDto {
    fn from(value: FieldType) -> Self {
        match value {
            FieldType::Number { min, max, default } => Self::Number { min, max, default },
            FieldType::Text {
                multiline,
                max_length,
            } => Self::Text {
                multiline,
                max_length,
            },
            FieldType::Checkbox { default } => Self::Checkbox { default },
            FieldType::Select { options } => Self::Select {
                options: options.into_iter().map(Into::into).collect(),
            },
            FieldType::SkillReference {
                categories,
                show_attribute,
            } => Self::SkillReference {
                categories,
                show_attribute,
            },
            FieldType::Derived { formula, depends_on } => Self::Derived { formula, depends_on },
            FieldType::Resource {
                max_field,
                default_max,
            } => Self::Resource {
                max_field,
                default_max,
            },
            FieldType::ItemList {
                item_type,
                max_items,
            } => Self::ItemList {
                item_type: item_type.into(),
                max_items,
            },
            FieldType::SkillList {
                show_modifier,
                show_proficiency,
            } => Self::SkillList {
                show_modifier,
                show_proficiency,
            },
        }
    }
}

impl From<FieldTypeDto> for FieldType {
    fn from(value: FieldTypeDto) -> Self {
        match value {
            FieldTypeDto::Number { min, max, default } => Self::Number { min, max, default },
            FieldTypeDto::Text {
                multiline,
                max_length,
            } => Self::Text {
                multiline,
                max_length,
            },
            FieldTypeDto::Checkbox { default } => Self::Checkbox { default },
            FieldTypeDto::Select { options } => Self::Select {
                options: options.into_iter().map(Into::into).collect(),
            },
            FieldTypeDto::SkillReference {
                categories,
                show_attribute,
            } => Self::SkillReference {
                categories,
                show_attribute,
            },
            FieldTypeDto::Derived { formula, depends_on } => Self::Derived { formula, depends_on },
            FieldTypeDto::Resource {
                max_field,
                default_max,
            } => Self::Resource {
                max_field,
                default_max,
            },
            FieldTypeDto::ItemList {
                item_type,
                max_items,
            } => Self::ItemList {
                item_type: item_type.into(),
                max_items,
            },
            FieldTypeDto::SkillList {
                show_modifier,
                show_proficiency,
            } => Self::SkillList {
                show_modifier,
                show_proficiency,
            },
        }
    }
}

