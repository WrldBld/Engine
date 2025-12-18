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

use crate::application::dto::{AdHocOutcomesDto, ChallengeOutcomeDecision, DMAction};
use crate::application::services::scene_service::SceneService;
use crate::application::services::scene_resolution_service::SceneResolutionService;
use crate::application::services::player_character_service::PlayerCharacterService;
use crate::application::services::location_service::LocationService;
use crate::application::services::interaction_service::InteractionService;
use crate::application::services::session_join_service as sjs;
use crate::application::services::challenge_resolution_service as crs;
use crate::application::ports::outbound::{PlayerCharacterRepositoryPort, SessionParticipantRole};
use crate::domain::value_objects::{TimeOfDay, ActionId, RegionRelationshipType};
use crate::infrastructure::session::ClientId;
use crate::infrastructure::state::AppState;

// Conversion helpers for adapting between infrastructure message types and service DTOs

/// Convert wire format ParticipantRole to canonical SessionParticipantRole
fn wire_to_canonical_role(role: ParticipantRole) -> SessionParticipantRole {
    match role {
        ParticipantRole::DungeonMaster => SessionParticipantRole::DungeonMaster,
        ParticipantRole::Player => SessionParticipantRole::Player,
        ParticipantRole::Spectator => SessionParticipantRole::Spectator,
    }
}

/// Convert canonical SessionParticipantRole to wire format ParticipantRole
fn canonical_to_wire_role(role: SessionParticipantRole) -> ParticipantRole {
    match role {
        SessionParticipantRole::DungeonMaster => ParticipantRole::DungeonMaster,
        SessionParticipantRole::Player => ParticipantRole::Player,
        SessionParticipantRole::Spectator => ParticipantRole::Spectator,
    }
}

/// Convert session_join_service::ParticipantInfo to wire format messages::ParticipantInfo
fn from_service_participant(p: sjs::ParticipantInfo) -> ParticipantInfo {
    ParticipantInfo {
        user_id: p.user_id,
        role: canonical_to_wire_role(p.role),
        character_name: p.character_name,
    }
}

/// Convert messages::DiceInputType to challenge_resolution_service::DiceInputType
fn to_service_dice_input(input: messages::DiceInputType) -> crs::DiceInputType {
    match input {
        messages::DiceInputType::Formula(f) => crs::DiceInputType::Formula(f),
        messages::DiceInputType::Manual(v) => crs::DiceInputType::Manual(v),
    }
}

/// Convert messages::AdHocOutcomes to application dto AdHocOutcomesDto
fn to_adhoc_outcomes_dto(outcomes: messages::AdHocOutcomes) -> AdHocOutcomesDto {
    AdHocOutcomesDto {
        success: outcomes.success,
        failure: outcomes.failure,
        critical_success: outcomes.critical_success,
        critical_failure: outcomes.critical_failure,
    }
}

/// Try to deserialize a serde_json::Value into a ServerMessage
fn value_to_server_message(value: serde_json::Value) -> Option<ServerMessage> {
    serde_json::from_value(value).ok()
}

/// Convert wire format ChallengeOutcomeDecisionData to application DTO ChallengeOutcomeDecision
fn to_challenge_outcome_decision(decision: messages::ChallengeOutcomeDecisionData) -> ChallengeOutcomeDecision {
    match decision {
        messages::ChallengeOutcomeDecisionData::Accept => ChallengeOutcomeDecision::Accept,
        messages::ChallengeOutcomeDecisionData::Edit { modified_description } => {
            ChallengeOutcomeDecision::Edit { modified_description }
        }
        messages::ChallengeOutcomeDecisionData::Suggest { guidance } => {
            ChallengeOutcomeDecision::Suggest { guidance }
        }
    }
}

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

    // Clean up: remove client from session via async port
    if let Some((session_id, participant)) = state.async_session_port.client_leave_session(&client_id.to_string()).await {
        tracing::info!(
            "Client {} (user: {}) disconnected from session {}",
            client_id,
            participant.user_id,
            session_id
        );
    }

    // Cancel the send task
    send_task.abort();

    tracing::info!("WebSocket connection terminated: {}", client_id);
}

// NPC presence determination is now in domain::value_objects::region::RegionRelationshipType::is_npc_present

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

            // Create a JSON-value sender that forwards to the ServerMessage sender
            let (json_tx, mut json_rx) = mpsc::unbounded_channel::<serde_json::Value>();
            let server_tx = sender.clone();
            tokio::spawn(async move {
                while let Some(value) = json_rx.recv().await {
                    if let Ok(msg) = serde_json::from_value::<ServerMessage>(value) {
                        let _ = server_tx.send(msg);
                    }
                }
            });

            // Delegate to injected SessionJoinService to join or create a session
            match state.player.session_join_service.join_or_create_session_for_world(
                client_id.to_string(),
                user_id.clone(),
                wire_to_canonical_role(role),
                world_id,
                json_tx,
            )
            .await
            {
                Ok(session_joined_info) => {
                    // Broadcast PlayerJoined to other participants via async port
                    // Note: character_name is None at join time; it's set when player selects a character
                    let player_joined_msg = ServerMessage::PlayerJoined {
                        user_id: user_id.clone(),
                        role,
                        character_name: None,
                    };
                    if let Ok(msg_json) = serde_json::to_value(&player_joined_msg) {
                        let _ = state.async_session_port.broadcast_except(
                            session_joined_info.session_id,
                            msg_json,
                            &client_id.to_string(),
                        ).await;
                    }

                    // Convert service's ParticipantInfo to messages::ParticipantInfo
                    let participants: Vec<ParticipantInfo> = session_joined_info
                        .participants
                        .into_iter()
                        .map(from_service_participant)
                        .collect();

                    Some(ServerMessage::SessionJoined {
                        session_id: session_joined_info.session_id.to_string(),
                        role,
                        participants,
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

            // Get the client's session and user info via async port
            let client_id_str = client_id.to_string();
            let session_id = match state.async_session_port.get_client_session(&client_id_str).await {
                Some(sid) => sid,
                None => {
                    tracing::warn!("Client {} sent action but is not in any session", client_id);
                    return Some(ServerMessage::Error {
                        code: "NOT_IN_SESSION".to_string(),
                        message: "You must join a session before performing actions".to_string(),
                    });
                }
            };
            let player_id = state.async_session_port.get_client_user_id(&client_id_str).await
                .unwrap_or_else(|| "unknown".to_string());

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
                .player.player_character_service
                        .get_pc_by_user_and_session(&player_id, session_id)
                        .await
                    {
                        Ok(Some(pc)) => {
                            // Update PC location
                            if let Err(e) = state
                .player.player_character_service
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
                .player.scene_resolution_service
                                .resolve_scene_for_pc(pc.id)
                                .await
                            {
                                Ok(Some(scene)) => {
                                    // Load scene with relations to build SceneUpdate
                                    match state.core.scene_service.get_scene_with_relations(scene.id).await {
                                        Ok(Some(scene_with_relations)) => {
                                            // Load interactions for the scene
                                            let interaction_templates = match state.core.interaction_service.list_interactions(scene.id).await {
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

                                            // Update scene and send to player via async port
                                            let _ = state.async_session_port.update_session_scene(session_id, scene.id.to_string()).await;
                                            if let Ok(scene_msg) = serde_json::to_value(&scene_update) {
                                                let _ = state.async_session_port.send_to_participant(session_id, &player_id, scene_msg).await;
                                            }
                                            tracing::info!(
                                                "Sent scene update to player {} after travel to location {}",
                                                player_id,
                                                location_id_str
                                            );

                                            // Check for split party and notify DM
                                            if let Ok(resolution_result) = state
                .player.scene_resolution_service
                                                .resolve_scene_for_session(session_id)
                                                .await
                                            {
                                                if resolution_result.is_split_party {
                                                    // Get location details for notification
                                                    let mut split_locations = Vec::new();
                                                    let pcs = match state
                .player.player_character_service
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
                                                            .core.location_service
                                                            .get_location(crate::domain::value_objects::LocationId::from_uuid(
                                                                uuid::Uuid::parse_str(loc_id_str).unwrap_or_default()
                                                            ))
                                                            .await
                                                        {
                                                            if let Some(loc) = location {
                                                                split_locations.push(crate::infrastructure::websocket::messages::SplitPartyLocation {
                                                                    location_id: loc_id_str.to_string(),
                                                                    location_name: loc.name,
                                                                    pc_count: pcs_at_loc.len(),
                                                                    pc_names: pcs_at_loc.iter().map(|pc| pc.name.clone()).collect(),
                                                                });
                                                            }
                                                        }
                                                    }

                                                    // Send notification to DM via async port
                                                    if state.async_session_port.session_has_dm(session_id).await {
                                                        let dm_msg = ServerMessage::SplitPartyNotification {
                                                            location_count: split_locations.len(),
                                                            locations: split_locations,
                                                        };
                                                        if let Ok(dm_json) = serde_json::to_value(&dm_msg) {
                                                            let _ = state.async_session_port.send_to_dm(session_id, dm_json).await;
                                                        }
                                                    }
                                                }
                                            }

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

            // Look up the player's character ID for challenge targeting
            let pc_id = match state
                .player.player_character_service
                .get_pc_by_user_and_session(&player_id, session_id)
                .await
            {
                Ok(Some(pc)) => Some(pc.id),
                Ok(None) => {
                    tracing::debug!("Player {} has no character selected in session {}", player_id, session_id);
                    None
                }
                Err(e) => {
                    tracing::warn!("Failed to look up PC for player {}: {}", player_id, e);
                    None
                }
            };

            // Enqueue to PlayerActionQueue - returns immediately
            match state
                .queues.player_action_queue_service
                .enqueue_action(
                        session_id,
                    player_id.clone(),
                    pc_id,
                        action_type.clone(),
                        target.clone(),
                        dialogue.clone(),
                    )
                .await
            {
                Ok(_) => {
                    // Get queue depth for status update
                    let depth = state
                        .queues.player_action_queue_service
                        .depth()
                        .await
                        .unwrap_or(0);

                    // Send ActionQueued event to DM via async port
                    if state.async_session_port.session_has_dm(session_id).await {
                        let dm_msg = ServerMessage::ActionQueued {
                            action_id: action_id_str.clone(),
                            player_name: player_id.clone(),
                            action_type: action_type.clone(),
                            queue_depth: depth,
                        };
                        if let Ok(dm_json) = serde_json::to_value(&dm_msg) {
                            let _ = state.async_session_port.send_to_dm(session_id, dm_json).await;
                        }
                    }

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

            // Get the client's session via async port
            let session_id = match state.async_session_port.get_client_session(&client_id.to_string()).await {
                Some(sid) => sid,
                None => {
                    return Some(ServerMessage::Error {
                        code: "NOT_IN_SESSION".to_string(),
                        message: "You must join a session before requesting scene changes".to_string(),
                    });
                }
            };

            // Load scene from database with relations
            let scene_with_relations = match state.core.scene_service.get_scene_with_relations(scene_uuid).await {
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
            let interactions = match state.core.interaction_service.list_interactions(scene_uuid).await {
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

            // Update session's current scene and broadcast via async port
            let _ = state.async_session_port.update_session_scene(session_id, scene_id.clone()).await;
            if let Ok(scene_json) = serde_json::to_value(&scene_update) {
                let _ = state.async_session_port.broadcast_to_session(session_id, scene_json).await;
            }

            tracing::info!("Scene change to {} broadcast to session {}", scene_id, session_id);

            None // SceneUpdate is broadcast, no direct response needed
        }

        ClientMessage::DirectorialUpdate { context: _ } => {
            tracing::debug!("Received directorial update");

            // Only DMs should send directorial updates - check via async port
            let client_id_str = client_id.to_string();
            if state.async_session_port.is_client_dm(&client_id_str).await {
                if let Some(session_id) = state.async_session_port.get_client_session(&client_id_str).await {
                    // TODO: Update directorial context and store in session
                    tracing::info!(
                        "DM updated directorial context for session {}",
                        session_id
                    );
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

            // Only DMs should approve - check via async port
            let client_id_str = client_id.to_string();
            let session_id = state.async_session_port.get_client_session(&client_id_str).await;
            let is_dm = state.async_session_port.is_client_dm(&client_id_str).await;
            let dm_id = if is_dm {
                state.async_session_port.get_client_user_id(&client_id_str).await
            } else {
                None
            };

            if let (Some(session_id), Some(dm_id)) = (session_id, dm_id) {
                // Enqueue to DMActionQueue - returns immediately
                // The DM action queue worker will process this asynchronously
                let dm_action = DMAction::ApprovalDecision {
                    request_id: request_id.clone(),
                    decision: decision.clone(),
                };

                match state
                    .queues.dm_action_queue_service
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
                .game.challenge_resolution_service
                .handle_roll(client_id.to_string(), challenge_id, roll)
                .await
                .and_then(value_to_server_message)
        }

        ClientMessage::ChallengeRollInput {
            challenge_id,
            input_type,
        } => {
            tracing::debug!(
                "Received challenge roll input: {:?} for challenge {}",
                input_type,
                challenge_id
            );
            state
                .game.challenge_resolution_service
                .handle_roll_input(client_id.to_string(), challenge_id, to_service_dice_input(input_type))
                .await
                .and_then(value_to_server_message)
        }

        ClientMessage::TriggerChallenge {
            challenge_id,
            target_character_id,
        } => {
            state
                .game.challenge_resolution_service
                .handle_trigger(client_id.to_string(), challenge_id, target_character_id)
                .await
                .and_then(value_to_server_message)
        }

        ClientMessage::ChallengeSuggestionDecision {
            request_id,
            approved,
            modified_difficulty,
        } => state
            .game.challenge_resolution_service
            .handle_suggestion_decision(client_id.to_string(), request_id, approved, modified_difficulty)
            .await
            .and_then(value_to_server_message),

        ClientMessage::NarrativeEventSuggestionDecision {
            request_id,
            event_id,
            approved,
            selected_outcome,
        } => state
            .game.narrative_event_approval_service
            .handle_decision(
                client_id.to_string(),
                request_id,
                event_id,
                approved,
                selected_outcome,
            )
            .await
            .and_then(value_to_server_message),

        ClientMessage::RegenerateOutcome {
            request_id,
            outcome_type,
            guidance,
        } => {
            tracing::debug!(
                "DM requested outcome regeneration for request {} outcome {:?}",
                request_id,
                outcome_type
            );

            // Best-effort: look up the approval item for context
            let maybe_approval = state
                .queues.dm_approval_queue_service
                .get_by_id(&request_id)
                .await
                .ok()
                .flatten();

            let base_flavor = if let Some(item) = maybe_approval {
                format!(
                    "{} (regenerated)",
                    item.payload.proposed_dialogue.trim()
                )
            } else {
                "Regenerated outcome (no approval context found)".to_string()
            };

            let flavor_text = if let Some(g) = guidance {
                if g.trim().is_empty() {
                    base_flavor
                } else {
                    format!("{} — Guidance: {}", base_flavor, g.trim())
                }
            } else {
                base_flavor
            };

            let outcome_type_str = outcome_type.unwrap_or_else(|| "all".to_string());

            Some(ServerMessage::OutcomeRegenerated {
                request_id,
                outcome_type: outcome_type_str,
                new_outcome: crate::infrastructure::websocket::messages::OutcomeDetailData {
                    flavor_text,
                    scene_direction: "DM: narrate this regenerated outcome to the table."
                        .to_string(),
                    proposed_tools: Vec::new(),
                },
            })
        }

        ClientMessage::DiscardChallenge {
            request_id,
            feedback,
        } => {
            tracing::info!(
                "DM discarded challenge for request {}, feedback: {:?}",
                request_id,
                feedback
            );
            // Remove the challenge suggestion from the approval queue
            // The approval will be re-queued for a non-challenge response
            state
                .queues.dm_approval_queue_service
                .discard_challenge(&client_id.to_string(), &request_id)
                .await;
            Some(ServerMessage::ChallengeDiscarded { request_id })
        }

        ClientMessage::CreateAdHocChallenge {
            challenge_name,
            skill_name,
            difficulty,
            target_pc_id,
            outcomes,
        } => {
            tracing::info!(
                "DM creating ad-hoc challenge '{}' for PC {}",
                challenge_name,
                target_pc_id
            );
            state
                .game.challenge_resolution_service
                .handle_adhoc_challenge(
                    client_id.to_string(),
                    challenge_name,
                    skill_name,
                    difficulty,
                    target_pc_id,
                    to_adhoc_outcomes_dto(outcomes),
                )
                .await
                .and_then(value_to_server_message)
        }

        // =====================================================================
        // Challenge Outcome Approval (P3.3)
        // =====================================================================
        ClientMessage::ChallengeOutcomeDecision {
            resolution_id,
            decision,
        } => {
            tracing::info!(
                "DM decision on challenge outcome {}: {:?}",
                resolution_id,
                decision
            );

            // Only DMs should approve - check via async port
            let client_id_str = client_id.to_string();
            if !state.async_session_port.is_client_dm(&client_id_str).await {
                return Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can approve challenge outcomes".to_string(),
                });
            }

            let session_id = match state.async_session_port.get_client_session(&client_id_str).await {
                Some(sid) => sid,
                None => {
                    return Some(ServerMessage::Error {
                        code: "NO_SESSION".to_string(),
                        message: "Client is not in a session".to_string(),
                    });
                }
            };

            // Convert wire decision to service decision
            let svc_decision = to_challenge_outcome_decision(decision);

            // Process the decision via the approval service
            match state.game.challenge_outcome_approval_service
                .process_decision(session_id, &resolution_id, svc_decision)
                .await
            {
                Ok(()) => {
                    // Success - resolution broadcast is handled by the service
                    None
                }
                Err(e) => {
                    tracing::error!("Failed to process challenge outcome decision: {}", e);
                    Some(ServerMessage::Error {
                        code: "APPROVAL_ERROR".to_string(),
                        message: format!("Failed to process decision: {}", e),
                    })
                }
            }
        }

        ClientMessage::RequestOutcomeSuggestion {
            resolution_id,
            guidance,
        } => {
            tracing::info!(
                "DM requesting outcome suggestion for {}: {:?}",
                resolution_id,
                guidance
            );

            // Only DMs should request suggestions
            let client_id_str = client_id.to_string();
            if !state.async_session_port.is_client_dm(&client_id_str).await {
                return Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can request outcome suggestions".to_string(),
                });
            }

            let session_id = match state.async_session_port.get_client_session(&client_id_str).await {
                Some(sid) => sid,
                None => {
                    return Some(ServerMessage::Error {
                        code: "NO_SESSION".to_string(),
                        message: "Client is not in a session".to_string(),
                    });
                }
            };

            // Process as a Suggest decision - the service will handle LLM generation
            let svc_decision = ChallengeOutcomeDecision::Suggest { guidance };

            match state.game.challenge_outcome_approval_service
                .process_decision(session_id, &resolution_id, svc_decision)
                .await
            {
                Ok(()) => {
                    // Success - the service will send OutcomeSuggestionReady when LLM completes
                    None
                }
                Err(e) => {
                    tracing::error!("Failed to request outcome suggestions: {}", e);
                    Some(ServerMessage::Error {
                        code: "SUGGESTION_ERROR".to_string(),
                        message: format!("Failed to request suggestions: {}", e),
                    })
                }
            }
        }

        ClientMessage::RequestOutcomeBranches {
            resolution_id,
            guidance,
        } => {
            tracing::info!(
                "DM requesting outcome branches for {}: {:?}",
                resolution_id,
                guidance
            );

            // Only DMs should request branches
            let client_id_str = client_id.to_string();
            if !state.async_session_port.is_client_dm(&client_id_str).await {
                return Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can request outcome branches".to_string(),
                });
            }

            let session_id = match state.async_session_port.get_client_session(&client_id_str).await {
                Some(sid) => sid,
                None => {
                    return Some(ServerMessage::Error {
                        code: "NO_SESSION".to_string(),
                        message: "Client is not in a session".to_string(),
                    });
                }
            };

            // Request branches via the approval service
            match state.game.challenge_outcome_approval_service
                .request_branches(session_id, &resolution_id, guidance)
                .await
            {
                Ok(()) => {
                    // Success - the service will send OutcomeBranchesReady when LLM completes
                    None
                }
                Err(e) => {
                    tracing::error!("Failed to request outcome branches: {}", e);
                    Some(ServerMessage::Error {
                        code: "BRANCH_ERROR".to_string(),
                        message: format!("Failed to request branches: {}", e),
                    })
                }
            }
        }

        ClientMessage::SelectOutcomeBranch {
            resolution_id,
            branch_id,
            modified_description,
        } => {
            tracing::info!(
                "DM selecting branch {} for resolution {}",
                branch_id,
                resolution_id
            );

            // Only DMs should select branches
            let client_id_str = client_id.to_string();
            if !state.async_session_port.is_client_dm(&client_id_str).await {
                return Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can select outcome branches".to_string(),
                });
            }

            let session_id = match state.async_session_port.get_client_session(&client_id_str).await {
                Some(sid) => sid,
                None => {
                    return Some(ServerMessage::Error {
                        code: "NO_SESSION".to_string(),
                        message: "Client is not in a session".to_string(),
                    });
                }
            };

            // Process branch selection via the approval service
            match state.game.challenge_outcome_approval_service
                .select_branch(session_id, &resolution_id, &branch_id, modified_description)
                .await
            {
                Ok(()) => {
                    // Success - challenge is resolved
                    None
                }
                Err(e) => {
                    tracing::error!("Failed to select outcome branch: {}", e);
                    Some(ServerMessage::Error {
                        code: "BRANCH_SELECT_ERROR".to_string(),
                        message: format!("Failed to select branch: {}", e),
                    })
                }
            }
        }

        // =========================================================================
        // Phase 23D: NPC Location Sharing (HeardAbout observations)
        // =========================================================================

        ClientMessage::ShareNpcLocation {
            pc_id,
            npc_id,
            location_id,
            region_id,
            notes,
        } => {
            tracing::info!(
                "DM sharing NPC {} location with PC {}",
                npc_id,
                pc_id
            );

            // Only DMs can share NPC locations
            let client_id_str = client_id.to_string();
            if !state.async_session_port.is_client_dm(&client_id_str).await {
                return Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can share NPC locations".to_string(),
                });
            }

            let session_id = match state.async_session_port.get_client_session(&client_id_str).await {
                Some(sid) => sid,
                None => {
                    return Some(ServerMessage::Error {
                        code: "NO_SESSION".to_string(),
                        message: "Client is not in a session".to_string(),
                    });
                }
            };

            // Parse IDs
            let pc_uuid = match uuid::Uuid::parse_str(&pc_id) {
                Ok(uuid) => crate::domain::value_objects::PlayerCharacterId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_PC_ID".to_string(),
                        message: "Invalid PC ID format".to_string(),
                    });
                }
            };
            let npc_uuid = match uuid::Uuid::parse_str(&npc_id) {
                Ok(uuid) => crate::domain::value_objects::CharacterId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_NPC_ID".to_string(),
                        message: "Invalid NPC ID format".to_string(),
                    });
                }
            };
            let location_uuid = match uuid::Uuid::parse_str(&location_id) {
                Ok(uuid) => crate::domain::value_objects::LocationId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_LOCATION_ID".to_string(),
                        message: "Invalid location ID format".to_string(),
                    });
                }
            };
            let region_uuid = match uuid::Uuid::parse_str(&region_id) {
                Ok(uuid) => crate::domain::value_objects::RegionId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_REGION_ID".to_string(),
                        message: "Invalid region ID format".to_string(),
                    });
                }
            };

            // Get game time from session
            let game_time = {
                let sessions = state.sessions.read().await;
                match sessions.get_session(session_id) {
                    Some(session) => session.game_time().current(),
                    None => {
                        return Some(ServerMessage::Error {
                            code: "SESSION_NOT_FOUND".to_string(),
                            message: "Session not found".to_string(),
                        });
                    }
                }
            };

            // Create HeardAbout observation
            let observation = crate::domain::entities::NpcObservation::heard_about(
                pc_uuid,
                npc_uuid,
                location_uuid,
                region_uuid,
                game_time,
                notes,
            );

            // Store the observation
            match state.repository.observations().upsert(&observation).await {
                Ok(()) => {
                    tracing::info!(
                        "Created HeardAbout observation: PC {} now knows NPC {} was at region {}",
                        pc_id,
                        npc_id,
                        region_id
                    );
                    // Could broadcast to the player here if we had their client ID
                    None
                }
                Err(e) => {
                    tracing::error!("Failed to create observation: {}", e);
                    Some(ServerMessage::Error {
                        code: "OBSERVATION_ERROR".to_string(),
                        message: format!("Failed to record observation: {}", e),
                    })
                }
            }
        }

        // =========================================================================
        // Phase 23E: DM Event System
        // =========================================================================

        ClientMessage::TriggerApproachEvent {
            npc_id,
            target_pc_id,
            description,
        } => {
            tracing::info!(
                "DM triggering approach event: NPC {} approaching PC {}",
                npc_id,
                target_pc_id
            );

            // Only DMs can trigger approach events
            let client_id_str = client_id.to_string();
            if !state.async_session_port.is_client_dm(&client_id_str).await {
                return Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can trigger approach events".to_string(),
                });
            }

            let session_id = match state.async_session_port.get_client_session(&client_id_str).await {
                Some(sid) => sid,
                None => {
                    return Some(ServerMessage::Error {
                        code: "NO_SESSION".to_string(),
                        message: "Client is not in a session".to_string(),
                    });
                }
            };

            // Parse NPC ID and get NPC details
            let npc_uuid = match uuid::Uuid::parse_str(&npc_id) {
                Ok(uuid) => crate::domain::value_objects::CharacterId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_NPC_ID".to_string(),
                        message: "Invalid NPC ID format".to_string(),
                    });
                }
            };

            // Get NPC details
            let npc = match state.repository.characters().get(npc_uuid).await {
                Ok(Some(npc)) => npc,
                Ok(None) => {
                    return Some(ServerMessage::Error {
                        code: "NPC_NOT_FOUND".to_string(),
                        message: "NPC not found".to_string(),
                    });
                }
                Err(e) => {
                    return Some(ServerMessage::Error {
                        code: "DATABASE_ERROR".to_string(),
                        message: format!("Failed to fetch NPC: {}", e),
                    });
                }
            };

            // Parse PC ID and get PC details (for region)
            let pc_uuid = match uuid::Uuid::parse_str(&target_pc_id) {
                Ok(uuid) => crate::domain::value_objects::PlayerCharacterId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_PC_ID".to_string(),
                        message: "Invalid PC ID format".to_string(),
                    });
                }
            };

            let pc = match state.repository.player_characters().get(pc_uuid).await {
                Ok(Some(pc)) => pc,
                Ok(None) => {
                    return Some(ServerMessage::Error {
                        code: "PC_NOT_FOUND".to_string(),
                        message: "Player character not found".to_string(),
                    });
                }
                Err(e) => {
                    return Some(ServerMessage::Error {
                        code: "DATABASE_ERROR".to_string(),
                        message: format!("Failed to fetch PC: {}", e),
                    });
                }
            };

            // Create Direct observation for the PC (they now see the NPC)
            if let Some(region_id) = pc.current_region_id {
                let game_time = {
                    let sessions = state.sessions.read().await;
                    sessions.get_session(session_id)
                        .map(|s| s.game_time().current())
                        .unwrap_or_else(chrono::Utc::now)
                };

                let observation = crate::domain::entities::NpcObservation::direct(
                    pc_uuid,
                    npc_uuid,
                    pc.current_location_id,
                    region_id,
                    game_time,
                );

                if let Err(e) = state.repository.observations().upsert(&observation).await {
                    tracing::warn!("Failed to create observation for approach event: {}", e);
                }
            }

            // Build the ApproachEvent message
            let approach_event = ServerMessage::ApproachEvent {
                npc_id: npc_id.clone(),
                npc_name: npc.name.clone(),
                npc_sprite: npc.sprite_asset.clone(),
                description,
            };

            // Broadcast to the target PC (via session broadcast)
            // For now, broadcast to all in session - client filters by PC
            if let Ok(msg_json) = serde_json::to_value(&approach_event) {
                // Use broadcast_except with empty string to broadcast to all
                let _ = state.async_session_port.broadcast_except(session_id, msg_json, "").await;
            }

            tracing::info!(
                "Approach event triggered: {} approached by {}",
                target_pc_id,
                npc.name
            );
            None
        }

        ClientMessage::TriggerLocationEvent {
            region_id,
            description,
        } => {
            tracing::info!(
                "DM triggering location event in region {}",
                region_id
            );

            // Only DMs can trigger location events
            let client_id_str = client_id.to_string();
            if !state.async_session_port.is_client_dm(&client_id_str).await {
                return Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can trigger location events".to_string(),
                });
            }

            let session_id = match state.async_session_port.get_client_session(&client_id_str).await {
                Some(sid) => sid,
                None => {
                    return Some(ServerMessage::Error {
                        code: "NO_SESSION".to_string(),
                        message: "Client is not in a session".to_string(),
                    });
                }
            };

            // Build the LocationEvent message
            let location_event = ServerMessage::LocationEvent {
                region_id: region_id.clone(),
                description,
            };

            // Broadcast to all in session - clients filter by their current region
            if let Ok(msg_json) = serde_json::to_value(&location_event) {
                // Use broadcast_except with empty string to broadcast to all
                let _ = state.async_session_port.broadcast_except(session_id, msg_json, "").await;
            }

            tracing::info!("Location event triggered in region {}", region_id);
            None
        }

        // =========================================================================
        // Phase 23F: Game Time Control
        // =========================================================================

        ClientMessage::AdvanceGameTime { hours } => {
            tracing::info!("DM advancing game time by {} hours", hours);

            // Only DMs can advance time
            let client_id_str = client_id.to_string();
            if !state.async_session_port.is_client_dm(&client_id_str).await {
                return Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can advance game time".to_string(),
                });
            }

            let session_id = match state.async_session_port.get_client_session(&client_id_str).await {
                Some(sid) => sid,
                None => {
                    return Some(ServerMessage::Error {
                        code: "NO_SESSION".to_string(),
                        message: "Client is not in a session".to_string(),
                    });
                }
            };

            // Advance game time in session
            let game_time_info = {
                let mut sessions = state.sessions.write().await;
                match sessions.get_session_mut(session_id) {
                    Some(session) => {
                        session.advance_time_hours(hours);
                        (
                            session.display_game_time(),
                            session.time_of_day().to_string(),
                            session.is_time_paused(),
                        )
                    }
                    None => {
                        return Some(ServerMessage::Error {
                            code: "SESSION_NOT_FOUND".to_string(),
                            message: "Session not found".to_string(),
                        });
                    }
                }
            };

            // Build the GameTimeUpdated message
            let time_updated = ServerMessage::GameTimeUpdated {
                display: game_time_info.0,
                time_of_day: game_time_info.1,
                is_paused: game_time_info.2,
            };

            // Broadcast to all in session
            if let Ok(msg_json) = serde_json::to_value(&time_updated) {
                // Use broadcast_except with empty string to broadcast to all
                let _ = state.async_session_port.broadcast_except(session_id, msg_json, "").await;
            }

            tracing::info!("Game time advanced by {} hours", hours);
            None
        }

        // =========================================================================
        // Phase 23C: Navigation
        // =========================================================================

        ClientMessage::SelectPlayerCharacter { pc_id } => {
            tracing::info!("Player selecting PC {}", pc_id);

            let client_id_str = client_id.to_string();
            let session_id = match state.async_session_port.get_client_session(&client_id_str).await {
                Some(sid) => sid,
                None => {
                    return Some(ServerMessage::Error {
                        code: "NO_SESSION".to_string(),
                        message: "Client is not in a session".to_string(),
                    });
                }
            };

            // Parse PC ID
            let pc_uuid = match uuid::Uuid::parse_str(&pc_id) {
                Ok(uuid) => crate::domain::value_objects::PlayerCharacterId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_PC_ID".to_string(),
                        message: "Invalid PC ID format".to_string(),
                    });
                }
            };

            // Get PC details
            let pc = match state.repository.player_characters().get(pc_uuid).await {
                Ok(Some(pc)) => pc,
                Ok(None) => {
                    return Some(ServerMessage::Error {
                        code: "PC_NOT_FOUND".to_string(),
                        message: "Player character not found".to_string(),
                    });
                }
                Err(e) => {
                    return Some(ServerMessage::Error {
                        code: "DATABASE_ERROR".to_string(),
                        message: format!("Failed to fetch PC: {}", e),
                    });
                }
            };

            // Send PcSelected response
            Some(ServerMessage::PcSelected {
                pc_id: pc_id.clone(),
                pc_name: pc.name.clone(),
                location_id: pc.current_location_id.to_string(),
                region_id: pc.current_region_id.map(|r| r.to_string()),
            })
        }

        ClientMessage::MoveToRegion { pc_id, region_id } => {
            tracing::info!("PC {} moving to region {}", pc_id, region_id);

            let client_id_str = client_id.to_string();
            let session_id = match state.async_session_port.get_client_session(&client_id_str).await {
                Some(sid) => sid,
                None => {
                    return Some(ServerMessage::Error {
                        code: "NO_SESSION".to_string(),
                        message: "Client is not in a session".to_string(),
                    });
                }
            };

            // Parse IDs
            let pc_uuid = match uuid::Uuid::parse_str(&pc_id) {
                Ok(uuid) => crate::domain::value_objects::PlayerCharacterId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_PC_ID".to_string(),
                        message: "Invalid PC ID format".to_string(),
                    });
                }
            };
            let region_uuid = match uuid::Uuid::parse_str(&region_id) {
                Ok(uuid) => crate::domain::value_objects::RegionId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_REGION_ID".to_string(),
                        message: "Invalid region ID format".to_string(),
                    });
                }
            };

            // Get current PC to check if movement is valid
            let pc = match state.repository.player_characters().get(pc_uuid).await {
                Ok(Some(pc)) => pc,
                Ok(None) => {
                    return Some(ServerMessage::Error {
                        code: "PC_NOT_FOUND".to_string(),
                        message: "Player character not found".to_string(),
                    });
                }
                Err(e) => {
                    return Some(ServerMessage::Error {
                        code: "DATABASE_ERROR".to_string(),
                        message: format!("Failed to fetch PC: {}", e),
                    });
                }
            };

            // Check if movement is allowed (check for locked connections)
            if let Some(current_region_id) = pc.current_region_id {
                let connections = match state.repository.regions().get_connections(current_region_id).await {
                    Ok(conns) => conns,
                    Err(e) => {
                        return Some(ServerMessage::Error {
                            code: "DATABASE_ERROR".to_string(),
                            message: format!("Failed to fetch connections: {}", e),
                        });
                    }
                };

                // Find the connection to target region
                if let Some(conn) = connections.iter().find(|c| c.to_region == region_uuid) {
                    if conn.is_locked {
                        return Some(ServerMessage::MovementBlocked {
                            pc_id: pc_id.clone(),
                            reason: conn.lock_description.clone().unwrap_or_else(|| "The path is locked.".to_string()),
                        });
                    }
                }
            }

            // Get target region
            let target_region = match state.repository.regions().get(region_uuid).await {
                Ok(Some(region)) => region,
                Ok(None) => {
                    return Some(ServerMessage::Error {
                        code: "REGION_NOT_FOUND".to_string(),
                        message: "Target region not found".to_string(),
                    });
                }
                Err(e) => {
                    return Some(ServerMessage::Error {
                        code: "DATABASE_ERROR".to_string(),
                        message: format!("Failed to fetch region: {}", e),
                    });
                }
            };

            // Update PC position
            if let Err(e) = state.repository.player_characters()
                .update_position(pc_uuid, target_region.location_id, Some(region_uuid))
                .await
            {
                return Some(ServerMessage::Error {
                    code: "UPDATE_ERROR".to_string(),
                    message: format!("Failed to update position: {}", e),
                });
            }

            // Get location for name
            let location = state.repository.locations().get(target_region.location_id).await
                .ok().flatten();
            let location_name = location.as_ref().map(|l| l.name.clone()).unwrap_or_default();
            let backdrop = target_region.backdrop_asset.clone()
                .or_else(|| location.as_ref().and_then(|l| l.backdrop_asset.clone()));

            // Get NPCs present (simple rule-based)
            let game_time = {
                let sessions = state.sessions.read().await;
                sessions.get_session(session_id)
                    .map(|s| s.game_time().clone())
                    .unwrap_or_default()
            };

            let npc_relationships = state.repository.characters()
                .get_npcs_related_to_region(region_uuid)
                .await
                .unwrap_or_default();

            let time_of_day = game_time.time_of_day();
            let npcs_present: Vec<messages::NpcPresenceData> = npc_relationships
                .into_iter()
                .filter_map(|(character, rel_type)| {
                    let is_present = rel_type.is_npc_present(time_of_day);
                    if is_present {
                        Some(messages::NpcPresenceData {
                            character_id: character.id.to_string(),
                            name: character.name,
                            sprite_asset: character.sprite_asset,
                            portrait_asset: character.portrait_asset,
                        })
                    } else {
                        None
                    }
                })
                .collect();

            // Get navigation options
            let connections = state.repository.regions().get_connections(region_uuid).await.unwrap_or_default();
            let exits = state.repository.regions().get_exits(region_uuid).await.unwrap_or_default();

            let mut connected_regions = Vec::new();
            for conn in connections {
                if let Ok(Some(target)) = state.repository.regions().get(conn.to_region).await {
                    connected_regions.push(messages::NavigationTarget {
                        region_id: conn.to_region.to_string(),
                        name: target.name,
                        is_locked: conn.is_locked,
                        lock_description: conn.lock_description,
                    });
                }
            }

            let mut exit_targets = Vec::new();
            for exit in exits {
                if let Ok(Some(target_loc)) = state.repository.locations().get(exit.to_location).await {
                    exit_targets.push(messages::NavigationExit {
                        location_id: exit.to_location.to_string(),
                        location_name: target_loc.name,
                        arrival_region_id: exit.arrival_region_id.to_string(),
                        description: exit.description,
                    });
                }
            }

            Some(ServerMessage::SceneChanged {
                pc_id,
                region: messages::RegionData {
                    id: region_uuid.to_string(),
                    name: target_region.name,
                    location_id: target_region.location_id.to_string(),
                    location_name,
                    backdrop_asset: backdrop,
                    atmosphere: target_region.atmosphere,
                },
                npcs_present,
                navigation: messages::NavigationData {
                    connected_regions,
                    exits: exit_targets,
                },
            })
        }

        ClientMessage::ExitToLocation { pc_id, location_id, arrival_region_id } => {
            tracing::info!("PC {} exiting to location {}", pc_id, location_id);

            let client_id_str = client_id.to_string();
            let session_id = match state.async_session_port.get_client_session(&client_id_str).await {
                Some(sid) => sid,
                None => {
                    return Some(ServerMessage::Error {
                        code: "NO_SESSION".to_string(),
                        message: "Client is not in a session".to_string(),
                    });
                }
            };

            // Parse IDs
            let pc_uuid = match uuid::Uuid::parse_str(&pc_id) {
                Ok(uuid) => crate::domain::value_objects::PlayerCharacterId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_PC_ID".to_string(),
                        message: "Invalid PC ID format".to_string(),
                    });
                }
            };
            let location_uuid = match uuid::Uuid::parse_str(&location_id) {
                Ok(uuid) => crate::domain::value_objects::LocationId::from_uuid(uuid),
                Err(_) => {
                    return Some(ServerMessage::Error {
                        code: "INVALID_LOCATION_ID".to_string(),
                        message: "Invalid location ID format".to_string(),
                    });
                }
            };

            // Get target location
            let target_location = match state.repository.locations().get(location_uuid).await {
                Ok(Some(loc)) => loc,
                Ok(None) => {
                    return Some(ServerMessage::Error {
                        code: "LOCATION_NOT_FOUND".to_string(),
                        message: "Target location not found".to_string(),
                    });
                }
                Err(e) => {
                    return Some(ServerMessage::Error {
                        code: "DATABASE_ERROR".to_string(),
                        message: format!("Failed to fetch location: {}", e),
                    });
                }
            };

            // Determine arrival region
            let arrival_region_uuid = if let Some(arrival_id) = arrival_region_id {
                match uuid::Uuid::parse_str(&arrival_id) {
                    Ok(uuid) => crate::domain::value_objects::RegionId::from_uuid(uuid),
                    Err(_) => {
                        return Some(ServerMessage::Error {
                            code: "INVALID_REGION_ID".to_string(),
                            message: "Invalid arrival region ID format".to_string(),
                        });
                    }
                }
            } else if let Some(default_region) = target_location.default_region_id {
                default_region
            } else {
                // Find first spawn point in location
                let regions = state.repository.regions().list_by_location(location_uuid).await.unwrap_or_default();
                match regions.into_iter().find(|r| r.is_spawn_point).map(|r| r.id) {
                    Some(id) => id,
                    None => {
                        return Some(ServerMessage::Error {
                            code: "NO_ARRIVAL_REGION".to_string(),
                            message: "No arrival region specified and location has no default or spawn point".to_string(),
                        });
                    }
                }
            };

            // Get arrival region
            let arrival_region = match state.repository.regions().get(arrival_region_uuid).await {
                Ok(Some(region)) => region,
                Ok(None) => {
                    return Some(ServerMessage::Error {
                        code: "REGION_NOT_FOUND".to_string(),
                        message: "Arrival region not found".to_string(),
                    });
                }
                Err(e) => {
                    return Some(ServerMessage::Error {
                        code: "DATABASE_ERROR".to_string(),
                        message: format!("Failed to fetch region: {}", e),
                    });
                }
            };

            // Update PC position
            if let Err(e) = state.repository.player_characters()
                .update_position(pc_uuid, location_uuid, Some(arrival_region_uuid))
                .await
            {
                return Some(ServerMessage::Error {
                    code: "UPDATE_ERROR".to_string(),
                    message: format!("Failed to update position: {}", e),
                });
            }

            // Get backdrop
            let backdrop = arrival_region.backdrop_asset.clone()
                .or_else(|| target_location.backdrop_asset.clone());

            // Get NPCs present
            let game_time = {
                let sessions = state.sessions.read().await;
                sessions.get_session(session_id)
                    .map(|s| s.game_time().clone())
                    .unwrap_or_default()
            };

            let npc_relationships = state.repository.characters()
                .get_npcs_related_to_region(arrival_region_uuid)
                .await
                .unwrap_or_default();

            let time_of_day = game_time.time_of_day();
            let npcs_present: Vec<messages::NpcPresenceData> = npc_relationships
                .into_iter()
                .filter_map(|(character, rel_type)| {
                    let is_present = rel_type.is_npc_present(time_of_day);
                    if is_present {
                        Some(messages::NpcPresenceData {
                            character_id: character.id.to_string(),
                            name: character.name,
                            sprite_asset: character.sprite_asset,
                            portrait_asset: character.portrait_asset,
                        })
                    } else {
                        None
                    }
                })
                .collect();

            // Get navigation options
            let connections = state.repository.regions().get_connections(arrival_region_uuid).await.unwrap_or_default();
            let exits = state.repository.regions().get_exits(arrival_region_uuid).await.unwrap_or_default();

            let mut connected_regions = Vec::new();
            for conn in connections {
                if let Ok(Some(target)) = state.repository.regions().get(conn.to_region).await {
                    connected_regions.push(messages::NavigationTarget {
                        region_id: conn.to_region.to_string(),
                        name: target.name,
                        is_locked: conn.is_locked,
                        lock_description: conn.lock_description,
                    });
                }
            }

            let mut exit_targets = Vec::new();
            for exit in exits {
                if let Ok(Some(target_loc)) = state.repository.locations().get(exit.to_location).await {
                    exit_targets.push(messages::NavigationExit {
                        location_id: exit.to_location.to_string(),
                        location_name: target_loc.name,
                        arrival_region_id: exit.arrival_region_id.to_string(),
                        description: exit.description,
                    });
                }
            }

            Some(ServerMessage::SceneChanged {
                pc_id,
                region: messages::RegionData {
                    id: arrival_region_uuid.to_string(),
                    name: arrival_region.name,
                    location_id: location_uuid.to_string(),
                    location_name: target_location.name,
                    backdrop_asset: backdrop,
                    atmosphere: arrival_region.atmosphere,
                },
                npcs_present,
                navigation: messages::NavigationData {
                    connected_regions,
                    exits: exit_targets,
                },
            })
        }
    }
}

// Re-export message types from the dedicated messages module
pub mod messages;
pub use messages::{
    CharacterData, CharacterPosition, ClientMessage, InteractionData,
    ParticipantInfo, ParticipantRole, SceneData, ServerMessage,
};
