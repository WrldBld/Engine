//! Workflow configuration REST API routes

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::application::services::WorkflowService;
use crate::domain::entities::{
    InputDefault, PromptMapping, WorkflowAnalysis, WorkflowConfiguration, WorkflowSlot,
};
use crate::infrastructure::state::AppState;

// ============================================================================
// Request/Response DTOs
// ============================================================================

/// Response for a single workflow configuration
#[derive(Debug, Serialize)]
pub struct WorkflowConfigResponse {
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

impl From<&WorkflowConfiguration> for WorkflowConfigResponse {
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

/// Response for workflow slot status
#[derive(Debug, Serialize)]
pub struct WorkflowSlotStatus {
    pub slot: String,
    pub display_name: String,
    pub category: String,
    pub default_width: u32,
    pub default_height: u32,
    pub configured: bool,
    pub config: Option<WorkflowConfigResponse>,
}

/// Request to create/update a workflow configuration
#[derive(Debug, Deserialize)]
pub struct CreateWorkflowConfigRequest {
    pub name: String,
    pub workflow_json: serde_json::Value,
    #[serde(default)]
    pub prompt_mappings: Vec<PromptMapping>,
    #[serde(default)]
    pub input_defaults: Vec<InputDefault>,
    #[serde(default)]
    pub locked_inputs: Vec<String>,
}

/// Request to analyze a workflow (without saving)
#[derive(Debug, Deserialize)]
pub struct AnalyzeWorkflowRequest {
    pub workflow_json: serde_json::Value,
}

/// Full configuration response (includes workflow JSON)
#[derive(Debug, Serialize)]
pub struct WorkflowConfigFullResponse {
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

impl From<&WorkflowConfiguration> for WorkflowConfigFullResponse {
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

// ============================================================================
// Route Handlers
// ============================================================================

/// List all workflow slots with their configuration status
pub async fn list_workflow_slots(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<WorkflowSlotStatus>>, (StatusCode, String)> {
    let configs = state
        .repository
        .workflows()
        .list_all()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut statuses = Vec::new();

    for slot in WorkflowSlot::all() {
        let config = configs.iter().find(|c| c.slot == *slot);
        let (width, height) = slot.default_dimensions();

        statuses.push(WorkflowSlotStatus {
            slot: slot.as_str().to_string(),
            display_name: slot.display_name().to_string(),
            category: slot.category().to_string(),
            default_width: width,
            default_height: height,
            configured: config.is_some(),
            config: config.map(WorkflowConfigResponse::from),
        });
    }

    Ok(Json(statuses))
}

/// Get a workflow configuration by slot
pub async fn get_workflow_config(
    State(state): State<Arc<AppState>>,
    Path(slot): Path<String>,
) -> Result<Json<WorkflowConfigFullResponse>, (StatusCode, String)> {
    let workflow_slot = WorkflowSlot::from_str(&slot)
        .ok_or_else(|| (StatusCode::BAD_REQUEST, format!("Invalid slot: {}", slot)))?;

    let config = state
        .repository
        .workflows()
        .get_by_slot(workflow_slot)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("No workflow configured for slot: {}", slot),
            )
        })?;

    Ok(Json(WorkflowConfigFullResponse::from(&config)))
}

/// Create or update a workflow configuration
pub async fn save_workflow_config(
    State(state): State<Arc<AppState>>,
    Path(slot): Path<String>,
    Json(req): Json<CreateWorkflowConfigRequest>,
) -> Result<(StatusCode, Json<WorkflowConfigFullResponse>), (StatusCode, String)> {
    let workflow_slot = WorkflowSlot::from_str(&slot)
        .ok_or_else(|| (StatusCode::BAD_REQUEST, format!("Invalid slot: {}", slot)))?;

    // Validate the workflow JSON
    WorkflowService::validate_workflow(&req.workflow_json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    // Check if we're updating or creating
    let existing = state
        .repository
        .workflows()
        .get_by_slot(workflow_slot)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let is_update = existing.is_some();

    let config = if let Some(mut existing_config) = existing {
        // Update existing
        existing_config.name = req.name;
        existing_config.update_workflow(req.workflow_json);
        existing_config.set_prompt_mappings(req.prompt_mappings);
        existing_config.set_input_defaults(req.input_defaults);
        existing_config.set_locked_inputs(req.locked_inputs);
        existing_config
    } else {
        // Create new
        let mut config = WorkflowConfiguration::new(workflow_slot, req.name, req.workflow_json);
        config.set_prompt_mappings(req.prompt_mappings);
        config.set_input_defaults(req.input_defaults);
        config.set_locked_inputs(req.locked_inputs);
        config
    };

    state
        .repository
        .workflows()
        .save(&config)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let status = if is_update {
        StatusCode::OK
    } else {
        StatusCode::CREATED
    };

    Ok((status, Json(WorkflowConfigFullResponse::from(&config))))
}

/// Delete a workflow configuration
pub async fn delete_workflow_config(
    State(state): State<Arc<AppState>>,
    Path(slot): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let workflow_slot = WorkflowSlot::from_str(&slot)
        .ok_or_else(|| (StatusCode::BAD_REQUEST, format!("Invalid slot: {}", slot)))?;

    let deleted = state
        .repository
        .workflows()
        .delete_by_slot(workflow_slot)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            format!("No workflow configured for slot: {}", slot),
        ))
    }
}

/// Analyze a workflow JSON without saving
pub async fn analyze_workflow(
    Json(req): Json<AnalyzeWorkflowRequest>,
) -> Result<Json<WorkflowAnalysisResponse>, (StatusCode, String)> {
    // Validate first
    if let Err(e) = WorkflowService::validate_workflow(&req.workflow_json) {
        return Err((StatusCode::BAD_REQUEST, e.to_string()));
    }

    let analysis = WorkflowService::analyze_workflow(&req.workflow_json);
    let auto_mappings = WorkflowService::auto_detect_prompt_mappings(&req.workflow_json);

    Ok(Json(WorkflowAnalysisResponse {
        is_valid: analysis.is_valid(),
        analysis,
        suggested_prompt_mappings: auto_mappings,
    }))
}

#[derive(Debug, Serialize)]
pub struct WorkflowAnalysisResponse {
    pub is_valid: bool,
    pub analysis: WorkflowAnalysis,
    pub suggested_prompt_mappings: Vec<PromptMapping>,
}

/// Export all workflow configurations
pub async fn export_workflows(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let configs = state
        .repository
        .workflows()
        .list_all()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let export = WorkflowService::export_configs(&configs);
    Ok(Json(export))
}

/// Import workflow configurations
#[derive(Debug, Deserialize)]
pub struct ImportWorkflowsRequest {
    pub data: serde_json::Value,
    #[serde(default)]
    pub replace_existing: bool,
}

pub async fn import_workflows(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ImportWorkflowsRequest>,
) -> Result<Json<ImportWorkflowsResponse>, (StatusCode, String)> {
    let configs = WorkflowService::import_configs(&req.data)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let mut imported = 0;
    let mut skipped = 0;

    for config in configs {
        let existing = state
            .repository
            .workflows()
            .get_by_slot(config.slot)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        if existing.is_some() && !req.replace_existing {
            skipped += 1;
            continue;
        }

        state
            .repository
            .workflows()
            .save(&config)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        imported += 1;
    }

    Ok(Json(ImportWorkflowsResponse { imported, skipped }))
}

#[derive(Debug, Serialize)]
pub struct ImportWorkflowsResponse {
    pub imported: usize,
    pub skipped: usize,
}

/// Test a workflow configuration
#[derive(Debug, Deserialize)]
pub struct TestWorkflowRequest {
    pub prompt: String,
    #[serde(default)]
    pub negative_prompt: Option<String>,
}

pub async fn test_workflow(
    State(state): State<Arc<AppState>>,
    Path(slot): Path<String>,
    Json(req): Json<TestWorkflowRequest>,
) -> Result<Json<TestWorkflowResponse>, (StatusCode, String)> {
    let workflow_slot = WorkflowSlot::from_str(&slot)
        .ok_or_else(|| (StatusCode::BAD_REQUEST, format!("Invalid slot: {}", slot)))?;

    let config = state
        .repository
        .workflows()
        .get_by_slot(workflow_slot)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("No workflow configured for slot: {}", slot),
            )
        })?;

    // Prepare the workflow with the test prompt
    let prepared_workflow = WorkflowService::prepare_workflow(
        &config,
        &req.prompt,
        req.negative_prompt.as_deref(),
        &[], // No overrides for test
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Queue the workflow with ComfyUI
    let queue_result = state
        .comfyui_client
        .queue_prompt(prepared_workflow)
        .await
        .map_err(|e| (StatusCode::SERVICE_UNAVAILABLE, e.to_string()))?;

    Ok(Json(TestWorkflowResponse {
        prompt_id: queue_result.prompt_id,
        queue_position: queue_result.number,
    }))
}

#[derive(Debug, Serialize)]
pub struct TestWorkflowResponse {
    pub prompt_id: String,
    pub queue_position: u32,
}
