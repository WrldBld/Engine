//! Character Sheet Template API routes
//!
//! Endpoints for managing character sheet templates within a world.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::application::services::WorldService;
use crate::domain::entities::{
    CharacterSheetTemplate, FieldType, SectionLayout, SheetField, SheetSection, SheetTemplateId,
};
use crate::domain::value_objects::WorldId;
use crate::infrastructure::state::AppState;

/// Response for a sheet template
#[derive(Debug, Serialize)]
pub struct SheetTemplateResponse {
    pub id: String,
    pub world_id: String,
    pub name: String,
    pub description: String,
    pub variant: String,
    pub sections: Vec<SheetSection>,
    pub is_default: bool,
}

impl From<CharacterSheetTemplate> for SheetTemplateResponse {
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

/// Summary response (without sections)
#[derive(Debug, Serialize)]
pub struct SheetTemplateSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub is_default: bool,
    pub section_count: usize,
    pub field_count: usize,
}

impl From<CharacterSheetTemplate> for SheetTemplateSummary {
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

/// Request to create a custom section
#[derive(Debug, Deserialize)]
pub struct CreateSectionRequest {
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

/// Request to create a custom field
#[derive(Debug, Deserialize)]
pub struct CreateFieldRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub field_type: FieldType,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub read_only: bool,
}

/// Get the sheet template for a world
///
/// Returns the default template if one exists, or generates one based on the rule system.
pub async fn get_template(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
) -> Result<Json<SheetTemplateResponse>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(uuid);

    // Try to get existing template
    if let Some(template) = state
        .sheet_template_service
        .get_default_for_world(&world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    {
        return Ok(Json(SheetTemplateResponse::from(template)));
    }

    // No template exists, generate from rule system
    let world = state
        .world_service
        .get_world(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "World not found".to_string()))?;

    let template =
        CharacterSheetTemplate::default_for_variant(world_id, &world.rule_system.variant);

    Ok(Json(SheetTemplateResponse::from(template)))
}

/// List all sheet templates for a world
pub async fn list_templates(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
) -> Result<Json<Vec<SheetTemplateSummary>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(uuid);

    // Check world exists
    let world = state
        .world_service
        .get_world(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "World not found".to_string()))?;

    // Get existing templates
    let templates = state
        .sheet_template_service
        .list_by_world(&world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // If no templates exist, return the default template as a summary
    if templates.is_empty() {
        let default =
            CharacterSheetTemplate::default_for_variant(world_id, &world.rule_system.variant);
        return Ok(Json(vec![SheetTemplateSummary::from(default)]));
    }

    Ok(Json(
        templates.into_iter().map(SheetTemplateSummary::from).collect(),
    ))
}

/// Get a specific template by ID
pub async fn get_template_by_id(
    State(state): State<Arc<AppState>>,
    Path((world_id, template_id)): Path<(String, String)>,
) -> Result<Json<SheetTemplateResponse>, (StatusCode, String)> {
    let world_uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(world_uuid);

    let template_id = SheetTemplateId::from_string(template_id);

    let template = state
        .sheet_template_service
        .get(&template_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Template not found".to_string()))?;

    // Verify template belongs to world
    if template.world_id != world_id {
        return Err((
            StatusCode::FORBIDDEN,
            "Template does not belong to this world".to_string(),
        ));
    }

    Ok(Json(SheetTemplateResponse::from(template)))
}

/// Initialize the default template for a world
///
/// Creates and persists the default template based on the world's rule system.
pub async fn initialize_template(
    State(state): State<Arc<AppState>>,
    Path(world_id): Path<String>,
) -> Result<Json<SheetTemplateResponse>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(uuid);

    // Get the world
    let world = state
        .world_service
        .get_world(world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "World not found".to_string()))?;

    // Check if template already exists
    if state
        .sheet_template_service
        .has_templates(&world_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    {
        return Err((
            StatusCode::CONFLICT,
            "World already has a sheet template".to_string(),
        ));
    }

    // Create the default template
    let template =
        CharacterSheetTemplate::default_for_variant(world_id, &world.rule_system.variant);

    // Save it
    state
        .sheet_template_service
        .create(&template)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SheetTemplateResponse::from(template)))
}

/// Add a custom section to a template
pub async fn add_section(
    State(state): State<Arc<AppState>>,
    Path((world_id, template_id)): Path<(String, String)>,
    Json(req): Json<CreateSectionRequest>,
) -> Result<Json<SheetTemplateResponse>, (StatusCode, String)> {
    let world_uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(world_uuid);
    let template_id = SheetTemplateId::from_string(template_id);

    // Get existing template
    let mut template = state
        .sheet_template_service
        .get(&template_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Template not found".to_string()))?;

    // Verify template belongs to world
    if template.world_id != world_id {
        return Err((
            StatusCode::FORBIDDEN,
            "Template does not belong to this world".to_string(),
        ));
    }

    // Create the section
    let section_id = format!("custom_{}", uuid::Uuid::new_v4());
    let mut section = SheetSection::new(&section_id, &req.name);

    if let Some(desc) = req.description {
        section = section.with_description(desc);
    }
    if let Some(layout) = req.layout {
        section = section.with_layout(layout);
    }
    if req.collapsible {
        section = section.collapsible();
    }
    if req.collapsed_by_default {
        section = section.collapsed();
    }

    // Set order to be after existing sections
    let max_order = template.sections.iter().map(|s| s.order).max().unwrap_or(0);
    section = section.with_order(max_order + 1);

    template.sections.push(section);

    // Save updates
    state
        .sheet_template_service
        .update(&template)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SheetTemplateResponse::from(template)))
}

/// Add a custom field to a section
pub async fn add_field(
    State(state): State<Arc<AppState>>,
    Path((world_id, template_id, section_id)): Path<(String, String, String)>,
    Json(req): Json<CreateFieldRequest>,
) -> Result<Json<SheetTemplateResponse>, (StatusCode, String)> {
    let world_uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(world_uuid);
    let template_id = SheetTemplateId::from_string(template_id);

    // Get existing template
    let mut template = state
        .sheet_template_service
        .get(&template_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Template not found".to_string()))?;

    // Verify template belongs to world
    if template.world_id != world_id {
        return Err((
            StatusCode::FORBIDDEN,
            "Template does not belong to this world".to_string(),
        ));
    }

    // Find the section
    let section = template
        .sections
        .iter_mut()
        .find(|s| s.id == section_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Section not found".to_string()))?;

    // Create the field
    let field_id = format!("custom_{}", uuid::Uuid::new_v4());
    let mut field = SheetField::new(&field_id, &req.name, req.field_type);

    if let Some(desc) = req.description {
        field = field.with_description(desc);
    }
    if req.required {
        field = field.required();
    }
    if req.read_only {
        field = field.read_only();
    }

    // Set order to be after existing fields
    let max_order = section.fields.iter().map(|f| f.order).max().unwrap_or(0);
    field = field.with_order(max_order + 1);

    section.fields.push(field);

    // Save updates
    state
        .sheet_template_service
        .update(&template)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SheetTemplateResponse::from(template)))
}

/// Delete a template
pub async fn delete_template(
    State(state): State<Arc<AppState>>,
    Path((world_id, template_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let world_uuid = Uuid::parse_str(&world_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid world ID".to_string()))?;
    let world_id = WorldId::from_uuid(world_uuid);
    let template_id = SheetTemplateId::from_string(template_id);

    // Get existing template
    let template = state
        .sheet_template_service
        .get(&template_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Template not found".to_string()))?;

    // Verify template belongs to world
    if template.world_id != world_id {
        return Err((
            StatusCode::FORBIDDEN,
            "Template does not belong to this world".to_string(),
        ));
    }

    // Delete
    state
        .sheet_template_service
        .delete(&template_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}
