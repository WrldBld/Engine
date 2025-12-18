//! Challenge Outcome Approval Service (P3.3)
//!
//! Manages the DM approval workflow for challenge resolutions.
//! After a player rolls, the outcome goes to this service before
//! being broadcast to all players.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::RwLock;

use crate::application::dto::{
    ChallengeOutcomeApprovalItem, ChallengeOutcomeDecision, ChallengeOutcomePendingNotification,
    ChallengeResolvedNotification, ChallengeRollSubmittedNotification, OutcomeBranchDto,
    OutcomeBranchesReadyNotification, OutcomeSuggestionReadyNotification, OutcomeSuggestionRequest,
    PendingChallengeResolutionDto,
};
use crate::application::ports::outbound::{AsyncSessionPort, LlmPort};
use crate::application::services::{OutcomeSuggestionService, OutcomeTriggerService, SettingsService};
use crate::domain::value_objects::SessionId;

/// Error type for challenge outcome approval operations
#[derive(Debug, thiserror::Error)]
pub enum ChallengeOutcomeError {
    #[error("Resolution not found: {0}")]
    NotFound(String),
    #[error("Session error: {0}")]
    SessionError(String),
    #[error("Invalid state: {0}")]
    InvalidState(String),
}

/// Service for managing challenge outcome approvals
///
/// This service holds pending challenge resolutions in memory until the DM
/// approves, edits, or requests suggestions for them.
///
/// Generic over `L: LlmPort` for LLM suggestion generation.
pub struct ChallengeOutcomeApprovalService<L: LlmPort> {
    /// Pending resolutions indexed by resolution_id
    pending: Arc<RwLock<HashMap<String, ChallengeOutcomeApprovalItem>>>,
    /// Session port for broadcasting messages
    sessions: Arc<dyn AsyncSessionPort>,
    /// Outcome trigger service for executing triggers
    outcome_trigger_service: Arc<OutcomeTriggerService>,
    /// LLM port for generating outcome suggestions
    llm_port: Option<Arc<L>>,
    /// Settings service for configurable values
    settings_service: Option<Arc<SettingsService>>,
}

impl<L: LlmPort + 'static> ChallengeOutcomeApprovalService<L> {
    /// Create a new challenge outcome approval service
    pub fn new(
        sessions: Arc<dyn AsyncSessionPort>,
        outcome_trigger_service: Arc<OutcomeTriggerService>,
    ) -> Self {
        Self {
            pending: Arc::new(RwLock::new(HashMap::new())),
            sessions,
            outcome_trigger_service,
            llm_port: None,
            settings_service: None,
        }
    }

    /// Set the LLM port for generating outcome suggestions
    pub fn with_llm_port(mut self, llm_port: Arc<L>) -> Self {
        self.llm_port = Some(llm_port);
        self
    }

    /// Set the settings service for configurable values
    pub fn with_settings_service(mut self, settings_service: Arc<SettingsService>) -> Self {
        self.settings_service = Some(settings_service);
        self
    }

    /// Queue a challenge resolution for DM approval
    ///
    /// Returns the resolution_id for tracking.
    pub async fn queue_for_approval(
        &self,
        session_id: SessionId,
        resolution: PendingChallengeResolutionDto,
    ) -> Result<String, ChallengeOutcomeError> {
        let resolution_id = resolution.resolution_id.clone();

        // Convert DTO to approval item
        let item = ChallengeOutcomeApprovalItem {
            resolution_id: resolution.resolution_id.clone(),
            session_id,
            challenge_id: resolution.challenge_id,
            challenge_name: resolution.challenge_name.clone(),
            challenge_description: resolution.challenge_description,
            skill_name: resolution.skill_name,
            character_id: resolution.character_id,
            character_name: resolution.character_name.clone(),
            roll: resolution.roll,
            modifier: resolution.modifier,
            total: resolution.total,
            outcome_type: resolution.outcome_type.clone(),
            outcome_description: resolution.outcome_description.clone(),
            outcome_triggers: resolution
                .outcome_triggers
                .into_iter()
                .map(|t| crate::domain::value_objects::ProposedToolInfo {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: format!("{:?}", t),
                    description: String::new(),
                    arguments: serde_json::json!({}),
                })
                .collect(),
            roll_breakdown: resolution.roll_breakdown,
            timestamp: Utc::now(),
            suggestions: None,
            is_generating_suggestions: false,
        };

        // Store in pending map
        {
            let mut pending = self.pending.write().await;
            pending.insert(resolution_id.clone(), item.clone());
        }

        // Notify DM of pending outcome
        self.notify_dm_pending_outcome(session_id, &item).await?;

        // Notify player that roll is awaiting approval
        self.notify_player_awaiting_approval(session_id, &item).await?;

        tracing::info!(
            "Challenge resolution {} queued for DM approval",
            resolution_id
        );

        Ok(resolution_id)
    }

    /// Process DM's decision on a challenge outcome
    pub async fn process_decision(
        &self,
        session_id: SessionId,
        resolution_id: &str,
        decision: ChallengeOutcomeDecision,
    ) -> Result<(), ChallengeOutcomeError> {
        // Get the pending item
        let item = {
            let pending = self.pending.read().await;
            pending
                .get(resolution_id)
                .cloned()
                .ok_or_else(|| ChallengeOutcomeError::NotFound(resolution_id.to_string()))?
        };

        // Verify session matches
        if item.session_id != session_id {
            return Err(ChallengeOutcomeError::InvalidState(
                "Session mismatch".to_string(),
            ));
        }

        match decision {
            ChallengeOutcomeDecision::Accept => {
                // Broadcast resolution with original description
                self.broadcast_resolution(session_id, &item, None).await?;
                // Remove from pending
                self.remove_pending(resolution_id).await;
            }
            ChallengeOutcomeDecision::Edit {
                modified_description,
            } => {
                // Broadcast resolution with modified description
                self.broadcast_resolution(session_id, &item, Some(modified_description))
                    .await?;
                // Remove from pending
                self.remove_pending(resolution_id).await;
            }
            ChallengeOutcomeDecision::Suggest { guidance } => {
                // Mark as generating suggestions
                self.set_generating_suggestions(resolution_id, true).await;

                // Check if LLM port is configured
                if let Some(ref llm_port) = self.llm_port {
                    tracing::info!(
                        "Generating LLM suggestions for {}: {:?}",
                        resolution_id,
                        guidance
                    );

                    // Build suggestion request
                    let request = OutcomeSuggestionRequest {
                        challenge_id: item.challenge_id.clone(),
                        challenge_name: item.challenge_name.clone(),
                        challenge_description: item.challenge_description.clone(),
                        skill_name: item.skill_name.clone().unwrap_or_default(),
                        outcome_type: item.outcome_type.clone(),
                        roll_context: format!(
                            "rolled {} + {} = {} ({})",
                            item.roll, item.modifier, item.total, item.outcome_type
                        ),
                        guidance,
                        narrative_context: None,
                    };

                    // Spawn async task to generate suggestions
                    let llm = llm_port.clone();
                    let pending = self.pending.clone();
                    let sessions = self.sessions.clone();
                    let resolution_id_owned = resolution_id.to_string();

                    tokio::spawn(async move {
                        let suggestion_service = OutcomeSuggestionService::new(llm);
                        match suggestion_service.generate_suggestions(&request).await {
                            Ok(suggestions) => {
                                // Update suggestions in pending map
                                let mut pending_guard = pending.write().await;
                                if let Some(pending_item) = pending_guard.get_mut(&resolution_id_owned) {
                                    pending_item.suggestions = Some(suggestions.clone());
                                    pending_item.is_generating_suggestions = false;
                                    let session_id = pending_item.session_id;
                                    drop(pending_guard);

                                    // Notify DM
                                    let msg = OutcomeSuggestionReadyNotification::new(
                                        resolution_id_owned.clone(),
                                        suggestions,
                                    );
                                    match serde_json::to_value(&msg) {
                                        Ok(value) => {
                                            if let Err(e) = sessions.send_to_dm(session_id, value).await {
                                                tracing::error!("Failed to send suggestions to DM: {}", e);
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!("Failed to serialize suggestions message: {}", e);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Failed to generate outcome suggestions for {}: {}",
                                    resolution_id_owned,
                                    e
                                );
                                // Mark as no longer generating
                                let mut pending_guard = pending.write().await;
                                if let Some(pending_item) = pending_guard.get_mut(&resolution_id_owned) {
                                    pending_item.is_generating_suggestions = false;
                                }
                            }
                        }
                    });
                } else {
                    tracing::warn!(
                        "LLM port not configured, cannot generate suggestions for {}",
                        resolution_id
                    );
                    // Mark as no longer generating
                    self.set_generating_suggestions(resolution_id, false).await;
                }
            }
        }

        Ok(())
    }

    /// Update suggestions for a pending resolution
    pub async fn update_suggestions(
        &self,
        resolution_id: &str,
        suggestions: Vec<String>,
    ) -> Result<(), ChallengeOutcomeError> {
        let mut pending = self.pending.write().await;
        if let Some(item) = pending.get_mut(resolution_id) {
            item.suggestions = Some(suggestions.clone());
            item.is_generating_suggestions = false;

            // Notify DM that suggestions are ready
            let session_id = item.session_id;
            drop(pending);

            let msg = OutcomeSuggestionReadyNotification::new(
                resolution_id.to_string(),
                suggestions,
            );

            let value = serde_json::to_value(&msg)
                .map_err(|e| ChallengeOutcomeError::SessionError(format!("Failed to serialize: {}", e)))?;
            self.sessions
                .send_to_dm(session_id, value)
                .await
                .map_err(|e| ChallengeOutcomeError::SessionError(e.to_string()))?;

            Ok(())
        } else {
            Err(ChallengeOutcomeError::NotFound(resolution_id.to_string()))
        }
    }

    /// Get all pending resolutions for a session
    pub async fn get_pending_for_session(
        &self,
        session_id: SessionId,
    ) -> Vec<ChallengeOutcomeApprovalItem> {
        let pending = self.pending.read().await;
        pending
            .values()
            .filter(|item| item.session_id == session_id)
            .cloned()
            .collect()
    }

    /// Broadcast the final resolution to all players
    async fn broadcast_resolution(
        &self,
        session_id: SessionId,
        item: &ChallengeOutcomeApprovalItem,
        modified_description: Option<String>,
    ) -> Result<(), ChallengeOutcomeError> {
        let description = modified_description.unwrap_or_else(|| item.outcome_description.clone());

        // Build ChallengeResolved notification
        let msg = ChallengeResolvedNotification::new(
            item.challenge_id.clone(),
            item.challenge_name.clone(),
            item.character_name.clone(),
            item.roll,
            item.modifier,
            item.total,
            item.outcome_type.clone(),
            description.clone(),
            item.roll_breakdown.clone(),
            None,
        );

        // Broadcast to all session participants
        let value = serde_json::to_value(&msg)
            .map_err(|e| ChallengeOutcomeError::SessionError(format!("Failed to serialize: {}", e)))?;
        self.sessions
            .broadcast_to_session(session_id, value)
            .await
            .map_err(|e| ChallengeOutcomeError::SessionError(e.to_string()))?;

        // Execute outcome triggers
        // TODO: Parse outcome_triggers from ProposedToolInfo back to OutcomeTrigger
        // For now, log that we would execute triggers
        tracing::info!(
            "Challenge {} resolved with outcome: {}",
            item.challenge_id,
            item.outcome_type
        );

        Ok(())
    }

    /// Notify DM of a pending outcome approval
    async fn notify_dm_pending_outcome(
        &self,
        session_id: SessionId,
        item: &ChallengeOutcomeApprovalItem,
    ) -> Result<(), ChallengeOutcomeError> {
        let msg = ChallengeOutcomePendingNotification::new(
            item.resolution_id.clone(),
            item.challenge_id.clone(),
            item.challenge_name.clone(),
            item.character_id.clone(),
            item.character_name.clone(),
            item.roll,
            item.modifier,
            item.total,
            item.outcome_type.clone(),
            item.outcome_description.clone(),
            item.outcome_triggers.clone(),
            item.roll_breakdown.clone(),
        );

        let value = serde_json::to_value(&msg)
            .map_err(|e| ChallengeOutcomeError::SessionError(format!("Failed to serialize: {}", e)))?;
        self.sessions
            .send_to_dm(session_id, value)
            .await
            .map_err(|e| ChallengeOutcomeError::SessionError(e.to_string()))?;

        Ok(())
    }

    /// Notify player that their roll is awaiting DM approval
    async fn notify_player_awaiting_approval(
        &self,
        session_id: SessionId,
        item: &ChallengeOutcomeApprovalItem,
    ) -> Result<(), ChallengeOutcomeError> {
        let msg = ChallengeRollSubmittedNotification::new(
            item.challenge_id.clone(),
            item.challenge_name.clone(),
            item.roll,
            item.modifier,
            item.total,
            item.outcome_type.clone(),
        );

        // Broadcast to all session participants (they'll see the roll is pending)
        // In the future, we could add send_to_participant by looking up user_id from character_id
        let value = serde_json::to_value(&msg)
            .map_err(|e| ChallengeOutcomeError::SessionError(format!("Failed to serialize: {}", e)))?;
        self.sessions
            .broadcast_to_session(session_id, value)
            .await
            .map_err(|e| ChallengeOutcomeError::SessionError(e.to_string()))?;

        Ok(())
    }

    /// Remove a resolution from pending
    async fn remove_pending(&self, resolution_id: &str) {
        let mut pending = self.pending.write().await;
        pending.remove(resolution_id);
    }

    /// Mark a resolution as generating suggestions
    async fn set_generating_suggestions(&self, resolution_id: &str, generating: bool) {
        let mut pending = self.pending.write().await;
        if let Some(item) = pending.get_mut(resolution_id) {
            item.is_generating_suggestions = generating;
        }
    }

    /// Request LLM to generate outcome branches for DM selection
    ///
    /// This method is similar to `process_decision` with Suggest, but generates
    /// structured branches instead of simple text suggestions.
    pub async fn request_branches(
        &self,
        session_id: SessionId,
        resolution_id: &str,
        guidance: Option<String>,
    ) -> Result<(), ChallengeOutcomeError> {
        // Get the pending item
        let item = {
            let pending = self.pending.read().await;
            pending
                .get(resolution_id)
                .cloned()
                .ok_or_else(|| ChallengeOutcomeError::NotFound(resolution_id.to_string()))?
        };

        // Verify session matches
        if item.session_id != session_id {
            return Err(ChallengeOutcomeError::InvalidState(
                "Session mismatch".to_string(),
            ));
        }

        // Mark as generating
        self.set_generating_suggestions(resolution_id, true).await;

        // Check if LLM port is configured
        if let Some(ref llm_port) = self.llm_port {
            tracing::info!(
                "Generating LLM outcome branches for {}: {:?}",
                resolution_id,
                guidance
            );

            // Build suggestion request (same format)
            let request = OutcomeSuggestionRequest {
                challenge_id: item.challenge_id.clone(),
                challenge_name: item.challenge_name.clone(),
                challenge_description: item.challenge_description.clone(),
                skill_name: item.skill_name.clone().unwrap_or_default(),
                outcome_type: item.outcome_type.clone(),
                roll_context: format!(
                    "rolled {} + {} = {} ({})",
                    item.roll, item.modifier, item.total, item.outcome_type
                ),
                guidance,
                narrative_context: None,
            };

            // Get settings for branch count and tokens per branch
            let (branch_count, tokens_per_branch) = if let Some(ref settings_service) = self.settings_service {
                // Try to get world-specific settings
                if let Some(world_id) = self.sessions.get_session_world_id(session_id).await {
                    let settings = settings_service.get_for_world(world_id).await;
                    (settings.outcome_branch_count as usize, settings.suggestion_tokens_per_branch)
                } else {
                    // Fall back to global settings if no world_id available
                    let settings = settings_service.get().await;
                    (settings.outcome_branch_count as usize, settings.suggestion_tokens_per_branch)
                }
            } else {
                (2, 200) // Defaults if no settings service configured
            };

            // Spawn async task to generate branches
            let llm = llm_port.clone();
            let pending = self.pending.clone();
            let sessions = self.sessions.clone();
            let resolution_id_owned = resolution_id.to_string();
            let outcome_type = item.outcome_type.clone();

            tokio::spawn(async move {
                let suggestion_service = OutcomeSuggestionService::new(llm);
                match suggestion_service.generate_branches(&request, branch_count, tokens_per_branch).await {
                    Ok(branches) => {
                        // Update pending item
                        let mut pending_guard = pending.write().await;
                        if let Some(pending_item) = pending_guard.get_mut(&resolution_id_owned) {
                            // Store branches as suggestions (converted to strings for backward compat)
                            pending_item.suggestions = Some(
                                branches.iter().map(|b| b.description.clone()).collect(),
                            );
                            pending_item.is_generating_suggestions = false;
                            let session_id = pending_item.session_id;
                            drop(pending_guard);

                            // Notify DM with structured branches
                            let msg = OutcomeBranchesReadyNotification::new(
                                resolution_id_owned.clone(),
                                outcome_type,
                                branches,
                            );
                            match serde_json::to_value(&msg) {
                                Ok(value) => {
                                    if let Err(e) = sessions.send_to_dm(session_id, value).await {
                                        tracing::error!("Failed to send branches to DM: {}", e);
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("Failed to serialize branches message: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to generate outcome branches for {}: {}",
                            resolution_id_owned,
                            e
                        );
                        // Mark as no longer generating
                        let mut pending_guard = pending.write().await;
                        if let Some(pending_item) = pending_guard.get_mut(&resolution_id_owned) {
                            pending_item.is_generating_suggestions = false;
                        }
                    }
                }
            });
        } else {
            tracing::warn!(
                "LLM port not configured, cannot generate branches for {}",
                resolution_id
            );
            self.set_generating_suggestions(resolution_id, false).await;
        }

        Ok(())
    }

    /// Select an outcome branch and resolve the challenge
    ///
    /// The DM picks a branch by ID, optionally modifying the description.
    pub async fn select_branch(
        &self,
        session_id: SessionId,
        resolution_id: &str,
        _branch_id: &str,
        modified_description: Option<String>,
    ) -> Result<(), ChallengeOutcomeError> {
        // Get the pending item
        let item = {
            let pending = self.pending.read().await;
            pending
                .get(resolution_id)
                .cloned()
                .ok_or_else(|| ChallengeOutcomeError::NotFound(resolution_id.to_string()))?
        };

        // Verify session matches
        if item.session_id != session_id {
            return Err(ChallengeOutcomeError::InvalidState(
                "Session mismatch".to_string(),
            ));
        }

        // The branch_id would be used to look up the selected branch's description
        // For now, we use the modified_description if provided, or fall back to the original
        // TODO: Store branches in the approval item and look up by branch_id
        let final_description = modified_description.unwrap_or_else(|| item.outcome_description.clone());

        // Broadcast the resolution with the selected branch description
        self.broadcast_resolution(session_id, &item, Some(final_description))
            .await?;

        // Remove from pending
        self.remove_pending(resolution_id).await;

        Ok(())
    }
}
