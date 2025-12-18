//! Region relationship value objects
//!
//! These types define how characters relate to regions (locations) in the game world.
//! They are domain concepts used across the application for NPC presence determination.

use serde::{Deserialize, Serialize};

/// Work shift for a region (when NPC works there)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RegionShift {
    Day,
    Night,
    Always,
}

impl std::fmt::Display for RegionShift {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegionShift::Day => write!(f, "day"),
            RegionShift::Night => write!(f, "night"),
            RegionShift::Always => write!(f, "always"),
        }
    }
}

impl std::str::FromStr for RegionShift {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "day" => Ok(RegionShift::Day),
            "night" => Ok(RegionShift::Night),
            "always" | "" => Ok(RegionShift::Always),
            _ => Err(anyhow::anyhow!("Invalid region shift: {}", s)),
        }
    }
}

/// How often an NPC visits a region
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RegionFrequency {
    Often,
    Sometimes,
    Rarely,
}

impl std::fmt::Display for RegionFrequency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegionFrequency::Often => write!(f, "often"),
            RegionFrequency::Sometimes => write!(f, "sometimes"),
            RegionFrequency::Rarely => write!(f, "rarely"),
        }
    }
}

impl std::str::FromStr for RegionFrequency {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "often" => Ok(RegionFrequency::Often),
            "sometimes" | "" => Ok(RegionFrequency::Sometimes),
            "rarely" => Ok(RegionFrequency::Rarely),
            _ => Err(anyhow::anyhow!("Invalid region frequency: {}", s)),
        }
    }
}

/// Type of relationship between character and region
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RegionRelationshipType {
    Home,
    WorksAt { shift: RegionShift },
    Frequents { frequency: RegionFrequency },
    Avoids { reason: String },
}

/// A character's relationship to a region
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionRelationship {
    pub region_id: super::RegionId,
    pub region_name: String,
    pub relationship_type: RegionRelationshipType,
}
