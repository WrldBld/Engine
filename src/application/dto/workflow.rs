use serde::{Deserialize, Serialize};

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::application::services::WorkflowService;
use crate::domain::entities::{
    InputDefault, InputType, PromptMapping, PromptMappingType, WorkflowAnalysis, WorkflowConfiguration,
    WorkflowInput, WorkflowSlot,
};
use crate::domain::value_objects::WorkflowConfigId;

// ============================================================================
// DTOs for workflow configuration + analysis payloads
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptMappingDto {
    pub node_id: String,
    pub input_name: String,
    pub mapping_type: PromptMappingTypeDto,
}

impl From<PromptMapping> for PromptMappingDto {
    fn from(value: PromptMapping) -> Self {
        Self {
            node_id: value.node_id,
            input_name: value.input_name,
            mapping_type: value.mapping_type.into(),
        }
    }
}

impl From<PromptMappingDto> for PromptMapping {
    fn from(value: PromptMappingDto) -> Self {
        Self {
            node_id: value.node_id,
            input_name: value.input_name,
            mapping_type: value.mapping_type.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptMappingTypeDto {
    Primary,
    Negative,
}

impl From<PromptMappingType> for PromptMappingTypeDto {
    fn from(value: PromptMappingType) -> Self {
        match value {
            PromptMappingType::Primary => Self::Primary,
            PromptMappingType::Negative => Self::Negative,
        }
    }
}

impl From<PromptMappingTypeDto> for PromptMappingType {
    fn from(value: PromptMappingTypeDto) -> Self {
        match value {
            PromptMappingTypeDto::Primary => Self::Primary,
            PromptMappingTypeDto::Negative => Self::Negative,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputDefaultDto {
    pub node_id: String,
    pub input_name: String,
    pub default_value: serde_json::Value,
}

impl From<InputDefault> for InputDefaultDto {
    fn from(value: InputDefault) -> Self {
        Self {
            node_id: value.node_id,
            input_name: value.input_name,
            default_value: value.default_value,
        }
    }
}

impl From<InputDefaultDto> for InputDefault {
    fn from(value: InputDefaultDto) -> Self {
        Self {
            node_id: value.node_id,
            input_name: value.input_name,
            default_value: value.default_value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputTypeDto {
    Text,
    Integer,
    Float,
    Boolean,
    Select(Vec<String>),
    Unknown,
}

impl From<InputType> for InputTypeDto {
    fn from(value: InputType) -> Self {
        match value {
            InputType::Text => Self::Text,
            InputType::Integer => Self::Integer,
            InputType::Float => Self::Float,
            InputType::Boolean => Self::Boolean,
            InputType::Select(opts) => Self::Select(opts),
            InputType::Unknown => Self::Unknown,
        }
    }
}

impl From<InputTypeDto> for InputType {
    fn from(value: InputTypeDto) -> Self {
        match value {
            InputTypeDto::Text => Self::Text,
            InputTypeDto::Integer => Self::Integer,
            InputTypeDto::Float => Self::Float,
            InputTypeDto::Boolean => Self::Boolean,
            InputTypeDto::Select(opts) => Self::Select(opts),
            InputTypeDto::Unknown => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInputDto {
    pub node_id: String,
    pub node_type: String,
    pub node_title: Option<String>,
    pub input_name: String,
    pub input_type: InputTypeDto,
    pub current_value: serde_json::Value,
}

impl From<WorkflowInput> for WorkflowInputDto {
    fn from(value: WorkflowInput) -> Self {
        Self {
            node_id: value.node_id,
            node_type: value.node_type,
            node_title: value.node_title,
            input_name: value.input_name,
            input_type: value.input_type.into(),
            current_value: value.current_value,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowAnalysisDto {
    pub node_count: usize,
    pub inputs: Vec<WorkflowInputDto>,
    pub text_inputs: Vec<WorkflowInputDto>,
    pub errors: Vec<String>,
}

impl From<WorkflowAnalysis> for WorkflowAnalysisDto {
    fn from(value: WorkflowAnalysis) -> Self {
        Self {
            node_count: value.node_count,
            inputs: value.inputs.into_iter().map(WorkflowInputDto::from).collect(),
            text_inputs: value
                .text_inputs
                .into_iter()
                .map(WorkflowInputDto::from)
                .collect(),
            errors: value.errors,
        }
    }
}

/// Response for a single workflow configuration.
#[derive(Debug, Serialize)]
pub struct WorkflowConfigResponseDto {
    pub id: String,
    pub slot: String,
    pub slot_display_name: String,
    pub name: String,
    pub node_count: usize,
    pub input_count: usize,
    pub prompt_mappings: Vec<PromptMappingDto>,
    pub has_primary_prompt: bool,
    pub has_negative_prompt: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<&WorkflowConfiguration> for WorkflowConfigResponseDto {
    fn from(config: &WorkflowConfiguration) -> Self {
        let analysis = WorkflowService::analyze_workflow(&config.workflow_json);
        Self {
            id: config.id.to_string(),
            slot: config.slot.as_str().to_string(),
            slot_display_name: config.slot.display_name().to_string(),
            name: config.name.clone(),
            node_count: analysis.node_count,
            input_count: analysis.inputs.len(),
            prompt_mappings: config
                .prompt_mappings
                .clone()
                .into_iter()
                .map(Into::into)
                .collect(),
            has_primary_prompt: config.primary_prompt_mapping().is_some(),
            has_negative_prompt: config.negative_prompt_mapping().is_some(),
            created_at: config.created_at.to_rfc3339(),
            updated_at: config.updated_at.to_rfc3339(),
        }
    }
}

/// Response for workflow slot status.
#[derive(Debug, Serialize)]
pub struct WorkflowSlotStatusDto {
    pub slot: String,
    pub display_name: String,
    pub default_width: u32,
    pub default_height: u32,
    pub configured: bool,
    pub config: Option<WorkflowConfigResponseDto>,
}

/// A category of workflow slots (e.g., "Character Assets").
#[derive(Debug, Serialize)]
pub struct WorkflowSlotCategoryDto {
    pub name: String,
    pub slots: Vec<WorkflowSlotStatusDto>,
}

/// Response containing all workflow slots grouped by category.
#[derive(Debug, Serialize)]
pub struct WorkflowSlotsResponseDto {
    pub categories: Vec<WorkflowSlotCategoryDto>,
}

/// Request to create/update a workflow configuration.
#[derive(Debug, Deserialize)]
pub struct CreateWorkflowConfigRequestDto {
    pub name: String,
    pub workflow_json: serde_json::Value,
    #[serde(default)]
    pub prompt_mappings: Vec<PromptMappingDto>,
    #[serde(default)]
    pub input_defaults: Vec<InputDefaultDto>,
    #[serde(default)]
    pub locked_inputs: Vec<String>,
}

/// Request to update just the defaults of a workflow (without re-uploading the workflow JSON).
#[derive(Debug, Deserialize)]
pub struct UpdateWorkflowDefaultsRequestDto {
    #[serde(default)]
    pub input_defaults: Vec<InputDefaultDto>,
    #[serde(default)]
    pub locked_inputs: Option<Vec<String>>,
}

/// Request to analyze a workflow (without saving).
#[derive(Debug, Deserialize)]
pub struct AnalyzeWorkflowRequestDto {
    pub workflow_json: serde_json::Value,
}

/// Full configuration response (includes workflow JSON).
#[derive(Debug, Serialize)]
pub struct WorkflowConfigFullResponseDto {
    pub id: String,
    pub slot: String,
    pub slot_display_name: String,
    pub name: String,
    pub workflow_json: serde_json::Value,
    pub prompt_mappings: Vec<PromptMappingDto>,
    pub input_defaults: Vec<InputDefaultDto>,
    pub locked_inputs: Vec<String>,
    pub analysis: WorkflowAnalysisDto,
    pub created_at: String,
    pub updated_at: String,
}

impl From<&WorkflowConfiguration> for WorkflowConfigFullResponseDto {
    fn from(config: &WorkflowConfiguration) -> Self {
        let analysis = WorkflowService::analyze_workflow(&config.workflow_json);
        Self {
            id: config.id.to_string(),
            slot: config.slot.as_str().to_string(),
            slot_display_name: config.slot.display_name().to_string(),
            name: config.name.clone(),
            workflow_json: config.workflow_json.clone(),
            prompt_mappings: config
                .prompt_mappings
                .clone()
                .into_iter()
                .map(Into::into)
                .collect(),
            input_defaults: config
                .input_defaults
                .clone()
                .into_iter()
                .map(Into::into)
                .collect(),
            locked_inputs: config.locked_inputs.clone(),
            analysis: analysis.into(),
            created_at: config.created_at.to_rfc3339(),
            updated_at: config.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct WorkflowAnalysisResponseDto {
    pub is_valid: bool,
    pub analysis: WorkflowAnalysisDto,
    pub suggested_prompt_mappings: Vec<PromptMappingDto>,
}

/// Import workflow configurations request.
#[derive(Debug, Deserialize)]
pub struct ImportWorkflowsRequestDto {
    pub data: serde_json::Value,
    #[serde(default)]
    pub replace_existing: bool,
}

#[derive(Debug, Serialize)]
pub struct ImportWorkflowsResponseDto {
    pub imported: usize,
    pub skipped: usize,
}

/// Test a workflow configuration request.
#[derive(Debug, Deserialize)]
pub struct TestWorkflowRequestDto {
    pub prompt: String,
    #[serde(default)]
    pub negative_prompt: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TestWorkflowResponseDto {
    pub prompt_id: String,
    pub queue_position: u32,
}

/// Export/import DTO for workflow configurations (backup/restore).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfigExportDto {
    pub id: String,
    pub slot: String,
    pub name: String,
    pub workflow_json: serde_json::Value,
    #[serde(default)]
    pub prompt_mappings: Vec<PromptMappingDto>,
    #[serde(default)]
    pub input_defaults: Vec<InputDefaultDto>,
    #[serde(default)]
    pub locked_inputs: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<WorkflowConfiguration> for WorkflowConfigExportDto {
    fn from(value: WorkflowConfiguration) -> Self {
        Self {
            id: value.id.to_string(),
            slot: value.slot.as_str().to_string(),
            name: value.name,
            workflow_json: value.workflow_json,
            prompt_mappings: value.prompt_mappings.into_iter().map(Into::into).collect(),
            input_defaults: value.input_defaults.into_iter().map(Into::into).collect(),
            locked_inputs: value.locked_inputs,
            created_at: value.created_at.to_rfc3339(),
            updated_at: value.updated_at.to_rfc3339(),
        }
    }
}

impl TryFrom<WorkflowConfigExportDto> for WorkflowConfiguration {
    type Error = anyhow::Error;

    fn try_from(value: WorkflowConfigExportDto) -> anyhow::Result<Self> {
        let id = Uuid::parse_str(&value.id)
            .map(WorkflowConfigId::from_uuid)
            .unwrap_or_else(|_| WorkflowConfigId::new());

        let slot = WorkflowSlot::from_str(&value.slot)
            .ok_or_else(|| anyhow::anyhow!("Invalid workflow slot: {}", value.slot))?;

        let created_at = DateTime::parse_from_rfc3339(&value.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        let updated_at = DateTime::parse_from_rfc3339(&value.updated_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(Self {
            id,
            slot,
            name: value.name,
            workflow_json: value.workflow_json,
            prompt_mappings: value.prompt_mappings.into_iter().map(Into::into).collect(),
            input_defaults: value.input_defaults.into_iter().map(Into::into).collect(),
            locked_inputs: value.locked_inputs,
            created_at,
            updated_at,
        })
    }
}

pub fn parse_workflow_slot(slot: &str) -> Result<WorkflowSlot, String> {
    WorkflowSlot::from_str(slot).ok_or_else(|| format!("Invalid slot: {}", slot))
}

