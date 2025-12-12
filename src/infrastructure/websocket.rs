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
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::domain::value_objects::{ActionId, SessionId, WorldId};
use crate::infrastructure::export::{load_world_snapshot, PlayerWorldSnapshot};
use crate::infrastructure::session::{ClientId, SessionError, WorldSnapshot};
use crate::infrastructure::state::AppState;
use crate::application::services::llm_service::{
    GamePromptRequest, PlayerActionContext, SceneContext, CharacterContext,
};

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

/// Process a player action through the LLM and send approval request to DM
async fn process_player_action_with_llm(
    state: &AppState,
    session_id: SessionId,
    action_id: String,
    action_type: String,
    target: Option<String>,
    dialogue: Option<String>,
) {
    // Acquire session and world snapshot
    let (session_data, npc_name) = {
        let sessions = state.sessions.read().await;
        let Some(session) = sessions.get_session(session_id) else {
            tracing::warn!("Session {} not found for LLM processing", session_id);
            return;
        };

        // Get the current scene from the snapshot
        let world_snapshot = &session.world_snapshot;
        let current_scene = match &world_snapshot.current_scene_id {
            Some(scene_id_str) => {
                world_snapshot
                    .scenes
                    .iter()
                    .find(|s| s.id.to_string() == *scene_id_str)
            }
            None => {
                tracing::warn!("No current scene set in world snapshot");
                world_snapshot.scenes.first()
            }
        };

        let Some(current_scene) = current_scene else {
            tracing::warn!("No scenes available in world snapshot");
            return;
        };

        // Get the location for scene context
        let location = world_snapshot
            .locations
            .iter()
            .find(|l| l.id == current_scene.location_id);

        // Determine the responding character (target NPC or first available character)
        let responding_character = if let Some(target_name) = &target {
            world_snapshot
                .characters
                .iter()
                .find(|c| c.name.eq_ignore_ascii_case(target_name))
        } else {
            // Use first featured character in scene if no target specified
            current_scene
                .featured_characters
                .first()
                .and_then(|char_id| {
                    world_snapshot.characters.iter().find(|c| c.id == *char_id)
                })
        };

        let Some(responding_character) = responding_character else {
            tracing::warn!(
                "No responding character found for action in scene {}",
                current_scene.id
            );
            return;
        };

        // Build the scene context
        let scene_context = SceneContext {
            scene_name: current_scene.name.clone(),
            location_name: location.map(|l| l.name.clone()).unwrap_or_else(|| "Unknown".to_string()),
            time_context: match &current_scene.time_context {
                crate::domain::entities::TimeContext::Unspecified => "Unspecified".to_string(),
                crate::domain::entities::TimeContext::TimeOfDay(tod) => format!("{:?}", tod),
                crate::domain::entities::TimeContext::During(s) => s.clone(),
                crate::domain::entities::TimeContext::Custom(s) => s.clone(),
            },
            present_characters: current_scene
                .featured_characters
                .iter()
                .filter_map(|char_id| {
                    world_snapshot
                        .characters
                        .iter()
                        .find(|c| c.id == *char_id)
                        .map(|c| c.name.clone())
                })
                .collect(),
        };

        // Build the character context
        let character_context = CharacterContext {
            name: responding_character.name.clone(),
            archetype: format!("{:?}", responding_character.current_archetype),
            current_mood: None, // TODO: Load from character state/session storage
            wants: responding_character
                .wants
                .iter()
                .map(|w| format!("{:?}", w))
                .collect(),
            relationship_to_player: None, // TODO: Load from relationship tracking
        };

        // Build the directorial notes (empty for now; would come from DM updates)
        let directorial_notes = current_scene.directorial_notes.clone();

        // Build the game prompt request with empty active challenges for now
        // TODO: Fetch active challenges from the repository when available
        let prompt_request = GamePromptRequest {
            player_action: PlayerActionContext {
                action_type: action_type.clone(),
                target: target.clone(),
                dialogue: dialogue.clone(),
            },
            scene_context,
            directorial_notes,
            conversation_history: vec![], // TODO: Load from conversation history
            responding_character: character_context.clone(),
            active_challenges: vec![], // TODO: Fetch from challenge repository
        };

        (prompt_request, character_context.name.clone())
    }; // Release the read lock here

    // Call the LLM service
    let llm_service = crate::application::services::LLMService::new(state.llm_client.clone());
    match llm_service.generate_npc_response(session_data).await {
        Ok(response) => {
            // Convert proposed tool calls to ProposedTool format for the message
            let proposed_tools: Vec<ProposedTool> = response
                .proposed_tool_calls
                .iter()
                .map(|tool| ProposedTool {
                    id: format!("{}_{}", tool.tool_name, uuid::Uuid::new_v4()),
                    name: tool.tool_name.clone(),
                    description: tool.description.clone(),
                    arguments: tool.arguments.clone(),
                })
                .collect();

            // Convert challenge suggestion if present
            let challenge_suggestion = response.challenge_suggestion.as_ref().map(|cs| {
                ChallengeSuggestionInfo {
                    challenge_id: cs.challenge_id.clone(),
                    challenge_name: String::new(), // Will be enriched from challenge data
                    skill_name: String::new(),
                    difficulty_display: String::new(),
                    confidence: format!("{:?}", cs.confidence).to_lowercase(),
                    reasoning: cs.reasoning.clone(),
                }
            });

            // Send ApprovalRequired message to DM
            let approval_msg = ServerMessage::ApprovalRequired {
                request_id: action_id.clone(),
                npc_name: npc_name.clone(),
                proposed_dialogue: response.npc_dialogue.clone(),
                internal_reasoning: response.internal_reasoning.clone(),
                proposed_tools: proposed_tools.clone(),
                challenge_suggestion,
            };

            // Store pending approval and send to DM
            let mut sessions = state.sessions.write().await;
            if let Some(session) = sessions.get_session_mut(session_id) {
                // Create and store the pending approval
                let pending = crate::infrastructure::session::PendingApproval::new(
                    action_id.clone(),
                    npc_name,
                    response.npc_dialogue,
                    response.internal_reasoning,
                    proposed_tools,
                );
                session.add_pending_approval(pending);

                // Send approval request to DM
                session.send_to_dm(&approval_msg);
                tracing::info!(
                    "Sent ApprovalRequired for action {} to DM",
                    action_id
                );
            }
        }
        Err(e) => {
            tracing::error!(
                "LLM processing failed for action {}: {:?}",
                action_id,
                e
            );

            // Send error notification to DM
            let error_msg = ServerMessage::Error {
                code: "LLM_ERROR".to_string(),
                message: format!("Failed to generate NPC response: {}", e),
            };

            let sessions = state.sessions.read().await;
            if let Some(session) = sessions.get_session(session_id) {
                session.send_to_dm(&error_msg);
            }
        }
    }
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

            // Try to join or create a session
            match join_or_create_session(state, client_id, user_id.clone(), role, world_id, sender)
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

            if let Some(session) = sessions.get_session(session_id) {
                // Send ActionReceived acknowledgment to the acting player
                let action_received = ServerMessage::ActionReceived {
                    action_id: action_id_str.clone(),
                    player_id: player_id.clone(),
                    action_type: action_type.clone(),
                };

                // Send acknowledgment back to the player (will be returned below)
                // Also broadcast to DM for awareness
                if session.has_dm() {
                    // Send LLM processing notification to DM
                    let processing_msg = ServerMessage::LLMProcessing {
                        action_id: action_id_str.clone(),
                    };
                    session.send_to_dm(&processing_msg);

                    // Broadcast action received to DM as well
                    session.send_to_dm(&action_received);

                    // Process action through LLM and send ApprovalRequired to DM
                    process_player_action_with_llm(
                        &state,
                        session_id,
                        action_id_str.clone(),
                        action_type.clone(),
                        target.clone(),
                        dialogue.clone(),
                    )
                    .await;
                }

                tracing::info!(
                    "Action {} from player {} in session {}: {} -> {:?}",
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

            None // No response from here; responses come from LLM processing or DM approval
        }

        ClientMessage::RequestSceneChange { scene_id } => {
            tracing::debug!("Scene change requested: {}", scene_id);

            // Get the client's session
            let sessions = state.sessions.read().await;
            if let Some(session_id) = sessions.get_client_session(client_id) {
                if let Some(_session) = sessions.get_session(session_id) {
                    // TODO: Load scene from database and broadcast SceneUpdate to all participants
                    // For now, this is a placeholder
                }
            }

            None // Will implement scene loading
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
            let mut sessions = state.sessions.write().await;
            if let Some(session_id) = sessions.get_client_session(client_id) {
                if let Some(session) = sessions.get_session_mut(session_id) {
                    // Verify this client is the DM
                    if let Some(dm) = session.get_dm() {
                        if dm.client_id == client_id {
                            match decision {
                                ApprovalDecision::Accept => {
                                    // Task 1.2.1: Handle Accept - broadcast approved response
                                    if let Some(pending) = session.get_pending_approval(&request_id)
                                    {
                                        let npc_name = pending.npc_name.clone();
                                        let dialogue = pending.proposed_dialogue.clone();
                                        let executed_tools: Vec<String> = pending
                                            .proposed_tools
                                            .iter()
                                            .map(|t| t.id.clone())
                                            .collect();

                                        // Broadcast approved response to all players
                                        let response = ServerMessage::DialogueResponse {
                                            speaker_id: npc_name.clone(),
                                            speaker_name: npc_name.clone(),
                                            text: dialogue.clone(),
                                            choices: vec![],
                                        };
                                        session.broadcast_to_players(&response);

                                        // Store in conversation history
                                        session.add_npc_response(&npc_name, &dialogue);

                                        // Remove from pending approvals
                                        session.remove_pending_approval(&request_id);

                                        tracing::info!(
                                            "Approved NPC response for request {}. Executed tools: {:?}",
                                            request_id,
                                            executed_tools
                                        );
                                        return None;
                                    }
                                }
                                ApprovalDecision::AcceptWithModification {
                                    modified_dialogue,
                                    approved_tools,
                                    rejected_tools: _,
                                } => {
                                    // Task 1.2.2: Handle AcceptWithModification - use modified dialogue
                                    if let Some(pending) = session.get_pending_approval(&request_id)
                                    {
                                        let npc_name = pending.npc_name.clone();

                                        // Filter tools to only approved ones
                                        let filtered_tools: Vec<String> = pending
                                            .proposed_tools
                                            .iter()
                                            .filter(|t| approved_tools.contains(&t.id))
                                            .map(|t| t.id.clone())
                                            .collect();

                                        // Broadcast modified response to all players
                                        let response = ServerMessage::DialogueResponse {
                                            speaker_id: npc_name.clone(),
                                            speaker_name: npc_name.clone(),
                                            text: modified_dialogue.clone(),
                                            choices: vec![],
                                        };
                                        session.broadcast_to_players(&response);

                                        // Store modified dialogue in conversation history
                                        session.add_npc_response(&npc_name, &modified_dialogue);

                                        // Remove from pending approvals
                                        session.remove_pending_approval(&request_id);

                                        tracing::info!(
                                            "Approved modified NPC response for request {}. Approved tools: {:?}",
                                            request_id,
                                            filtered_tools
                                        );
                                        return None;
                                    }
                                }
                                ApprovalDecision::Reject { feedback } => {
                                    // Task 1.2.3: Handle Reject - re-call LLM with feedback
                                    if let Some(pending) =
                                        session.get_pending_approval_mut(&request_id)
                                    {
                                        pending.retry_count += 1;

                                        // Check max retries (limit to 3)
                                        if pending.retry_count >= 3 {
                                            tracing::warn!(
                                                "Max retries (3) exceeded for request {}. Rejecting.",
                                                request_id
                                            );
                                            session.remove_pending_approval(&request_id);
                                            let error_msg = ServerMessage::Error {
                                                code: "APPROVAL_MAX_RETRIES".to_string(),
                                                message: "Maximum approval retries exceeded. Please use TakeOver instead.".to_string(),
                                            };
                                            session.send_to_dm(&error_msg);
                                            return None;
                                        }

                                        tracing::info!(
                                            "Rejection #{} for request {}. Feedback: {}",
                                            pending.retry_count,
                                            request_id,
                                            feedback
                                        );

                                        // Store the rejection feedback for LLM context
                                        // This would be used in the next LLM call
                                        let _rejection_context = format!(
                                            "Previous response was rejected with feedback: {}",
                                            feedback
                                        );

                                        // Send a notification to DM that re-processing is happening
                                        let processing_msg = ServerMessage::LLMProcessing {
                                            action_id: request_id.clone(),
                                        };
                                        session.send_to_dm(&processing_msg);

                                        tracing::info!(
                                            "Queuing LLM reprocessing for request {} with feedback",
                                            request_id
                                        );

                                        // Note: The actual re-call to LLM would happen asynchronously
                                        // For now, we just track that it should be re-processed
                                        // This would be implemented as a separate task in production
                                    }
                                    return None;
                                }
                                ApprovalDecision::TakeOver { dm_response } => {
                                    // Task 1.2.4: Handle TakeOver - use DM's custom response
                                    if let Some(pending) = session.get_pending_approval(&request_id)
                                    {
                                        let npc_name = pending.npc_name.clone();

                                        // Broadcast DM's response as NPC dialogue (no tool calls)
                                        let response = ServerMessage::DialogueResponse {
                                            speaker_id: npc_name.clone(),
                                            speaker_name: npc_name.clone(),
                                            text: dm_response.clone(),
                                            choices: vec![],
                                        };
                                        session.broadcast_to_players(&response);

                                        // Store DM takeover in conversation history
                                        session.add_npc_response(&npc_name, &dm_response);

                                        // Remove from pending approvals
                                        session.remove_pending_approval(&request_id);

                                        tracing::info!(
                                            "DM took over response for request {}",
                                            request_id
                                        );
                                        return None;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            None
        }

        ClientMessage::ChallengeRoll {
            challenge_id,
            roll,
        } => {
            tracing::debug!("Received challenge roll: {} for challenge {}", roll, challenge_id);
            // TODO: Implement challenge roll handling
            // This would validate the roll, look up the challenge, and broadcast results
            None
        }

        ClientMessage::TriggerChallenge {
            challenge_id,
            target_character_id,
        } => {
            tracing::debug!(
                "DM triggering challenge {} for character {}",
                challenge_id,
                target_character_id
            );
            // TODO: Implement manual challenge triggering
            // This would send a ChallengePrompt message to the target character
            None
        }

        ClientMessage::ChallengeSuggestionDecision {
            request_id,
            approved,
            modified_difficulty,
        } => {
            tracing::debug!(
                "Received challenge suggestion decision for {}: approved={}, modified_difficulty={:?}",
                request_id,
                approved,
                modified_difficulty
            );
            // TODO: Implement challenge suggestion approval/rejection
            // This would either trigger the challenge or discard the suggestion
            None
        }
    }
}

/// Information returned when a client successfully joins a session
struct SessionJoinedInfo {
    session_id: SessionId,
    participants: Vec<ParticipantInfo>,
    world_snapshot: serde_json::Value,
}

/// Join an existing session or create a new one
async fn join_or_create_session(
    state: &AppState,
    client_id: ClientId,
    user_id: String,
    role: ParticipantRole,
    world_id: Option<String>,
    sender: mpsc::UnboundedSender<ServerMessage>,
) -> Result<SessionJoinedInfo, SessionError> {
    // Parse the world ID if provided
    let world_id = if let Some(id_str) = world_id {
        let uuid = uuid::Uuid::parse_str(&id_str)
            .map_err(|_| SessionError::WorldNotFound(id_str.clone()))?;
        Some(WorldId::from_uuid(uuid))
    } else {
        None
    };

    let mut sessions = state.sessions.write().await;

    // Try to find an existing session for this world
    if let Some(wid) = world_id {
        if let Some(session_id) = sessions.find_session_for_world(wid) {
            // Join existing session
            let snapshot = sessions.join_session(session_id, client_id, user_id, role, sender)?;

            // Gather participant info
            let participants = gather_participants(&sessions, session_id);

            return Ok(SessionJoinedInfo {
                session_id,
                participants,
                world_snapshot: snapshot.to_json(),
            });
        }

        // Create new session for this world
        drop(sessions); // Release lock for database access

        // Load world data from database using the export module
        let player_snapshot = load_world_snapshot(wid, None, &state.repository)
            .await
            .map_err(SessionError::Database)?;

        // Convert PlayerWorldSnapshot to internal WorldSnapshot for session storage
        let internal_snapshot = convert_to_internal_snapshot(&player_snapshot);

        // Re-acquire lock and create session
        let mut sessions = state.sessions.write().await;
        let session_id = sessions.create_session(wid, internal_snapshot);

        // Join the newly created session
        let snapshot = sessions.join_session(session_id, client_id, user_id, role, sender)?;

        // Gather participant info (just the joining user at this point)
        let participants = gather_participants(&sessions, session_id);

        Ok(SessionJoinedInfo {
            session_id,
            participants,
            world_snapshot: snapshot.to_json(),
        })
    } else {
        // No world specified - create a demo session
        let demo_world = create_demo_world();
        let world_id = demo_world.world.id;
        let session_id = sessions.create_session(world_id, demo_world);
        let snapshot = sessions.join_session(session_id, client_id, user_id, role, sender)?;

        // Gather participant info
        let participants = gather_participants(&sessions, session_id);

        Ok(SessionJoinedInfo {
            session_id,
            participants,
            world_snapshot: snapshot.to_json(),
        })
    }
}

/// Gather participant info from a session
fn gather_participants(
    sessions: &crate::infrastructure::session::SessionManager,
    session_id: SessionId,
) -> Vec<ParticipantInfo> {
    sessions
        .get_session(session_id)
        .map(|session| {
            session
                .participants
                .values()
                .map(|p| ParticipantInfo {
                    user_id: p.user_id.clone(),
                    role: p.role,
                    character_name: None, // TODO: Load from character selection
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Convert PlayerWorldSnapshot to internal WorldSnapshot for session storage
fn convert_to_internal_snapshot(player_snapshot: &PlayerWorldSnapshot) -> WorldSnapshot {
    use crate::domain::entities::{
        Character, Location, LocationType, Scene, StatBlock, TimeContext, World,
    };
    use crate::domain::value_objects::{
        ActId, CampbellArchetype, CharacterId, LocationId, SceneId,
    };

    // Convert world data
    let world_id = uuid::Uuid::parse_str(&player_snapshot.world.id)
        .map(WorldId::from_uuid)
        .unwrap_or_else(|_| WorldId::new());

    use chrono::Utc;
    let now = Utc::now();

    let world = World {
        id: world_id,
        name: player_snapshot.world.name.clone(),
        description: player_snapshot.world.description.clone(),
        rule_system: player_snapshot.world.rule_system.clone(),
        created_at: now,
        updated_at: now,
    };

    // Convert locations
    let locations: Vec<Location> = player_snapshot
        .locations
        .iter()
        .map(|l| {
            let location_id = uuid::Uuid::parse_str(&l.id)
                .map(LocationId::from_uuid)
                .unwrap_or_else(|_| LocationId::new());
            let parent_id = l
                .parent_id
                .as_ref()
                .and_then(|pid| uuid::Uuid::parse_str(pid).map(LocationId::from_uuid).ok());

            Location {
                id: location_id,
                world_id,
                parent_id,
                name: l.name.clone(),
                description: l.description.clone(),
                location_type: LocationType::Interior, // Default to Interior
                backdrop_asset: l.backdrop_asset.clone(),
                grid_map_id: None,
                backdrop_regions: Vec::new(),
            }
        })
        .collect();

    // Convert characters
    let characters: Vec<Character> = player_snapshot
        .characters
        .iter()
        .map(|c| {
            let character_id = uuid::Uuid::parse_str(&c.id)
                .map(CharacterId::from_uuid)
                .unwrap_or_else(|_| CharacterId::new());

            Character {
                id: character_id,
                world_id,
                name: c.name.clone(),
                description: c.description.clone(),
                sprite_asset: c.sprite_asset.clone(),
                portrait_asset: c.portrait_asset.clone(),
                base_archetype: CampbellArchetype::Ally, // Default archetype
                current_archetype: CampbellArchetype::Ally,
                archetype_history: Vec::new(),
                wants: Vec::new(),
                stats: StatBlock::default(),
                inventory: Vec::new(),
                is_alive: c.is_alive,
                is_active: c.is_active,
            }
        })
        .collect();

    // Convert scenes
    let scenes: Vec<Scene> = player_snapshot
        .scenes
        .iter()
        .map(|s| {
            let scene_id = uuid::Uuid::parse_str(&s.id)
                .map(SceneId::from_uuid)
                .unwrap_or_else(|_| SceneId::new());
            let location_id = uuid::Uuid::parse_str(&s.location_id)
                .map(LocationId::from_uuid)
                .unwrap_or_else(|_| LocationId::new());
            let featured_characters: Vec<CharacterId> = s
                .featured_characters
                .iter()
                .filter_map(|cid| uuid::Uuid::parse_str(cid).map(CharacterId::from_uuid).ok())
                .collect();

            Scene {
                id: scene_id,
                act_id: ActId::new(), // Placeholder
                name: s.name.clone(),
                location_id,
                time_context: TimeContext::Unspecified,
                backdrop_override: s.backdrop_override.clone(),
                entry_conditions: Vec::new(),
                featured_characters,
                directorial_notes: s.directorial_notes.clone(),
                order: 0,
            }
        })
        .collect();

    WorldSnapshot {
        world,
        locations,
        characters,
        scenes,
        current_scene_id: player_snapshot.current_scene.as_ref().map(|s| s.id.clone()),
    }
}

/// Create a demo world snapshot for testing
fn create_demo_world() -> WorldSnapshot {
    use crate::domain::entities::World;
    use crate::domain::value_objects::RuleSystemConfig;
    use chrono::Utc;

    let world = World {
        id: WorldId::new(),
        name: "Demo World".to_string(),
        description: "A demonstration world for testing".to_string(),
        rule_system: RuleSystemConfig::default(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    WorldSnapshot {
        world,
        locations: vec![],
        characters: vec![],
        scenes: vec![],
        current_scene_id: None,
    }
}

// ============================================================================
// Message Types (aligned with Player/src/infrastructure/websocket/messages.rs)
// ============================================================================

/// Messages from client (Player) to server (Engine)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// Join a game session
    JoinSession {
        user_id: String,
        role: ParticipantRole,
        /// Optional world ID to join (creates demo session if not provided)
        #[serde(default)]
        world_id: Option<String>,
    },
    /// Player performs an action
    PlayerAction {
        action_type: String,
        target: Option<String>,
        dialogue: Option<String>,
    },
    /// Request to change scene
    RequestSceneChange { scene_id: String },
    /// DM updates directorial context
    DirectorialUpdate { context: DirectorialContext },
    /// DM approves/rejects LLM response
    ApprovalDecision {
        request_id: String,
        decision: ApprovalDecision,
    },
    /// Player submits a challenge roll
    ChallengeRoll {
        challenge_id: String,
        roll: i32,
    },
    /// DM triggers a challenge manually
    TriggerChallenge {
        challenge_id: String,
        target_character_id: String,
    },
    /// DM approves/rejects/modifies a suggested challenge
    ChallengeSuggestionDecision {
        request_id: String,
        approved: bool,
        modified_difficulty: Option<String>,
    },
    /// Heartbeat ping
    Heartbeat,
}

/// Messages from server (Engine) to client (Player)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// Session successfully joined with full details
    SessionJoined {
        session_id: String,
        role: ParticipantRole,
        participants: Vec<ParticipantInfo>,
        world_snapshot: serde_json::Value,
    },
    /// A player joined the session (broadcast to others)
    PlayerJoined {
        user_id: String,
        role: ParticipantRole,
        character_name: Option<String>,
    },
    /// A player left the session (broadcast to others)
    PlayerLeft { user_id: String },
    /// Player action was received and is being processed
    ActionReceived {
        action_id: String,
        player_id: String,
        action_type: String,
    },
    /// Scene update
    SceneUpdate {
        scene: SceneData,
        characters: Vec<CharacterData>,
        interactions: Vec<InteractionData>,
    },
    /// NPC dialogue response
    DialogueResponse {
        speaker_id: String,
        speaker_name: String,
        text: String,
        choices: Vec<DialogueChoice>,
    },
    /// LLM is processing (shown to DM)
    LLMProcessing { action_id: String },
    /// Approval required (sent to DM)
    ApprovalRequired {
        request_id: String,
        npc_name: String,
        proposed_dialogue: String,
        internal_reasoning: String,
        proposed_tools: Vec<ProposedTool>,
        challenge_suggestion: Option<ChallengeSuggestionInfo>,
    },
    /// Response was approved and executed
    ResponseApproved {
        npc_dialogue: String,
        executed_tools: Vec<String>,
    },
    /// Challenge prompt sent to player
    ChallengePrompt {
        challenge_id: String,
        challenge_name: String,
        skill_name: String,
        difficulty_display: String,
        description: String,
        character_modifier: i32,
    },
    /// Challenge result broadcast to all
    ChallengeResolved {
        challenge_id: String,
        challenge_name: String,
        character_name: String,
        roll: i32,
        modifier: i32,
        total: i32,
        outcome: String,
        outcome_description: String,
    },
    /// Error message
    Error { code: String, message: String },
    /// Heartbeat response
    Pong,

    // Generation events (for Creator Mode)
    /// A generation batch has been queued
    GenerationQueued {
        batch_id: String,
        entity_type: String,
        entity_id: String,
        asset_type: String,
        position: u32,
    },
    /// Generation progress update
    GenerationProgress { batch_id: String, progress: u8 },
    /// Generation batch completed
    GenerationComplete {
        batch_id: String,
        asset_count: u32,
    },
    /// Generation batch failed
    GenerationFailed { batch_id: String, error: String },
}

/// Information about a session participant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantInfo {
    pub user_id: String,
    pub role: ParticipantRole,
    pub character_name: Option<String>,
}

/// Participant role in the session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParticipantRole {
    DungeonMaster,
    Player,
    Spectator,
}

/// Scene data from server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneData {
    pub id: String,
    pub name: String,
    pub location_id: String,
    pub location_name: String,
    pub backdrop_asset: Option<String>,
    pub time_context: String,
    pub directorial_notes: String,
}

/// Character data for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterData {
    pub id: String,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub position: CharacterPosition,
    pub is_speaking: bool,
}

/// Character position on screen
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CharacterPosition {
    Left,
    Center,
    Right,
    OffScreen,
}

/// Available interaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionData {
    pub id: String,
    pub name: String,
    pub interaction_type: String,
    pub target_name: Option<String>,
    pub is_available: bool,
}

/// Dialogue choice for player
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueChoice {
    pub id: String,
    pub text: String,
    pub is_custom_input: bool,
}

/// Directorial context from DM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectorialContext {
    pub scene_notes: String,
    pub tone: String,
    pub npc_motivations: Vec<NpcMotivationData>,
    pub forbidden_topics: Vec<String>,
}

/// NPC motivation data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcMotivationData {
    pub character_id: String,
    pub mood: String,
    pub immediate_goal: String,
    pub secret_agenda: Option<String>,
}

/// DM's approval decision
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "decision")]
pub enum ApprovalDecision {
    Accept,
    AcceptWithModification {
        modified_dialogue: String,
        approved_tools: Vec<String>,
        rejected_tools: Vec<String>,
    },
    Reject {
        feedback: String,
    },
    TakeOver {
        dm_response: String,
    },
}

/// Proposed tool call from LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedTool {
    pub id: String,
    pub name: String,
    pub description: String,
    pub arguments: serde_json::Value,
}

/// Challenge suggestion information for DM approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeSuggestionInfo {
    pub challenge_id: String,
    pub challenge_name: String,
    pub skill_name: String,
    pub difficulty_display: String,
    pub confidence: String,
    pub reasoning: String,
}
