//! Queue processing services

use std::sync::Arc;

use crate::application::dto::{ApprovalItem, AssetGenerationItem, DMActionItem, LLMRequestItem, PlayerActionItem};
use crate::application::services::{
    AssetGenerationQueueService, DMActionQueueService, DMApprovalQueueService, LLMQueueService,
    PlayerActionQueueService,
};
use crate::infrastructure::comfyui::ComfyUIClient;
use crate::infrastructure::ollama::OllamaClient;

/// Queue processing services for asynchronous operations
///
/// This struct groups all queue-related services that handle background
/// processing of player actions, DM actions, LLM requests, asset generation,
/// and approval workflows.
pub struct QueueServices {
    pub player_action_queue_service: Arc<
        PlayerActionQueueService<
            crate::infrastructure::queues::QueueBackendEnum<PlayerActionItem>,
            crate::infrastructure::queues::QueueBackendEnum<LLMRequestItem>,
        >,
    >,
    pub dm_action_queue_service: Arc<DMActionQueueService<crate::infrastructure::queues::QueueBackendEnum<DMActionItem>>>,
    pub llm_queue_service: Arc<
        LLMQueueService<
            crate::infrastructure::queues::QueueBackendEnum<LLMRequestItem>,
            OllamaClient,
            crate::infrastructure::queues::InProcessNotifier,
        >,
    >,
    pub asset_generation_queue_service: Arc<
        AssetGenerationQueueService<
            crate::infrastructure::queues::QueueBackendEnum<AssetGenerationItem>,
            ComfyUIClient,
            crate::infrastructure::queues::InProcessNotifier,
        >,
    >,
    pub dm_approval_queue_service: Arc<DMApprovalQueueService<crate::infrastructure::queues::QueueBackendEnum<ApprovalItem>>>,
}

impl QueueServices {
    /// Creates a new QueueServices instance with all queue processing services
    pub fn new(
        player_action_queue_service: Arc<
            PlayerActionQueueService<
                crate::infrastructure::queues::QueueBackendEnum<PlayerActionItem>,
                crate::infrastructure::queues::QueueBackendEnum<LLMRequestItem>,
            >,
        >,
        dm_action_queue_service: Arc<DMActionQueueService<crate::infrastructure::queues::QueueBackendEnum<DMActionItem>>>,
        llm_queue_service: Arc<
            LLMQueueService<
                crate::infrastructure::queues::QueueBackendEnum<LLMRequestItem>,
                OllamaClient,
                crate::infrastructure::queues::InProcessNotifier,
            >,
        >,
        asset_generation_queue_service: Arc<
            AssetGenerationQueueService<
                crate::infrastructure::queues::QueueBackendEnum<AssetGenerationItem>,
                ComfyUIClient,
                crate::infrastructure::queues::InProcessNotifier,
            >,
        >,
        dm_approval_queue_service: Arc<DMApprovalQueueService<crate::infrastructure::queues::QueueBackendEnum<ApprovalItem>>>,
    ) -> Self {
        Self {
            player_action_queue_service,
            dm_action_queue_service,
            llm_queue_service,
            asset_generation_queue_service,
            dm_approval_queue_service,
        }
    }
}
