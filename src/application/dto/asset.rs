use serde::{Deserialize, Serialize};

use crate::domain::entities::{AssetType, BatchStatus, EntityType, GalleryAsset, GenerationBatch};

#[derive(Debug, Deserialize)]
pub struct UploadAssetRequestDto {
    pub asset_type: String,
    pub file_path: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub set_active: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAssetLabelRequestDto {
    pub label: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GalleryAssetResponseDto {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub asset_type: String,
    pub file_path: String,
    pub is_active: bool,
    pub label: Option<String>,
    pub is_generated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style_reference_id: Option<String>, // ID of asset used as style reference (if any)
    pub created_at: String,
}

impl From<GalleryAsset> for GalleryAssetResponseDto {
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
            style_reference_id: None,
            created_at: a.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct GenerateAssetRequestDto {
    /// World this asset belongs to
    pub world_id: String,
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
pub struct GenerationBatchResponseDto {
    pub id: String,
    /// World this batch belongs to
    pub world_id: String,
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

impl From<GenerationBatch> for GenerationBatchResponseDto {
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
            world_id: b.world_id.to_string(),
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
pub struct SelectFromBatchRequestDto {
    pub selected_assets: Vec<String>,
    #[serde(default)]
    pub discard_others: bool,
    #[serde(default)]
    pub labels: Vec<Option<String>>,
}

pub fn parse_entity_type(s: &str) -> Option<EntityType> {
    match s.to_lowercase().as_str() {
        "character" => Some(EntityType::Character),
        "location" => Some(EntityType::Location),
        "item" => Some(EntityType::Item),
        _ => None,
    }
}

pub fn parse_asset_type(asset_type: &str) -> Result<AssetType, String> {
    AssetType::from_str(asset_type).ok_or_else(|| format!("Invalid asset type: {}", asset_type))
}

