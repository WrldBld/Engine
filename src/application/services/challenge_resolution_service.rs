//! Challenge resolution service - encapsulates challenge roll handling, DM-triggered
//! challenges, and challenge suggestion approvals.
//!
//! This moves challenge-related business logic out of the websocket handler into a
//! dedicated application service, keeping the transport layer thin.

use std::sync::Arc;

use tokio::sync::RwLock;

use crate::application::dto::AppEvent;
use crate::application::services::{ChallengeService, SkillService};
use crate::domain::entities::OutcomeType;
use crate::domain::value_objects::ChallengeId;
use crate::infrastructure::session::{ClientId, SessionManager};
use crate::infrastructure::websocket::messages::ServerMessage;
use crate::application::ports::outbound::EventBusPort;
use crate::application::services::dm_approval_queue_service::DMApprovalQueueService;
use crate::application::ports::outbound::ApprovalQueuePort;

/// Service responsible for challenge-related flows.
pub struct ChallengeResolutionService<S: ChallengeService, K: SkillService, Q: ApprovalQueuePort<crate::application::dto::ApprovalItem>> {
    sessions: Arc<RwLock<SessionManager>>,
    challenge_service: Arc<S>,
    skill_service: Arc<K>,
    event_bus: Arc<dyn EventBusPort<AppEvent>>,
    dm_approval_queue_service: Arc<DMApprovalQueueService<Q>>,
}

impl<S, K, Q> ChallengeResolutionService<S, K, Q>
where
    S: ChallengeService,
    K: SkillService,
    Q: ApprovalQueuePort<crate::application::dto::ApprovalItem>,
{
    pub fn new(
        sessions: Arc<RwLock<SessionManager>>,
        challenge_service: Arc<S>,
        skill_service: Arc<K>,
        event_bus: Arc<dyn EventBusPort<AppEvent>>,
        dm_approval_queue_service: Arc<DMApprovalQueueService<Q>>,
    ) -> Self {
        Self {
            sessions,
            challenge_service,
            skill_service,
            event_bus,
            dm_approval_queue_service,
        }
    }

    /// Handle a player submitting a challenge roll.
    pub async fn handle_roll(
        &self,
        client_id: ClientId,
        challenge_id_str: String,
        roll: i32,
    ) -> Option<ServerMessage> {
        // Parse challenge_id
        let challenge_uuid = match uuid::Uuid::parse_str(&challenge_id_str) {
            Ok(uuid) => ChallengeId::from_uuid(uuid),
            Err(_) => {
                return Some(ServerMessage::Error {
                    code: "INVALID_CHALLENGE_ID".to_string(),
                    message: "Invalid challenge ID format".to_string(),
                });
            }
        };

        // Load challenge from service
        let challenge = match self.challenge_service.get_challenge(challenge_uuid).await {
            Ok(Some(challenge)) => challenge,
            Ok(None) => {
                return Some(ServerMessage::Error {
                    code: "CHALLENGE_NOT_FOUND".to_string(),
                    message: format!("Challenge {} not found", challenge_id_str),
                });
            }
            Err(e) => {
                tracing::error!("Failed to load challenge: {}", e);
                return Some(ServerMessage::Error {
                    code: "CHALLENGE_LOAD_ERROR".to_string(),
                    message: "Failed to load challenge".to_string(),
                });
            }
        };

        // Get session and player info
        let sessions_read = self.sessions.read().await;
        let (session_id, player_name) = match sessions_read.get_client_session(client_id) {
            Some(sid) => {
                let session = sessions_read.get_session(sid);
                let player_name = session
                    .and_then(|s| s.participants.get(&client_id))
                    .map(|p| p.user_id.clone())
                    .unwrap_or_else(|| "Unknown Player".to_string());
                (Some(sid), player_name)
            }
            None => {
                return Some(ServerMessage::Error {
                    code: "NOT_IN_SESSION".to_string(),
                    message:
                        "You must join a session before submitting challenge rolls".to_string(),
                });
            }
        };

        // TODO: integrate real character modifiers
        let character_modifier = 0;

        // Evaluate challenge result
        let (outcome_type, outcome) =
            evaluate_challenge_result(&challenge, roll, character_modifier);
        let success =
            outcome_type == OutcomeType::Success || outcome_type == OutcomeType::CriticalSuccess;

        // Publish AppEvent for challenge resolution
        let world_id = challenge.world_id;

        // TODO: derive real character_id from session participant mapping
        let character_id = "unknown".to_string();

        let app_event = AppEvent::ChallengeResolved {
            challenge_id: Some(challenge_id_str.clone()),
            challenge_name: challenge.name.clone(),
            world_id: world_id.to_string(),
            character_id,
            success,
            roll: Some(roll),
            total: Some(roll + character_modifier),
        };
        if let Err(e) = self.event_bus.publish(app_event).await {
            tracing::error!("Failed to publish ChallengeResolved event: {}", e);
        }

        // Broadcast ChallengeResolved to all participants
        if let Some(session_id) = session_id {
            let result_msg = ServerMessage::ChallengeResolved {
                challenge_id: challenge_id_str.clone(),
                challenge_name: challenge.name.clone(),
                character_name: player_name,
                roll,
                modifier: character_modifier,
                total: roll + character_modifier,
                outcome: outcome_type.display_name().to_string(),
                outcome_description: outcome.description.clone(),
            };
            drop(sessions_read);
            let sessions_write = self.sessions.write().await;
            sessions_write.broadcast_to_session(session_id, &result_msg);
        }

        None
    }

    /// Handle DM-triggered challenges.
    pub async fn handle_trigger(
        &self,
        client_id: ClientId,
        challenge_id_str: String,
        target_character_id: String,
    ) -> Option<ServerMessage> {
        let sessions_read = self.sessions.read().await;
        let session_id = sessions_read.get_client_session(client_id);
        let is_dm = session_id
            .and_then(|sid| sessions_read.get_session(sid))
            .and_then(|s| s.get_dm())
            .filter(|dm| dm.client_id == client_id)
            .is_some();

        if !is_dm {
            return Some(ServerMessage::Error {
                code: "NOT_AUTHORIZED".to_string(),
                message: "Only the DM can trigger challenges".to_string(),
            });
        }

        // Parse challenge_id
        let challenge_uuid = match uuid::Uuid::parse_str(&challenge_id_str) {
            Ok(uuid) => ChallengeId::from_uuid(uuid),
            Err(_) => {
                return Some(ServerMessage::Error {
                    code: "INVALID_CHALLENGE_ID".to_string(),
                    message: "Invalid challenge ID format".to_string(),
                });
            }
        };

        // Load challenge from service
        let challenge = match self.challenge_service.get_challenge(challenge_uuid).await {
            Ok(Some(challenge)) => challenge,
            Ok(None) => {
                return Some(ServerMessage::Error {
                    code: "CHALLENGE_NOT_FOUND".to_string(),
                    message: format!("Challenge {} not found", challenge_id_str),
                });
            }
            Err(e) => {
                tracing::error!("Failed to load challenge: {}", e);
                return Some(ServerMessage::Error {
                    code: "CHALLENGE_LOAD_ERROR".to_string(),
                    message: "Failed to load challenge".to_string(),
                });
            }
        };

        // Look up skill name from skill service
        let skill_name = match self.skill_service.get_skill(challenge.skill_id).await {
            Ok(Some(skill)) => skill.name,
            Ok(None) => {
                tracing::warn!("Skill {} not found for challenge", challenge.skill_id);
                challenge.skill_id.to_string()
            }
            Err(e) => {
                tracing::error!("Failed to look up skill {}: {}", challenge.skill_id, e);
                challenge.skill_id.to_string()
            }
        };

        let character_modifier = 0;

        let prompt = ServerMessage::ChallengePrompt {
            challenge_id: challenge_id_str.clone(),
            challenge_name: challenge.name.clone(),
            skill_name,
            difficulty_display: challenge.difficulty.display(),
            description: challenge.description.clone(),
            character_modifier,
        };

        if let Some(session_id) = session_id {
            drop(sessions_read);
            let mut sessions_write = self.sessions.write().await;
            if let Some(session) = sessions_write.get_session_mut(session_id) {
                session.broadcast_to_players(&prompt);
            }
        }

        tracing::info!(
            "DM triggered challenge {} for character {} in session {:?}",
            challenge_id_str,
            target_character_id,
            session_id
        );

        None
    }

    /// Handle DM approval/rejection of a challenge suggestion.
    pub async fn handle_suggestion_decision(
        &self,
        client_id: ClientId,
        request_id: String,
        approved: bool,
        modified_difficulty: Option<String>,
    ) -> Option<ServerMessage> {
        let sessions_read = self.sessions.read().await;
        let session_id = sessions_read.get_client_session(client_id);
        let is_dm = session_id
            .and_then(|sid| sessions_read.get_session(sid))
            .and_then(|s| s.get_dm())
            .filter(|dm| dm.client_id == client_id)
            .is_some();

        if !is_dm {
            return Some(ServerMessage::Error {
                code: "NOT_AUTHORIZED".to_string(),
                message: "Only the DM can approve challenge suggestions".to_string(),
            });
        }

        if approved {
            let approval_item = self.dm_approval_queue_service.get_by_id(&request_id).await;

            match approval_item {
                Ok(Some(item)) => {
                    if let Some(challenge_suggestion) = &item.payload.challenge_suggestion {
                        let challenge_uuid =
                            match uuid::Uuid::parse_str(&challenge_suggestion.challenge_id) {
                                Ok(uuid) => ChallengeId::from_uuid(uuid),
                                Err(_) => {
                                    tracing::error!(
                                        "Invalid challenge_id in suggestion: {}",
                                        challenge_suggestion.challenge_id
                                    );
                                    return Some(ServerMessage::Error {
                                        code: "INVALID_CHALLENGE_ID".to_string(),
                                        message: "Invalid challenge ID format".to_string(),
                                    });
                                }
                            };

                        let challenge =
                            match self.challenge_service.get_challenge(challenge_uuid).await {
                                Ok(Some(c)) => c,
                                Ok(None) => {
                                    tracing::error!(
                                        "Challenge {} not found",
                                        challenge_suggestion.challenge_id
                                    );
                                    return Some(ServerMessage::Error {
                                        code: "CHALLENGE_NOT_FOUND".to_string(),
                                        message: format!(
                                            "Challenge {} not found",
                                            challenge_suggestion.challenge_id
                                        ),
                                    });
                                }
                                Err(e) => {
                                    tracing::error!("Failed to load challenge: {}", e);
                                    return Some(ServerMessage::Error {
                                        code: "CHALLENGE_LOAD_ERROR".to_string(),
                                        message: format!("Failed to load challenge: {}", e),
                                    });
                                }
                            };

                        let difficulty_display = modified_difficulty
                            .unwrap_or_else(|| challenge.difficulty.display());

                        let character_modifier = 0;

                        let prompt = ServerMessage::ChallengePrompt {
                            challenge_id: challenge_suggestion.challenge_id.clone(),
                            challenge_name: challenge.name.clone(),
                            skill_name: challenge_suggestion.skill_name.clone(),
                            difficulty_display,
                            description: challenge.description.clone(),
                            character_modifier,
                        };

                        let sessions_read_inner = self.sessions.read().await;
                        if let Some(session_id) = sessions_read_inner.get_client_session(client_id)
                        {
                            sessions_read_inner.broadcast_to_session(session_id, &prompt);
                        }

                        tracing::info!(
                            "Triggered challenge '{}' for session via suggestion approval",
                            challenge.name
                        );
                    } else {
                        tracing::warn!(
                            "No challenge suggestion found in approval item {}",
                            request_id
                        );
                        return Some(ServerMessage::Error {
                            code: "NO_CHALLENGE_SUGGESTION".to_string(),
                            message: "No challenge suggestion found in approval request".to_string(),
                        });
                    }
                }
                Ok(None) => {
                    tracing::error!("Approval item {} not found", request_id);
                    return Some(ServerMessage::Error {
                        code: "APPROVAL_NOT_FOUND".to_string(),
                        message: format!("Approval request {} not found", request_id),
                    });
                }
                Err(e) => {
                    tracing::error!("Failed to get approval item: {}", e);
                    return Some(ServerMessage::Error {
                        code: "APPROVAL_LOOKUP_ERROR".to_string(),
                        message: format!("Failed to look up approval: {}", e),
                    });
                }
            }
        } else {
            tracing::info!("DM rejected challenge suggestion for request {}", request_id);
        }

        None
    }
}

/// Evaluate a challenge roll result (moved from websocket.rs)
fn evaluate_challenge_result(
    challenge: &crate::domain::entities::Challenge,
    roll: i32,
    modifier: i32,
) -> (OutcomeType, &crate::domain::entities::Outcome) {
    let total = roll + modifier;

    match &challenge.difficulty {
        crate::domain::entities::Difficulty::DC(dc) => {
            if roll == 20 {
                if let Some(ref critical_success) = challenge.outcomes.critical_success {
                    return (OutcomeType::CriticalSuccess, critical_success);
                }
            }
            if roll == 1 {
                if let Some(ref critical_failure) = challenge.outcomes.critical_failure {
                    return (OutcomeType::CriticalFailure, critical_failure);
                }
            }

            if total >= *dc as i32 {
                (OutcomeType::Success, &challenge.outcomes.success)
            } else {
                (OutcomeType::Failure, &challenge.outcomes.failure)
            }
        }
        crate::domain::entities::Difficulty::Percentage(target) => {
            if roll == 1 {
                if let Some(ref critical_success) = challenge.outcomes.critical_success {
                    return (OutcomeType::CriticalSuccess, critical_success);
                }
            }
            if roll == 100 {
                if let Some(ref critical_failure) = challenge.outcomes.critical_failure {
                    return (OutcomeType::CriticalFailure, critical_failure);
                }
            }

            if roll <= *target as i32 {
                (OutcomeType::Success, &challenge.outcomes.success)
            } else {
                (OutcomeType::Failure, &challenge.outcomes.failure)
            }
        }
        crate::domain::entities::Difficulty::Descriptor(_) => {
            if roll >= 11 {
                (OutcomeType::Success, &challenge.outcomes.success)
            } else {
                (OutcomeType::Failure, &challenge.outcomes.failure)
            }
        }
        crate::domain::entities::Difficulty::Opposed => {
            (OutcomeType::Success, &challenge.outcomes.success)
        }
        crate::domain::entities::Difficulty::Custom(_) => {
            (OutcomeType::Success, &challenge.outcomes.success)
        }
    }
}


