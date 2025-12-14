//! Campbell's character archetypes from "The Hero with a Thousand Faces"

/// Character archetypes based on Joseph Campbell's monomyth
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CampbellArchetype {
    /// The protagonist of the story
    Hero,

    /// The wise figure who guides the hero
    Mentor,

    /// Guards the boundary between worlds, tests the hero
    ThresholdGuardian,

    /// Announces the call to adventure
    Herald,

    /// Changes allegiance or appearance, creates doubt
    Shapeshifter,

    /// The villain, represents the hero's dark side
    Shadow,

    /// Provides comic relief and challenges the status quo
    Trickster,

    /// Supports the hero on their journey
    Ally,
}

impl CampbellArchetype {
    /// Returns a description of this archetype's narrative function
    pub fn description(&self) -> &'static str {
        match self {
            Self::Hero => "The protagonist who undergoes transformation through the journey",
            Self::Mentor => "The wise guide who provides knowledge, training, or magical gifts",
            Self::ThresholdGuardian => "Tests the hero's commitment and readiness at boundaries",
            Self::Herald => "Announces change and the call to adventure",
            Self::Shapeshifter => {
                "Keeps the hero guessing through changing loyalties or appearance"
            }
            Self::Shadow => "Represents the dark side, often the villain or internal conflict",
            Self::Trickster => "Brings humor and chaos, challenges rigid thinking",
            Self::Ally => "Provides support, companionship, and assistance on the journey",
        }
    }

    /// Returns typical behaviors for this archetype
    pub fn typical_behaviors(&self) -> &'static [&'static str] {
        match self {
            Self::Hero => &[
                "Takes action despite fear",
                "Learns and grows from challenges",
                "Sacrifices for others",
            ],
            Self::Mentor => &[
                "Shares wisdom and knowledge",
                "Provides tools or training",
                "Motivates the hero when discouraged",
            ],
            Self::ThresholdGuardian => &[
                "Blocks the hero's path",
                "Tests worthiness",
                "May become an ally once passed",
            ],
            Self::Herald => &[
                "Delivers important news",
                "Challenges the hero to act",
                "Represents the coming change",
            ],
            Self::Shapeshifter => &[
                "Appears trustworthy then betrays",
                "Creates romantic tension",
                "Keeps intentions unclear",
            ],
            Self::Shadow => &[
                "Opposes the hero's goals",
                "Mirrors the hero's dark potential",
                "Forces the hero to confront fears",
            ],
            Self::Trickster => &[
                "Uses humor and pranks",
                "Questions authority",
                "Brings perspective through chaos",
            ],
            Self::Ally => &[
                "Provides emotional support",
                "Assists in practical ways",
                "Shares the hero's values",
            ],
        }
    }
}

impl std::fmt::Display for CampbellArchetype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Hero => "Hero",
            Self::Mentor => "Mentor",
            Self::ThresholdGuardian => "Threshold Guardian",
            Self::Herald => "Herald",
            Self::Shapeshifter => "Shapeshifter",
            Self::Shadow => "Shadow",
            Self::Trickster => "Trickster",
            Self::Ally => "Ally",
        };
        write!(f, "{}", name)
    }
}

/// Record of an archetype change for a character
#[derive(Debug, Clone)]
pub struct ArchetypeChange {
    pub from: CampbellArchetype,
    pub to: CampbellArchetype,
    pub reason: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
