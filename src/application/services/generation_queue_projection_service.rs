//! Generation Queue Projection Service
//!
//! Builds a unified `GenerationQueueSnapshot` for a given (user, world) from:
//! - Active generation batches exposed by `AssetService`
//! - Suggestion-related `AppEvent`s from the event repository
//! - Per-user/world read markers from `GenerationReadStatePort`
//!
//! This keeps HTTP routes thin and centralizes queue reconstruction logic.

use std::collections::{HashMap, HashSet};

use crate::application::dto::{AppEvent, GenerationBatchResponseDto};
use crate::application::ports::outbound::{AppEventRepositoryPort, GenerationReadKind, GenerationReadStatePort};
use crate::application::services::asset_service::{AssetService, AssetServiceImpl};
use crate::domain::value_objects::WorldId;

/// Snapshot DTO for suggestion tasks, mirrored from `infrastructure::http::queue_routes`.
#[derive(Debug, serde::Serialize)]
pub struct SuggestionTaskSnapshot {
    pub request_id: String,
    pub field_type: String,
    pub entity_id: Option<String>,
    pub status: String,
    pub suggestions: Option<Vec<String>>,
    pub error: Option<String>,
    pub is_read: bool,
}

/// Unified generation queue snapshot (batches + suggestions).
///
/// This is intentionally kept in the application layer so HTTP routes and
/// WebSocket projections can share the same reconstruction logic.
#[derive(Debug, serde::Serialize)]
pub struct GenerationQueueSnapshot {
    pub batches: Vec<GenerationBatchResponseDtoWithRead>,
    pub suggestions: Vec<SuggestionTaskSnapshot>,
}

#[derive(Debug, serde::Serialize)]
pub struct GenerationBatchResponseDtoWithRead {
    #[serde(flatten)]
    pub batch: GenerationBatchResponseDto,
    pub is_read: bool,
}

/// Service responsible for projecting the current generation queue state.
pub struct GenerationQueueProjectionService {
    asset_service: AssetServiceImpl,
    app_event_repository: std::sync::Arc<dyn AppEventRepositoryPort>,
    read_state: std::sync::Arc<dyn GenerationReadStatePort>,
}

impl GenerationQueueProjectionService {
    pub fn new(
        asset_service: AssetServiceImpl,
        app_event_repository: std::sync::Arc<dyn AppEventRepositoryPort>,
        read_state: std::sync::Arc<dyn GenerationReadStatePort>,
    ) -> Self {
        Self {
            asset_service,
            app_event_repository,
            read_state,
        }
    }

    /// Build a `GenerationQueueSnapshot` for the given user + world.
    ///
    /// When `user_id` is `None`, the snapshot will treat all items as unread.
    pub async fn project_queue(
        &self,
        user_id: Option<&str>,
        world_id: WorldId,
    ) -> anyhow::Result<GenerationQueueSnapshot> {
        let world_key = world_id.to_string();
        
        // 1. Compute read markers for this user/world
        let mut read_batches: HashSet<String> = HashSet::new();
        let mut read_suggestions: HashSet<String> = HashSet::new();

        if let Some(uid) = user_id {
            if let Ok(markers) = self.read_state.list_read_for_user_world(uid, &world_key).await {
                for (item_id, kind) in markers {
                    match kind {
                        GenerationReadKind::Batch => {
                            read_batches.insert(item_id);
                        }
                        GenerationReadKind::Suggestion => {
                            read_suggestions.insert(item_id);
                        }
                    }
                }
            }
        }

        // 2. Image batches from AssetService - filtered by world_id
        let batches = AssetService::list_active_batches_by_world(&self.asset_service, world_id)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|b| {
                let dto = GenerationBatchResponseDto::from(b);
                let is_read = read_batches.contains(&dto.id);
                GenerationBatchResponseDtoWithRead { batch: dto, is_read }
            })
            .collect();

        // 3. Suggestion tasks reconstructed from recent AppEvents
        let mut suggestions_map: HashMap<String, SuggestionTaskSnapshot> = HashMap::new();

        if let Ok(events) = self.app_event_repository.fetch_since(0, 500).await {
            for (_id, event, _ts) in events {
                match event {
                    AppEvent::SuggestionQueued {
                        request_id,
                        field_type,
                        entity_id,
                        ..
                    } => {
                        let entry = suggestions_map
                            .entry(request_id.clone())
                            .or_insert(SuggestionTaskSnapshot {
                                request_id,
                                field_type,
                                entity_id,
                                status: "queued".to_string(),
                                suggestions: None,
                                error: None,
                                is_read: false,
                            });
                        entry.status = "queued".to_string();
                    }
                    AppEvent::SuggestionProgress { request_id, .. } => {
                        let entry = suggestions_map
                            .entry(request_id.clone())
                            .or_insert(SuggestionTaskSnapshot {
                                request_id,
                                field_type: String::new(),
                                entity_id: None,
                                status: "processing".to_string(),
                                suggestions: None,
                                error: None,
                                is_read: false,
                            });
                        entry.status = "processing".to_string();
                    }
                    AppEvent::SuggestionCompleted {
                        request_id,
                        field_type,
                        suggestions,
                        ..
                    } => {
                        let entry = suggestions_map
                            .entry(request_id.clone())
                            .or_insert(SuggestionTaskSnapshot {
                                request_id,
                                field_type: field_type.clone(),
                                entity_id: None,
                                status: "ready".to_string(),
                                suggestions: Some(suggestions.clone()),
                                error: None,
                                is_read: false,
                            });
                        entry.field_type = field_type;
                        entry.status = "ready".to_string();
                        entry.suggestions = Some(suggestions);
                        entry.error = None;
                    }
                    AppEvent::SuggestionFailed {
                        request_id,
                        field_type,
                        error,
                        ..
                    } => {
                        let entry = suggestions_map
                            .entry(request_id.clone())
                            .or_insert(SuggestionTaskSnapshot {
                                request_id,
                                field_type: field_type.clone(),
                                entity_id: None,
                                status: "failed".to_string(),
                                suggestions: None,
                                error: Some(error.clone()),
                                is_read: false,
                            });
                        entry.field_type = field_type;
                        entry.status = "failed".to_string();
                        entry.error = Some(error);
                    }
                    _ => {}
                }
            }
        }

        let mut suggestions: Vec<SuggestionTaskSnapshot> = suggestions_map.into_values().collect();
        for s in &mut suggestions {
            if read_suggestions.contains(&s.request_id) {
                s.is_read = true;
            }
        }

        Ok(GenerationQueueSnapshot { batches, suggestions })
    }
}


