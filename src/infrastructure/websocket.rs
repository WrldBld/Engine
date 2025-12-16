//! WebSocket handler for Player connections
//!
//! Message types are aligned between Engine and Player for seamless communication.

use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;

use crate::application::dto::DMAction;
use crate::application::services::scene_service::SceneService;
use crate::application::services::scene_resolution_service::SceneResolutionService;
use crate::application::services::player_character_service::PlayerCharacterService;
use crate::application::services::location_service::LocationService;
use crate::application::services::interaction_service::InteractionService;
use crate::domain::value_objects::ActionId;
use crate::infrastructure::session::ClientId;
use crate::infrastructure::state::AppState;

/// WebSocket upgrade handler
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Handle an individual WebSocket connection
async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Create a unique client ID for this connection
    let client_id = ClientId::new();

    // Create a channel for sending messages to this client
    let (tx, mut rx) = mpsc::unbounded_channel::<ServerMessage>();

    tracing::info!("New WebSocket connection established: {}", client_id);

    // Spawn a task to forward messages from the channel to the WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                if ws_sender.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    // Handle incoming messages
    while let Some(result) = ws_receiver.next().await {
        match result {
            Ok(Message::Text(text)) => match serde_json::from_str::<ClientMessage>(&text) {
                Ok(msg) => {
                    if let Some(response) = handle_message(msg, &state, client_id, tx.clone()).await
                    {
                        if tx.send(response).is_err() {
                            break;
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to parse message: {}", e);
                    let error = ServerMessage::Error {
                        code: "PARSE_ERROR".to_string(),
                        message: format!("Invalid message format: {}", e),
                    };
                    if tx.send(error).is_err() {
                        break;
                    }
                }
            },
            Ok(Message::Close(_)) => {
                tracing::info!("WebSocket connection closed by client: {}", client_id);
                break;
            }
            Ok(Message::Ping(data)) => {
                // Ping/Pong is handled by the send task through the channel
                let _ = tx.send(ServerMessage::Pong);
                let _ = data; // Acknowledge we received the ping data
            }
            Err(e) => {
                tracing::error!("WebSocket error for client {}: {}", client_id, e);
                break;
            }
            _ => {}
        }
    }

    // Clean up: remove client from session
    {
        let mut sessions = state.sessions.write().await;
        if let Some((session_id, participant)) = sessions.leave_session(client_id) {
            tracing::info!(
                "Client {} (user: {}) disconnected from session {}",
                client_id,
                participant.user_id,
                session_id
            );
        }
    }

    // Cancel the send task
    send_task.abort();

    tracing::info!("WebSocket connection terminated: {}", client_id);
}

/// Handle a parsed client message
async fn handle_message(
    msg: ClientMessage,
    state: &AppState,
    client_id: ClientId,
    sender: mpsc::UnboundedSender<ServerMessage>,
) -> Option<ServerMessage> {
    match msg {
        ClientMessage::Heartbeat => Some(ServerMessage::Pong),

        ClientMessage::JoinSession {
            user_id,
            role,
            world_id,
        } => {
            tracing::info!(
                "User {} joining as {:?}, world: {:?}",
                user_id,
                role,
                world_id
            );

            // Delegate to injected SessionJoinService to join or create a session
            match state.session_join_service.join_or_create_session_for_world(
                client_id,
                user_id.clone(),
                role,
                world_id,
                sender,
            )
            .await
            {
                Ok(session_joined_info) => {
                    // Broadcast PlayerJoined to other participants in the session
                    let player_joined_msg = ServerMessage::PlayerJoined {
                        user_id: user_id.clone(),
                        role,
                        character_name: None, // TODO: Load from character selection
                    };
                    let sessions = state.sessions.read().await;
                    sessions.broadcast_to_session_except(
                        session_joined_info.session_id,
                        &player_joined_msg,
                        client_id,
                    );

                    Some(ServerMessage::SessionJoined {
                        session_id: session_joined_info.session_id.to_string(),
                        role,
                        participants: session_joined_info.participants,
                        world_snapshot: session_joined_info.world_snapshot,
                    })
                }
                Err(e) => {
                    tracing::error!("Failed to join session: {}", e);
                    Some(ServerMessage::Error {
                        code: "SESSION_ERROR".to_string(),
                        message: format!("Failed to join session: {}", e),
                    })
                }
            }
        }

        ClientMessage::PlayerAction {
            action_type,
            target,
            dialogue,
        } => {
            tracing::debug!("Received player action: {} -> {:?}", action_type, target);

            // Generate a unique action ID for tracking
            let action_id = ActionId::new();
            let action_id_str = action_id.to_string();

            // Get the client's session and user info
            let sessions = state.sessions.read().await;
            let (session_id, player_id) = match sessions.get_client_session(client_id) {
                Some(sid) => {
                    let pid = sessions
                        .get_session(sid)
                        .and_then(|s| s.participants.get(&client_id))
                        .map(|p| p.user_id.clone())
                        .unwrap_or_else(|| "unknown".to_string());
                    (Some(sid), pid)
                }
                None => {
                    tracing::warn!("Client {} sent action but is not in any session", client_id);
                    return Some(ServerMessage::Error {
                        code: "NOT_IN_SESSION".to_string(),
                        message: "You must join a session before performing actions".to_string(),
                    });
                }
            };

            let session_id = session_id.expect("session_id should exist at this point");
            drop(sessions); // Release lock before async queue operation

            // Handle Travel actions immediately (update location and resolve scene)
            if action_type == "travel" {
                if let Some(location_id_str) = target.as_ref() {
                    // Parse location ID
                    let location_uuid = match uuid::Uuid::parse_str(location_id_str) {
                        Ok(uuid) => crate::domain::value_objects::LocationId::from_uuid(uuid),
                        Err(_) => {
                            return Some(ServerMessage::Error {
                                code: "INVALID_LOCATION_ID".to_string(),
                                message: "Invalid location ID format".to_string(),
                            });
                        }
                    };

                    // Get PC for this user
                    match state
                        .player_character_service
                        .get_pc_by_user_and_session(&player_id, session_id)
                        .await
                    {
                        Ok(Some(pc)) => {
                            // Update PC location
                            if let Err(e) = state
                                .player_character_service
                                .update_pc_location(pc.id, location_uuid)
                                .await
                            {
                                tracing::error!("Failed to update PC location: {}", e);
                                return Some(ServerMessage::Error {
                                    code: "LOCATION_UPDATE_FAILED".to_string(),
                                    message: format!("Failed to update location: {}", e),
                                });
                            }

                            // Resolve scene for the new location
                            match state
                                .scene_resolution_service
                                .resolve_scene_for_pc(pc.id)
                                .await
                            {
                                Ok(Some(scene)) => {
                                    // Load scene with relations to build SceneUpdate
                                    match state.scene_service.get_scene_with_relations(scene.id).await {
                                        Ok(Some(scene_with_relations)) => {
                                            // Load interactions for the scene
                                            let interaction_templates = match state.interaction_service.list_interactions(scene.id).await {
                                                Ok(templates) => templates,
                                                Err(_) => vec![],
                                            };

                                            // Build interactions
                                            let interactions: Vec<InteractionData> = interaction_templates
                                                .iter()
                                                .map(|i| {
                                                    let target_name = match &i.target {
                                                        crate::domain::entities::InteractionTarget::Character(char_id) => {
                                                            Some(format!("Character {}", char_id))
                                                        },
                                                        crate::domain::entities::InteractionTarget::Item(item_id) => {
                                                            Some(format!("Item {}", item_id))
                                                        },
                                                        crate::domain::entities::InteractionTarget::Environment(desc) => {
                                                            Some(desc.clone())
                                                        },
                                                        crate::domain::entities::InteractionTarget::None => None,
                                                    };
                                                    InteractionData {
                                                        id: i.id.to_string(),
                                                        name: i.name.clone(),
                                                        target_name,
                                                        interaction_type: format!("{:?}", i.interaction_type),
                                                        is_available: i.is_available,
                                                    }
                                                })
                                                .collect();

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

                                            // Broadcast SceneUpdate to the player
                                            // Also check for split party and notify DM
                                            let mut sessions = state.sessions.write().await;
                                            if let Some(session) = sessions.get_session_mut(session_id) {
                                                session.current_scene_id = Some(scene.id.to_string());
                                                session.send_to_participant(&player_id, &scene_update);
                                                tracing::info!(
                                                    "Sent scene update to player {} after travel to location {}",
                                                    player_id,
                                                    location_id_str
                                                );

                                                // Check for split party and notify DM
                                                if let Ok(resolution_result) = state
                                                    .scene_resolution_service
                                                    .resolve_scene_for_session(session_id)
                                                    .await
                                                {
                                                    if resolution_result.is_split_party {
                                                        // Get location details for notification
                                                        let mut split_locations = Vec::new();
                                                        let pcs = match state
                                                            .player_character_service
                                                            .get_pcs_by_session(session_id)
                                                            .await
                                                        {
                                                            Ok(pcs) => pcs,
                                                            Err(_) => vec![],
                                                        };

                                                        // Group PCs by location
                                                        let mut location_pcs: std::collections::HashMap<String, Vec<&_>> = std::collections::HashMap::new();
                                                        for pc in &pcs {
                                                            location_pcs
                                                                .entry(pc.current_location_id.to_string())
                                                                .or_insert_with(Vec::new)
                                                                .push(pc);
                                                        }

                                                        // Build location info
                                                        for (loc_id_str, pcs_at_loc) in location_pcs.iter() {
                                                            if let Ok(location) = state
                                                                .location_service
                                                                .get_location(crate::domain::value_objects::LocationId::from_uuid(
                                                                    uuid::Uuid::parse_str(loc_id_str).unwrap_or_default()
                                                                ))
                                                                .await
                                                            {
                                                                if let Some(loc) = location {
                                                                    split_locations.push(crate::infrastructure::websocket::messages::SplitPartyLocation {
                                                                        location_id: loc_id_str.clone(),
                                                                        location_name: loc.name,
                                                                        pc_count: pcs_at_loc.len(),
                                                                        pc_names: pcs_at_loc.iter().map(|pc| pc.name.clone()).collect(),
                                                                    });
                                                                }
                                                            }
                                                        }

                                                        // Send notification to DM
                                                        if session.has_dm() {
                                                            session.send_to_dm(&ServerMessage::SplitPartyNotification {
                                                                location_count: split_locations.len(),
                                                                locations: split_locations,
                                                            });
                                                        }
                                                    }
                                                }
                                            }
                                            drop(sessions);

                                            // Return acknowledgment
                                            return Some(ServerMessage::ActionReceived {
                                                action_id: action_id_str,
                                                player_id: player_id.clone(),
                                                action_type: action_type.clone(),
                                            });
                                        }
                                        Ok(None) => {
                                            tracing::warn!("Scene {} not found after resolution", scene.id);
                                        }
                                        Err(e) => {
                                            tracing::error!("Failed to load scene with relations: {}", e);
                                        }
                                    }
                                }
                                Ok(None) => {
                                    // No scene found, but location updated - still acknowledge
                                    tracing::warn!(
                                        "No scene found for location {} after travel",
                                        location_id_str
                                    );
                                }
                                Err(e) => {
                                    tracing::error!("Failed to resolve scene: {}", e);
                                }
                            }
                        }
                        Ok(None) => {
                            return Some(ServerMessage::Error {
                                code: "NO_PC".to_string(),
                                message: "You must create a character before traveling".to_string(),
                            });
                        }
                        Err(e) => {
                            tracing::error!("Failed to get PC: {}", e);
                            return Some(ServerMessage::Error {
                                code: "PC_LOOKUP_FAILED".to_string(),
                                message: format!("Failed to find your character: {}", e),
                            });
                        }
                    }
                } else {
                    return Some(ServerMessage::Error {
                        code: "MISSING_TARGET".to_string(),
                        message: "Travel action requires a target location".to_string(),
                    });
                }
            }

            // Enqueue to PlayerActionQueue - returns immediately
            match state
                .player_action_queue_service
                .enqueue_action(
                        session_id,
                    player_id.clone(),
                        action_type.clone(),
                        target.clone(),
                        dialogue.clone(),
                    )
                .await
            {
                Ok(_) => {
                    // Get queue depth for status update
                    let depth = state
                        .player_action_queue_service
                        .depth()
                        .await
                        .unwrap_or(0);

                    // Send ActionQueued event to DM
                    let sessions = state.sessions.read().await;
                    if let Some(session) = sessions.get_session(session_id) {
                        if session.has_dm() {
                            session.send_to_dm(&ServerMessage::ActionQueued {
                                action_id: action_id_str.clone(),
                                player_name: player_id.clone(),
                                action_type: action_type.clone(),
                                queue_depth: depth,
                            });
                        }
                    }
                    drop(sessions);

                tracing::info!(
                        "Enqueued action {} from player {} in session {}: {} -> {:?}",
                    action_id_str,
                    player_id,
                    session_id,
                    action_type,
                    target
                );

                // Send ActionReceived acknowledgment to the player
                let _ = sender.send(ServerMessage::ActionReceived {
                    action_id: action_id_str,
                    player_id,
                    action_type: action_type.clone(),
                });
                }
                Err(e) => {
                    tracing::error!("Failed to enqueue player action: {}", e);
                    return Some(ServerMessage::Error {
                        code: "QUEUE_ERROR".to_string(),
                        message: format!("Failed to queue action: {}", e),
                    });
                }
            }

            None // No response from here; responses come from LLM processing or DM approval
        }

        ClientMessage::RequestSceneChange { scene_id } => {
            tracing::debug!("Scene change requested: {}", scene_id);

            // Parse scene_id
            let scene_uuid = match uuid::Uuid::parse_str(&scene_id) {
                Ok(uuid) => crate::domain::value_objects::SceneId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_SCENE_ID".to_string(),
                        message: "Invalid scene ID format".to_string(),
                    });
                }
            };

            // Get the client's session
            let sessions_read = state.sessions.read().await;
            let session_id = match sessions_read.get_client_session(client_id) {
                Some(sid) => sid,
                None => {
                    return Some(ServerMessage::Error {
                        code: "NOT_IN_SESSION".to_string(),
                        message: "You must join a session before requesting scene changes".to_string(),
                    });
                }
            };
            drop(sessions_read);

            // Load scene from database with relations
            let scene_with_relations = match state.scene_service.get_scene_with_relations(scene_uuid).await {
                Ok(Some(scene_data)) => scene_data,
                Ok(None) => {
                    return Some(ServerMessage::Error {
                        code: "SCENE_NOT_FOUND".to_string(),
                        message: format!("Scene {} not found", scene_id),
                    });
                }
                Err(e) => {
                    tracing::error!("Failed to load scene: {}", e);
                    return Some(ServerMessage::Error {
                        code: "SCENE_LOAD_ERROR".to_string(),
                        message: "Failed to load scene".to_string(),
                    });
                }
            };

            // Load interactions for the scene
            let interactions = match state.interaction_service.list_interactions(scene_uuid).await {
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

            // Build character data from featured characters
            let characters: Vec<CharacterData> = scene_with_relations
                .featured_characters
                .iter()
                .map(|c| CharacterData {
                    id: c.id.to_string(),
                    name: c.name.clone(),
                    sprite_asset: c.sprite_asset.clone(),
                    portrait_asset: c.portrait_asset.clone(),
                    position: CharacterPosition::Center, // Default position, could be enhanced
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

            // Update session's current scene and broadcast to all participants
            let mut sessions_write = state.sessions.write().await;
            if let Some(session) = sessions_write.get_session_mut(session_id) {
                // Update current scene in session
                session.current_scene_id = Some(scene_id.clone());
                
                // Broadcast SceneUpdate to all participants
                sessions_write.broadcast_to_session(session_id, &scene_update);
            }
            drop(sessions_write);

            tracing::info!("Scene change to {} broadcast to session {}", scene_id, session_id);

            None // SceneUpdate is broadcast, no direct response needed
        }

        ClientMessage::DirectorialUpdate { context: _ } => {
            tracing::debug!("Received directorial update");

            // Only DMs should send directorial updates
            let sessions = state.sessions.read().await;
            if let Some(session_id) = sessions.get_client_session(client_id) {
                if let Some(session) = sessions.get_session(session_id) {
                    // Verify this client is the DM
                    if let Some(dm) = session.get_dm() {
                        if dm.client_id == client_id {
                            // TODO: Update directorial context and store in session
                            tracing::info!(
                                "DM updated directorial context for session {}",
                                session_id
                            );
                        }
                    }
                }
            }

            None // No response needed
        }

        ClientMessage::ApprovalDecision {
            request_id,
            decision,
        } => {
            tracing::debug!(
                "Received approval decision for {}: {:?}",
                request_id,
                decision
            );

            // Only DMs should approve
            let sessions = state.sessions.read().await;
            let session_id = sessions.get_client_session(client_id);
            let dm_id = session_id
                .and_then(|sid| sessions.get_session(sid))
                .and_then(|s| s.get_dm())
                .filter(|dm| dm.client_id == client_id)
                .map(|dm| dm.user_id.clone());
            drop(sessions);

            if let (Some(session_id), Some(dm_id)) = (session_id, dm_id) {
                // Enqueue to DMActionQueue - returns immediately
                // The DM action queue worker will process this asynchronously
                let dm_action = DMAction::ApprovalDecision {
                    request_id: request_id.clone(),
                    decision: decision.clone(),
                };

                match state
                    .dm_action_queue_service
                    .enqueue_action(session_id, dm_id, dm_action)
                    .await
                {
                    Ok(_) => {
                        tracing::info!("Enqueued approval decision for request {}", request_id);
                        // Return acknowledgment - processing happens in background worker
                        return None;
                    }
                    Err(e) => {
                        tracing::error!("Failed to enqueue approval decision: {}", e);
                        return Some(ServerMessage::Error {
                            code: "QUEUE_ERROR".to_string(),
                            message: format!("Failed to queue approval: {}", e),
                        });
                    }
                }
            } else {
                return Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can approve responses".to_string(),
                });
            }
        }

        ClientMessage::ChallengeRoll { challenge_id, roll } => {
            tracing::debug!(
                "Received challenge roll: {} for challenge {}",
                roll,
                challenge_id
            );
            state
                .challenge_resolution_service
                .handle_roll(client_id, challenge_id, roll)
                .await
        }

        ClientMessage::TriggerChallenge {
            challenge_id,
            target_character_id,
        } => {
            state
                .challenge_resolution_service
                .handle_trigger(client_id, challenge_id, target_character_id)
                .await
        }

        ClientMessage::ChallengeSuggestionDecision {
            request_id,
            approved,
            modified_difficulty,
        } => state
            .challenge_resolution_service
            .handle_suggestion_decision(client_id, request_id, approved, modified_difficulty)
            .await,

        ClientMessage::NarrativeEventSuggestionDecision {
            request_id,
            event_id,
            approved,
            selected_outcome,
        } => state
            .narrative_event_approval_service
            .handle_decision(client_id, request_id, event_id, approved, selected_outcome)
            .await,
    }
}

// Re-export message types from the dedicated messages module
pub mod messages;
pub use messages::{
    CharacterData, CharacterPosition, ClientMessage, DirectorialContext, InteractionData,
    ParticipantInfo, ParticipantRole, SceneData, ServerMessage,
};
