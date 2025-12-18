//! Asset repository implementation for Neo4j

use anyhow::Result;
use async_trait::async_trait;
use neo4rs::{query, Row};
use serde::{Deserialize, Serialize};

use super::connection::Neo4jConnection;
use crate::application::ports::outbound::AssetRepositoryPort;
use crate::domain::entities::{
    AssetType, BatchStatus, EntityType, GalleryAsset, GenerationBatch, GenerationMetadata,
};
use crate::domain::value_objects::{AssetId, BatchId};

/// Repository for GalleryAsset operations
pub struct Neo4jAssetRepository {
    connection: Neo4jConnection,
}

impl Neo4jAssetRepository {
    pub fn new(connection: Neo4jConnection) -> Self {
        Self { connection }
    }

    // ==================== GalleryAsset Operations ====================

    /// Create a new gallery asset
    pub async fn create_asset(&self, asset: &GalleryAsset) -> Result<()> {
        let generation_metadata_json = asset
            .generation_metadata
            .as_ref()
            .map(|m| serde_json::to_string(&GenerationMetadataStored::from(m.clone())))
            .transpose()?
            .unwrap_or_default();

        let q = query(
            "CREATE (a:GalleryAsset {
                id: $id,
                entity_type: $entity_type,
                entity_id: $entity_id,
                asset_type: $asset_type,
                file_path: $file_path,
                is_active: $is_active,
                label: $label,
                generation_metadata: $generation_metadata,
                created_at: $created_at
            })
            RETURN a.id as id",
        )
        .param("id", asset.id.to_string())
        .param("entity_type", asset.entity_type.to_string())
        .param("entity_id", asset.entity_id.clone())
        .param("asset_type", asset.asset_type.to_string())
        .param("file_path", asset.file_path.clone())
        .param("is_active", asset.is_active)
        .param("label", asset.label.clone().unwrap_or_default())
        .param("generation_metadata", generation_metadata_json)
        .param("created_at", asset.created_at.to_rfc3339());

        self.connection.graph().run(q).await?;

        // Create relationship to owning entity
        let relationship_query = match asset.entity_type {
            EntityType::Character => query(
                "MATCH (e:Character {id: $entity_id}), (a:GalleryAsset {id: $asset_id})
                    CREATE (e)-[:HAS_ASSET]->(a)",
            ),
            EntityType::Location => query(
                "MATCH (e:Location {id: $entity_id}), (a:GalleryAsset {id: $asset_id})
                    CREATE (e)-[:HAS_ASSET]->(a)",
            ),
            EntityType::Item => query(
                "MATCH (e:Item {id: $entity_id}), (a:GalleryAsset {id: $asset_id})
                    CREATE (e)-[:HAS_ASSET]->(a)",
            ),
        };

        self.connection
            .graph()
            .run(
                relationship_query
                    .param("entity_id", asset.entity_id.clone())
                    .param("asset_id", asset.id.to_string()),
            )
            .await?;

        tracing::debug!(
            "Created gallery asset: {} for {}",
            asset.id,
            asset.entity_id
        );
        Ok(())
    }

    /// Get an asset by ID
    pub async fn get_asset(&self, id: AssetId) -> Result<Option<GalleryAsset>> {
        let q = query(
            "MATCH (a:GalleryAsset {id: $id})
            RETURN a",
        )
        .param("id", id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            Ok(Some(row_to_gallery_asset(row)?))
        } else {
            Ok(None)
        }
    }

    /// List all assets for an entity
    pub async fn list_by_entity(
        &self,
        entity_type: EntityType,
        entity_id: &str,
    ) -> Result<Vec<GalleryAsset>> {
        let q = query(
            "MATCH (a:GalleryAsset {entity_type: $entity_type, entity_id: $entity_id})
            RETURN a
            ORDER BY a.created_at DESC",
        )
        .param("entity_type", entity_type.to_string())
        .param("entity_id", entity_id);

        let mut result = self.connection.graph().execute(q).await?;
        let mut assets = Vec::new();

        while let Some(row) = result.next().await? {
            assets.push(row_to_gallery_asset(row)?);
        }

        Ok(assets)
    }

    /// List assets of a specific type for an entity
    pub async fn list_by_entity_and_type(
        &self,
        entity_type: EntityType,
        entity_id: &str,
        asset_type: AssetType,
    ) -> Result<Vec<GalleryAsset>> {
        let q = query(
            "MATCH (a:GalleryAsset {
                entity_type: $entity_type,
                entity_id: $entity_id,
                asset_type: $asset_type
            })
            RETURN a
            ORDER BY a.created_at DESC",
        )
        .param("entity_type", entity_type.to_string())
        .param("entity_id", entity_id)
        .param("asset_type", asset_type.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut assets = Vec::new();

        while let Some(row) = result.next().await? {
            assets.push(row_to_gallery_asset(row)?);
        }

        Ok(assets)
    }

    /// Get the active asset for an entity and type
    pub async fn get_active_asset(
        &self,
        entity_type: EntityType,
        entity_id: &str,
        asset_type: AssetType,
    ) -> Result<Option<GalleryAsset>> {
        let q = query(
            "MATCH (a:GalleryAsset {
                entity_type: $entity_type,
                entity_id: $entity_id,
                asset_type: $asset_type,
                is_active: true
            })
            RETURN a
            LIMIT 1",
        )
        .param("entity_type", entity_type.to_string())
        .param("entity_id", entity_id)
        .param("asset_type", asset_type.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            Ok(Some(row_to_gallery_asset(row)?))
        } else {
            Ok(None)
        }
    }

    /// Set an asset as active (deactivates others of same type)
    pub async fn activate_asset(&self, id: AssetId) -> Result<()> {
        // First get the asset to know its entity and type
        let asset = self
            .get_asset(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Asset not found"))?;

        // Deactivate all other assets of same type for same entity
        let deactivate_q = query(
            "MATCH (a:GalleryAsset {
                entity_type: $entity_type,
                entity_id: $entity_id,
                asset_type: $asset_type
            })
            SET a.is_active = false",
        )
        .param("entity_type", asset.entity_type.to_string())
        .param("entity_id", asset.entity_id.clone())
        .param("asset_type", asset.asset_type.to_string());

        self.connection.graph().run(deactivate_q).await?;

        // Activate the specified asset
        let activate_q = query(
            "MATCH (a:GalleryAsset {id: $id})
            SET a.is_active = true",
        )
        .param("id", id.to_string());

        self.connection.graph().run(activate_q).await?;
        tracing::debug!("Activated asset: {}", id);
        Ok(())
    }

    /// Update asset label
    pub async fn update_label(&self, id: AssetId, label: Option<String>) -> Result<()> {
        let q = query(
            "MATCH (a:GalleryAsset {id: $id})
            SET a.label = $label",
        )
        .param("id", id.to_string())
        .param("label", label.unwrap_or_default());

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Delete an asset
    pub async fn delete_asset(&self, id: AssetId) -> Result<()> {
        let q = query(
            "MATCH (a:GalleryAsset {id: $id})
            DETACH DELETE a",
        )
        .param("id", id.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!("Deleted gallery asset: {}", id);
        Ok(())
    }

    // ==================== GenerationBatch Operations ====================

    /// Create a new generation batch
    pub async fn create_batch(&self, batch: &GenerationBatch) -> Result<()> {
        let assets_json = serde_json::to_string(
            &batch.assets.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
        )?;
        let status_json = serde_json::to_string(&BatchStatusStored::from(batch.status.clone()))?;

        let q = query(
            "CREATE (b:GenerationBatch {
                id: $id,
                world_id: $world_id,
                entity_type: $entity_type,
                entity_id: $entity_id,
                asset_type: $asset_type,
                workflow: $workflow,
                prompt: $prompt,
                negative_prompt: $negative_prompt,
                count: $count,
                status: $status,
                assets: $assets,
                style_reference_id: $style_reference_id,
                requested_at: $requested_at,
                completed_at: $completed_at
            })
            RETURN b.id as id",
        )
        .param("id", batch.id.to_string())
        .param("world_id", batch.world_id.to_string())
        .param("entity_type", batch.entity_type.to_string())
        .param("entity_id", batch.entity_id.clone())
        .param("asset_type", batch.asset_type.to_string())
        .param("workflow", batch.workflow.clone())
        .param("prompt", batch.prompt.clone())
        .param(
            "negative_prompt",
            batch.negative_prompt.clone().unwrap_or_default(),
        )
        .param("count", batch.count as i64)
        .param("status", status_json)
        .param("assets", assets_json)
        .param(
            "style_reference_id",
            batch
                .style_reference_id
                .map(|id| id.to_string())
                .unwrap_or_default(),
        )
        .param("requested_at", batch.requested_at.to_rfc3339())
        .param(
            "completed_at",
            batch
                .completed_at
                .map(|t| t.to_rfc3339())
                .unwrap_or_default(),
        );

        self.connection.graph().run(q).await?;
        tracing::debug!("Created generation batch: {}", batch.id);
        Ok(())
    }

    /// Get a batch by ID
    pub async fn get_batch(&self, id: BatchId) -> Result<Option<GenerationBatch>> {
        let q = query(
            "MATCH (b:GenerationBatch {id: $id})
            RETURN b",
        )
        .param("id", id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            Ok(Some(row_to_generation_batch(row)?))
        } else {
            Ok(None)
        }
    }

    /// List all non-terminal batches for a specific world
    pub async fn list_active_batches_by_world(&self, world_id: crate::domain::value_objects::WorldId) -> Result<Vec<GenerationBatch>> {
        let q = query(
            "MATCH (b:GenerationBatch)
            WHERE b.world_id = $world_id
              AND NOT b.status CONTAINS 'Completed' 
              AND NOT b.status CONTAINS 'Failed'
            RETURN b
            ORDER BY b.requested_at ASC",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut batches = Vec::new();

        while let Some(row) = result.next().await? {
            batches.push(row_to_generation_batch(row)?);
        }

        Ok(batches)
    }

    /// List batches ready for selection
    pub async fn list_ready_batches(&self) -> Result<Vec<GenerationBatch>> {
        let q = query(
            "MATCH (b:GenerationBatch)
            WHERE b.status CONTAINS 'ReadyForSelection'
            RETURN b
            ORDER BY b.completed_at DESC",
        );

        let mut result = self.connection.graph().execute(q).await?;
        let mut batches = Vec::new();

        while let Some(row) = result.next().await? {
            batches.push(row_to_generation_batch(row)?);
        }

        Ok(batches)
    }

    /// List batches for an entity
    pub async fn list_batches_by_entity(
        &self,
        entity_type: EntityType,
        entity_id: &str,
    ) -> Result<Vec<GenerationBatch>> {
        let q = query(
            "MATCH (b:GenerationBatch {entity_type: $entity_type, entity_id: $entity_id})
            RETURN b
            ORDER BY b.requested_at DESC",
        )
        .param("entity_type", entity_type.to_string())
        .param("entity_id", entity_id);

        let mut result = self.connection.graph().execute(q).await?;
        let mut batches = Vec::new();

        while let Some(row) = result.next().await? {
            batches.push(row_to_generation_batch(row)?);
        }

        Ok(batches)
    }

    /// Update batch status
    pub async fn update_batch_status(&self, id: BatchId, status: &BatchStatus) -> Result<()> {
        let status_json = serde_json::to_string(&BatchStatusStored::from(status.clone()))?;
        let completed_at = if status.is_terminal() {
            chrono::Utc::now().to_rfc3339()
        } else {
            String::new()
        };

        let q = query(
            "MATCH (b:GenerationBatch {id: $id})
            SET b.status = $status,
                b.completed_at = CASE WHEN $completed_at <> '' THEN $completed_at ELSE b.completed_at END"
        )
        .param("id", id.to_string())
        .param("status", status_json)
        .param("completed_at", completed_at);

        self.connection.graph().run(q).await?;
        tracing::debug!("Updated batch {} status to {:?}", id, status);
        Ok(())
    }

    /// Update batch with generated assets
    pub async fn update_batch_assets(&self, id: BatchId, assets: &[AssetId]) -> Result<()> {
        let assets_json = serde_json::to_string(
            &assets.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
        )?;

        let q = query(
            "MATCH (b:GenerationBatch {id: $id})
            SET b.assets = $assets",
        )
        .param("id", id.to_string())
        .param("assets", assets_json);

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Delete a batch
    pub async fn delete_batch(&self, id: BatchId) -> Result<()> {
        let q = query(
            "MATCH (b:GenerationBatch {id: $id})
            DETACH DELETE b",
        )
        .param("id", id.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!("Deleted generation batch: {}", id);
        Ok(())
    }
}

fn row_to_gallery_asset(row: Row) -> Result<GalleryAsset> {
    let node: neo4rs::Node = row.get("a")?;

    let id_str: String = node.get("id")?;
    let entity_type_str: String = node.get("entity_type")?;
    let entity_id: String = node.get("entity_id")?;
    let asset_type_str: String = node.get("asset_type")?;
    let file_path: String = node.get("file_path")?;
    let is_active: bool = node.get("is_active")?;
    let label: String = node.get("label")?;
    let generation_metadata_json: String = node.get("generation_metadata")?;
    let created_at_str: String = node.get("created_at")?;

    let id = uuid::Uuid::parse_str(&id_str)?;
    let entity_type = parse_entity_type(&entity_type_str);
    let asset_type = AssetType::from_str(&asset_type_str)
        .ok_or_else(|| anyhow::anyhow!("Invalid asset type: {}", asset_type_str))?;
    let generation_metadata: Option<GenerationMetadata> = if generation_metadata_json.is_empty() {
        None
    } else {
        serde_json::from_str::<GenerationMetadataStored>(&generation_metadata_json)
            .ok()
            .map(Into::into)
    };
    let created_at =
        chrono::DateTime::parse_from_rfc3339(&created_at_str)?.with_timezone(&chrono::Utc);

    Ok(GalleryAsset {
        id: AssetId::from_uuid(id),
        entity_type,
        entity_id,
        asset_type,
        file_path,
        is_active,
        label: if label.is_empty() { None } else { Some(label) },
        generation_metadata,
        created_at,
    })
}

fn row_to_generation_batch(row: Row) -> Result<GenerationBatch> {
    use crate::domain::value_objects::WorldId;
    
    let node: neo4rs::Node = row.get("b")?;

    let id_str: String = node.get("id")?;
    let world_id_str: String = node.get("world_id")?;
    let entity_type_str: String = node.get("entity_type")?;
    let entity_id: String = node.get("entity_id")?;
    let asset_type_str: String = node.get("asset_type")?;
    let workflow: String = node.get("workflow")?;
    let prompt: String = node.get("prompt")?;
    let negative_prompt: String = node.get("negative_prompt")?;
    let count: i64 = node.get("count")?;
    let status_json: String = node.get("status")?;
    let assets_json: String = node.get("assets")?;
    let style_reference_id_str: String = node.get("style_reference_id")?;
    let requested_at_str: String = node.get("requested_at")?;
    let completed_at_str: String = node.get("completed_at")?;

    let id = uuid::Uuid::parse_str(&id_str)?;
    let world_id = WorldId::from_uuid(uuid::Uuid::parse_str(&world_id_str)?);
    let entity_type = parse_entity_type(&entity_type_str);
    let asset_type = AssetType::from_str(&asset_type_str)
        .ok_or_else(|| anyhow::anyhow!("Invalid asset type: {}", asset_type_str))?;
    let status: BatchStatus = serde_json::from_str::<BatchStatusStored>(&status_json)?.into();
    let assets: Vec<AssetId> = serde_json::from_str::<Vec<String>>(&assets_json)?
        .into_iter()
        .filter_map(|s| uuid::Uuid::parse_str(&s).ok().map(AssetId::from_uuid))
        .collect();
    let style_reference_id = if style_reference_id_str.is_empty() {
        None
    } else {
        uuid::Uuid::parse_str(&style_reference_id_str)
            .ok()
            .map(AssetId::from_uuid)
    };
    let requested_at =
        chrono::DateTime::parse_from_rfc3339(&requested_at_str)?.with_timezone(&chrono::Utc);
    let completed_at = if completed_at_str.is_empty() {
        None
    } else {
        chrono::DateTime::parse_from_rfc3339(&completed_at_str)
            .ok()
            .map(|t| t.with_timezone(&chrono::Utc))
    };

    Ok(GenerationBatch {
        id: BatchId::from_uuid(id),
        world_id,
        entity_type,
        entity_id,
        asset_type,
        workflow,
        prompt,
        negative_prompt: if negative_prompt.is_empty() {
            None
        } else {
            Some(negative_prompt)
        },
        count: count as u8,
        status,
        assets,
        style_reference_id,
        requested_at,
        completed_at,
    })
}

// ============================================================================
// Persistence serde models (so domain doesn't require serde)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
enum BatchStatusStored {
    Queued,
    Generating { progress: u8 },
    ReadyForSelection,
    Completed,
    Failed { error: String },
}

impl From<BatchStatus> for BatchStatusStored {
    fn from(value: BatchStatus) -> Self {
        match value {
            BatchStatus::Queued => Self::Queued,
            BatchStatus::Generating { progress } => Self::Generating { progress },
            BatchStatus::ReadyForSelection => Self::ReadyForSelection,
            BatchStatus::Completed => Self::Completed,
            BatchStatus::Failed { error } => Self::Failed { error },
        }
    }
}

impl From<BatchStatusStored> for BatchStatus {
    fn from(value: BatchStatusStored) -> Self {
        match value {
            BatchStatusStored::Queued => Self::Queued,
            BatchStatusStored::Generating { progress } => Self::Generating { progress },
            BatchStatusStored::ReadyForSelection => Self::ReadyForSelection,
            BatchStatusStored::Completed => Self::Completed,
            BatchStatusStored::Failed { error } => Self::Failed { error },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GenerationMetadataStored {
    pub workflow: String,
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub seed: i64,
    pub style_reference_id: Option<String>,
    pub batch_id: String,
}

impl From<GenerationMetadata> for GenerationMetadataStored {
    fn from(value: GenerationMetadata) -> Self {
        Self {
            workflow: value.workflow,
            prompt: value.prompt,
            negative_prompt: value.negative_prompt,
            seed: value.seed,
            style_reference_id: value.style_reference_id.map(|id| id.to_string()),
            batch_id: value.batch_id.to_string(),
        }
    }
}

impl From<GenerationMetadataStored> for GenerationMetadata {
    fn from(value: GenerationMetadataStored) -> Self {
        let style_reference_id = value
            .style_reference_id
            .and_then(|s| uuid::Uuid::parse_str(&s).ok())
            .map(AssetId::from_uuid);
        let batch_id = uuid::Uuid::parse_str(&value.batch_id)
            .ok()
            .map(BatchId::from_uuid)
            .unwrap_or_else(BatchId::new);

        Self {
            workflow: value.workflow,
            prompt: value.prompt,
            negative_prompt: value.negative_prompt,
            seed: value.seed,
            style_reference_id,
            batch_id,
        }
    }
}

fn parse_entity_type(s: &str) -> EntityType {
    match s {
        "Character" => EntityType::Character,
        "Location" => EntityType::Location,
        "Item" => EntityType::Item,
        _ => EntityType::Character, // Default fallback
    }
}

// =============================================================================
// AssetRepositoryPort Implementation
// =============================================================================

#[async_trait]
impl AssetRepositoryPort for Neo4jAssetRepository {
    async fn create(&self, asset: &GalleryAsset) -> Result<()> {
        Neo4jAssetRepository::create_asset(self, asset).await
    }

    async fn get(&self, id: AssetId) -> Result<Option<GalleryAsset>> {
        Neo4jAssetRepository::get_asset(self, id).await
    }

    async fn list_for_entity(&self, entity_type: &str, entity_id: &str) -> Result<Vec<GalleryAsset>> {
        let entity_type = parse_entity_type(entity_type);
        Neo4jAssetRepository::list_by_entity(self, entity_type, entity_id).await
    }

    async fn activate(&self, id: AssetId) -> Result<()> {
        Neo4jAssetRepository::activate_asset(self, id).await
    }

    async fn update_label(&self, id: AssetId, label: Option<String>) -> Result<()> {
        Neo4jAssetRepository::update_label(self, id, label).await
    }

    async fn delete(&self, id: AssetId) -> Result<()> {
        Neo4jAssetRepository::delete_asset(self, id).await
    }

    async fn create_batch(&self, batch: &GenerationBatch) -> Result<()> {
        Neo4jAssetRepository::create_batch(self, batch).await
    }

    async fn get_batch(&self, id: BatchId) -> Result<Option<GenerationBatch>> {
        Neo4jAssetRepository::get_batch(self, id).await
    }

    async fn update_batch_status(&self, id: BatchId, status: &BatchStatus) -> Result<()> {
        Neo4jAssetRepository::update_batch_status(self, id, status).await
    }

    async fn update_batch_assets(&self, id: BatchId, assets: &[AssetId]) -> Result<()> {
        Neo4jAssetRepository::update_batch_assets(self, id, assets).await
    }

    async fn list_active_batches_by_world(&self, world_id: crate::domain::value_objects::WorldId) -> Result<Vec<GenerationBatch>> {
        Neo4jAssetRepository::list_active_batches_by_world(self, world_id).await
    }

    async fn list_ready_batches(&self) -> Result<Vec<GenerationBatch>> {
        Neo4jAssetRepository::list_ready_batches(self).await
    }

    async fn delete_batch(&self, id: BatchId) -> Result<()> {
        Neo4jAssetRepository::delete_batch(self, id).await
    }
}
