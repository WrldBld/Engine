//! GalleryAsset entity - Assets stored in entity galleries with history

use chrono::{DateTime, Utc};

use crate::domain::value_objects::{AssetId, BatchId};

/// Type of entity that owns this asset
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntityType {
    Character,
    Location,
    Item,
}

impl std::fmt::Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Character => write!(f, "Character"),
            Self::Location => write!(f, "Location"),
            Self::Item => write!(f, "Item"),
        }
    }
}

impl EntityType {
    /// Get the lowercase string representation for file paths
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Character => "character",
            Self::Location => "location",
            Self::Item => "item",
        }
    }
}

/// Type of asset (determines which slot it occupies)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetType {
    /// Character face portrait (256x256)
    Portrait,
    /// Character full-body sprite (512x512)
    Sprite,
    /// Scene/location backdrop (1920x1080)
    Backdrop,
    /// Grid map tilesheet (512x512)
    Tilesheet,
    /// Item icon (64x64)
    ItemIcon,
    /// Grid of character expressions (768x768)
    EmotionSheet,
    /// Backdrop for clickable map region (1280x720)
    RegionBackdrop,
}

impl std::fmt::Display for AssetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Portrait => write!(f, "Portrait"),
            Self::Sprite => write!(f, "Sprite"),
            Self::Backdrop => write!(f, "Backdrop"),
            Self::Tilesheet => write!(f, "Tilesheet"),
            Self::ItemIcon => write!(f, "ItemIcon"),
            Self::EmotionSheet => write!(f, "EmotionSheet"),
            Self::RegionBackdrop => write!(f, "RegionBackdrop"),
        }
    }
}

impl AssetType {
    /// Parse from string (case-insensitive)
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "portrait" => Some(Self::Portrait),
            "sprite" => Some(Self::Sprite),
            "backdrop" => Some(Self::Backdrop),
            "tilesheet" => Some(Self::Tilesheet),
            "itemicon" | "item_icon" => Some(Self::ItemIcon),
            "emotionsheet" | "emotion_sheet" => Some(Self::EmotionSheet),
            "regionbackdrop" | "region_backdrop" => Some(Self::RegionBackdrop),
            _ => None,
        }
    }

    /// Get the lowercase string representation for file paths
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Portrait => "portrait",
            Self::Sprite => "sprite",
            Self::Backdrop => "backdrop",
            Self::Tilesheet => "tilesheet",
            Self::ItemIcon => "item_icon",
            Self::EmotionSheet => "emotion_sheet",
            Self::RegionBackdrop => "region_backdrop",
        }
    }

    /// Get default dimensions for this asset type
    pub fn default_dimensions(&self) -> (u32, u32) {
        match self {
            Self::Portrait => (256, 256),
            Self::Sprite => (512, 512),
            Self::Backdrop => (1920, 1080),
            Self::Tilesheet => (512, 512),
            Self::ItemIcon => (64, 64),
            Self::EmotionSheet => (768, 768),
            Self::RegionBackdrop => (1280, 720),
        }
    }
}

/// Metadata about how an asset was generated
#[derive(Debug, Clone)]
pub struct GenerationMetadata {
    /// ComfyUI workflow used
    pub workflow: String,
    /// Prompt used for generation
    pub prompt: String,
    /// Negative prompt (if any)
    pub negative_prompt: Option<String>,
    /// Seed used for reproducibility
    pub seed: i64,
    /// Style reference asset (if any)
    pub style_reference_id: Option<AssetId>,
    /// Batch this asset was generated in
    pub batch_id: BatchId,
}

impl GenerationMetadata {
    pub fn new(
        workflow: impl Into<String>,
        prompt: impl Into<String>,
        seed: i64,
        batch_id: BatchId,
    ) -> Self {
        Self {
            workflow: workflow.into(),
            prompt: prompt.into(),
            negative_prompt: None,
            seed,
            style_reference_id: None,
            batch_id,
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
}

/// An asset stored in an entity's gallery
#[derive(Debug, Clone)]
pub struct GalleryAsset {
    pub id: AssetId,
    /// Type of entity that owns this asset
    pub entity_type: EntityType,
    /// ID of the owning entity (Character, Location, or Item)
    pub entity_id: String,
    /// Type of asset (Portrait, Sprite, Backdrop, etc.)
    pub asset_type: AssetType,
    /// Path to the stored asset file
    pub file_path: String,
    /// Whether this is the currently active asset for this slot
    pub is_active: bool,
    /// User-defined label (e.g., "Angry", "Winter Outfit", "Night")
    pub label: Option<String>,
    /// Metadata about generation (if AI-generated)
    pub generation_metadata: Option<GenerationMetadata>,
    /// When the asset was created/uploaded
    pub created_at: DateTime<Utc>,
}

impl GalleryAsset {
    /// Create a new gallery asset (uploaded, not generated)
    pub fn new(
        entity_type: EntityType,
        entity_id: impl Into<String>,
        asset_type: AssetType,
        file_path: impl Into<String>,
    ) -> Self {
        Self {
            id: AssetId::new(),
            entity_type,
            entity_id: entity_id.into(),
            asset_type,
            file_path: file_path.into(),
            is_active: false,
            label: None,
            generation_metadata: None,
            created_at: Utc::now(),
        }
    }

    /// Create a new generated asset
    pub fn new_generated(
        entity_type: EntityType,
        entity_id: impl Into<String>,
        asset_type: AssetType,
        file_path: impl Into<String>,
        metadata: GenerationMetadata,
    ) -> Self {
        Self {
            id: AssetId::new(),
            entity_type,
            entity_id: entity_id.into(),
            asset_type,
            file_path: file_path.into(),
            is_active: false,
            label: None,
            generation_metadata: Some(metadata),
            created_at: Utc::now(),
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn set_active(mut self, active: bool) -> Self {
        self.is_active = active;
        self
    }

    /// Activate this asset (mark as current for its slot)
    pub fn activate(&mut self) {
        self.is_active = true;
    }

    /// Deactivate this asset
    pub fn deactivate(&mut self) {
        self.is_active = false;
    }

    /// Update the label
    pub fn set_label(&mut self, label: Option<String>) {
        self.label = label;
    }

    /// Check if this asset was AI-generated
    pub fn is_generated(&self) -> bool {
        self.generation_metadata.is_some()
    }
}
