//! Challenge resolution service - encapsulates challenge roll handling, DM-triggered
//! challenges, and challenge suggestion approvals.
//!
//! This moves challenge-related business logic out of the websocket handler into a
//! dedicated application service, keeping the transport layer thin.
//!
//! # Architecture Note: Hexagonal Violation
//!
//! This service currently imports `SessionManager` directly from the infrastructure layer:
//! ```ignore
//! use crate::infrastructure::session::{ClientId, SessionManager};
//! ```
//!
//! This violates hexagonal architecture rules where application services should depend only
//! on domain types and port traits, not infrastructure implementations.
//!
//! ## Planned Refactoring
//!
//! To fix this violation, the service should be refactored to:
//! 1. Accept `AsyncSessionPort` trait bound instead of concrete `SessionManager`
//! 2. Replace all `SessionManager` method calls with equivalent `AsyncSessionPort` methods
//! 3. Move `ClientId` to domain or port definitions if it's not already there
//!
//! The port trait already exists at: `application/ports/outbound/async_session_port.rs`
//! The adapter already exists at: `infrastructure/session_adapter.rs`
//!
//! This service is tightly coupled to SessionManager's session and participant management API
//! and broadcasts directly to sessions. A full refactoring should carefully preserve this
//! functionality while routing through the port trait.

use std::sync::Arc;

use crate::application::dto::AppEvent;
use crate::application::ports::outbound::AsyncSessionPort;
use crate::application::ports::outbound::EventBusPort;
use crate::application::ports::outbound::{ApprovalQueuePort, ChallengeRepositoryPort};
use crate::application::services::{
    ChallengeService, OutcomeTriggerService, PlayerCharacterService, SkillService,
};
use crate::domain::entities::OutcomeType;
use crate::domain::value_objects::{ChallengeId, DiceRollInput, SessionId, PlayerCharacterId};
use crate::infrastructure::session::ClientId;
use crate::infrastructure::websocket::messages::{DiceInputType, ServerMessage};
use tracing::{debug, info};

/// Service responsible for challenge-related flows.
///
/// # TODO: Architecture Violation
///
/// This service previously depended on `SessionManager` (a concrete infrastructure type)
/// rather than the async session port trait, which violated hexagonal architecture rules.
/// It now uses `AsyncSessionPort`, so all session lookups and broadcasts go through a port.
pub struct ChallengeResolutionService<S: ChallengeService, K: SkillService, Q: ApprovalQueuePort<crate::application::dto::ApprovalItem>, P: PlayerCharacterService> {
    sessions: Arc<dyn AsyncSessionPort>,
    challenge_service: Arc<S>,
    skill_service: Arc<K>,
    player_character_service: Arc<P>,
    event_bus: Arc<dyn EventBusPort<AppEvent>>,
    dm_approval_queue_service: Arc<DMApprovalQueueService<Q>>,
    outcome_trigger_service: Arc<OutcomeTriggerService>,
}

impl<S, K, Q, P> ChallengeResolutionService<S, K, Q, P>
where
    S: ChallengeService,
    K: SkillService,
    Q: ApprovalQueuePort<crate::application::dto::ApprovalItem>,
    P: PlayerCharacterService,
{
    pub fn new(
        sessions: Arc<dyn AsyncSessionPort>,
        challenge_service: Arc<S>,
        skill_service: Arc<K>,
        player_character_service: Arc<P>,
        event_bus: Arc<dyn EventBusPort<AppEvent>>,
        dm_approval_queue_service: Arc<DMApprovalQueueService<Q>>,
        outcome_trigger_service: Arc<OutcomeTriggerService>,
    ) -> Self {
        Self {
            sessions,
            challenge_service,
            skill_service,
            player_character_service,
            event_bus,
            dm_approval_queue_service,
            outcome_trigger_service,
        }
    }

    /// Get a player character ID for a client in a session.
    ///
    /// This looks up the client's PC by matching their user_id in the session
    /// participants with a PlayerCharacter in the session.
    async fn get_client_player_character(
        &self,
        client_id: &ClientId,
        session_id: SessionId,
    ) -> Option<PlayerCharacterId> {
        // Resolve the client's user_id via the async session port, then map to a player character.
        let client_id_str = client_id.to_string();
        let Some(participant) = self.sessions.get_participant_info(&client_id_str).await else {
            debug!(
                client_id = %client_id,
                session_id = %session_id,
                "No participant info found for client when resolving player character"
            );
            return None;
        };

        let user_id = participant.user_id;

        // We rely on PlayerCharacterService to perform the actual lookup.
        match self
            .player_character_service
            .get_pc_for_user_in_session(&user_id, session_id)
            .await
        {
            Ok(Some(pc)) => Some(pc.id),
            Ok(None) => {
                debug!(
                    client_id = %client_id,
                    session_id = %session_id,
                    user_id = %user_id,
                    "No player character found for user in session"
                );
                None
            }
            Err(e) => {
                debug!(
                    client_id = %client_id,
                    session_id = %session_id,
                    user_id = %user_id,
                    error = %e,
                    "Error while looking up player character for user in session"
                );
                None
            }
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

        // Get session and player info via async session port
        let client_id_str = client_id.to_string();
        let session_id = match self.sessions.get_client_session(&client_id_str).await {
            Some(sid) => Some(sid),
            None => {
                return Some(ServerMessage::Error {
                    code: "NOT_IN_SESSION".to_string(),
                    message:
                        "You must join a session before submitting challenge rolls".to_string(),
                });
            }
        };

        let player_name = self
            .sessions
            .get_client_user_id(&client_id_str)
            .await
            .unwrap_or_else(|| "Unknown Player".to_string());

        // Look up character's skill modifier from PlayerCharacterService
        let character_modifier = if let Some(session_id_val) = session_id {
            if let Some(pc_id) = self.get_client_player_character(&client_id, session_id_val).await {
                match self
                    .player_character_service
                    .get_skill_modifier(pc_id, challenge.skill_id.clone())
                    .await
                {
                    Ok(modifier) => {
                        debug!(
                            pc_id = %pc_id,
                            skill_id = %challenge.skill_id,
                            modifier = modifier,
                            "Found skill modifier for player character (legacy roll path)"
                        );
                        modifier
                    }
                    Err(e) => {
                        debug!(
                            pc_id = %pc_id,
                            skill_id = %challenge.skill_id,
                            error = %e,
                            "Failed to get skill modifier, defaulting to 0 (legacy roll path)"
                        );
                        0
                    }
                }
            } else {
                debug!(
                    session_id = %session_id_val,
                    client_id = %client_id,
                    "Could not find player character for client (legacy roll path)"
                );
                0
            }
        } else {
            0
        };

        // Evaluate challenge result
        let (outcome_type, outcome) =
            evaluate_challenge_result(&challenge, roll, character_modifier);
        let success =
            outcome_type == OutcomeType::Success || outcome_type == OutcomeType::CriticalSuccess;

        // Publish AppEvent for challenge resolution
        let world_id = challenge.world_id;

        // Get character ID from player character lookup
        let character_id = if let Some(session_id_val) = session_id {
            self.get_client_player_character(&client_id, session_id_val)
                .await
                .map(|id| id.to_string())
                .unwrap_or_else(|| player_name.clone())
        } else {
            player_name.clone()
        };

        let app_event = AppEvent::ChallengeResolved {
            challenge_id: Some(challenge_id_str.clone()),
            challenge_name: challenge.name.clone(),
            world_id: world_id.to_string(),
            character_id,
            success,
            roll: Some(roll),
            total: Some(roll + character_modifier),
            session_id: session_id.map(|sid| sid.to_string()),
        };
        if let Err(e) = self.event_bus.publish(app_event).await {
            tracing::error!("Failed to publish ChallengeResolved event: {}", e);
        }

        // Execute outcome triggers (Phase 22D integration) for legacy path as well
        if let Some(sid) = session_id {
            let trigger_result = self
                .outcome_trigger_service
                .execute_triggers(&outcome.triggers, self.sessions.as_ref(), sid)
                .await;

            if !trigger_result.warnings.is_empty() {
                info!(
                    trigger_count = trigger_result.trigger_count,
                    warnings = ?trigger_result.warnings,
                    "Outcome triggers (legacy roll) executed with warnings"
                );
            }
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
                roll_breakdown: None, // Legacy method doesn't have formula info
                individual_rolls: None,
            };
            if let Ok(json) = serde_json::to_value(&result_msg) {
                if let Err(e) = self
                    .sessions
                    .broadcast_to_session(session_id, json)
                    .await
                {
                    tracing::error!("Failed to broadcast ChallengeResolved: {}", e);
                }
            } else {
                tracing::error!(
                    "Failed to serialize ChallengeResolved message for challenge {}",
                    challenge_id_str
                );
            }
        }

        None
    }

    /// Handle a player submitting a challenge roll with dice input (formula or manual).
    /// This is the enhanced version that supports dice formulas like "1d20+5".
    pub async fn handle_roll_input(
        &self,
        client_id: ClientId,
        challenge_id_str: String,
        dice_input: DiceInputType,
    ) -> Option<ServerMessage> {
        // Convert DiceInputType to DiceRollInput
        let roll_input = match dice_input {
            DiceInputType::Formula(formula) => DiceRollInput::Formula(formula),
            DiceInputType::Manual(value) => DiceRollInput::ManualResult(value),
        };

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

        // Get session and player info via async session port
        let client_id_str = client_id.to_string();
        let session_id = match self.sessions.get_client_session(&client_id_str).await {
            Some(sid) => Some(sid),
            None => {
                return Some(ServerMessage::Error {
                    code: "NOT_IN_SESSION".to_string(),
                    message:
                        "You must join a session before submitting challenge rolls".to_string(),
                });
            }
        };

        let player_name = self
            .sessions
            .get_client_user_id(&client_id_str)
            .await
            .unwrap_or_else(|| "Unknown Player".to_string());

        // Look up character's skill modifier from PlayerCharacterService
        let character_modifier = if let Some(session_id_val) = session_id {
            if let Some(pc_id) = self.get_client_player_character(&client_id, session_id_val).await {
                // Try to get the skill modifier from the player character service
                match self.player_character_service
                    .get_skill_modifier(pc_id, challenge.skill_id.clone())
                    .await
                {
                    Ok(modifier) => {
                        debug!(
                            pc_id = %pc_id,
                            skill_id = %challenge.skill_id,
                            modifier = modifier,
                            "Found skill modifier for player character"
                        );
                        modifier
                    }
                    Err(e) => {
                        debug!(
                            pc_id = %pc_id,
                            skill_id = %challenge.skill_id,
                            error = %e,
                            "Failed to get skill modifier, defaulting to 0"
                        );
                        0
                    }
                }
            } else {
                debug!(
                    session_id = %session_id_val,
                    client_id = %client_id,
                    "Could not find player character for client"
                );
                0
            }
        } else {
            0
        };

        // Resolve the dice roll
        let roll_result = match roll_input.resolve_with_modifier(character_modifier) {
            Ok(result) => result,
            Err(e) => {
                return Some(ServerMessage::Error {
                    code: "INVALID_DICE_FORMULA".to_string(),
                    message: format!("Invalid dice formula: {}", e),
                });
            }
        };

        // For d20 systems, check natural 1/20 using the raw die roll (before modifier)
        let raw_roll = if roll_result.is_manual() {
            roll_result.total // For manual, we use the total as the "roll"
        } else {
            roll_result.dice_total // For formula, use just the dice total
        };

        // Evaluate challenge result
        let (outcome_type, outcome) =
            evaluate_challenge_result(&challenge, raw_roll, character_modifier);
        let success =
            outcome_type == OutcomeType::Success || outcome_type == OutcomeType::CriticalSuccess;

        // Publish AppEvent for challenge resolution
        let world_id = challenge.world_id;

        // Get character ID from player character lookup
        let character_id = if let Some(session_id_val) = session_id {
            self.get_client_player_character(&client_id, session_id_val)
                .await
                .map(|id| id.to_string())
                .unwrap_or_else(|| player_name.clone())
        } else {
            player_name.clone()
        };

        let app_event = AppEvent::ChallengeResolved {
            challenge_id: Some(challenge_id_str.clone()),
            challenge_name: challenge.name.clone(),
            world_id: world_id.to_string(),
            character_id,
            success,
            roll: Some(raw_roll),
            total: Some(roll_result.total),
            session_id: session_id.map(|sid| sid.to_string()),
        };
        if let Err(e) = self.event_bus.publish(app_event).await {
            tracing::error!("Failed to publish ChallengeResolved event: {}", e);
        }

        // Execute outcome triggers (Phase 22D integration)
        if let Some(sid) = session_id {
            let trigger_result = self
                .outcome_trigger_service
                .execute_triggers(&outcome.triggers, self.sessions.as_ref(), sid)
                .await;

            if !trigger_result.warnings.is_empty() {
                info!(
                    trigger_count = trigger_result.trigger_count,
                    warnings = ?trigger_result.warnings,
                    "Outcome triggers executed with warnings"
                );
            }
        }

        // Broadcast ChallengeResolved to all participants
        if let Some(session_id) = session_id {
            let result_msg = ServerMessage::ChallengeResolved {
                challenge_id: challenge_id_str.clone(),
                challenge_name: challenge.name.clone(),
                character_name: player_name,
                roll: raw_roll,
                modifier: roll_result.modifier_applied,
                total: roll_result.total,
                outcome: outcome_type.display_name().to_string(),
                outcome_description: outcome.description.clone(),
                roll_breakdown: Some(roll_result.breakdown()),
                individual_rolls: if roll_result.is_manual() {
                    None
                } else {
                    Some(roll_result.individual_rolls.clone())
                },
            };
            if let Ok(json) = serde_json::to_value(&result_msg) {
                if let Err(e) = self
                    .sessions
                    .broadcast_to_session(session_id, json)
                    .await
                {
                    tracing::error!("Failed to broadcast ChallengeResolved: {}", e);
                }
            } else {
                tracing::error!(
                    "Failed to serialize ChallengeResolved message for challenge {}",
                    challenge_id_str
                );
            }
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

        // Get suggested dice based on difficulty type
        let (suggested_dice, rule_system_hint) = get_dice_suggestion_for_challenge(&challenge);

        let prompt = ServerMessage::ChallengePrompt {
            challenge_id: challenge_id_str.clone(),
            challenge_name: challenge.name.clone(),
            skill_name: skill_name.clone(),
            difficulty_display: challenge.difficulty.display(),
            description: challenge.description.clone(),
            character_modifier,
            suggested_dice: Some(suggested_dice),
            rule_system_hint: Some(rule_system_hint),
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

                        // Get suggested dice based on difficulty type
                        let (suggested_dice, rule_system_hint) =
                            get_dice_suggestion_for_challenge(&challenge);

                        let prompt = ServerMessage::ChallengePrompt {
                            challenge_id: challenge_suggestion.challenge_id.clone(),
                            challenge_name: challenge.name.clone(),
                            skill_name: challenge_suggestion.skill_name.clone(),
                            difficulty_display,
                            description: challenge.description.clone(),
                            character_modifier,
                            suggested_dice: Some(suggested_dice),
                            rule_system_hint: Some(rule_system_hint),
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

    /// Handle DM creating an ad-hoc challenge (no LLM involved)
    pub async fn handle_adhoc_challenge(
        &self,
        client_id: ClientId,
        challenge_name: String,
        skill_name: String,
        difficulty: String,
        target_pc_id: String,
        outcomes: crate::infrastructure::websocket::messages::AdHocOutcomes,
    ) -> Option<ServerMessage> {
        let sessions_read = self.sessions.read().await;
        let session_id = sessions_read.get_client_session(client_id);

        // Verify DM
        let is_dm = session_id
            .and_then(|sid| sessions_read.get_session(sid))
            .and_then(|s| s.get_dm())
            .filter(|dm| dm.client_id == client_id)
            .is_some();

        if !is_dm {
            return Some(ServerMessage::Error {
                code: "NOT_AUTHORIZED".to_string(),
                message: "Only the DM can create ad-hoc challenges".to_string(),
            });
        }

        // Generate a temporary challenge ID for this ad-hoc challenge
        let adhoc_challenge_id = uuid::Uuid::new_v4().to_string();

        // Store the ad-hoc outcomes in the session for later resolution
        // For now, we just broadcast the challenge prompt to the target player
        tracing::info!(
            "DM created ad-hoc challenge '{}' for PC {}: difficulty {}",
            challenge_name,
            target_pc_id,
            difficulty
        );

        // Determine suggested dice from difficulty string
        let (suggested_dice, rule_system_hint) = if difficulty.to_uppercase().starts_with("DC") {
            ("1d20".to_string(), "Roll 1d20 and add your modifier".to_string())
        } else if difficulty.ends_with('%') {
            ("1d100".to_string(), "Roll percentile dice".to_string())
        } else {
            ("2d6".to_string(), "Roll 2d6 and add your modifier".to_string())
        };

        let prompt = ServerMessage::ChallengePrompt {
            challenge_id: adhoc_challenge_id.clone(),
            challenge_name: challenge_name.clone(),
            skill_name,
            difficulty_display: difficulty,
            description: format!("Ad-hoc challenge created by DM"),
            character_modifier: 0, // DM would need to specify this
            suggested_dice: Some(suggested_dice),
            rule_system_hint: Some(rule_system_hint),
        };

        // Broadcast to session (the target player will see it)
        if let Some(sid) = session_id {
            sessions_read.broadcast_to_session(sid, &prompt);
        }

        // Notify DM that challenge was created
        Some(ServerMessage::AdHocChallengeCreated {
            challenge_id: adhoc_challenge_id,
            challenge_name,
            target_pc_id,
        })
    }
}

/// Get suggested dice and rule system hint based on challenge difficulty type.
fn get_dice_suggestion_for_challenge(
    challenge: &crate::domain::entities::Challenge,
) -> (String, String) {
    match &challenge.difficulty {
        crate::domain::entities::Difficulty::DC(_) => {
            // D20 systems (D&D, Pathfinder, etc.)
            (
                "1d20".to_string(),
                "Roll 1d20 and add your skill modifier".to_string(),
            )
        }
        crate::domain::entities::Difficulty::Percentage(_) => {
            // Percentile systems (Call of Cthulhu, etc.)
            (
                "1d100".to_string(),
                "Roll percentile dice (1d100), lower is better".to_string(),
            )
        }
        crate::domain::entities::Difficulty::Descriptor(desc) => {
            // Narrative systems - suggest 2d6 for PbtA-style games
            (
                "2d6".to_string(),
                format!("Roll 2d6 for {} difficulty", desc.display_name()),
            )
        }
        crate::domain::entities::Difficulty::Opposed => {
            // Opposed rolls - both parties roll
            (
                "1d20".to_string(),
                "Opposed roll - both parties roll and compare".to_string(),
            )
        }
        crate::domain::entities::Difficulty::Custom(desc) => {
            // Custom difficulty - let the hint explain
            (
                "1d20".to_string(),
                format!("Custom difficulty: {}", desc),
            )
        }
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


