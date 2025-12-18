//! StoryEvent Service - Automatically records gameplay events to the timeline
//!
//! This service provides convenient methods for creating StoryEvents when
//! gameplay actions occur, such as dialogue exchanges, challenge results,
//! scene transitions, and more.
//!
//! # Graph-First Architecture
//!
//! StoryEvent relationships (session, location, scene, involved characters, triggered_by,
//! recorded_challenge) are stored as graph edges, not embedded fields. This service
//! handles creating the event node and all related edges in a single operation.

use anyhow::Result;
use std::sync::Arc;

use crate::application::dto::AppEvent;
use crate::application::ports::outbound::{EventBusPort, StoryEventRepositoryPort};
use crate::domain::entities::{
    ChallengeEventOutcome, DmMarkerType, InfoImportance, InfoType, InvolvedCharacter,
    ItemSource, MarkerImportance, StoryEvent, StoryEventType,
};
use crate::domain::value_objects::{
    ChallengeId, CharacterId, LocationId, NarrativeEventId, SceneId, SessionId, StoryEventId,
    WorldId,
};

/// Service for recording gameplay events to the story timeline
#[derive(Clone)]
pub struct StoryEventService {
    repository: Arc<dyn StoryEventRepositoryPort>,
    event_bus: Arc<dyn EventBusPort<AppEvent>>,
}

impl StoryEventService {
    pub fn new(repository: Arc<dyn StoryEventRepositoryPort>, event_bus: Arc<dyn EventBusPort<AppEvent>>) -> Self {
        Self { repository, event_bus }
    }

    /// Helper to publish StoryEventCreated after persisting
    async fn publish_event_created(&self, event: &StoryEvent) {
        let app_event = AppEvent::StoryEventCreated {
            story_event_id: event.id.to_string(),
            world_id: event.world_id.to_string(),
            event_type: format!("{:?}", event.event_type), // Debug format for event type
        };
        
        if let Err(e) = self.event_bus.publish(app_event).await {
            tracing::error!("Failed to publish StoryEventCreated for {}: {}", event.id, e);
        }
    }

    /// Record a dialogue exchange between player and NPC
    pub async fn record_dialogue_exchange(
        &self,
        world_id: WorldId,
        session_id: SessionId,
        scene_id: Option<SceneId>,
        location_id: Option<LocationId>,
        npc_id: CharacterId,
        npc_name: String,
        player_dialogue: String,
        npc_response: String,
        topics: Vec<String>,
        tone: Option<String>,
        involved_characters: Vec<CharacterId>,
        game_time: Option<String>,
    ) -> Result<StoryEventId> {
        let event_type = StoryEventType::DialogueExchange {
            npc_id,
            npc_name: npc_name.clone(),
            player_dialogue,
            npc_response,
            topics_discussed: topics,
            tone,
        };

        let mut event = StoryEvent::new(world_id, event_type)
            .with_summary(format!("Spoke with {}", npc_name));

        if let Some(gt) = game_time {
            event = event.with_game_time(gt);
        }

        let event_id = event.id;
        self.repository.create(&event).await?;

        // Create edges for relationships
        self.repository.set_session(event_id, session_id).await?;
        if let Some(sid) = scene_id {
            self.repository.set_scene(event_id, sid).await?;
        }
        if let Some(lid) = location_id {
            self.repository.set_location(event_id, lid).await?;
        }
        for char_id in involved_characters {
            self.repository.add_involved_character(event_id, InvolvedCharacter::actor(char_id)).await?;
        }

        self.publish_event_created(&event).await;

        tracing::debug!("Recorded dialogue exchange event: {}", event_id);
        Ok(event_id)
    }

    /// Record a challenge attempt and its result
    pub async fn record_challenge_attempted(
        &self,
        world_id: WorldId,
        session_id: SessionId,
        scene_id: Option<SceneId>,
        location_id: Option<LocationId>,
        challenge_id: Option<ChallengeId>,
        challenge_name: String,
        character_id: CharacterId,
        skill_used: Option<String>,
        difficulty: Option<String>,
        roll_result: Option<i32>,
        modifier: Option<i32>,
        outcome: ChallengeEventOutcome,
        game_time: Option<String>,
    ) -> Result<StoryEventId> {
        let event_type = StoryEventType::ChallengeAttempted {
            challenge_id,
            challenge_name: challenge_name.clone(),
            character_id,
            skill_used,
            difficulty,
            roll_result,
            modifier,
            outcome,
        };

        let outcome_text = match outcome {
            ChallengeEventOutcome::CriticalSuccess => "Critical Success",
            ChallengeEventOutcome::Success => "Success",
            ChallengeEventOutcome::PartialSuccess => "Partial Success",
            ChallengeEventOutcome::Failure => "Failure",
            ChallengeEventOutcome::CriticalFailure => "Critical Failure",
        };

        let mut event = StoryEvent::new(world_id, event_type)
            .with_summary(format!("{}: {}", challenge_name, outcome_text));

        if let Some(gt) = game_time {
            event = event.with_game_time(gt);
        }

        let event_id = event.id;
        self.repository.create(&event).await?;

        // Create edges for relationships
        self.repository.set_session(event_id, session_id).await?;
        if let Some(sid) = scene_id {
            self.repository.set_scene(event_id, sid).await?;
        }
        if let Some(lid) = location_id {
            self.repository.set_location(event_id, lid).await?;
        }
        self.repository.add_involved_character(event_id, InvolvedCharacter::actor(character_id)).await?;
        if let Some(cid) = challenge_id {
            self.repository.set_recorded_challenge(event_id, cid).await?;
        }

        self.publish_event_created(&event).await;

        tracing::debug!("Recorded challenge event: {}", event_id);
        Ok(event_id)
    }

    /// Record a scene transition
    pub async fn record_scene_transition(
        &self,
        world_id: WorldId,
        session_id: SessionId,
        from_scene: Option<SceneId>,
        to_scene: SceneId,
        from_scene_name: Option<String>,
        to_scene_name: String,
        trigger_reason: String,
        location_id: Option<LocationId>,
        game_time: Option<String>,
    ) -> Result<StoryEventId> {
        let event_type = StoryEventType::SceneTransition {
            from_scene,
            to_scene,
            from_scene_name,
            to_scene_name: to_scene_name.clone(),
            trigger_reason,
        };

        let mut event = StoryEvent::new(world_id, event_type)
            .with_summary(format!("Entered: {}", to_scene_name));

        if let Some(gt) = game_time {
            event = event.with_game_time(gt);
        }

        let event_id = event.id;
        self.repository.create(&event).await?;

        // Create edges for relationships
        self.repository.set_session(event_id, session_id).await?;
        self.repository.set_scene(event_id, to_scene).await?;
        if let Some(lid) = location_id {
            self.repository.set_location(event_id, lid).await?;
        }

        self.publish_event_created(&event).await;

        tracing::debug!("Recorded scene transition event: {}", event_id);
        Ok(event_id)
    }

    /// Record a DM marker (note, plot point, etc.)
    pub async fn record_dm_marker(
        &self,
        world_id: WorldId,
        session_id: SessionId,
        scene_id: Option<SceneId>,
        location_id: Option<LocationId>,
        title: String,
        note: String,
        importance: MarkerImportance,
        marker_type: DmMarkerType,
        is_hidden: bool,
        tags: Vec<String>,
        game_time: Option<String>,
    ) -> Result<StoryEventId> {
        let event_type = StoryEventType::DmMarker {
            title: title.clone(),
            note,
            importance,
            marker_type,
        };

        let mut event = StoryEvent::new(world_id, event_type)
            .with_summary(title);

        if let Some(gt) = game_time {
            event = event.with_game_time(gt);
        }
        for tag in tags {
            event = event.with_tag(tag);
        }
        if is_hidden {
            event = event.hidden();
        }

        let event_id = event.id;
        self.repository.create(&event).await?;

        // Create edges for relationships
        self.repository.set_session(event_id, session_id).await?;
        if let Some(sid) = scene_id {
            self.repository.set_scene(event_id, sid).await?;
        }
        if let Some(lid) = location_id {
            self.repository.set_location(event_id, lid).await?;
        }

        self.publish_event_created(&event).await;

        tracing::debug!("Recorded DM marker event: {}", event_id);
        Ok(event_id)
    }

    /// Record information revealed to players
    pub async fn record_information_revealed(
        &self,
        world_id: WorldId,
        session_id: SessionId,
        scene_id: Option<SceneId>,
        location_id: Option<LocationId>,
        info_type: InfoType,
        title: String,
        content: String,
        source: Option<CharacterId>,
        importance: InfoImportance,
        persist_to_journal: bool,
        involved_characters: Vec<CharacterId>,
        game_time: Option<String>,
    ) -> Result<StoryEventId> {
        let event_type = StoryEventType::InformationRevealed {
            info_type,
            title: title.clone(),
            content,
            source,
            importance,
            persist_to_journal,
        };

        let mut event = StoryEvent::new(world_id, event_type)
            .with_summary(format!("Discovered: {}", title));

        if let Some(gt) = game_time {
            event = event.with_game_time(gt);
        }

        let event_id = event.id;
        self.repository.create(&event).await?;

        // Create edges for relationships
        self.repository.set_session(event_id, session_id).await?;
        if let Some(sid) = scene_id {
            self.repository.set_scene(event_id, sid).await?;
        }
        if let Some(lid) = location_id {
            self.repository.set_location(event_id, lid).await?;
        }
        for char_id in involved_characters {
            self.repository.add_involved_character(event_id, InvolvedCharacter::witness(char_id)).await?;
        }

        self.publish_event_created(&event).await;

        tracing::debug!("Recorded information revealed event: {}", event_id);
        Ok(event_id)
    }

    /// Record a relationship change
    pub async fn record_relationship_changed(
        &self,
        world_id: WorldId,
        session_id: SessionId,
        scene_id: Option<SceneId>,
        from_character: CharacterId,
        to_character: CharacterId,
        previous_sentiment: Option<f32>,
        new_sentiment: f32,
        reason: String,
        game_time: Option<String>,
    ) -> Result<StoryEventId> {
        let sentiment_change = new_sentiment - previous_sentiment.unwrap_or(0.0);

        let event_type = StoryEventType::RelationshipChanged {
            from_character,
            to_character,
            previous_sentiment,
            new_sentiment,
            sentiment_change,
            reason: reason.clone(),
        };

        let mut event = StoryEvent::new(world_id, event_type)
            .with_summary(reason);

        if let Some(gt) = game_time {
            event = event.with_game_time(gt);
        }

        let event_id = event.id;
        self.repository.create(&event).await?;

        // Create edges for relationships
        self.repository.set_session(event_id, session_id).await?;
        if let Some(sid) = scene_id {
            self.repository.set_scene(event_id, sid).await?;
        }
        self.repository.add_involved_character(event_id, InvolvedCharacter::actor(from_character)).await?;
        self.repository.add_involved_character(event_id, InvolvedCharacter::target(to_character)).await?;

        self.publish_event_created(&event).await;

        tracing::debug!("Recorded relationship change event: {}", event_id);
        Ok(event_id)
    }

    /// Record an item being acquired
    pub async fn record_item_acquired(
        &self,
        world_id: WorldId,
        session_id: SessionId,
        scene_id: Option<SceneId>,
        location_id: Option<LocationId>,
        item_name: String,
        item_description: Option<String>,
        character_id: CharacterId,
        source: ItemSource,
        quantity: u32,
        game_time: Option<String>,
    ) -> Result<StoryEventId> {
        let event_type = StoryEventType::ItemAcquired {
            item_name: item_name.clone(),
            item_description,
            character_id,
            source,
            quantity,
        };

        let mut event = StoryEvent::new(world_id, event_type)
            .with_summary(format!("Acquired {}", item_name));

        if let Some(gt) = game_time {
            event = event.with_game_time(gt);
        }

        let event_id = event.id;
        self.repository.create(&event).await?;

        // Create edges for relationships
        self.repository.set_session(event_id, session_id).await?;
        if let Some(sid) = scene_id {
            self.repository.set_scene(event_id, sid).await?;
        }
        if let Some(lid) = location_id {
            self.repository.set_location(event_id, lid).await?;
        }
        self.repository.add_involved_character(event_id, InvolvedCharacter::actor(character_id)).await?;

        self.publish_event_created(&event).await;

        tracing::debug!("Recorded item acquired event: {}", event_id);
        Ok(event_id)
    }

    /// Record when a narrative event is triggered
    pub async fn record_narrative_event_triggered(
        &self,
        world_id: WorldId,
        session_id: SessionId,
        scene_id: Option<SceneId>,
        location_id: Option<LocationId>,
        narrative_event_id: NarrativeEventId,
        narrative_event_name: String,
        outcome_branch: Option<String>,
        effects_applied: Vec<String>,
        involved_characters: Vec<CharacterId>,
        game_time: Option<String>,
    ) -> Result<StoryEventId> {
        let event_type = StoryEventType::NarrativeEventTriggered {
            narrative_event_id,
            narrative_event_name: narrative_event_name.clone(),
            outcome_branch,
            effects_applied,
        };

        let mut event = StoryEvent::new(world_id, event_type)
            .with_summary(format!("Event: {}", narrative_event_name));

        if let Some(gt) = game_time {
            event = event.with_game_time(gt);
        }

        let event_id = event.id;
        self.repository.create(&event).await?;

        // Create edges for relationships
        self.repository.set_session(event_id, session_id).await?;
        self.repository.set_triggered_by(event_id, narrative_event_id).await?;
        if let Some(sid) = scene_id {
            self.repository.set_scene(event_id, sid).await?;
        }
        if let Some(lid) = location_id {
            self.repository.set_location(event_id, lid).await?;
        }
        for char_id in involved_characters {
            self.repository.add_involved_character(event_id, InvolvedCharacter::actor(char_id)).await?;
        }

        self.publish_event_created(&event).await;

        tracing::debug!("Recorded narrative event triggered: {}", event_id);
        Ok(event_id)
    }

    /// Record session start
    pub async fn record_session_started(
        &self,
        world_id: WorldId,
        session_id: SessionId,
        session_number: u32,
        session_name: Option<String>,
        players_present: Vec<String>,
    ) -> Result<StoryEventId> {
        let event_type = StoryEventType::SessionStarted {
            session_number,
            session_name: session_name.clone(),
            players_present,
        };

        let event = StoryEvent::new(world_id, event_type)
            .with_summary(format!("Session {} started", session_number));

        let event_id = event.id;
        self.repository.create(&event).await?;

        // Create edge for session relationship
        self.repository.set_session(event_id, session_id).await?;

        self.publish_event_created(&event).await;

        tracing::debug!("Recorded session started event: {}", event_id);
        Ok(event_id)
    }

    /// Record session end
    pub async fn record_session_ended(
        &self,
        world_id: WorldId,
        session_id: SessionId,
        duration_minutes: u32,
        summary: String,
    ) -> Result<StoryEventId> {
        let event_type = StoryEventType::SessionEnded {
            duration_minutes,
            summary: summary.clone(),
        };

        let event = StoryEvent::new(world_id, event_type).with_summary(summary);

        let event_id = event.id;
        self.repository.create(&event).await?;

        // Create edge for session relationship
        self.repository.set_session(event_id, session_id).await?;

        self.publish_event_created(&event).await;

        tracing::debug!("Recorded session ended event: {}", event_id);
        Ok(event_id)
    }

    // =========================================================================
    // Query Methods (used by HTTP routes)
    // =========================================================================

    /// Get a single story event by ID
    pub async fn get_event(&self, event_id: StoryEventId) -> Result<Option<StoryEvent>> {
        self.repository.get(event_id).await
    }

    /// List story events for a session
    pub async fn list_by_session(&self, session_id: SessionId) -> Result<Vec<StoryEvent>> {
        self.repository.list_by_session(session_id).await
    }

    /// List story events for a world
    pub async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<StoryEvent>> {
        self.repository.list_by_world(world_id).await
    }

    /// List story events for a world with pagination
    pub async fn list_by_world_paginated(
        &self,
        world_id: WorldId,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<StoryEvent>> {
        self.repository
            .list_by_world_paginated(world_id, limit, offset)
            .await
    }

    /// List visible (non-hidden) story events for a world
    pub async fn list_visible(&self, world_id: WorldId, limit: u32) -> Result<Vec<StoryEvent>> {
        self.repository.list_visible(world_id, limit).await
    }

    /// Search story events by tags
    pub async fn search_by_tags(
        &self,
        world_id: WorldId,
        tags: Vec<String>,
    ) -> Result<Vec<StoryEvent>> {
        self.repository.search_by_tags(world_id, tags).await
    }

    /// Search story events by text in summary
    pub async fn search_by_text(&self, world_id: WorldId, search_text: &str) -> Result<Vec<StoryEvent>> {
        self.repository.search_by_text(world_id, search_text).await
    }

    /// List events involving a specific character
    pub async fn list_by_character(&self, character_id: CharacterId) -> Result<Vec<StoryEvent>> {
        self.repository.list_by_character(character_id).await
    }

    /// List events at a specific location
    pub async fn list_by_location(&self, location_id: LocationId) -> Result<Vec<StoryEvent>> {
        self.repository.list_by_location(location_id).await
    }

    /// Update story event summary
    pub async fn update_summary(&self, event_id: StoryEventId, summary: &str) -> Result<bool> {
        self.repository.update_summary(event_id, summary).await
    }

    /// Update event visibility
    pub async fn set_hidden(&self, event_id: StoryEventId, is_hidden: bool) -> Result<bool> {
        self.repository.set_hidden(event_id, is_hidden).await
    }

    /// Update event tags
    pub async fn update_tags(&self, event_id: StoryEventId, tags: Vec<String>) -> Result<bool> {
        self.repository.update_tags(event_id, tags).await
    }

    /// Delete a story event
    pub async fn delete(&self, event_id: StoryEventId) -> Result<bool> {
        self.repository.delete(event_id).await
    }

    /// Count events for a world
    pub async fn count_by_world(&self, world_id: WorldId) -> Result<u64> {
        self.repository.count_by_world(world_id).await
    }
}
