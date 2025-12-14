//! Skill entity - Character skills tied to rule systems
//!
//! Skills are defined per-world and can be either:
//! - Default skills from a rule system preset
//! - Custom skills created by the DM

use crate::domain::value_objects::{RuleSystemVariant, SkillId, WorldId};

/// A skill that characters can use for challenges
#[derive(Debug, Clone)]
pub struct Skill {
    pub id: SkillId,
    pub world_id: WorldId,
    pub name: String,
    pub description: String,
    /// Category for UI grouping (e.g., "Physical", "Mental", "Social")
    pub category: SkillCategory,
    /// The base attribute this skill derives from (e.g., "DEX", "INT")
    pub base_attribute: Option<String>,
    /// Whether this is a custom skill (not from the preset)
    pub is_custom: bool,
    /// Whether to hide this skill from players
    pub is_hidden: bool,
    /// Display order within category
    pub order: u32,
}

impl Skill {
    pub fn new(world_id: WorldId, name: impl Into<String>, category: SkillCategory) -> Self {
        Self {
            id: SkillId::new(),
            world_id,
            name: name.into(),
            description: String::new(),
            category,
            base_attribute: None,
            is_custom: false,
            is_hidden: false,
            order: 0,
        }
    }

    pub fn custom(world_id: WorldId, name: impl Into<String>, category: SkillCategory) -> Self {
        let mut skill = Self::new(world_id, name, category);
        skill.is_custom = true;
        skill
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_base_attribute(mut self, attribute: impl Into<String>) -> Self {
        self.base_attribute = Some(attribute.into());
        self
    }

    pub fn with_order(mut self, order: u32) -> Self {
        self.order = order;
        self
    }
}

/// Skill categories for UI organization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SkillCategory {
    // D20 style categories
    Physical,
    Mental,
    Social,

    // D100/CoC style categories
    Interpersonal,
    Investigation,
    Academic,
    Practical,
    Combat,

    // Narrative style
    Approach,
    Aspect,

    // General
    Other,
    Custom,
}

impl SkillCategory {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Physical => "Physical",
            Self::Mental => "Mental",
            Self::Social => "Social",
            Self::Interpersonal => "Interpersonal",
            Self::Investigation => "Investigation",
            Self::Academic => "Academic",
            Self::Practical => "Practical",
            Self::Combat => "Combat",
            Self::Approach => "Approach",
            Self::Aspect => "Aspect",
            Self::Other => "Other",
            Self::Custom => "Custom",
        }
    }
}

/// Default skills for each rule system variant
pub fn default_skills_for_variant(world_id: WorldId, variant: &RuleSystemVariant) -> Vec<Skill> {
    match variant {
        RuleSystemVariant::Dnd5e => dnd5e_skills(world_id),
        RuleSystemVariant::Pathfinder2e => pathfinder2e_skills(world_id),
        RuleSystemVariant::GenericD20 => generic_d20_skills(world_id),
        RuleSystemVariant::CallOfCthulhu7e => coc7e_skills(world_id),
        RuleSystemVariant::RuneQuest => runequest_skills(world_id),
        RuleSystemVariant::GenericD100 => generic_d100_skills(world_id),
        RuleSystemVariant::KidsOnBikes => kids_on_bikes_skills(world_id),
        RuleSystemVariant::FateCore => fate_core_skills(world_id),
        RuleSystemVariant::PoweredByApocalypse => pbta_skills(world_id),
        RuleSystemVariant::Custom(_) => vec![],
    }
}

// D&D 5e Skills
fn dnd5e_skills(world_id: WorldId) -> Vec<Skill> {
    vec![
        // STR skills
        Skill::new(world_id, "Athletics", SkillCategory::Physical)
            .with_base_attribute("STR")
            .with_description("Climbing, swimming, jumping, and physical exertion")
            .with_order(1),

        // DEX skills
        Skill::new(world_id, "Acrobatics", SkillCategory::Physical)
            .with_base_attribute("DEX")
            .with_description("Balance, tumbling, and aerial maneuvers")
            .with_order(2),
        Skill::new(world_id, "Sleight of Hand", SkillCategory::Physical)
            .with_base_attribute("DEX")
            .with_description("Pickpocketing, concealing objects, manual trickery")
            .with_order(3),
        Skill::new(world_id, "Stealth", SkillCategory::Physical)
            .with_base_attribute("DEX")
            .with_description("Moving silently and hiding")
            .with_order(4),

        // INT skills
        Skill::new(world_id, "Arcana", SkillCategory::Mental)
            .with_base_attribute("INT")
            .with_description("Knowledge of spells, magic items, and magical traditions")
            .with_order(5),
        Skill::new(world_id, "History", SkillCategory::Mental)
            .with_base_attribute("INT")
            .with_description("Knowledge of historical events, people, and legends")
            .with_order(6),
        Skill::new(world_id, "Investigation", SkillCategory::Mental)
            .with_base_attribute("INT")
            .with_description("Deduction, searching for clues, making inferences")
            .with_order(7),
        Skill::new(world_id, "Nature", SkillCategory::Mental)
            .with_base_attribute("INT")
            .with_description("Knowledge of terrain, plants, animals, and weather")
            .with_order(8),
        Skill::new(world_id, "Religion", SkillCategory::Mental)
            .with_base_attribute("INT")
            .with_description("Knowledge of deities, rites, and religious traditions")
            .with_order(9),

        // WIS skills
        Skill::new(world_id, "Animal Handling", SkillCategory::Social)
            .with_base_attribute("WIS")
            .with_description("Calming, training, and directing animals")
            .with_order(10),
        Skill::new(world_id, "Insight", SkillCategory::Social)
            .with_base_attribute("WIS")
            .with_description("Reading body language, detecting lies, understanding motivations")
            .with_order(11),
        Skill::new(world_id, "Medicine", SkillCategory::Mental)
            .with_base_attribute("WIS")
            .with_description("Diagnosing illnesses, stabilizing the dying, treating wounds")
            .with_order(12),
        Skill::new(world_id, "Perception", SkillCategory::Mental)
            .with_base_attribute("WIS")
            .with_description("Noticing threats, spotting hidden objects, general awareness")
            .with_order(13),
        Skill::new(world_id, "Survival", SkillCategory::Physical)
            .with_base_attribute("WIS")
            .with_description("Tracking, foraging, navigating wilderness")
            .with_order(14),

        // CHA skills
        Skill::new(world_id, "Deception", SkillCategory::Social)
            .with_base_attribute("CHA")
            .with_description("Lying, misleading, disguising intentions")
            .with_order(15),
        Skill::new(world_id, "Intimidation", SkillCategory::Social)
            .with_base_attribute("CHA")
            .with_description("Threatening, coercing, inspiring fear")
            .with_order(16),
        Skill::new(world_id, "Performance", SkillCategory::Social)
            .with_base_attribute("CHA")
            .with_description("Acting, music, storytelling, entertainment")
            .with_order(17),
        Skill::new(world_id, "Persuasion", SkillCategory::Social)
            .with_base_attribute("CHA")
            .with_description("Convincing, negotiating, influencing through tact")
            .with_order(18),
    ]
}

// Pathfinder 2e Skills
fn pathfinder2e_skills(world_id: WorldId) -> Vec<Skill> {
    vec![
        Skill::new(world_id, "Acrobatics", SkillCategory::Physical)
            .with_base_attribute("DEX")
            .with_order(1),
        Skill::new(world_id, "Arcana", SkillCategory::Mental)
            .with_base_attribute("INT")
            .with_order(2),
        Skill::new(world_id, "Athletics", SkillCategory::Physical)
            .with_base_attribute("STR")
            .with_order(3),
        Skill::new(world_id, "Crafting", SkillCategory::Mental)
            .with_base_attribute("INT")
            .with_order(4),
        Skill::new(world_id, "Deception", SkillCategory::Social)
            .with_base_attribute("CHA")
            .with_order(5),
        Skill::new(world_id, "Diplomacy", SkillCategory::Social)
            .with_base_attribute("CHA")
            .with_order(6),
        Skill::new(world_id, "Intimidation", SkillCategory::Social)
            .with_base_attribute("CHA")
            .with_order(7),
        Skill::new(world_id, "Medicine", SkillCategory::Mental)
            .with_base_attribute("WIS")
            .with_order(8),
        Skill::new(world_id, "Nature", SkillCategory::Mental)
            .with_base_attribute("WIS")
            .with_order(9),
        Skill::new(world_id, "Occultism", SkillCategory::Mental)
            .with_base_attribute("INT")
            .with_order(10),
        Skill::new(world_id, "Performance", SkillCategory::Social)
            .with_base_attribute("CHA")
            .with_order(11),
        Skill::new(world_id, "Religion", SkillCategory::Mental)
            .with_base_attribute("WIS")
            .with_order(12),
        Skill::new(world_id, "Society", SkillCategory::Mental)
            .with_base_attribute("INT")
            .with_order(13),
        Skill::new(world_id, "Stealth", SkillCategory::Physical)
            .with_base_attribute("DEX")
            .with_order(14),
        Skill::new(world_id, "Survival", SkillCategory::Physical)
            .with_base_attribute("WIS")
            .with_order(15),
        Skill::new(world_id, "Thievery", SkillCategory::Physical)
            .with_base_attribute("DEX")
            .with_order(16),
    ]
}

// Generic D20 (simplified)
fn generic_d20_skills(world_id: WorldId) -> Vec<Skill> {
    vec![
        Skill::new(world_id, "Athletics", SkillCategory::Physical)
            .with_base_attribute("STR")
            .with_order(1),
        Skill::new(world_id, "Agility", SkillCategory::Physical)
            .with_base_attribute("DEX")
            .with_order(2),
        Skill::new(world_id, "Endurance", SkillCategory::Physical)
            .with_base_attribute("CON")
            .with_order(3),
        Skill::new(world_id, "Knowledge", SkillCategory::Mental)
            .with_base_attribute("INT")
            .with_order(4),
        Skill::new(world_id, "Awareness", SkillCategory::Mental)
            .with_base_attribute("WIS")
            .with_order(5),
        Skill::new(world_id, "Influence", SkillCategory::Social)
            .with_base_attribute("CHA")
            .with_order(6),
    ]
}

// Call of Cthulhu 7e Skills
fn coc7e_skills(world_id: WorldId) -> Vec<Skill> {
    vec![
        // Interpersonal
        Skill::new(world_id, "Charm", SkillCategory::Interpersonal)
            .with_description("Physical attraction, seduction, flattery")
            .with_order(1),
        Skill::new(world_id, "Fast Talk", SkillCategory::Interpersonal)
            .with_description("Con, deceive, lie, misdirect")
            .with_order(2),
        Skill::new(world_id, "Intimidate", SkillCategory::Interpersonal)
            .with_description("Threats, physical coercion")
            .with_order(3),
        Skill::new(world_id, "Persuade", SkillCategory::Interpersonal)
            .with_description("Reasoned argument, debate")
            .with_order(4),
        Skill::new(world_id, "Psychology", SkillCategory::Interpersonal)
            .with_description("Understand motives, see through lies")
            .with_order(5),

        // Investigation
        Skill::new(world_id, "Library Use", SkillCategory::Investigation)
            .with_description("Navigate libraries, find information in documents")
            .with_order(6),
        Skill::new(world_id, "Spot Hidden", SkillCategory::Investigation)
            .with_description("Spot concealed objects, notice things")
            .with_order(7),
        Skill::new(world_id, "Listen", SkillCategory::Investigation)
            .with_description("Hear sounds, eavesdrop")
            .with_order(8),
        Skill::new(world_id, "Track", SkillCategory::Investigation)
            .with_description("Follow tracks, signs of passage")
            .with_order(9),

        // Academic
        Skill::new(world_id, "Accounting", SkillCategory::Academic)
            .with_order(10),
        Skill::new(world_id, "Anthropology", SkillCategory::Academic)
            .with_order(11),
        Skill::new(world_id, "Archaeology", SkillCategory::Academic)
            .with_order(12),
        Skill::new(world_id, "History", SkillCategory::Academic)
            .with_order(13),
        Skill::new(world_id, "Law", SkillCategory::Academic)
            .with_order(14),
        Skill::new(world_id, "Medicine", SkillCategory::Academic)
            .with_order(15),
        Skill::new(world_id, "Natural World", SkillCategory::Academic)
            .with_order(16),
        Skill::new(world_id, "Occult", SkillCategory::Academic)
            .with_description("Knowledge of the Mythos and supernatural")
            .with_order(17),
        Skill::new(world_id, "Science", SkillCategory::Academic)
            .with_order(18),

        // Practical
        Skill::new(world_id, "Art/Craft", SkillCategory::Practical)
            .with_order(19),
        Skill::new(world_id, "Disguise", SkillCategory::Practical)
            .with_order(20),
        Skill::new(world_id, "Drive Auto", SkillCategory::Practical)
            .with_order(21),
        Skill::new(world_id, "Electrical Repair", SkillCategory::Practical)
            .with_order(22),
        Skill::new(world_id, "First Aid", SkillCategory::Practical)
            .with_order(23),
        Skill::new(world_id, "Locksmith", SkillCategory::Practical)
            .with_order(24),
        Skill::new(world_id, "Mechanical Repair", SkillCategory::Practical)
            .with_order(25),
        Skill::new(world_id, "Navigate", SkillCategory::Practical)
            .with_order(26),
        Skill::new(world_id, "Sleight of Hand", SkillCategory::Practical)
            .with_order(27),
        Skill::new(world_id, "Stealth", SkillCategory::Practical)
            .with_order(28),

        // Combat
        Skill::new(world_id, "Dodge", SkillCategory::Combat)
            .with_order(29),
        Skill::new(world_id, "Fighting (Brawl)", SkillCategory::Combat)
            .with_order(30),
        Skill::new(world_id, "Firearms (Handgun)", SkillCategory::Combat)
            .with_order(31),
        Skill::new(world_id, "Firearms (Rifle/Shotgun)", SkillCategory::Combat)
            .with_order(32),
        Skill::new(world_id, "Throw", SkillCategory::Combat)
            .with_order(33),
    ]
}

// RuneQuest (simplified)
fn runequest_skills(world_id: WorldId) -> Vec<Skill> {
    vec![
        Skill::new(world_id, "Athletics", SkillCategory::Physical).with_order(1),
        Skill::new(world_id, "Brawn", SkillCategory::Physical).with_order(2),
        Skill::new(world_id, "Endurance", SkillCategory::Physical).with_order(3),
        Skill::new(world_id, "Evade", SkillCategory::Combat).with_order(4),
        Skill::new(world_id, "Perception", SkillCategory::Mental).with_order(5),
        Skill::new(world_id, "Stealth", SkillCategory::Physical).with_order(6),
        Skill::new(world_id, "Willpower", SkillCategory::Mental).with_order(7),
        Skill::new(world_id, "Deceit", SkillCategory::Social).with_order(8),
        Skill::new(world_id, "Influence", SkillCategory::Social).with_order(9),
        Skill::new(world_id, "Insight", SkillCategory::Social).with_order(10),
        Skill::new(world_id, "Locale", SkillCategory::Mental).with_order(11),
        Skill::new(world_id, "Customs", SkillCategory::Mental).with_order(12),
        Skill::new(world_id, "First Aid", SkillCategory::Practical).with_order(13),
        Skill::new(world_id, "Craft", SkillCategory::Practical).with_order(14),
    ]
}

// Generic D100
fn generic_d100_skills(world_id: WorldId) -> Vec<Skill> {
    vec![
        Skill::new(world_id, "Athletics", SkillCategory::Physical).with_order(1),
        Skill::new(world_id, "Perception", SkillCategory::Mental).with_order(2),
        Skill::new(world_id, "Stealth", SkillCategory::Physical).with_order(3),
        Skill::new(world_id, "Investigation", SkillCategory::Mental).with_order(4),
        Skill::new(world_id, "Persuasion", SkillCategory::Social).with_order(5),
        Skill::new(world_id, "Deception", SkillCategory::Social).with_order(6),
        Skill::new(world_id, "Combat", SkillCategory::Combat).with_order(7),
        Skill::new(world_id, "First Aid", SkillCategory::Practical).with_order(8),
    ]
}

// Kids on Bikes (stats as "skills")
fn kids_on_bikes_skills(world_id: WorldId) -> Vec<Skill> {
    vec![
        Skill::new(world_id, "Brains", SkillCategory::Approach)
            .with_description("Book smarts, problem solving, trivia")
            .with_order(1),
        Skill::new(world_id, "Brawn", SkillCategory::Approach)
            .with_description("Physical strength, endurance, toughness")
            .with_order(2),
        Skill::new(world_id, "Fight", SkillCategory::Approach)
            .with_description("Combat ability, self-defense")
            .with_order(3),
        Skill::new(world_id, "Flight", SkillCategory::Approach)
            .with_description("Running away, escaping, hiding")
            .with_order(4),
        Skill::new(world_id, "Charm", SkillCategory::Approach)
            .with_description("Social graces, persuasion, likability")
            .with_order(5),
        Skill::new(world_id, "Grit", SkillCategory::Approach)
            .with_description("Willpower, courage, determination")
            .with_order(6),
    ]
}

// FATE Core Skills
fn fate_core_skills(world_id: WorldId) -> Vec<Skill> {
    vec![
        Skill::new(world_id, "Athletics", SkillCategory::Physical)
            .with_description("Running, jumping, climbing, general physical activity")
            .with_order(1),
        Skill::new(world_id, "Burglary", SkillCategory::Practical)
            .with_description("Bypassing security, lockpicking, sleight of hand")
            .with_order(2),
        Skill::new(world_id, "Contacts", SkillCategory::Social)
            .with_description("Knowing people, gathering information through networks")
            .with_order(3),
        Skill::new(world_id, "Crafts", SkillCategory::Practical)
            .with_description("Making and breaking things")
            .with_order(4),
        Skill::new(world_id, "Deceive", SkillCategory::Social)
            .with_description("Lying, misdirection, creating false impressions")
            .with_order(5),
        Skill::new(world_id, "Drive", SkillCategory::Practical)
            .with_description("Operating vehicles")
            .with_order(6),
        Skill::new(world_id, "Empathy", SkillCategory::Social)
            .with_description("Reading people's emotions and intentions")
            .with_order(7),
        Skill::new(world_id, "Fight", SkillCategory::Combat)
            .with_description("Close quarters combat")
            .with_order(8),
        Skill::new(world_id, "Investigate", SkillCategory::Mental)
            .with_description("Finding clues, solving mysteries")
            .with_order(9),
        Skill::new(world_id, "Lore", SkillCategory::Mental)
            .with_description("Specialized knowledge")
            .with_order(10),
        Skill::new(world_id, "Notice", SkillCategory::Mental)
            .with_description("Spotting things, general awareness")
            .with_order(11),
        Skill::new(world_id, "Physique", SkillCategory::Physical)
            .with_description("Raw strength, endurance")
            .with_order(12),
        Skill::new(world_id, "Provoke", SkillCategory::Social)
            .with_description("Intimidation, getting emotional reactions")
            .with_order(13),
        Skill::new(world_id, "Rapport", SkillCategory::Social)
            .with_description("Building trust, making friends")
            .with_order(14),
        Skill::new(world_id, "Resources", SkillCategory::Practical)
            .with_description("Wealth and material assets")
            .with_order(15),
        Skill::new(world_id, "Shoot", SkillCategory::Combat)
            .with_description("Ranged attacks")
            .with_order(16),
        Skill::new(world_id, "Stealth", SkillCategory::Physical)
            .with_description("Staying unseen, tailing people")
            .with_order(17),
        Skill::new(world_id, "Will", SkillCategory::Mental)
            .with_description("Mental fortitude, resisting influence")
            .with_order(18),
    ]
}

// Powered by the Apocalypse (basic moves as "skills")
fn pbta_skills(world_id: WorldId) -> Vec<Skill> {
    vec![
        Skill::new(world_id, "Act Under Pressure", SkillCategory::Approach)
            .with_base_attribute("Cool")
            .with_description("Stay calm, keep your head when things go south")
            .with_order(1),
        Skill::new(world_id, "Help/Interfere", SkillCategory::Approach)
            .with_base_attribute("Bond")
            .with_description("Aid or hinder another character's action")
            .with_order(2),
        Skill::new(world_id, "Go Aggro", SkillCategory::Approach)
            .with_base_attribute("Hard")
            .with_description("Threaten violence to get what you want")
            .with_order(3),
        Skill::new(world_id, "Seize by Force", SkillCategory::Approach)
            .with_base_attribute("Hard")
            .with_description("Take something through direct violence")
            .with_order(4),
        Skill::new(world_id, "Read a Sitch", SkillCategory::Approach)
            .with_base_attribute("Sharp")
            .with_description("Assess a dangerous situation")
            .with_order(5),
        Skill::new(world_id, "Read a Person", SkillCategory::Approach)
            .with_base_attribute("Sharp")
            .with_description("Figure out what someone is really about")
            .with_order(6),
        Skill::new(world_id, "Manipulate", SkillCategory::Approach)
            .with_base_attribute("Hot")
            .with_description("Get someone to do what you want")
            .with_order(7),
        Skill::new(world_id, "Seduce", SkillCategory::Approach)
            .with_base_attribute("Hot")
            .with_description("Use attraction to influence")
            .with_order(8),
        Skill::new(world_id, "Open Your Brain", SkillCategory::Approach)
            .with_base_attribute("Weird")
            .with_description("Tap into the psychic maelstrom")
            .with_order(9),
    ]
}
