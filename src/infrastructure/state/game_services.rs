//! Game mechanics and narrative services

use std::sync::Arc;

use crate::application::dto::ApprovalItem;
use crate::application::ports::outbound::LlmPort;
use crate::application::services::{
    challenge_resolution_service::ChallengeResolutionService, ChallengeOutcomeApprovalService,
    ChallengeServiceImpl, EventChainServiceImpl, NarrativeEventApprovalService,
    NarrativeEventServiceImpl, PlayerCharacterServiceImpl, SkillServiceImpl, StoryEventService,
};

/// Services for game mechanics, challenges, and narrative events
///
/// This struct groups services related to the gameplay and storytelling
/// aspects: story events, challenges, narrative events, and their approval workflows.
///
/// Generic over `L: LlmPort` for LLM-powered suggestion generation.
pub struct GameServices<L: LlmPort> {
    pub story_event_service: StoryEventService,
    pub challenge_service: ChallengeServiceImpl,
    pub challenge_resolution_service: Arc<
        ChallengeResolutionService<
            ChallengeServiceImpl,
            SkillServiceImpl,
            crate::infrastructure::queues::QueueBackendEnum<ApprovalItem>,
            PlayerCharacterServiceImpl,
            L,
        >,
    >,
    pub challenge_outcome_approval_service: Arc<ChallengeOutcomeApprovalService<L>>,
    pub narrative_event_service: NarrativeEventServiceImpl,
    pub narrative_event_approval_service: Arc<NarrativeEventApprovalService<NarrativeEventServiceImpl>>,
    pub event_chain_service: EventChainServiceImpl,
}

impl<L: LlmPort + 'static> GameServices<L> {
    /// Creates a new GameServices instance with all game mechanic services
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        story_event_service: StoryEventService,
        challenge_service: ChallengeServiceImpl,
        challenge_resolution_service: Arc<
            ChallengeResolutionService<
                ChallengeServiceImpl,
                SkillServiceImpl,
                crate::infrastructure::queues::QueueBackendEnum<ApprovalItem>,
                PlayerCharacterServiceImpl,
                L,
            >,
        >,
        challenge_outcome_approval_service: Arc<ChallengeOutcomeApprovalService<L>>,
        narrative_event_service: NarrativeEventServiceImpl,
        narrative_event_approval_service: Arc<NarrativeEventApprovalService<NarrativeEventServiceImpl>>,
        event_chain_service: EventChainServiceImpl,
    ) -> Self {
        Self {
            story_event_service,
            challenge_service,
            challenge_resolution_service,
            challenge_outcome_approval_service,
            narrative_event_service,
            narrative_event_approval_service,
            event_chain_service,
        }
    }
}
