//! Player-specific services

use std::sync::Arc;

use crate::application::services::{
    PlayerCharacterServiceImpl, SceneResolutionServiceImpl, SessionJoinService, SheetTemplateService,
};

/// Services for player management and character operations
///
/// This struct groups services related to players: character sheets,
/// player characters, scene resolution, and session joining.
pub struct PlayerServices {
    pub sheet_template_service: SheetTemplateService,
    pub player_character_service: PlayerCharacterServiceImpl,
    pub scene_resolution_service: SceneResolutionServiceImpl,
    pub session_join_service: Arc<SessionJoinService>,
}

impl PlayerServices {
    /// Creates a new PlayerServices instance with all player-related services
    pub fn new(
        sheet_template_service: SheetTemplateService,
        player_character_service: PlayerCharacterServiceImpl,
        scene_resolution_service: SceneResolutionServiceImpl,
        session_join_service: Arc<SessionJoinService>,
    ) -> Self {
        Self {
            sheet_template_service,
            player_character_service,
            scene_resolution_service,
            session_join_service,
        }
    }
}
