//! NPC Observation Entity (Phase 23D)
//!
//! Tracks a PC's knowledge of where NPCs are/were located.
//! This creates a "fog of war" where player knowledge differs from reality,
//! enabling mystery/investigation gameplay.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::value_objects::{CharacterId, LocationId, PlayerCharacterId, RegionId};

/// How the PC learned about an NPC's location
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationType {
    /// PC directly saw the NPC at this location
    Direct,
    /// PC heard about the NPC's location from someone else
    HeardAbout,
    /// PC deduced/investigated the NPC's location
    Deduced,
}

impl ObservationType {
    /// Get a display name for the observation type
    pub fn display_name(&self) -> &'static str {
        match self {
            ObservationType::Direct => "Saw directly",
            ObservationType::HeardAbout => "Heard about",
            ObservationType::Deduced => "Deduced",
        }
    }

    /// Get an icon identifier for the observation type
    pub fn icon(&self) -> &'static str {
        match self {
            ObservationType::Direct => "eye",
            ObservationType::HeardAbout => "ear",
            ObservationType::Deduced => "brain",
        }
    }
}

impl std::fmt::Display for ObservationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl std::str::FromStr for ObservationType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "direct" => Ok(ObservationType::Direct),
            "heard_about" | "heardabout" => Ok(ObservationType::HeardAbout),
            "deduced" => Ok(ObservationType::Deduced),
            _ => Err(anyhow::anyhow!("Invalid observation type: {}", s)),
        }
    }
}

/// A PC's observation of an NPC's location
///
/// Stored as Neo4j edge: `(PlayerCharacter)-[:OBSERVED_NPC {...}]->(Character)`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcObservation {
    /// The PC who made the observation
    pub pc_id: PlayerCharacterId,
    /// The NPC who was observed
    pub npc_id: CharacterId,
    /// The location where the NPC was observed
    pub location_id: LocationId,
    /// The region within the location (more specific)
    pub region_id: RegionId,
    /// When this observation was made (in game time)
    pub game_time: DateTime<Utc>,
    /// How the PC learned this information
    pub observation_type: ObservationType,
    /// Optional notes (e.g., "The bartender mentioned seeing them at the docks")
    pub notes: Option<String>,
    /// When this observation was recorded (real time)
    pub created_at: DateTime<Utc>,
}

impl NpcObservation {
    /// Create a new direct observation (PC saw the NPC)
    pub fn direct(
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        location_id: LocationId,
        region_id: RegionId,
        game_time: DateTime<Utc>,
    ) -> Self {
        Self {
            pc_id,
            npc_id,
            location_id,
            region_id,
            game_time,
            observation_type: ObservationType::Direct,
            notes: None,
            created_at: Utc::now(),
        }
    }

    /// Create a new heard-about observation (PC was told about the NPC)
    pub fn heard_about(
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        location_id: LocationId,
        region_id: RegionId,
        game_time: DateTime<Utc>,
        notes: Option<String>,
    ) -> Self {
        Self {
            pc_id,
            npc_id,
            location_id,
            region_id,
            game_time,
            observation_type: ObservationType::HeardAbout,
            notes,
            created_at: Utc::now(),
        }
    }

    /// Create a new deduced observation (PC figured out the NPC's location)
    pub fn deduced(
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        location_id: LocationId,
        region_id: RegionId,
        game_time: DateTime<Utc>,
        notes: Option<String>,
    ) -> Self {
        Self {
            pc_id,
            npc_id,
            location_id,
            region_id,
            game_time,
            observation_type: ObservationType::Deduced,
            notes,
            created_at: Utc::now(),
        }
    }

    /// Add notes to this observation
    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }
}

/// Summary of an observation for display purposes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationSummary {
    /// The NPC's ID
    pub npc_id: String,
    /// The NPC's name
    pub npc_name: String,
    /// The NPC's portrait asset (if any)
    pub npc_portrait: Option<String>,
    /// The location name where they were observed
    pub location_name: String,
    /// The region name where they were observed
    pub region_name: String,
    /// When (game time)
    pub game_time: DateTime<Utc>,
    /// How the PC knows this
    pub observation_type: ObservationType,
    /// Any notes
    pub notes: Option<String>,
    /// How long ago in game time (for display)
    pub time_ago_description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_observation_type_from_str() {
        assert_eq!(
            "direct".parse::<ObservationType>().unwrap(),
            ObservationType::Direct
        );
        assert_eq!(
            "heard_about".parse::<ObservationType>().unwrap(),
            ObservationType::HeardAbout
        );
        assert_eq!(
            "deduced".parse::<ObservationType>().unwrap(),
            ObservationType::Deduced
        );
    }

    #[test]
    fn test_direct_observation() {
        let pc_id = PlayerCharacterId::new();
        let npc_id = CharacterId::new();
        let location_id = LocationId::new();
        let region_id = RegionId::new();
        let game_time = Utc::now();

        let obs = NpcObservation::direct(pc_id, npc_id, location_id, region_id, game_time);

        assert_eq!(obs.observation_type, ObservationType::Direct);
        assert!(obs.notes.is_none());
    }

    #[test]
    fn test_heard_about_with_notes() {
        let pc_id = PlayerCharacterId::new();
        let npc_id = CharacterId::new();
        let location_id = LocationId::new();
        let region_id = RegionId::new();
        let game_time = Utc::now();

        let obs = NpcObservation::heard_about(
            pc_id,
            npc_id,
            location_id,
            region_id,
            game_time,
            Some("The bartender told me".to_string()),
        );

        assert_eq!(obs.observation_type, ObservationType::HeardAbout);
        assert_eq!(obs.notes.as_deref(), Some("The bartender told me"));
    }
}
