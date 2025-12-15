//! Background workers for processing queue items
//!
//! These workers process items from the queues and handle notifications,
//! approvals, and other async operations.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;

use crate::application::services::{DMActionQueueService, DMApprovalQueueService};
use crate::domain::value_objects::{ApprovalDecision, DMAction};
use crate::infrastructure::session::SessionManager;
use crate::infrastructure::websocket::ServerMessage;

/// Worker that processes approval items and sends ApprovalRequired messages to DM
pub async fn approval_notification_worker<Q: crate::application::ports::outbound::ApprovalQueuePort<crate::domain::value_objects::ApprovalItem>>(
    approval_queue_service: Arc<DMApprovalQueueService<Arc<Q>>>,
    sessions: Arc<RwLock<SessionManager>>,
) {
    tracing::info!("Starting approval notification worker");
    loop {
        // Get all pending approvals from the queue
        // We need to check each active session for pending approvals
        let sessions_read = sessions.read().await;
        let session_ids: Vec<_> = sessions_read.sessions.keys().copied().collect();
        drop(sessions_read);

        for session_id in session_ids {
            let pending = match approval_queue_service.get_pending(session_id).await {
                Ok(items) => items,
                Err(e) => {
                    tracing::error!("Failed to get pending approvals for session {}: {}", session_id, e);
                    continue;
                }
            };

            // Send ApprovalRequired messages for new approvals
            let mut sessions_write = sessions.write().await;
            for item in pending {
                let approval_id = item.id.to_string();
                
                // Check if we've already notified about this approval
                if let Some(session) = sessions_write.get_session_mut(item.payload.session_id) {
                    if session.get_pending_approval(&approval_id).is_none() {
                    // Convert to PendingApproval and store in session
                    let proposed_tools: Vec<crate::infrastructure::websocket::ProposedTool> = item
                        .payload
                        .proposed_tools
                        .iter()
                        .map(|t| crate::infrastructure::websocket::ProposedTool {
                            id: t.id.clone(),
                            name: t.name.clone(),
                            description: t.description.clone(),
                            arguments: t.arguments.clone(),
                        })
                        .collect();

                    let pending_approval = crate::infrastructure::session::PendingApproval::new(
                        approval_id.clone(),
                        item.payload.npc_name.clone(),
                        item.payload.proposed_dialogue.clone(),
                        item.payload.internal_reasoning.clone(),
                        proposed_tools.clone(),
                    );

                    session.add_pending_approval(pending_approval);

                    // Send ApprovalRequired message to DM
                    let approval_msg = ServerMessage::ApprovalRequired {
                        request_id: approval_id.clone(),
                        npc_name: item.payload.npc_name.clone(),
                        proposed_dialogue: item.payload.proposed_dialogue.clone(),
                        internal_reasoning: item.payload.internal_reasoning.clone(),
                        proposed_tools,
                        challenge_suggestion: None,
                        narrative_event_suggestion: None,
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

        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

/// Worker that processes DM action queue items
pub async fn dm_action_worker<Q: crate::application::ports::outbound::QueuePort<crate::domain::value_objects::DMActionItem>>(
    dm_action_queue_service: Arc<DMActionQueueService<Arc<Q>>>,
    approval_queue_service: Arc<DMApprovalQueueService<Arc<crate::infrastructure::queues::QueueBackendEnum<crate::domain::value_objects::ApprovalItem>>>>,
    sessions: Arc<RwLock<SessionManager>>,
) {
    tracing::info!("Starting DM action queue worker");
    loop {
        match dm_action_queue_service
            .process_next(|action| async move {
                process_dm_action(
                    &sessions,
                    &approval_queue_service,
                    action,
                )
                .await
            })
            .await
        {
            Ok(Some(_)) => {
                // Action processed successfully
            }
            Ok(None) => {
                // Queue empty, wait a bit
                tokio::time::sleep(Duration::from_millis(100)).await;
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
    approval_queue_service: &Arc<DMApprovalQueueService<Arc<crate::infrastructure::queues::QueueBackendEnum<crate::domain::value_objects::ApprovalItem>>>>,
    action: &crate::domain::value_objects::DMActionItem,
) -> Result<(), crate::application::ports::outbound::QueueError> {
    match &action.action {
        DMAction::ApprovalDecision {
            request_id,
            decision,
        } => {
            // Parse request_id as QueueItemId (UUID string)
            let approval_item_id = match uuid::Uuid::parse_str(request_id) {
                Ok(uuid) => crate::domain::value_objects::QueueItemId::from_uuid(uuid),
                Err(_) => {
                    tracing::error!("Invalid approval item ID: {}", request_id);
                    return Err(crate::application::ports::outbound::QueueError::NotFound(request_id.clone()));
                }
            };

            // The approval service's process_decision expects domain ApprovalDecision
            // which matches what we have from the DMAction

            // Process the decision using the approval queue service
            // This requires access to session manager and game session
            let mut sessions_write = sessions.write().await;
            if let Some(session) = sessions_write.get_session_mut(action.session_id) {
                // Use the approval service's process_decision method
                // The service expects domain ApprovalDecision which matches what we have
                match approval_queue_service
                    .process_decision(&mut *sessions_write, session, approval_item_id, decision.clone())
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
        DMAction::TriggerEvent { event_id: _ } => {
            // TODO: Implement event triggering
            tracing::info!("Event triggering not yet implemented");
        }
        DMAction::TransitionScene { scene_id: _ } => {
            // TODO: Implement scene transition
            tracing::info!("Scene transition not yet implemented");
        }
    }

    Ok(())
}
