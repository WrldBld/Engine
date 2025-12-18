//! GenerationBatch entity - Tracks batches of AI-generated assets

use chrono::{DateTime, Utc};

use super::gallery_asset::{AssetType, EntityType};
use crate::domain::value_objects::{AssetId, BatchId, WorldId};

/// Status of a generation batch
#[derive(Debug, Clone, PartialEq)]
pub enum BatchStatus {
    /// Waiting in queue to be processed
    Queued,
    /// Currently being generated
    Generating {
        /// Progress 0-100
        progress: u8,
    },
    /// Generation complete, awaiting user selection
    ReadyForSelection,
    /// User has selected assets, batch is complete
    Completed,
    /// Generation failed
    Failed { error: String },
}

impl BatchStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed { .. })
    }

    pub fn is_active(&self) -> bool {
        matches!(self, Self::Generating { .. })
    }

    pub fn is_queued(&self) -> bool {
        matches!(self, Self::Queued)
    }

    pub fn is_ready(&self) -> bool {
        matches!(self, Self::ReadyForSelection)
    }
}

impl std::fmt::Display for BatchStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Queued => write!(f, "Queued"),
            Self::Generating { progress } => write!(f, "Generating ({}%)", progress),
            Self::ReadyForSelection => write!(f, "Ready"),
            Self::Completed => write!(f, "Completed"),
            Self::Failed { error } => write!(f, "Failed: {}", error),
        }
    }
}

/// A batch of assets being generated together
#[derive(Debug, Clone)]
pub struct GenerationBatch {
    pub id: BatchId,
    /// World this batch belongs to
    pub world_id: WorldId,
    /// Type of entity this batch is for
    pub entity_type: EntityType,
    /// ID of the entity (Character, Location, or Item)
    pub entity_id: String,
    /// Type of asset being generated
    pub asset_type: AssetType,
    /// ComfyUI workflow to use
    pub workflow: String,
    /// Prompt for generation
    pub prompt: String,
    /// Negative prompt (if any)
    pub negative_prompt: Option<String>,
    /// Number of variations to generate
    pub count: u8,
    /// Current status
    pub status: BatchStatus,
    /// Generated asset IDs (populated when complete)
    pub assets: Vec<AssetId>,
    /// Style reference asset ID (if using consistent style)
    pub style_reference_id: Option<AssetId>,
    /// When the batch was requested
    pub requested_at: DateTime<Utc>,
    /// When the batch completed (success or failure)
    pub completed_at: Option<DateTime<Utc>>,
}

impl GenerationBatch {
    /// Create a new generation batch request
    pub fn new(
        world_id: WorldId,
        entity_type: EntityType,
        entity_id: impl Into<String>,
        asset_type: AssetType,
        workflow: impl Into<String>,
        prompt: impl Into<String>,
        count: u8,
    ) -> Self {
        Self {
            id: BatchId::new(),
            world_id,
            entity_type,
            entity_id: entity_id.into(),
            asset_type,
            workflow: workflow.into(),
            prompt: prompt.into(),
            negative_prompt: None,
            count,
            status: BatchStatus::Queued,
            assets: Vec::new(),
            style_reference_id: None,
            requested_at: Utc::now(),
            completed_at: None,
        }
    }

    pub fn with_negative_prompt(mut self, negative_prompt: impl Into<String>) -> Self {
        self.negative_prompt = Some(negative_prompt.into());
        self
    }

    pub fn with_style_reference(mut self, style_reference_id: AssetId) -> Self {
        self.style_reference_id = Some(style_reference_id);
        self
    }

    /// Start generating this batch
    pub fn start_generating(&mut self) {
        self.status = BatchStatus::Generating { progress: 0 };
    }

    /// Update generation progress (0-100)
    pub fn update_progress(&mut self, progress: u8) {
        self.status = BatchStatus::Generating {
            progress: progress.min(100),
        };
    }

    /// Mark generation as complete, ready for selection
    pub fn complete_generation(&mut self, asset_ids: Vec<AssetId>) {
        self.assets = asset_ids;
        self.status = BatchStatus::ReadyForSelection;
        self.completed_at = Some(Utc::now());
    }

    /// Mark batch as fully completed (user has selected assets)
    pub fn finalize(&mut self) {
        self.status = BatchStatus::Completed;
        if self.completed_at.is_none() {
            self.completed_at = Some(Utc::now());
        }
    }

    /// Mark batch as failed
    pub fn fail(&mut self, error: impl Into<String>) {
        self.status = BatchStatus::Failed {
            error: error.into(),
        };
        self.completed_at = Some(Utc::now());
    }

    /// Get position in queue (for display purposes)
    pub fn queue_position(&self) -> Option<u32> {
        // This would be set by the queue manager
        None
    }
}

/// Request to create a new generation batch
#[derive(Debug, Clone)]
pub struct GenerationRequest {
    pub world_id: WorldId,
    pub entity_type: EntityType,
    pub entity_id: String,
    pub asset_type: AssetType,
    pub workflow: String,
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub count: u8,
    pub style_reference_id: Option<AssetId>,
}

impl GenerationRequest {
    pub fn into_batch(self) -> GenerationBatch {
        let mut batch = GenerationBatch::new(
            self.world_id,
            self.entity_type,
            self.entity_id,
            self.asset_type,
            self.workflow,
            self.prompt,
            self.count,
        );
        if let Some(neg) = self.negative_prompt {
            batch = batch.with_negative_prompt(neg);
        }
        if let Some(style_ref) = self.style_reference_id {
            batch = batch.with_style_reference(style_ref);
        }
        batch
    }
}

/// Selection made from a completed batch
#[derive(Debug, Clone)]
pub struct BatchSelection {
    pub batch_id: BatchId,
    /// Assets to add to gallery
    pub selected_assets: Vec<AssetId>,
    /// Whether to discard unselected assets
    pub discard_others: bool,
    /// Labels to apply to selected assets
    pub labels: Vec<Option<String>>,
}
