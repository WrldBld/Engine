//! Asset Service - Application service for asset gallery and generation management
//!
//! This service provides use case implementations for managing gallery assets,
//! including uploads, activation, deletion, and AI generation batches.

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, info, instrument};

use crate::application::ports::outbound::AssetRepositoryPort;
use crate::domain::entities::{BatchStatus, EntityType, GalleryAsset, GenerationBatch};
use crate::domain::value_objects::{AssetId, BatchId, WorldId};

/// Request to create a new asset
#[derive(Debug, Clone)]
pub struct CreateAssetRequest {
    pub entity_type: EntityType,
    pub entity_id: String,
    pub asset_type: crate::domain::entities::AssetType,
    pub file_path: String,
    pub label: Option<String>,
}

/// Request to update an asset's label
#[derive(Debug, Clone)]
pub struct UpdateAssetLabelRequest {
    pub label: Option<String>,
}

/// Asset service trait defining the application use cases
#[async_trait]
pub trait AssetService: Send + Sync {
    /// List all assets for an entity
    async fn list_assets(
        &self,
        entity_type: EntityType,
        entity_id: &str,
    ) -> Result<Vec<GalleryAsset>>;

    /// Get a single asset by ID
    async fn get_asset(&self, asset_id: AssetId) -> Result<Option<GalleryAsset>>;

    /// Create a new asset
    async fn create_asset(&self, request: CreateAssetRequest) -> Result<GalleryAsset>;

    /// Update an asset's label
    async fn update_asset_label(&self, asset_id: AssetId, label: Option<String>) -> Result<()>;

    /// Delete an asset
    async fn delete_asset(&self, asset_id: AssetId) -> Result<()>;

    /// Activate an asset (set as current for its entity/type slot)
    async fn activate_asset(&self, asset_id: AssetId) -> Result<()>;

    /// Create a generation batch
    async fn create_batch(&self, batch: GenerationBatch) -> Result<GenerationBatch>;

    /// Get a batch by ID
    async fn get_batch(&self, batch_id: BatchId) -> Result<Option<GenerationBatch>>;

    /// List all active batches (queued or generating) for a specific world
    async fn list_active_batches_by_world(&self, world_id: WorldId) -> Result<Vec<GenerationBatch>>;

    /// List batches ready for selection
    async fn list_ready_batches(&self) -> Result<Vec<GenerationBatch>>;

    /// Update batch status
    async fn update_batch_status(&self, batch_id: BatchId, status: BatchStatus) -> Result<()>;

    /// Update batch assets
    async fn update_batch_assets(&self, batch_id: BatchId, assets: Vec<AssetId>) -> Result<()>;

    /// Delete a batch
    async fn delete_batch(&self, batch_id: BatchId) -> Result<()>;
}

/// Default implementation of AssetService using port abstractions
#[derive(Clone)]
pub struct AssetServiceImpl {
    repository: Arc<dyn AssetRepositoryPort>,
}

impl AssetServiceImpl {
    /// Create a new AssetServiceImpl with the given repository
    pub fn new(repository: Arc<dyn AssetRepositoryPort>) -> Self {
        Self { repository }
    }

    /// Validate an asset creation request
    fn validate_create_request(request: &CreateAssetRequest) -> Result<()> {
        if request.file_path.trim().is_empty() {
            anyhow::bail!("Asset file path cannot be empty");
        }
        if request.entity_id.trim().is_empty() {
            anyhow::bail!("Entity ID cannot be empty");
        }
        Ok(())
    }
}

#[async_trait]
impl AssetService for AssetServiceImpl {
    #[instrument(skip(self))]
    async fn list_assets(
        &self,
        entity_type: EntityType,
        entity_id: &str,
    ) -> Result<Vec<GalleryAsset>> {
        debug!(
            entity_type = %entity_type,
            entity_id = %entity_id,
            "Listing assets for entity"
        );
        self.repository
            .list_for_entity(&entity_type.to_string(), entity_id)
            .await
            .context("Failed to list assets from repository")
    }

    #[instrument(skip(self))]
    async fn get_asset(&self, asset_id: AssetId) -> Result<Option<GalleryAsset>> {
        debug!(asset_id = %asset_id, "Fetching asset");
        self.repository
            .get(asset_id)
            .await
            .context("Failed to get asset from repository")
    }

    #[instrument(skip(self), fields(entity_id = %request.entity_id))]
    async fn create_asset(&self, request: CreateAssetRequest) -> Result<GalleryAsset> {
        Self::validate_create_request(&request)?;

        let mut asset = GalleryAsset::new(
            request.entity_type,
            &request.entity_id,
            request.asset_type,
            &request.file_path,
        );

        if let Some(label) = request.label {
            asset = asset.with_label(label);
        }

        self.repository
            .create(&asset)
            .await
            .context("Failed to create asset in repository")?;

        info!(
            asset_id = %asset.id,
            entity_type = %request.entity_type,
            entity_id = %request.entity_id,
            "Created new asset"
        );
        Ok(asset)
    }

    #[instrument(skip(self))]
    async fn update_asset_label(&self, asset_id: AssetId, label: Option<String>) -> Result<()> {
        debug!(asset_id = %asset_id, "Updating asset label");

        // Verify asset exists
        let _ = self
            .repository
            .get(asset_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Asset not found: {}", asset_id))?;

        self.repository
            .update_label(asset_id, label)
            .await
            .context("Failed to update asset label in repository")?;

        info!(asset_id = %asset_id, "Updated asset label");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn delete_asset(&self, asset_id: AssetId) -> Result<()> {
        // Verify the asset exists before deletion
        let _asset = self
            .repository
            .get(asset_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Asset not found: {}", asset_id))?;

        self.repository
            .delete(asset_id)
            .await
            .context("Failed to delete asset from repository")?;

        info!(asset_id = %asset_id, "Deleted asset");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn activate_asset(&self, asset_id: AssetId) -> Result<()> {
        debug!(asset_id = %asset_id, "Activating asset");

        // Verify asset exists
        let _ = self
            .repository
            .get(asset_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Asset not found: {}", asset_id))?;

        self.repository
            .activate(asset_id)
            .await
            .context("Failed to activate asset in repository")?;

        info!(asset_id = %asset_id, "Activated asset");
        Ok(())
    }

    #[instrument(skip(self, batch), fields(batch_id = %batch.id))]
    async fn create_batch(&self, batch: GenerationBatch) -> Result<GenerationBatch> {
        self.repository
            .create_batch(&batch)
            .await
            .context("Failed to create batch in repository")?;

        info!(
            batch_id = %batch.id,
            entity_type = %batch.entity_type,
            entity_id = %batch.entity_id,
            "Created generation batch"
        );
        Ok(batch)
    }

    #[instrument(skip(self))]
    async fn get_batch(&self, batch_id: BatchId) -> Result<Option<GenerationBatch>> {
        debug!(batch_id = %batch_id, "Fetching batch");
        self.repository
            .get_batch(batch_id)
            .await
            .context("Failed to get batch from repository")
    }

    #[instrument(skip(self))]
    async fn list_active_batches_by_world(&self, world_id: WorldId) -> Result<Vec<GenerationBatch>> {
        debug!(%world_id, "Listing active batches for world");
        self.repository
            .list_active_batches_by_world(world_id)
            .await
            .context("Failed to list active batches from repository")
    }

    #[instrument(skip(self))]
    async fn list_ready_batches(&self) -> Result<Vec<GenerationBatch>> {
        debug!("Listing ready batches");
        self.repository
            .list_ready_batches()
            .await
            .context("Failed to list ready batches from repository")
    }

    #[instrument(skip(self))]
    async fn update_batch_status(&self, batch_id: BatchId, status: BatchStatus) -> Result<()> {
        debug!(batch_id = %batch_id, "Updating batch status");

        // Verify batch exists
        let _ = self
            .repository
            .get_batch(batch_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Batch not found: {}", batch_id))?;

        self.repository
            .update_batch_status(batch_id, &status)
            .await
            .context("Failed to update batch status in repository")?;

        info!(batch_id = %batch_id, "Updated batch status");
        Ok(())
    }

    #[instrument(skip(self, assets))]
    async fn update_batch_assets(&self, batch_id: BatchId, assets: Vec<AssetId>) -> Result<()> {
        debug!(batch_id = %batch_id, asset_count = assets.len(), "Updating batch assets");

        // Verify batch exists
        let _ = self
            .repository
            .get_batch(batch_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Batch not found: {}", batch_id))?;

        self.repository
            .update_batch_assets(batch_id, &assets)
            .await
            .context("Failed to update batch assets in repository")?;

        info!(batch_id = %batch_id, asset_count = assets.len(), "Updated batch assets");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn delete_batch(&self, batch_id: BatchId) -> Result<()> {
        // Verify the batch exists before deletion
        let _batch = self
            .repository
            .get_batch(batch_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Batch not found: {}", batch_id))?;

        self.repository
            .delete_batch(batch_id)
            .await
            .context("Failed to delete batch from repository")?;

        info!(batch_id = %batch_id, "Deleted batch");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_asset_request_validation() {
        // Empty file path should fail
        let request = CreateAssetRequest {
            entity_type: EntityType::Character,
            entity_id: "test".to_string(),
            asset_type: crate::domain::entities::AssetType::Portrait,
            file_path: "".to_string(),
            label: None,
        };
        assert!(AssetServiceImpl::validate_create_request(&request).is_err());

        // Empty entity ID should fail
        let request = CreateAssetRequest {
            entity_type: EntityType::Character,
            entity_id: "".to_string(),
            asset_type: crate::domain::entities::AssetType::Portrait,
            file_path: "/path/to/file.png".to_string(),
            label: None,
        };
        assert!(AssetServiceImpl::validate_create_request(&request).is_err());

        // Valid request should pass
        let request = CreateAssetRequest {
            entity_type: EntityType::Character,
            entity_id: "test-char".to_string(),
            asset_type: crate::domain::entities::AssetType::Portrait,
            file_path: "/path/to/file.png".to_string(),
            label: Some("Test Portrait".to_string()),
        };
        assert!(AssetServiceImpl::validate_create_request(&request).is_ok());
    }
}
