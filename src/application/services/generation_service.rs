//! Generation Service - Manages the asset generation queue
//!
//! This service handles:
//! - Queueing generation requests
//! - Processing batches through ComfyUI
//! - Tracking progress and notifying clients via WebSocket

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use chrono::Utc;
use tokio::sync::{mpsc, RwLock};

use crate::application::ports::outbound::{AssetRepositoryPort, ComfyUIPort};
use crate::domain::entities::{
    AssetType, BatchStatus, EntityType, GalleryAsset, GenerationBatch, GenerationMetadata,
};
use crate::domain::value_objects::{AssetId, BatchId, WorldId};

/// Events emitted by the generation service
#[derive(Debug, Clone)]
pub enum GenerationEvent {
    /// A batch has been queued
    BatchQueued {
        batch_id: BatchId,
        entity_type: EntityType,
        entity_id: String,
        asset_type: AssetType,
        position: u32,
    },
    /// A batch is generating (progress update)
    BatchProgress {
        batch_id: BatchId,
        progress: u8,
    },
    /// A batch has completed
    BatchComplete {
        batch_id: BatchId,
        entity_type: EntityType,
        entity_id: String,
        asset_type: AssetType,
        asset_count: u32,
    },
    /// A batch has failed
    BatchFailed {
        batch_id: BatchId,
        entity_type: EntityType,
        entity_id: String,
        asset_type: AssetType,
        error: String,
    },
    /// A suggestion request has been queued
    SuggestionQueued {
        request_id: String,
        field_type: String,
        entity_id: Option<String>,
        world_id: Option<WorldId>,
    },
    /// A suggestion request is being processed
    SuggestionProgress {
        request_id: String,
        status: String,
        world_id: Option<WorldId>,
    },
    /// A suggestion request has completed
    SuggestionComplete {
        request_id: String,
        field_type: String,
        suggestions: Vec<String>,
        world_id: Option<WorldId>,
    },
    /// A suggestion request has failed
    SuggestionFailed {
        request_id: String,
        field_type: String,
        error: String,
        world_id: Option<WorldId>,
    },
}

/// Request to generate assets
#[derive(Debug, Clone)]
pub struct GenerationRequest {
    pub world_id: WorldId,
    pub entity_type: EntityType,
    pub entity_id: String,
    pub asset_type: AssetType,
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub count: u8,
    pub style_reference_id: Option<AssetId>,
}

/// Generation service for managing asset generation
pub struct GenerationService {
    /// ComfyUI client for sending generation requests
    comfyui_client: Arc<dyn ComfyUIPort>,
    /// Asset repository for persisting results
    repository: Arc<dyn AssetRepositoryPort>,
    /// Directory to save generated assets
    output_dir: PathBuf,
    /// Active batches being processed
    active_batches: RwLock<HashMap<BatchId, BatchTracker>>,
    /// Event sender for notifying about generation progress
    event_sender: mpsc::UnboundedSender<GenerationEvent>,
    /// Workflow templates directory
    workflow_dir: PathBuf,
}

/// Tracks an active batch being processed
struct BatchTracker {
    batch: GenerationBatch,
    prompt_ids: Vec<String>,
    completed_count: u8,
}

impl GenerationService {
    /// Create a new generation service
    pub fn new(
        comfyui_client: Arc<dyn ComfyUIPort>,
        repository: Arc<dyn AssetRepositoryPort>,
        output_dir: PathBuf,
        workflow_dir: PathBuf,
        event_sender: mpsc::UnboundedSender<GenerationEvent>,
    ) -> Self {
        Self {
            comfyui_client,
            repository,
            output_dir,
            active_batches: RwLock::new(HashMap::new()),
            event_sender,
            workflow_dir,
        }
    }

    /// Queue a new generation batch
    pub async fn queue_generation(&self, request: GenerationRequest) -> Result<BatchId> {
        // Create a new batch
        let batch_id = BatchId::new();
        let workflow = self.get_workflow_name(&request.asset_type);

        let batch = GenerationBatch {
            id: batch_id,
            world_id: request.world_id,
            entity_type: request.entity_type,
            entity_id: request.entity_id.clone(),
            asset_type: request.asset_type,
            workflow: workflow.clone(),
            prompt: request.prompt.clone(),
            negative_prompt: request.negative_prompt.clone(),
            count: request.count,
            status: BatchStatus::Queued,
            assets: vec![],
            style_reference_id: request.style_reference_id,
            requested_at: Utc::now(),
            completed_at: None,
        };

        // Persist the batch
        self.repository.create_batch(&batch).await?;

        // Get queue position
        let active_batches = self.active_batches.read().await;
        let position = active_batches.len() as u32 + 1;
        drop(active_batches);

        // Send queued event
        let _ = self.event_sender.send(GenerationEvent::BatchQueued {
            batch_id,
            entity_type: batch.entity_type,
            entity_id: batch.entity_id.clone(),
            asset_type: batch.asset_type,
            position,
        });

        // Start processing (this would normally be done by a background worker)
        self.start_batch_processing(batch).await?;

        Ok(batch_id)
    }

    /// Start processing a batch
    pub async fn start_batch_processing(&self, batch: GenerationBatch) -> Result<()> {
        let batch_id = batch.id;

        // Update status to generating
        self.repository
            .update_batch_status(batch_id, &BatchStatus::Generating { progress: 0 })
            .await?;

        // Create tracker
        let tracker = BatchTracker {
            batch: batch.clone(),
            prompt_ids: vec![],
            completed_count: 0,
        };

        self.active_batches.write().await.insert(batch_id, tracker);

        // Load the workflow template
        let workflow_template = self.load_workflow_template(&batch.workflow).await?;

        // Queue each generation
        let mut prompt_ids = Vec::new();
        for i in 0..batch.count {
            // Modify the workflow with our parameters
            let workflow = self.prepare_workflow(
                workflow_template.clone(),
                &batch.prompt,
                batch.negative_prompt.as_deref(),
                i as i64, // Use index as seed variation
                &batch.asset_type,
                batch.style_reference_id,
            )
            .await?;

            // Queue with ComfyUI
            match self.comfyui_client.queue_prompt(workflow).await {
                Ok(response) => {
                    prompt_ids.push(response.prompt_id);
                }
                Err(e) => {
                    tracing::error!("Failed to queue prompt {} for batch {}: {}", i, batch_id, e);
                    // Continue with other prompts
                }
            }
        }

        // Update tracker with prompt IDs
        if let Some(tracker) = self.active_batches.write().await.get_mut(&batch_id) {
            tracker.prompt_ids = prompt_ids;
        }

        Ok(())
    }

    /// Poll for batch completion (call periodically)
    pub async fn poll_batch_progress(&self, batch_id: BatchId) -> Result<Option<BatchStatus>> {
        let active_batches = self.active_batches.read().await;
        let tracker = match active_batches.get(&batch_id) {
            Some(t) => t,
            None => return Ok(None),
        };

        let prompt_ids = tracker.prompt_ids.clone();
        let batch = tracker.batch.clone();
        drop(active_batches);

        let mut completed_count = 0u8;
        let mut generated_assets = Vec::new();

        for prompt_id in &prompt_ids {
            match self.comfyui_client.get_history(prompt_id).await {
                Ok(history) => {
                    if let Some(prompt_history) = history.prompts.get(prompt_id) {
                        if prompt_history.status.completed {
                            completed_count += 1;

                            // Extract generated images
                            for (_node_id, output) in &prompt_history.outputs {
                                if let Some(images) = &output.images {
                                    for image in images {
                                        // Download and save the image
                                        match self
                                            .download_and_save_asset(
                                                &batch,
                                                &image.filename,
                                                &image.subfolder,
                                                &image.r#type,
                                            )
                                            .await
                                        {
                                            Ok(asset) => generated_assets.push(asset),
                                            Err(e) => {
                                                tracing::error!("Failed to download asset: {}", e);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to get history for prompt {}: {}", prompt_id, e);
                }
            }
        }

        // Calculate progress
        let progress = if prompt_ids.is_empty() {
            0
        } else {
            ((completed_count as f32 / prompt_ids.len() as f32) * 100.0) as u8
        };

        // Update progress
        let _ = self
            .event_sender
            .send(GenerationEvent::BatchProgress { batch_id, progress });

        // Check if all completed
        if completed_count as usize >= prompt_ids.len() {
            // Batch is complete
            let asset_ids: Vec<AssetId> = generated_assets.iter().map(|a| a.id).collect();

            self.repository
                .update_batch_status(batch_id, &BatchStatus::ReadyForSelection)
                .await?;
            self.repository
                .update_batch_assets(batch_id, &asset_ids)
                .await?;

            let _ = self.event_sender.send(GenerationEvent::BatchComplete {
                batch_id,
                entity_type: batch.entity_type,
                entity_id: batch.entity_id.clone(),
                asset_type: batch.asset_type,
                asset_count: generated_assets.len() as u32,
            });

            // Remove from active batches
            self.active_batches.write().await.remove(&batch_id);

            return Ok(Some(BatchStatus::ReadyForSelection));
        }

        Ok(Some(BatchStatus::Generating { progress }))
    }

    /// Download an asset from ComfyUI and save it locally
    async fn download_and_save_asset(
        &self,
        batch: &GenerationBatch,
        filename: &str,
        subfolder: &str,
        folder_type: &str,
    ) -> Result<GalleryAsset> {
        // Download the image
        let image_data = self
            .comfyui_client
            .get_image(filename, subfolder, folder_type)
            .await?;

        // Create output directory structure
        let entity_dir = self
            .output_dir
            .join(batch.entity_type.as_str())
            .join(&batch.entity_id)
            .join(batch.asset_type.as_str());
        tokio::fs::create_dir_all(&entity_dir).await?;

        // Generate unique filename
        let asset_id = AssetId::new();
        let extension = std::path::Path::new(filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png");
        let output_filename = format!("{}.{}", asset_id, extension);
        let output_path = entity_dir.join(&output_filename);

        // Save the file
        tokio::fs::write(&output_path, &image_data).await?;

        // Create generation metadata
        let mut metadata = GenerationMetadata::new(
            batch.workflow.clone(),
            batch.prompt.clone(),
            0, // seed - will be set when we have access to actual seed from ComfyUI
            batch.id,
        );
        if let Some(ref neg) = batch.negative_prompt {
            metadata = metadata.with_negative_prompt(neg.clone());
        }
        if let Some(style_ref) = batch.style_reference_id {
            metadata = metadata.with_style_reference(style_ref);
        }

        // Create asset entity
        let mut asset = GalleryAsset::new_generated(
            batch.entity_type.clone(),
            batch.entity_id.clone(),
            batch.asset_type.clone(),
            output_path.to_string_lossy().to_string(),
            metadata,
        );
        // Override the auto-generated ID with our pre-generated one for the filename
        asset.id = asset_id;

        // Persist the asset
        self.repository.create(&asset).await?;

        Ok(asset)
    }

    /// Load a workflow template from file
    async fn load_workflow_template(&self, workflow_name: &str) -> Result<serde_json::Value> {
        let path = self.workflow_dir.join(format!("{}.json", workflow_name));

        if path.exists() {
            let content = tokio::fs::read_to_string(&path).await?;
            let workflow: serde_json::Value = serde_json::from_str(&content)?;
            Ok(workflow)
        } else {
            // Return a basic placeholder workflow
            tracing::warn!(
                "Workflow template not found: {}, using placeholder",
                workflow_name
            );
            Ok(self.get_placeholder_workflow())
        }
    }

    /// Prepare a workflow with generation parameters
    async fn prepare_workflow(
        &self,
        mut workflow: serde_json::Value,
        prompt: &str,
        negative_prompt: Option<&str>,
        seed_offset: i64,
        asset_type: &AssetType,
        style_reference_id: Option<AssetId>,
    ) -> Result<serde_json::Value> {
        // Get dimensions for this asset type
        let (width, height) = asset_type.default_dimensions();

        // This is a simplified workflow preparation
        // In practice, you'd need to navigate the workflow JSON structure
        // and update specific nodes

        if let Some(obj) = workflow.as_object_mut() {
            // Try to find and update common node types
            for (_node_id, node) in obj.iter_mut() {
                if let Some(node_obj) = node.as_object_mut() {
                    if let Some(inputs) = node_obj.get_mut("inputs") {
                        if let Some(inputs_obj) = inputs.as_object_mut() {
                            // Update text prompts
                            if inputs_obj.contains_key("text") {
                                inputs_obj.insert(
                                    "text".to_string(),
                                    serde_json::Value::String(prompt.to_string()),
                                );
                            }

                            // Update positive prompt
                            if inputs_obj.contains_key("positive") {
                                if let Some(s) = inputs_obj.get("positive") {
                                    if s.is_string() {
                                        inputs_obj.insert(
                                            "positive".to_string(),
                                            serde_json::Value::String(prompt.to_string()),
                                        );
                                    }
                                }
                            }

                            // Update negative prompt
                            if let Some(neg) = negative_prompt {
                                if inputs_obj.contains_key("negative") {
                                    if let Some(s) = inputs_obj.get("negative") {
                                        if s.is_string() {
                                            inputs_obj.insert(
                                                "negative".to_string(),
                                                serde_json::Value::String(neg.to_string()),
                                            );
                                        }
                                    }
                                }
                            }

                            // Update seed with offset
                            if inputs_obj.contains_key("seed") {
                                let base_seed =
                                    inputs_obj.get("seed").and_then(|s| s.as_i64()).unwrap_or(0);
                                inputs_obj.insert(
                                    "seed".to_string(),
                                    serde_json::Value::Number((base_seed + seed_offset).into()),
                                );
                            }

                            // Update dimensions
                            if inputs_obj.contains_key("width") {
                                inputs_obj.insert(
                                    "width".to_string(),
                                    serde_json::Value::Number(width.into()),
                                );
                            }
                            if inputs_obj.contains_key("height") {
                                inputs_obj.insert(
                                    "height".to_string(),
                                    serde_json::Value::Number(height.into()),
                                );
                            }
                        }
                    }
                }
            }
        }

        // Inject style reference if provided
        if let Some(ref_id) = style_reference_id {
            // Try to load the style reference asset
            if let Ok(Some(ref_asset)) = self.repository.get(ref_id).await {
                // Try to find IPAdapter node and inject image
                if let Some(obj) = workflow.as_object_mut() {
                    let mut ipadapter_found = false;
                    
                    // First pass: find IPAdapter nodes
                    for (_node_id, node) in obj.iter() {
                        if let Some(node_obj) = node.as_object() {
                            if let Some(class_type) = node_obj.get("class_type").and_then(|c| c.as_str()) {
                                if class_type.contains("IPAdapter") {
                                    ipadapter_found = true;
                                    break;
                                }
                            }
                        }
                    }
                    
                    // If IPAdapter found, inject image path
                    if ipadapter_found {
                        for (_node_id, node) in obj.iter_mut() {
                            if let Some(node_obj) = node.as_object_mut() {
                                if let Some(class_type) = node_obj.get("class_type").and_then(|c| c.as_str()) {
                                    if class_type.contains("IPAdapter") {
                                        if let Some(inputs) = node_obj.get_mut("inputs") {
                                            if let Some(inputs_obj) = inputs.as_object_mut() {
                                                // Inject image path - ComfyUI expects format: "filename" or full path
                                                // For now, use the file_path from the asset
                                                inputs_obj.insert(
                                                    "image".to_string(),
                                                    serde_json::Value::String(ref_asset.file_path.clone()),
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        // No IPAdapter found - inject style keywords into prompt
                        // This is a simplified approach - in production, you'd extract style keywords
                        // from the reference asset's generation metadata or analyze the image
                        let enhanced_prompt = format!("{}, in the style of: {}", prompt, "reference style");
                        
                        // Update prompt in workflow
                        for (_node_id, node) in obj.iter_mut() {
                            if let Some(node_obj) = node.as_object_mut() {
                                if let Some(inputs) = node_obj.get_mut("inputs") {
                                    if let Some(inputs_obj) = inputs.as_object_mut() {
                                        if inputs_obj.contains_key("text") {
                                            inputs_obj.insert(
                                                "text".to_string(),
                                                serde_json::Value::String(enhanced_prompt.clone()),
                                            );
                                        }
                                        if inputs_obj.contains_key("positive") {
                                            if let Some(s) = inputs_obj.get("positive") {
                                                if s.is_string() {
                                                    inputs_obj.insert(
                                                        "positive".to_string(),
                                                        serde_json::Value::String(enhanced_prompt.clone()),
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(workflow)
    }

    /// Get the workflow name for an asset type
    fn get_workflow_name(&self, asset_type: &AssetType) -> String {
        match asset_type {
            AssetType::Portrait => "character-portrait-generation".to_string(),
            AssetType::Sprite => "character-sprite-generation".to_string(),
            AssetType::Backdrop => "backdrop-generation".to_string(),
            AssetType::Tilesheet => "tilesheet-generation".to_string(),
            AssetType::ItemIcon => "item-icon-generation".to_string(),
            AssetType::EmotionSheet => "character-emotion-sheet".to_string(),
            AssetType::RegionBackdrop => "map-region-backdrop".to_string(),
        }
    }

    /// Get a placeholder workflow for testing
    fn get_placeholder_workflow(&self) -> serde_json::Value {
        serde_json::json!({
            "3": {
                "class_type": "KSampler",
                "inputs": {
                    "seed": 0,
                    "steps": 20,
                    "cfg": 7.0,
                    "sampler_name": "euler",
                    "scheduler": "normal",
                    "denoise": 1.0
                }
            },
            "6": {
                "class_type": "CLIPTextEncode",
                "inputs": {
                    "text": "placeholder prompt"
                }
            }
        })
    }

    /// Get the status of a batch
    pub async fn get_batch_status(&self, batch_id: BatchId) -> Result<Option<GenerationBatch>> {
        self.repository.get_batch(batch_id).await
    }

    /// List all active (queued or generating) batches for a specific world
    pub async fn list_active_batches_by_world(&self, world_id: WorldId) -> Result<Vec<GenerationBatch>> {
        self.repository.list_active_batches_by_world(world_id).await
    }

    /// List batches ready for selection
    pub async fn list_ready_batches(&self) -> Result<Vec<GenerationBatch>> {
        self.repository.list_ready_batches().await
    }

    /// Mark batch as completed after user selection
    pub async fn complete_batch(&self, batch_id: BatchId) -> Result<()> {
        self.repository
            .update_batch_status(batch_id, &BatchStatus::Completed)
            .await
    }

    /// Cancel a batch
    pub async fn cancel_batch(&self, batch_id: BatchId) -> Result<()> {
        self.active_batches.write().await.remove(&batch_id);
        self.repository.delete_batch(batch_id).await
    }
}
