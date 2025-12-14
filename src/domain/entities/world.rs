//! World entity - The top-level container for a campaign setting

use chrono::{DateTime, Utc};

use crate::domain::value_objects::{RuleSystemConfig, WorldId};

/// A complete campaign world
#[derive(Debug, Clone)]
pub struct World {
    pub id: WorldId,
    pub name: String,
    pub description: String,
    pub rule_system: RuleSystemConfig,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl World {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: WorldId::new(),
            name: name.into(),
            description: description.into(),
            rule_system: RuleSystemConfig::default(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_rule_system(mut self, rule_system: RuleSystemConfig) -> Self {
        self.rule_system = rule_system;
        self
    }

    pub fn update_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
        self.updated_at = Utc::now();
    }

    pub fn update_description(&mut self, description: impl Into<String>) {
        self.description = description.into();
        self.updated_at = Utc::now();
    }
}

/// The stage of the monomyth (Hero's Journey)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MonomythStage {
    /// The hero's normal life before the adventure
    OrdinaryWorld,
    /// The hero receives a challenge or quest
    CallToAdventure,
    /// The hero hesitates or refuses the call
    RefusalOfTheCall,
    /// The hero encounters a mentor figure
    MeetingTheMentor,
    /// The hero commits to the adventure
    CrossingTheThreshold,
    /// The hero faces challenges and makes allies/enemies
    TestsAlliesEnemies,
    /// The hero approaches the main challenge
    ApproachToInnermostCave,
    /// The hero faces the greatest challenge
    Ordeal,
    /// The hero gains something from the ordeal
    Reward,
    /// The hero begins the journey home
    TheRoadBack,
    /// The hero is transformed by the experience
    Resurrection,
    /// The hero returns with new wisdom
    ReturnWithElixir,
}

impl MonomythStage {
    pub fn description(&self) -> &'static str {
        match self {
            Self::OrdinaryWorld => "The hero's normal life before the adventure begins",
            Self::CallToAdventure => "Something disrupts the ordinary world",
            Self::RefusalOfTheCall => "The hero hesitates, showing their humanity",
            Self::MeetingTheMentor => "A guide appears to prepare the hero",
            Self::CrossingTheThreshold => "The hero commits and enters the special world",
            Self::TestsAlliesEnemies => "The hero learns the rules of the new world",
            Self::ApproachToInnermostCave => "The hero prepares for the central ordeal",
            Self::Ordeal => "The hero faces their greatest fear",
            Self::Reward => "The hero takes possession of the treasure",
            Self::TheRoadBack => "The hero deals with consequences of the ordeal",
            Self::Resurrection => "The hero is tested once more and transformed",
            Self::ReturnWithElixir => "The hero returns with something to share",
        }
    }
}

/// A story arc within a world
#[derive(Debug, Clone)]
pub struct Act {
    pub id: crate::domain::value_objects::ActId,
    pub world_id: WorldId,
    pub name: String,
    pub stage: MonomythStage,
    pub description: String,
    pub order: u32,
}

impl Act {
    pub fn new(
        world_id: WorldId,
        name: impl Into<String>,
        stage: MonomythStage,
        order: u32,
    ) -> Self {
        Self {
            id: crate::domain::value_objects::ActId::new(),
            world_id,
            name: name.into(),
            stage,
            description: String::new(),
            order,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }
}
