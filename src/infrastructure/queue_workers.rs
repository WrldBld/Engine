//! Background workers for processing queue items
//!
//! These workers process items from the queues and handle notifications,
//! approvals, and other async operations.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;

use crate::application::dto::{ChallengeSuggestionInfo, DMAction, DMActionItem, NarrativeEventSuggestionInfo};
use crate::application::ports::outbound::QueueNotificationPort;
use crate::application::services::{
    DMActionQueueService, DMApprovalQueueService, InteractionService, InteractionServiceImpl,
    NarrativeEventService, NarrativeEventServiceImpl, SceneService, SceneServiceImpl,
};
use crate::domain::value_objects::{NarrativeEventId, ProposedToolInfo};
use crate::infrastructure::session::SessionManager;
use crate::infrastructure::websocket::{
    CharacterData, CharacterPosition, InteractionData, SceneData, ServerMessage,
};

/// Worker that processes approval items and sends ApprovalRequired messages to DM
pub async fn approval_notification_worker(
    approval_queue_service: Arc<DMApprovalQueueService<crate::infrastructure::queues::QueueBackendEnum<crate::application::dto::ApprovalItem>>>,
    sessions: Arc<RwLock<SessionManager>>,
    recovery_interval: Duration,
) {
    tracing::info!("Starting approval notification worker");
    let notifier = approval_queue_service.queue.notifier();
    loop {
        // Get all pending approvals from the queue
        // We need to check each active session for pending approvals
        let sessions_read = sessions.read().await;
        let session_ids = sessions_read.get_session_ids();
        drop(sessions_read);

        let mut has_work = false;
        for session_id in session_ids {
            let pending = match approval_queue_service.get_pending(session_id).await {
                Ok(items) => items,
                Err(e) => {
                    tracing::error!("Failed to get pending approvals for session {}: {}", session_id, e);
                    continue;
                }
            };

            if !pending.is_empty() {
                has_work = true;
            }

            // Send ApprovalRequired messages for new approvals
            let mut sessions_write = sessions.write().await;
            for item in pending {
                let approval_id = item.id.to_string();
                
                // Check if we've already notified about this approval
                if let Some(session) = sessions_write.get_session_mut(item.payload.session_id) {
                    if session.get_pending_approval(&approval_id).is_none() {
                    // Convert to PendingApproval and store in session
                    let proposed_tools: Vec<ProposedToolInfo> = item.payload.proposed_tools.clone();

                    let pending_approval = crate::infrastructure::session::PendingApproval::new(
                        approval_id.clone(),
                        item.payload.npc_name.clone(),
                        item.payload.proposed_dialogue.clone(),
                        item.payload.internal_reasoning.clone(),
                        proposed_tools.clone(),
                    );

                    session.add_pending_approval(pending_approval);

                    // Use DTO types directly (they match the WebSocket message types)
                    let challenge_suggestion: Option<ChallengeSuggestionInfo> = item.payload.challenge_suggestion.clone();
                    let narrative_event_suggestion: Option<NarrativeEventSuggestionInfo> = item.payload.narrative_event_suggestion.clone();

                    // Send ApprovalRequired message to DM
                    // Note: ChallengeSuggestionInfo and NarrativeEventSuggestionInfo are imported from application::dto
                    // and match the types expected by ServerMessage::ApprovalRequired
                    let approval_msg = ServerMessage::ApprovalRequired {
                        request_id: approval_id.clone(),
                        npc_name: item.payload.npc_name.clone(),
                        proposed_dialogue: item.payload.proposed_dialogue.clone(),
                        internal_reasoning: item.payload.internal_reasoning.clone(),
                        proposed_tools,
                        challenge_suggestion,
                        narrative_event_suggestion,
                    };
                    session.send_to_dm(&approval_msg);

                        tracing::info!(
                            "Sent ApprovalRequired for approval {} to DM",
                            approval_id
                        );
                    }
                }
            }
            drop(sessions_write);
        }

        // Wait for notification if no work, otherwise check again immediately
        if !has_work {
            let _ = notifier.wait_for_work(recovery_interval).await;
        }
    }
}

/// Worker that processes DM action queue items
pub async fn dm_action_worker(
    dm_action_queue_service: Arc<DMActionQueueService<crate::infrastructure::queues::QueueBackendEnum<DMActionItem>>>,
    approval_queue_service: Arc<DMApprovalQueueService<crate::infrastructure::queues::QueueBackendEnum<crate::application::dto::ApprovalItem>>>,
    narrative_event_service: Arc<NarrativeEventServiceImpl>,
    scene_service: Arc<SceneServiceImpl>,
    interaction_service: Arc<InteractionServiceImpl>,
    sessions: Arc<RwLock<SessionManager>>,
    recovery_interval: Duration,
) {
    tracing::info!("Starting DM action queue worker");
    let notifier = dm_action_queue_service.queue.notifier();
    loop {
        let sessions_clone = sessions.clone();
        let approval_queue_service_clone = approval_queue_service.clone();
        let narrative_event_service_clone = narrative_event_service.clone();
        let scene_service_clone = scene_service.clone();
        let interaction_service_clone = interaction_service.clone();
        match dm_action_queue_service
            .process_next(|action| {
                let sessions = sessions_clone.clone();
                let approval_queue_service = approval_queue_service_clone.clone();
                let narrative_event_service = narrative_event_service_clone.clone();
                let scene_service = scene_service_clone.clone();
                let interaction_service = interaction_service_clone.clone();
                async move {
                process_dm_action(
                    &sessions,
                    &approval_queue_service,
                        &narrative_event_service,
                        &scene_service,
                        &interaction_service,
                        &action,
                )
                .await
                }
            })
            .await
        {
            Ok(Some(_)) => {
                // Action processed successfully
            }
            Ok(None) => {
                // Queue empty - wait for notification or recovery timeout
                let _ = notifier.wait_for_work(recovery_interval).await;
            }
            Err(e) => {
                tracing::error!("Error processing DM action: {}", e);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

async fn process_dm_action(
    sessions: &Arc<RwLock<SessionManager>>,
    approval_queue_service: &Arc<DMApprovalQueueService<crate::infrastructure::queues::QueueBackendEnum<crate::application::dto::ApprovalItem>>>,
    narrative_event_service: &NarrativeEventServiceImpl,
    scene_service: &SceneServiceImpl,
    interaction_service: &InteractionServiceImpl,
    action: &DMActionItem,
) -> Result<(), crate::application::ports::outbound::QueueError> {
    match &action.action {
        DMAction::ApprovalDecision {
            request_id,
            decision,
        } => {
            // Parse request_id as QueueItemId (UUID string)
            let approval_item_id = match uuid::Uuid::parse_str(&request_id) {
                Ok(uuid) => crate::domain::value_objects::QueueItemId::from_uuid(uuid),
                Err(_) => {
                    tracing::error!("Invalid approval item ID: {}", request_id);
                    return Err(crate::application::ports::outbound::QueueError::NotFound(request_id.clone()));
                }
            };

            // The approval service's process_decision expects domain ApprovalDecision
            // which matches what we have from the DMAction

            // Process the decision using the approval queue service
            // The service now only needs SessionManagementPort (session manager) and session_id
            let mut sessions_write = sessions.write().await;
            // Verify session exists
            if sessions_write.get_session_mut(action.session_id).is_some() {
                // Use the approval service's process_decision method
                // The service expects domain ApprovalDecision which matches what we have
                match approval_queue_service
                    .process_decision(&mut *sessions_write, action.session_id, approval_item_id, decision.clone())
                    .await
                {
                    Ok(outcome) => {
                        tracing::info!("Processed approval decision: {:?}", outcome);
                    }
                    Err(e) => {
                        tracing::error!("Failed to process approval decision: {}", e);
                        drop(sessions_write);
                        return Err(e);
                    }
                }
            } else {
                tracing::warn!("Session {} not found for approval processing", action.session_id);
                drop(sessions_write);
                return Err(crate::application::ports::outbound::QueueError::Backend(
                    format!("Session {} not found", action.session_id)
                ));
            }
            drop(sessions_write);
        }
        DMAction::DirectNPCControl { npc_id: _, dialogue } => {
            // Broadcast direct NPC control
            let mut sessions_write = sessions.write().await;
            if let Some(session) = sessions_write.get_session_mut(action.session_id) {
                let response = ServerMessage::DialogueResponse {
                    speaker_id: "NPC".to_string(),
                    speaker_name: "NPC".to_string(),
                    text: dialogue.clone(),
                    choices: vec![],
                };
                session.broadcast_to_players(&response);
            }
        }
        DMAction::TriggerEvent { event_id } => {
            // Parse event ID
            let event_uuid = match uuid::Uuid::parse_str(&event_id) {
                Ok(uuid) => NarrativeEventId::from_uuid(uuid),
                Err(_) => {
                    tracing::error!("Invalid event ID: {}", event_id);
                    return Err(crate::application::ports::outbound::QueueError::Backend(
                        format!("Invalid event ID: {}", event_id)
                    ));
                }
            };

            // Load the narrative event
            let narrative_event = match narrative_event_service.get(event_uuid).await {
                Ok(Some(event)) => event,
                Ok(None) => {
                    tracing::error!("Narrative event not found: {}", event_id);
                    return Err(crate::application::ports::outbound::QueueError::NotFound(
                        event_id.clone()
                    ));
                }
                Err(e) => {
                    tracing::error!("Failed to load narrative event: {}", e);
                    return Err(crate::application::ports::outbound::QueueError::Backend(
                        format!("Failed to load narrative event: {}", e)
                    ));
                }
            };

            // Mark event as triggered
            if let Err(e) = narrative_event_service
                .mark_triggered(event_uuid, None)
                .await
            {
                tracing::error!("Failed to mark narrative event as triggered: {}", e);
                return Err(crate::application::ports::outbound::QueueError::Backend(
                    format!("Failed to mark event as triggered: {}", e)
                ));
            }

            // Broadcast notification to session
            let mut sessions_write = sessions.write().await;
            if let Some(session) = sessions_write.get_session_mut(action.session_id) {
                // Create a simple notification message
                // Note: A proper NarrativeEventTriggered message type could be added to ServerMessage
                let notification = ServerMessage::Error {
                    code: "NARRATIVE_EVENT_TRIGGERED".to_string(),
                    message: format!("Narrative event '{}' has been triggered", narrative_event.name),
                };
                session.broadcast_to_players(&notification);

                tracing::info!(
                    "Triggered narrative event {} ({}) in session {}",
                    event_id,
                    narrative_event.name,
                    action.session_id
                );
            }
            drop(sessions_write);
        }
        DMAction::TransitionScene { scene_id } => {
            // Load scene with relations
            let scene_with_relations = match scene_service.get_scene_with_relations(*scene_id).await {
                Ok(Some(scene_data)) => scene_data,
                Ok(None) => {
                    tracing::error!("Scene not found: {}", scene_id);
                    return Err(crate::application::ports::outbound::QueueError::NotFound(
                        scene_id.to_string()
                    ));
                }
                Err(e) => {
                    tracing::error!("Failed to load scene: {}", e);
                    return Err(crate::application::ports::outbound::QueueError::Backend(
                        format!("Failed to load scene: {}", e)
                    ));
                }
            };

            // Load interactions for the scene
            let interactions = match interaction_service
                .list_interactions(*scene_id)
                .await
            {
                Ok(interactions) => interactions
                    .into_iter()
                    .map(|i| {
                        let target_name = match &i.target {
                            crate::domain::entities::InteractionTarget::Character(_) => {
                                Some("Character".to_string())
                            }
                            crate::domain::entities::InteractionTarget::Item(_) => {
                                Some("Item".to_string())
                            }
                            crate::domain::entities::InteractionTarget::Environment(name) => {
                                Some(name.clone())
                            }
                            crate::domain::entities::InteractionTarget::None => None,
                        };
                        InteractionData {
                            id: i.id.to_string(),
                            name: i.name.clone(),
                            interaction_type: format!("{:?}", i.interaction_type),
                            target_name,
                            is_available: i.is_available,
                        }
                    })
                    .collect(),
                Err(e) => {
                    tracing::warn!("Failed to load interactions for scene: {}", e);
                    vec![]
                }
            };

            // Build character data
            let characters: Vec<CharacterData> = scene_with_relations
                .featured_characters
                .iter()
                .map(|c| CharacterData {
                    id: c.id.to_string(),
                    name: c.name.clone(),
                    sprite_asset: c.sprite_asset.clone(),
                    portrait_asset: c.portrait_asset.clone(),
                    position: CharacterPosition::Center,
                    is_speaking: false,
                })
                .collect();

            // Build SceneUpdate message
            let scene_update = ServerMessage::SceneUpdate {
                scene: SceneData {
                    id: scene_with_relations.scene.id.to_string(),
                    name: scene_with_relations.scene.name.clone(),
                    location_id: scene_with_relations.scene.location_id.to_string(),
                    location_name: scene_with_relations.location.name.clone(),
                    backdrop_asset: scene_with_relations
                        .scene
                        .backdrop_override
                        .or(scene_with_relations.location.backdrop_asset.clone()),
                    time_context: match &scene_with_relations.scene.time_context {
                        crate::domain::entities::TimeContext::Unspecified => "Unspecified".to_string(),
                        crate::domain::entities::TimeContext::TimeOfDay(tod) => format!("{:?}", tod),
                        crate::domain::entities::TimeContext::During(s) => s.clone(),
                        crate::domain::entities::TimeContext::Custom(s) => s.clone(),
                    },
                    directorial_notes: scene_with_relations.scene.directorial_notes.clone(),
                },
                characters,
                interactions,
            };

            // Update session's current scene and broadcast
            let mut sessions_write = sessions.write().await;
            if let Some(session) = sessions_write.get_session_mut(action.session_id) {
                // Update current scene in session
                session.current_scene_id = Some(scene_id.to_string());
                
                // Broadcast SceneUpdate to all participants
                sessions_write.broadcast_to_session(action.session_id, &scene_update);

                tracing::info!(
                    "DM transitioned scene to {} in session {}",
                    scene_id,
                    action.session_id
                );
            }
            drop(sessions_write);
        }
    }

    Ok(())
}
