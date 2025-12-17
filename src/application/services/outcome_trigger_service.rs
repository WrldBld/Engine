//! Outcome Trigger Service - Executes triggers after challenge resolution
//!
//! This service handles the execution of outcome triggers from challenge outcomes.
//! It works alongside the ToolExecutionService but handles the static triggers
//! defined in challenge definitions rather than LLM-proposed tools.

use std::sync::Arc;
use tracing::{debug, info, instrument};

use crate::application::ports::outbound::{
    AsyncSessionPort, ChallengeRepositoryPort,
};
use crate::application::services::tool_execution_service::StateChange;
use crate::domain::entities::OutcomeTrigger;
use crate::domain::value_objects::SessionId;

/// Result of executing outcome triggers
#[derive(Debug, Clone)]
pub struct TriggerExecutionResult {
    /// Number of triggers executed
    pub trigger_count: usize,
    /// State changes that occurred
    pub state_changes: Vec<StateChange>,
    /// Any errors that occurred (non-fatal)
    pub warnings: Vec<String>,
}

/// Service for executing outcome triggers
pub struct OutcomeTriggerService {
    challenge_repository: Arc<dyn ChallengeRepositoryPort>,
}

impl OutcomeTriggerService {
    /// Create a new OutcomeTriggerService
    pub fn new(challenge_repository: Arc<dyn ChallengeRepositoryPort>) -> Self {
        Self {
            challenge_repository,
        }
    }

    /// Execute a list of outcome triggers
    ///
    /// This method processes each trigger and generates appropriate state changes.
    /// Some triggers may require async operations (like enabling/disabling challenges).
    ///
    /// It uses the async session port to record conversation history and any
    /// session-scoped side effects, preserving the application/infra boundary.
    #[instrument(skip(self, session_port))]
    pub async fn execute_triggers(
        &self,
        triggers: &[OutcomeTrigger],
        session_port: &dyn AsyncSessionPort,
        session_id: SessionId,
    ) -> TriggerExecutionResult {
        let mut state_changes = Vec::new();
        let mut warnings = Vec::new();

        for trigger in triggers {
            match self
                .execute_single_trigger(trigger, session_port, session_id)
                .await
            {
                Ok(changes) => state_changes.extend(changes),
                Err(e) => {
                    warnings.push(format!("Trigger execution warning: {}", e));
                }
            }
        }

        info!(
            trigger_count = triggers.len(),
            state_changes = state_changes.len(),
            warnings = warnings.len(),
            "Executed outcome triggers"
        );

        TriggerExecutionResult {
            trigger_count: triggers.len(),
            state_changes,
            warnings,
        }
    }

    /// Execute a single trigger and return state changes
    async fn execute_single_trigger(
        &self,
        trigger: &OutcomeTrigger,
        session_port: &dyn AsyncSessionPort,
        session_id: SessionId,
    ) -> Result<Vec<StateChange>, String> {
        match trigger {
            OutcomeTrigger::RevealInformation { info, persist } => {
                debug!(info = %info, persist = %persist, "Revealing information");

                session_port
                    .add_to_conversation_history(
                        session_id,
                        "System",
                        &format!("[REVELATION] {}", info),
                    )
                    .await
                    .map_err(|e| e.to_string())?;

                Ok(vec![StateChange::InfoRevealed {
                    info: if *persist {
                        format!("[JOURNAL] {}", info)
                    } else {
                        info.clone()
                    },
                }])
            }

            OutcomeTrigger::EnableChallenge { challenge_id } => {
                debug!(challenge_id = %challenge_id, "Enabling challenge");

                // Update the challenge in the repository
                if let Err(e) = self
                    .challenge_repository
                    .set_active(*challenge_id, true)
                    .await
                {
                    return Err(format!("Failed to enable challenge: {}", e));
                }

                session_port
                    .add_to_conversation_history(
                        session_id,
                        "System",
                        &format!("[CHALLENGE ENABLED] {}", challenge_id),
                    )
                    .await
                    .map_err(|e| e.to_string())?;

                Ok(vec![StateChange::EventTriggered {
                    name: format!("Challenge {} enabled", challenge_id),
                }])
            }

            OutcomeTrigger::DisableChallenge { challenge_id } => {
                debug!(challenge_id = %challenge_id, "Disabling challenge");

                // Update the challenge in the repository
                if let Err(e) = self
                    .challenge_repository
                    .set_active(*challenge_id, false)
                    .await
                {
                    return Err(format!("Failed to disable challenge: {}", e));
                }

                session_port
                    .add_to_conversation_history(
                        session_id,
                        "System",
                        &format!("[CHALLENGE DISABLED] {}", challenge_id),
                    )
                    .await
                    .map_err(|e| e.to_string())?;

                Ok(vec![StateChange::EventTriggered {
                    name: format!("Challenge {} disabled", challenge_id),
                }])
            }

            OutcomeTrigger::ModifyCharacterStat { stat, modifier } => {
                debug!(stat = %stat, modifier = %modifier, "Modifying character stat");

                session_port
                    .add_to_conversation_history(
                        session_id,
                        "System",
                        &format!(
                            "[STAT] {} {}{}",
                            stat,
                            if *modifier >= 0 { "+" } else { "" },
                            modifier
                        ),
                    )
                    .await
                    .map_err(|e| e.to_string())?;

                Ok(vec![StateChange::CharacterStatUpdated {
                    character_id: "active_pc".to_string(), // Will be resolved by caller
                    stat_name: stat.clone(),
                    delta: *modifier,
                }])
            }

            OutcomeTrigger::TriggerScene { scene_id } => {
                debug!(scene_id = %scene_id, "Triggering scene transition");

                session_port
                    .add_to_conversation_history(
                        session_id,
                        "System",
                        &format!("[SCENE TRANSITION] Moving to scene {}", scene_id),
                    )
                    .await
                    .map_err(|e| e.to_string())?;

                Ok(vec![StateChange::EventTriggered {
                    name: format!("Scene transition to {}", scene_id),
                }])
            }

            OutcomeTrigger::GiveItem {
                item_name,
                item_description,
            } => {
                debug!(item_name = %item_name, "Giving item");

                let desc = item_description
                    .as_ref()
                    .map(|d| format!(" - {}", d))
                    .unwrap_or_default();

                session_port
                    .add_to_conversation_history(
                        session_id,
                        "System",
                        &format!("[ITEM RECEIVED] {}{}", item_name, desc),
                    )
                    .await
                    .map_err(|e| e.to_string())?;

                Ok(vec![StateChange::ItemAdded {
                    character: "active_pc".to_string(),
                    item: item_name.clone(),
                }])
            }

            OutcomeTrigger::Custom { description } => {
                debug!(description = %description, "Custom trigger");

                session_port
                    .add_to_conversation_history(
                        session_id,
                        "System",
                        &format!("[CUSTOM] {}", description),
                    )
                    .await
                    .map_err(|e| e.to_string())?;

                Ok(vec![StateChange::EventTriggered {
                    name: format!("Custom: {}", description),
                }])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::outbound::{
        BroadcastMessage, PendingApprovalInfo, SessionManagementError, SessionManagementPort,
        SessionWorldContext,
    };
    use crate::domain::entities::Challenge;
    use crate::domain::value_objects::{ChallengeId, WorldId};
    use async_trait::async_trait;
    use std::sync::Mutex;

    /// Fake session manager for testing
    struct FakeSessionManager {
        history: Mutex<Vec<String>>,
    }

    impl FakeSessionManager {
        fn new() -> Self {
            Self {
                history: Mutex::new(Vec::new()),
            }
        }
    }

    impl SessionManagementPort for FakeSessionManager {
        fn get_client_session(&self, _client_id: &str) -> Option<SessionId> {
            Some(SessionId::new())
        }

        fn is_client_dm(&self, _client_id: &str) -> bool {
            false
        }

        fn get_client_user_id(&self, _client_id: &str) -> Option<String> {
            None
        }

        fn get_pending_approval(
            &self,
            _session_id: SessionId,
            _request_id: &str,
        ) -> Option<PendingApprovalInfo> {
            None
        }

        fn add_pending_approval(
            &mut self,
            _session_id: SessionId,
            _approval: PendingApprovalInfo,
        ) -> Result<(), SessionManagementError> {
            Ok(())
        }

        fn remove_pending_approval(
            &mut self,
            _session_id: SessionId,
            _request_id: &str,
        ) -> Result<(), SessionManagementError> {
            Ok(())
        }

        fn increment_retry_count(
            &mut self,
            _session_id: SessionId,
            _request_id: &str,
        ) -> Result<u32, SessionManagementError> {
            Ok(0)
        }

        fn broadcast_to_players(
            &self,
            _session_id: SessionId,
            _message: &BroadcastMessage,
        ) -> Result<(), SessionManagementError> {
            Ok(())
        }

        fn send_to_dm(
            &self,
            _session_id: SessionId,
            _message: &BroadcastMessage,
        ) -> Result<(), SessionManagementError> {
            Ok(())
        }

        fn broadcast_except(
            &self,
            _session_id: SessionId,
            _message: &BroadcastMessage,
            _exclude_client: &str,
        ) -> Result<(), SessionManagementError> {
            Ok(())
        }

        fn broadcast_to_session(
            &self,
            _session_id: SessionId,
            _message: &BroadcastMessage,
        ) -> Result<(), SessionManagementError> {
            Ok(())
        }

        fn add_to_conversation_history(
            &mut self,
            _session_id: SessionId,
            _speaker: &str,
            text: &str,
        ) -> Result<(), SessionManagementError> {
            self.history.lock().unwrap().push(text.to_string());
            Ok(())
        }

        fn session_has_dm(&self, _session_id: SessionId) -> bool {
            false
        }

        fn get_session_world_context(
            &self,
            _session_id: SessionId,
        ) -> Option<SessionWorldContext> {
            None
        }
    }

    /// Fake challenge repository for testing
    struct FakeChallengeRepository {
        active_states: Mutex<std::collections::HashMap<ChallengeId, bool>>,
    }

    impl FakeChallengeRepository {
        fn new() -> Self {
            Self {
                active_states: Mutex::new(std::collections::HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl ChallengeRepositoryPort for FakeChallengeRepository {
        async fn create(
            &self,
            _challenge: &Challenge,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        async fn get(
            &self,
            _id: ChallengeId,
        ) -> anyhow::Result<Option<Challenge>>
        {
            Ok(None)
        }

        async fn list_by_world(
            &self,
            _world_id: WorldId,
        ) -> anyhow::Result<Vec<Challenge>> {
            Ok(vec![])
        }

        async fn list_by_scene(
            &self,
            _scene_id: crate::domain::value_objects::SceneId,
        ) -> anyhow::Result<Vec<Challenge>> {
            Ok(vec![])
        }

        async fn list_active(
            &self,
            _world_id: WorldId,
        ) -> anyhow::Result<Vec<Challenge>> {
            Ok(vec![])
        }

        async fn list_favorites(
            &self,
            _world_id: WorldId,
        ) -> anyhow::Result<Vec<Challenge>> {
            Ok(vec![])
        }

        async fn update(
            &self,
            _challenge: &Challenge,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        async fn delete(
            &self,
            _id: ChallengeId,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        async fn set_active(
            &self,
            id: ChallengeId,
            active: bool,
        ) -> anyhow::Result<()> {
            self.active_states.lock().unwrap().insert(id, active);
            Ok(())
        }

        async fn toggle_favorite(
            &self,
            _id: ChallengeId,
        ) -> anyhow::Result<bool> {
            Ok(false)
        }
    }

    #[tokio::test]
    async fn test_reveal_information() {
        let repo = Arc::new(FakeChallengeRepository::new());
        let service = OutcomeTriggerService::new(repo);
        let mut session = FakeSessionManager::new();
        let session_id = SessionId::new();

        let triggers = vec![OutcomeTrigger::reveal("A secret passage is revealed!")];

        let result = service
            .execute_triggers(&triggers, &mut session, session_id)
            .await;

        assert_eq!(result.trigger_count, 1);
        assert_eq!(result.state_changes.len(), 1);
        assert!(result.warnings.is_empty());
        assert!(matches!(
            &result.state_changes[0],
            StateChange::InfoRevealed { info } if info.contains("secret passage")
        ));
    }

    #[tokio::test]
    async fn test_give_item() {
        let repo = Arc::new(FakeChallengeRepository::new());
        let service = OutcomeTriggerService::new(repo);
        let mut session = FakeSessionManager::new();
        let session_id = SessionId::new();

        let triggers = vec![OutcomeTrigger::GiveItem {
            item_name: "Magic Sword".to_string(),
            item_description: Some("A blade that glows blue".to_string()),
        }];

        let result = service
            .execute_triggers(&triggers, &mut session, session_id)
            .await;

        assert_eq!(result.trigger_count, 1);
        assert!(matches!(
            &result.state_changes[0],
            StateChange::ItemAdded { item, .. } if item == "Magic Sword"
        ));
    }

    #[tokio::test]
    async fn test_multiple_triggers() {
        let repo = Arc::new(FakeChallengeRepository::new());
        let service = OutcomeTriggerService::new(repo);
        let mut session = FakeSessionManager::new();
        let session_id = SessionId::new();

        let triggers = vec![
            OutcomeTrigger::reveal("You learn the password"),
            OutcomeTrigger::GiveItem {
                item_name: "Key".to_string(),
                item_description: None,
            },
            OutcomeTrigger::modify_stat("reputation", 10),
        ];

        let result = service
            .execute_triggers(&triggers, &mut session, session_id)
            .await;

        assert_eq!(result.trigger_count, 3);
        assert_eq!(result.state_changes.len(), 3);
        assert!(result.warnings.is_empty());
    }
}
