use serde::{Deserialize, Serialize};

use crate::application::services::WorkflowService;
use crate::domain::entities::{
    InputDefault, PromptMapping, WorkflowAnalysis, WorkflowConfiguration, WorkflowSlot,
};

/// Response for a single workflow configuration.
#[derive(Debug, Serialize)]
pub struct WorkflowConfigResponseDto {
    pub id: String,
    pub slot: String,
    pub slot_display_name: String,
    pub name: String,
    pub node_count: usize,
    pub input_count: usize,
    pub prompt_mappings: Vec<PromptMapping>,
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
            prompt_mappings: config.prompt_mappings.clone(),
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
    pub prompt_mappings: Vec<PromptMapping>,
    #[serde(default)]
    pub input_defaults: Vec<InputDefault>,
    #[serde(default)]
    pub locked_inputs: Vec<String>,
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
    pub prompt_mappings: Vec<PromptMapping>,
    pub input_defaults: Vec<InputDefault>,
    pub locked_inputs: Vec<String>,
    pub analysis: WorkflowAnalysis,
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
            prompt_mappings: config.prompt_mappings.clone(),
            input_defaults: config.input_defaults.clone(),
            locked_inputs: config.locked_inputs.clone(),
            analysis,
            created_at: config.created_at.to_rfc3339(),
            updated_at: config.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct WorkflowAnalysisResponseDto {
    pub is_valid: bool,
    pub analysis: WorkflowAnalysis,
    pub suggested_prompt_mappings: Vec<PromptMapping>,
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

pub fn parse_workflow_slot(slot: &str) -> Result<WorkflowSlot, String> {
    WorkflowSlot::from_str(slot).ok_or_else(|| format!("Invalid slot: {}", slot))
}

