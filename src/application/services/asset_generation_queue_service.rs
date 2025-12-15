//! Asset Generation Queue Service - Concurrency-controlled asset generation
//!
//! This service manages the AssetGenerationQueue, which processes ComfyUI
//! requests with controlled concurrency (typically batch_size=1).

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Semaphore;

use crate::application::ports::outbound::{
    AssetRepositoryPort, ComfyUIPort, ProcessingQueuePort, QueueError, QueueItemId, QueueNotificationPort,
};
use crate::application::dto::AssetGenerationItem;

/// Priority constants for queue operations
const PRIORITY_NORMAL: u8 = 0;

/// Service for managing the asset generation queue
pub struct AssetGenerationQueueService<
    Q: ProcessingQueuePort<AssetGenerationItem>,
    C: ComfyUIPort,
    N: QueueNotificationPort,
> {
    pub(crate) queue: Arc<Q>,
    comfyui_client: Arc<C>,
    asset_repository: Arc<dyn AssetRepositoryPort>,
    semaphore: Arc<Semaphore>,
    notifier: N,
}

impl<Q: ProcessingQueuePort<AssetGenerationItem> + 'static, C: ComfyUIPort + 'static, N: QueueNotificationPort + 'static>
    AssetGenerationQueueService<Q, C, N>
{
    /// Create a new asset generation queue service
    ///
    /// # Arguments
    ///
    /// * `queue` - The asset generation queue
    /// * `comfyui_client` - The ComfyUI client for processing requests
    /// * `asset_repository` - The asset repository for persisting results
    /// * `batch_size` - Maximum concurrent ComfyUI requests (typically 1)
    /// * `notifier` - The notifier for waking workers
    pub fn new(
        queue: Arc<Q>,
        comfyui_client: Arc<C>,
        asset_repository: Arc<dyn AssetRepositoryPort>,
        batch_size: usize,
        notifier: N,
    ) -> Self {
        Self {
            queue,
            comfyui_client,
            asset_repository,
            semaphore: Arc::new(Semaphore::new(batch_size.max(1))),
            notifier,
        }
    }

    /// Enqueue an asset generation request
    pub async fn enqueue(&self, request: AssetGenerationItem) -> Result<QueueItemId, QueueError> {
        self.queue.enqueue(request, PRIORITY_NORMAL).await
    }

    /// Background worker that processes asset generation requests
    ///
    /// This method runs in a loop, processing items from the queue with
    /// concurrency control via semaphore. Each request is processed in
    /// a spawned task to allow parallel processing up to batch_size.
    ///
    /// # Arguments
    /// * `recovery_interval` - Fallback poll interval for crash recovery
    pub async fn run_worker(self: Arc<Self>, recovery_interval: Duration) {
        loop {
            // Try to get next item
            let item = match self.queue.dequeue().await {
                Ok(Some(item)) => item,
                Ok(None) => {
                    // Queue empty - wait for notification or recovery timeout
                    let _ = self.notifier.wait_for_work(recovery_interval).await;
                    continue;
                }
                Err(e) => {
                    tracing::error!("Failed to dequeue asset generation request: {}", e);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };

            // Process in spawned task - acquire permit inside the task for proper lifetime
            // Clone all needed data before spawning to avoid lifetime issues
            let semaphore = self.semaphore.clone();
            let client = self.comfyui_client.clone();
            let _repository = self.asset_repository.clone();
            let queue_clone = self.queue.clone();
            let request = item.payload.clone();
            let item_id = item.id;

            tokio::spawn(async move {
                // Wait for capacity inside the spawned task
                let _permit = match semaphore.acquire().await {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::error!("Semaphore error: {}", e);
                        return;
                    }
                };

                tracing::info!(
                    "Processing asset generation: entity_type={}, entity_id={}, workflow_id={}",
                    request.entity_type,
                    request.entity_id,
                    request.workflow_id
                );

                // Load workflow template (simplified - would load from file system)
                // For now, create a basic workflow JSON
                let workflow = serde_json::json!({
                    "prompt": request.prompt,
                    "workflow_id": request.workflow_id,
                });

                // Submit to ComfyUI
                match client.queue_prompt(workflow).await {
                    Ok(response) => {
                        tracing::info!(
                            "Queued ComfyUI prompt {} for asset generation {}",
                            response.prompt_id,
                            item_id
                        );

                        // Poll for completion (simplified - would poll in a loop)
                        // For now, mark as completed after a delay
                        // In production, this would poll get_history() until complete
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

                        // Create asset records in repository
                        // TODO: Download images and create proper asset records
                        // For now, we'll just mark as completed
                        match queue_clone.complete(item_id).await {
                            Ok(()) => {
                                tracing::info!("Asset generation completed: {}", item_id);
                            }
                            Err(e) => {
                                tracing::error!("Failed to mark asset generation as complete: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to queue ComfyUI prompt: {}", e);
                        let _ = queue_clone.fail(item_id, &format!("ComfyUI error: {}", e)).await;
                    }
                }
            });
        }
    }

    /// Get queue depth (number of pending requests)
    pub async fn depth(&self) -> Result<usize, QueueError> {
        self.queue.depth().await
    }

    /// Get number of items currently processing
    pub async fn processing_count(&self) -> Result<usize, QueueError> {
        self.queue.processing_count().await
    }
}
