//! Asset Gallery and Generation API routes

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::entities::{AssetType, BatchStatus, EntityType, GalleryAsset, GenerationBatch};
use crate::domain::value_objects::{AssetId, BatchId};
use crate::infrastructure::state::AppState;

// ==================== Request/Response DTOs ====================

#[derive(Debug, Deserialize)]
pub struct UploadAssetRequest {
    pub asset_type: String,
    pub file_path: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub set_active: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAssetLabelRequest {
    pub label: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GalleryAssetResponse {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub asset_type: String,
    pub file_path: String,
    pub is_active: bool,
    pub label: Option<String>,
    pub is_generated: bool,
    pub created_at: String,
}

impl From<GalleryAsset> for GalleryAssetResponse {
    fn from(a: GalleryAsset) -> Self {
        let is_generated = a.is_generated();
        Self {
            id: a.id.to_string(),
            entity_type: a.entity_type.to_string(),
            entity_id: a.entity_id,
            asset_type: a.asset_type.to_string(),
            file_path: a.file_path,
            is_active: a.is_active,
            label: a.label,
            is_generated,
            created_at: a.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct GenerateAssetRequest {
    pub entity_type: String,
    pub entity_id: String,
    pub asset_type: String,
    pub workflow: String,
    pub prompt: String,
    #[serde(default)]
    pub negative_prompt: Option<String>,
    #[serde(default = "default_count")]
    pub count: u8,
    #[serde(default)]
    pub style_reference_id: Option<String>,
}

fn default_count() -> u8 {
    4
}

#[derive(Debug, Serialize)]
pub struct GenerationBatchResponse {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub asset_type: String,
    pub workflow: String,
    pub prompt: String,
    pub count: u8,
    pub status: String,
    pub progress: Option<u8>,
    pub asset_count: usize,
    pub requested_at: String,
    pub completed_at: Option<String>,
}

impl From<GenerationBatch> for GenerationBatchResponse {
    fn from(b: GenerationBatch) -> Self {
        let (status, progress) = match &b.status {
            BatchStatus::Queued => ("Queued".to_string(), None),
            BatchStatus::Generating { progress } => ("Generating".to_string(), Some(*progress)),
            BatchStatus::ReadyForSelection => ("ReadyForSelection".to_string(), Some(100)),
            BatchStatus::Completed => ("Completed".to_string(), Some(100)),
            BatchStatus::Failed { error } => (format!("Failed: {}", error), None),
        };

        Self {
            id: b.id.to_string(),
            entity_type: b.entity_type.to_string(),
            entity_id: b.entity_id,
            asset_type: b.asset_type.to_string(),
            workflow: b.workflow,
            prompt: b.prompt,
            count: b.count,
            status,
            progress,
            asset_count: b.assets.len(),
            requested_at: b.requested_at.to_rfc3339(),
            completed_at: b.completed_at.map(|t| t.to_rfc3339()),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SelectFromBatchRequest {
    pub selected_assets: Vec<String>,
    #[serde(default)]
    pub discard_others: bool,
    #[serde(default)]
    pub labels: Vec<Option<String>>,
}

// ==================== Character Gallery Routes ====================

/// List all assets for a character
pub async fn list_character_assets(
    State(state): State<Arc<AppState>>,
    Path(character_id): Path<String>,
) -> Result<Json<Vec<GalleryAssetResponse>>, (StatusCode, String)> {
    let assets = state
        .repository
        .assets()
        .list_by_entity(EntityType::Character, &character_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        assets.into_iter().map(GalleryAssetResponse::from).collect(),
    ))
}

/// Upload an asset to a character's gallery
pub async fn upload_character_asset(
    State(state): State<Arc<AppState>>,
    Path(character_id): Path<String>,
    Json(req): Json<UploadAssetRequest>,
) -> Result<(StatusCode, Json<GalleryAssetResponse>), (StatusCode, String)> {
    let asset_type = AssetType::from_str(&req.asset_type).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid asset type: {}", req.asset_type),
        )
    })?;

    let mut asset = GalleryAsset::new(
        EntityType::Character,
        &character_id,
        asset_type,
        &req.file_path,
    );

    if let Some(label) = req.label {
        asset = asset.with_label(label);
    }

    state
        .repository
        .assets()
        .create_asset(&asset)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if req.set_active {
        state
            .repository
            .assets()
            .activate_asset(asset.id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        asset.is_active = true;
    }

    Ok((StatusCode::CREATED, Json(GalleryAssetResponse::from(asset))))
}

/// Activate an asset in a character's gallery
pub async fn activate_character_asset(
    State(state): State<Arc<AppState>>,
    Path((_character_id, asset_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&asset_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid asset ID".to_string()))?;

    state
        .repository
        .assets()
        .activate_asset(AssetId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}

/// Update an asset's label
pub async fn update_character_asset_label(
    State(state): State<Arc<AppState>>,
    Path((_character_id, asset_id)): Path<(String, String)>,
    Json(req): Json<UpdateAssetLabelRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&asset_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid asset ID".to_string()))?;

    state
        .repository
        .assets()
        .update_label(AssetId::from_uuid(uuid), req.label)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}

/// Delete an asset from a character's gallery
pub async fn delete_character_asset(
    State(state): State<Arc<AppState>>,
    Path((_character_id, asset_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&asset_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid asset ID".to_string()))?;

    state
        .repository
        .assets()
        .delete_asset(AssetId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

// ==================== Location Gallery Routes ====================

/// List all assets for a location
pub async fn list_location_assets(
    State(state): State<Arc<AppState>>,
    Path(location_id): Path<String>,
) -> Result<Json<Vec<GalleryAssetResponse>>, (StatusCode, String)> {
    let assets = state
        .repository
        .assets()
        .list_by_entity(EntityType::Location, &location_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        assets.into_iter().map(GalleryAssetResponse::from).collect(),
    ))
}

/// Upload an asset to a location's gallery
pub async fn upload_location_asset(
    State(state): State<Arc<AppState>>,
    Path(location_id): Path<String>,
    Json(req): Json<UploadAssetRequest>,
) -> Result<(StatusCode, Json<GalleryAssetResponse>), (StatusCode, String)> {
    let asset_type = AssetType::from_str(&req.asset_type).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid asset type: {}", req.asset_type),
        )
    })?;

    let mut asset = GalleryAsset::new(
        EntityType::Location,
        &location_id,
        asset_type,
        &req.file_path,
    );

    if let Some(label) = req.label {
        asset = asset.with_label(label);
    }

    state
        .repository
        .assets()
        .create_asset(&asset)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if req.set_active {
        state
            .repository
            .assets()
            .activate_asset(asset.id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        asset.is_active = true;
    }

    Ok((StatusCode::CREATED, Json(GalleryAssetResponse::from(asset))))
}

/// Activate an asset in a location's gallery
pub async fn activate_location_asset(
    State(state): State<Arc<AppState>>,
    Path((_location_id, asset_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&asset_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid asset ID".to_string()))?;

    state
        .repository
        .assets()
        .activate_asset(AssetId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}

/// Delete an asset from a location's gallery
pub async fn delete_location_asset(
    State(state): State<Arc<AppState>>,
    Path((_location_id, asset_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&asset_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid asset ID".to_string()))?;

    state
        .repository
        .assets()
        .delete_asset(AssetId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

// ==================== Item Gallery Routes ====================

/// List all assets for an item
pub async fn list_item_assets(
    State(state): State<Arc<AppState>>,
    Path(item_id): Path<String>,
) -> Result<Json<Vec<GalleryAssetResponse>>, (StatusCode, String)> {
    let assets = state
        .repository
        .assets()
        .list_by_entity(EntityType::Item, &item_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        assets.into_iter().map(GalleryAssetResponse::from).collect(),
    ))
}

/// Upload an asset to an item's gallery
pub async fn upload_item_asset(
    State(state): State<Arc<AppState>>,
    Path(item_id): Path<String>,
    Json(req): Json<UploadAssetRequest>,
) -> Result<(StatusCode, Json<GalleryAssetResponse>), (StatusCode, String)> {
    let asset_type = AssetType::from_str(&req.asset_type).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid asset type: {}", req.asset_type),
        )
    })?;

    let mut asset = GalleryAsset::new(EntityType::Item, &item_id, asset_type, &req.file_path);

    if let Some(label) = req.label {
        asset = asset.with_label(label);
    }

    state
        .repository
        .assets()
        .create_asset(&asset)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if req.set_active {
        state
            .repository
            .assets()
            .activate_asset(asset.id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        asset.is_active = true;
    }

    Ok((StatusCode::CREATED, Json(GalleryAssetResponse::from(asset))))
}

/// Activate an asset in an item's gallery
pub async fn activate_item_asset(
    State(state): State<Arc<AppState>>,
    Path((_item_id, asset_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&asset_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid asset ID".to_string()))?;

    state
        .repository
        .assets()
        .activate_asset(AssetId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}

/// Delete an asset from an item's gallery
pub async fn delete_item_asset(
    State(state): State<Arc<AppState>>,
    Path((_item_id, asset_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&asset_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid asset ID".to_string()))?;

    state
        .repository
        .assets()
        .delete_asset(AssetId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

// ==================== Generation Queue Routes ====================

/// Queue a new asset generation request
pub async fn queue_generation(
    State(state): State<Arc<AppState>>,
    Json(req): Json<GenerateAssetRequest>,
) -> Result<(StatusCode, Json<GenerationBatchResponse>), (StatusCode, String)> {
    let entity_type = parse_entity_type(&req.entity_type).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid entity type: {}", req.entity_type),
        )
    })?;

    let asset_type = AssetType::from_str(&req.asset_type).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid asset type: {}", req.asset_type),
        )
    })?;

    let mut batch = GenerationBatch::new(
        entity_type,
        &req.entity_id,
        asset_type,
        &req.workflow,
        &req.prompt,
        req.count,
    );

    if let Some(neg) = req.negative_prompt {
        batch = batch.with_negative_prompt(neg);
    }

    if let Some(ref_id) = req.style_reference_id {
        let uuid = Uuid::parse_str(&ref_id).map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                "Invalid style reference ID".to_string(),
            )
        })?;
        batch = batch.with_style_reference(AssetId::from_uuid(uuid));
    }

    state
        .repository
        .assets()
        .create_batch(&batch)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // TODO: Actually queue the generation job with GenerationService
    // For now, just create the batch record

    tracing::info!(
        "Queued generation batch: {} for {} {}",
        batch.id,
        entity_type,
        req.entity_id
    );

    Ok((
        StatusCode::CREATED,
        Json(GenerationBatchResponse::from(batch)),
    ))
}

/// List the generation queue
pub async fn list_queue(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<GenerationBatchResponse>>, (StatusCode, String)> {
    let batches = state
        .repository
        .assets()
        .list_active_batches()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        batches
            .into_iter()
            .map(GenerationBatchResponse::from)
            .collect(),
    ))
}

/// List batches ready for selection
pub async fn list_ready_batches(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<GenerationBatchResponse>>, (StatusCode, String)> {
    let batches = state
        .repository
        .assets()
        .list_ready_batches()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        batches
            .into_iter()
            .map(GenerationBatchResponse::from)
            .collect(),
    ))
}

/// Get a batch by ID
pub async fn get_batch(
    State(state): State<Arc<AppState>>,
    Path(batch_id): Path<String>,
) -> Result<Json<GenerationBatchResponse>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&batch_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid batch ID".to_string()))?;

    let batch = state
        .repository
        .assets()
        .get_batch(BatchId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Batch not found".to_string()))?;

    Ok(Json(GenerationBatchResponse::from(batch)))
}

/// Get assets from a completed batch
pub async fn get_batch_assets(
    State(state): State<Arc<AppState>>,
    Path(batch_id): Path<String>,
) -> Result<Json<Vec<GalleryAssetResponse>>, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&batch_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid batch ID".to_string()))?;

    let batch = state
        .repository
        .assets()
        .get_batch(BatchId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Batch not found".to_string()))?;

    let mut assets = Vec::new();
    for asset_id in batch.assets {
        if let Some(asset) = state
            .repository
            .assets()
            .get_asset(asset_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        {
            assets.push(GalleryAssetResponse::from(asset));
        }
    }

    Ok(Json(assets))
}

/// Select assets from a completed batch
pub async fn select_from_batch(
    State(state): State<Arc<AppState>>,
    Path(batch_id): Path<String>,
    Json(req): Json<SelectFromBatchRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&batch_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid batch ID".to_string()))?;

    let batch = state
        .repository
        .assets()
        .get_batch(BatchId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Batch not found".to_string()))?;

    // Mark batch as completed
    state
        .repository
        .assets()
        .update_batch_status(batch.id, &BatchStatus::Completed)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Apply labels to selected assets
    for (i, asset_id_str) in req.selected_assets.iter().enumerate() {
        let asset_uuid = Uuid::parse_str(asset_id_str).map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                "Invalid asset ID in selection".to_string(),
            )
        })?;

        let label = req.labels.get(i).cloned().flatten();
        if label.is_some() {
            state
                .repository
                .assets()
                .update_label(AssetId::from_uuid(asset_uuid), label)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }
    }

    // Delete unselected assets if requested
    if req.discard_others {
        let selected_set: std::collections::HashSet<_> = req.selected_assets.iter().collect();
        for asset_id in &batch.assets {
            let asset_id_str = asset_id.to_string();
            if !selected_set.contains(&asset_id_str) {
                state
                    .repository
                    .assets()
                    .delete_asset(*asset_id)
                    .await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            }
        }
    }

    Ok(StatusCode::OK)
}

/// Cancel a queued batch
pub async fn cancel_batch(
    State(state): State<Arc<AppState>>,
    Path(batch_id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let uuid = Uuid::parse_str(&batch_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid batch ID".to_string()))?;

    let batch = state
        .repository
        .assets()
        .get_batch(BatchId::from_uuid(uuid))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Batch not found".to_string()))?;

    // Can only cancel queued batches
    if !batch.status.is_queued() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Can only cancel queued batches".to_string(),
        ));
    }

    state
        .repository
        .assets()
        .delete_batch(batch.id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

fn parse_entity_type(s: &str) -> Option<EntityType> {
    match s.to_lowercase().as_str() {
        "character" => Some(EntityType::Character),
        "location" => Some(EntityType::Location),
        "item" => Some(EntityType::Item),
        _ => None,
    }
}
