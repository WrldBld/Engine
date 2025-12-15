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

use crate::application::ports::outbound::PlayerWorldSnapshot;
use crate::application::services::WorldService;
use crate::domain::value_objects::{ActionId, SessionId, WorldId};
use crate::infrastructure::session::{ClientId, SessionError, WorldSnapshot};
use crate::infrastructure::state::AppState;
use crate::domain::value_objects::{
    ActiveNarrativeEventContext, ApprovalDecision, CharacterContext, GamePromptRequest,
    PlayerActionContext, ProposedToolInfo, SceneContext,
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
            drop(sessions); // Release lock before async queue operation

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
                let dm_action = crate::domain::value_objects::DMAction::ApprovalDecision {
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
                        None
                    }
                    Err(e) => {
                        tracing::error!("Failed to enqueue approval decision: {}", e);
                        Some(ServerMessage::Error {
                            code: "QUEUE_ERROR".to_string(),
                            message: format!("Failed to queue approval: {}", e),
                        })
                    }
                }
            } else {
                Some(ServerMessage::Error {
                    code: "NOT_AUTHORIZED".to_string(),
                    message: "Only the DM can approve responses".to_string(),
                })
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

        ClientMessage::NarrativeEventSuggestionDecision {
            request_id,
            event_id,
            approved,
            selected_outcome,
        } => {
            tracing::debug!(
                "Received narrative event suggestion decision for {}: event={}, approved={}, outcome={:?}",
                request_id,
                event_id,
                approved,
                selected_outcome
            );

            // Only DMs should approve narrative event triggers
            let sessions = state.sessions.read().await;
            if let Some(session_id) = sessions.get_client_session(client_id) {
                if let Some(session) = sessions.get_session(session_id) {
                    // Verify this client is the DM
                    if let Some(dm) = session.get_dm() {
                        if dm.client_id == client_id {
                            if approved {
                                // TODO: Implement narrative event triggering
                                // 1. Load the narrative event from repository
                                // 2. Mark it as triggered (with selected_outcome)
                                // 3. Execute any immediate effects
                                // 4. Record a StoryEvent for the timeline
                                // 5. Broadcast scene direction to DM
                                tracing::info!(
                                    "DM approved narrative event {} trigger for request {}",
                                    event_id,
                                    request_id
                                );
                            } else {
                                tracing::info!(
                                    "DM rejected narrative event {} trigger for request {}",
                                    event_id,
                                    request_id
                                );
                            }
                        }
                    }
                }
            }
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

        // Load world data from database using the world service
        let player_snapshot = state
            .world_service
            .export_world_snapshot(wid)
            .await
            .map_err(|e| SessionError::Database(e))?;

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
        rule_system: player_snapshot.world.rule_system.clone().into(),
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
    /// DM approves/rejects a suggested narrative event trigger
    NarrativeEventSuggestionDecision {
        request_id: String,
        event_id: String,
        approved: bool,
        /// Optional selected outcome if DM pre-selects an outcome
        selected_outcome: Option<String>,
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
    /// Action queued for processing
    ActionQueued {
        action_id: String,
        player_name: String,
        action_type: String,
        queue_depth: usize,
    },
    /// Queue status update (sent to DM)
    QueueStatus {
        player_actions_pending: usize,
        llm_requests_pending: usize,
        llm_requests_processing: usize,
        approvals_pending: usize,
    },
    /// Approval required (sent to DM)
    ApprovalRequired {
        request_id: String,
        npc_name: String,
        proposed_dialogue: String,
        internal_reasoning: String,
        proposed_tools: Vec<ProposedToolInfo>,
        challenge_suggestion: Option<ChallengeSuggestionInfo>,
        narrative_event_suggestion: Option<NarrativeEventSuggestionInfo>,
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

// ApprovalDecision and ProposedToolInfo are now imported from domain::value_objects

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

/// Narrative event suggestion information for DM approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeEventSuggestionInfo {
    pub event_id: String,
    pub event_name: String,
    pub description: String,
    pub scene_direction: String,
    pub confidence: String,
    pub reasoning: String,
    pub matched_triggers: Vec<String>,
}
