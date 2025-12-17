//! Workflow configuration REST API routes

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

use crate::application::services::WorkflowService;
use crate::application::dto::{
    parse_workflow_slot, AnalyzeWorkflowRequestDto, CreateWorkflowConfigRequestDto,
    ImportWorkflowsRequestDto, ImportWorkflowsResponseDto, TestWorkflowRequestDto,
    TestWorkflowResponseDto, UpdateWorkflowDefaultsRequestDto, WorkflowAnalysisResponseDto,
    WorkflowConfigFullResponseDto, WorkflowConfigResponseDto, WorkflowSlotCategoryDto,
    WorkflowSlotStatusDto, WorkflowSlotsResponseDto,
};
use crate::domain::entities::{WorkflowConfiguration, WorkflowSlot};
use crate::infrastructure::state::AppState;
// NOTE: workflow request/response DTOs live in `application/dto/workflow.rs`.

// ============================================================================
// Route Handlers
// ============================================================================

/// List all workflow slots grouped by category
pub async fn list_workflow_slots(
    State(state): State<Arc<AppState>>,
) -> Result<Json<WorkflowSlotsResponseDto>, (StatusCode, String)> {
    let configs = state
        .assets.workflow_config_service
        .list_all()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Group slots by category, maintaining order
    let category_order = ["Character Assets", "Location Assets", "Item Assets", "Map Assets"];
    let mut categories: Vec<WorkflowSlotCategoryDto> = category_order
        .iter()
        .map(|name| WorkflowSlotCategoryDto {
            name: name.to_string(),
            slots: Vec::new(),
        })
        .collect();

    for slot in WorkflowSlot::all() {
        let config = configs.iter().find(|c| c.slot == *slot);
        let (width, height) = slot.default_dimensions();
        let category_name = slot.category();

        let status = WorkflowSlotStatusDto {
            slot: slot.as_str().to_string(),
            display_name: slot.display_name().to_string(),
            default_width: width,
            default_height: height,
            configured: config.is_some(),
            config: config.map(WorkflowConfigResponseDto::from),
        };

        // Find the category and add the slot
        if let Some(category) = categories.iter_mut().find(|c| c.name == category_name) {
            category.slots.push(status);
        }
    }

    Ok(Json(WorkflowSlotsResponseDto { categories }))
}

/// Get a workflow configuration by slot
pub async fn get_workflow_config(
    State(state): State<Arc<AppState>>,
    Path(slot): Path<String>,
) -> Result<Json<WorkflowConfigFullResponseDto>, (StatusCode, String)> {
    let workflow_slot = parse_workflow_slot(&slot)
        .map_err(|msg| (StatusCode::BAD_REQUEST, msg))?;

    let config = state
        .assets.workflow_config_service
        .get_by_slot(workflow_slot)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("No workflow configured for slot: {}", slot),
            )
        })?;

    Ok(Json(WorkflowConfigFullResponseDto::from(&config)))
}

/// Create or update a workflow configuration
pub async fn save_workflow_config(
    State(state): State<Arc<AppState>>,
    Path(slot): Path<String>,
    Json(req): Json<CreateWorkflowConfigRequestDto>,
) -> Result<(StatusCode, Json<WorkflowConfigFullResponseDto>), (StatusCode, String)> {
    let workflow_slot = parse_workflow_slot(&slot)
        .map_err(|msg| (StatusCode::BAD_REQUEST, msg))?;

    // Validate the workflow JSON
    WorkflowService::validate_workflow(&req.workflow_json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    // Check if we're updating or creating
    let existing = state
        .assets.workflow_config_service
        .get_by_slot(workflow_slot)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let is_update = existing.is_some();

    let config = if let Some(mut existing_config) = existing {
        // Update existing
        existing_config.name = req.name;
        existing_config.update_workflow(req.workflow_json);
        existing_config.set_prompt_mappings(req.prompt_mappings.into_iter().map(Into::into).collect());
        existing_config.set_input_defaults(req.input_defaults.into_iter().map(Into::into).collect());
        existing_config.set_locked_inputs(req.locked_inputs);
        existing_config
    } else {
        // Create new
        let mut config = WorkflowConfiguration::new(workflow_slot, req.name, req.workflow_json);
        config.set_prompt_mappings(req.prompt_mappings.into_iter().map(Into::into).collect());
        config.set_input_defaults(req.input_defaults.into_iter().map(Into::into).collect());
        config.set_locked_inputs(req.locked_inputs);
        config
    };

    state
        .assets.workflow_config_service
        .save(&config)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let status = if is_update {
        StatusCode::OK
    } else {
        StatusCode::CREATED
    };

    Ok((status, Json(WorkflowConfigFullResponseDto::from(&config))))
}

/// Delete a workflow configuration
pub async fn delete_workflow_config(
    State(state): State<Arc<AppState>>,
    Path(slot): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let workflow_slot = parse_workflow_slot(&slot)
        .map_err(|msg| (StatusCode::BAD_REQUEST, msg))?;

    let deleted = state
        .assets.workflow_config_service
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

/// Update just the defaults of a workflow configuration (without re-uploading the workflow JSON)
pub async fn update_workflow_defaults(
    State(state): State<Arc<AppState>>,
    Path(slot): Path<String>,
    Json(req): Json<UpdateWorkflowDefaultsRequestDto>,
) -> Result<Json<WorkflowConfigFullResponseDto>, (StatusCode, String)> {
    let workflow_slot = parse_workflow_slot(&slot)
        .map_err(|msg| (StatusCode::BAD_REQUEST, msg))?;

    let mut config = state
        .assets.workflow_config_service
        .get_by_slot(workflow_slot)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("No workflow configured for slot: {}", slot)))?;

    // Update defaults
    config.set_input_defaults(req.input_defaults.into_iter().map(Into::into).collect());

    // Update locked inputs if provided
    if let Some(locked) = req.locked_inputs {
        config.set_locked_inputs(locked);
    }

    state
        .assets.workflow_config_service
        .save(&config)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(WorkflowConfigFullResponseDto::from(&config)))
}

/// Analyze a workflow JSON without saving
pub async fn analyze_workflow(
    Json(req): Json<AnalyzeWorkflowRequestDto>,
) -> Result<Json<WorkflowAnalysisResponseDto>, (StatusCode, String)> {
    // Validate first
    if let Err(e) = WorkflowService::validate_workflow(&req.workflow_json) {
        return Err((StatusCode::BAD_REQUEST, e.to_string()));
    }

    let analysis = WorkflowService::analyze_workflow(&req.workflow_json);
    let auto_mappings = WorkflowService::auto_detect_prompt_mappings(&req.workflow_json);

    Ok(Json(WorkflowAnalysisResponseDto {
        is_valid: analysis.is_valid(),
        analysis: analysis.into(),
        suggested_prompt_mappings: auto_mappings.into_iter().map(Into::into).collect(),
    }))
}

/// Export all workflow configurations
pub async fn export_workflows(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let configs = state
        .assets.workflow_config_service
        .list_all()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let export = WorkflowService::export_configs(&configs);
    Ok(Json(export))
}

pub async fn import_workflows(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ImportWorkflowsRequestDto>,
) -> Result<Json<ImportWorkflowsResponseDto>, (StatusCode, String)> {
    let configs = WorkflowService::import_configs(&req.data)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let mut imported = 0;
    let mut skipped = 0;

    for config in configs {
        let existing = state
            .assets.workflow_config_service
            .get_by_slot(config.slot)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        if existing.is_some() && !req.replace_existing {
            skipped += 1;
            continue;
        }

        state
            .assets.workflow_config_service
            .save(&config)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        imported += 1;
    }

    Ok(Json(ImportWorkflowsResponseDto { imported, skipped }))
}

pub async fn test_workflow(
    State(state): State<Arc<AppState>>,
    Path(slot): Path<String>,
    Json(req): Json<TestWorkflowRequestDto>,
) -> Result<Json<TestWorkflowResponseDto>, (StatusCode, String)> {
    let workflow_slot = parse_workflow_slot(&slot)
        .map_err(|msg| (StatusCode::BAD_REQUEST, msg))?;

    let config = state
        .assets.workflow_config_service
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

    Ok(Json(TestWorkflowResponseDto {
        prompt_id: queue_result.prompt_id,
        queue_position: queue_result.number,
    }))
}
