//! Character Sheet Template - Defines the structure of character sheets per rule system
//!
//! Templates are tied to worlds and define:
//! - Sections (groups of related fields)
//! - Fields (individual data points with types)
//! - Derived values (calculated from other fields)
//! - Layout hints for UI rendering
//!
//! # Architectural Note (ADR-001: Domain Serialization)
//!
//! `CharacterSheetData` and `FieldValue` types intentionally include serde derives because:
//! 1. They are stored as JSON blobs in Neo4j (not normalized tables)
//! 2. Creating separate infrastructure DTOs would add significant boilerplate
//!    with no actual decoupling benefit (the JSON schema IS the domain contract)
//! 3. The types are value objects with no behavior, so serialization is intrinsic
//!
//! This is an accepted exception to the "no serde in domain" rule.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::domain::value_objects::{RuleSystemVariant, WorldId};

/// Unique identifier for a sheet template
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SheetTemplateId(pub String);

impl SheetTemplateId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl Default for SheetTemplateId {
    fn default() -> Self {
        Self::new()
    }
}

/// A character sheet template defining the structure of character data
#[derive(Debug, Clone)]
pub struct CharacterSheetTemplate {
    pub id: SheetTemplateId,
    pub world_id: WorldId,
    pub name: String,
    pub description: String,
    /// The rule system variant this template is for
    pub variant: RuleSystemVariant,
    /// Sections of the character sheet in display order
    pub sections: Vec<SheetSection>,
    /// Whether this is the default template (created from preset)
    pub is_default: bool,
}

impl CharacterSheetTemplate {
    pub fn new(world_id: WorldId, name: impl Into<String>, variant: RuleSystemVariant) -> Self {
        Self {
            id: SheetTemplateId::new(),
            world_id,
            name: name.into(),
            description: String::new(),
            variant,
            sections: Vec::new(),
            is_default: false,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_section(mut self, section: SheetSection) -> Self {
        self.sections.push(section);
        self
    }

    pub fn as_default(mut self) -> Self {
        self.is_default = true;
        self
    }

    /// Get all field IDs in this template
    pub fn all_field_ids(&self) -> Vec<&str> {
        self.sections
            .iter()
            .flat_map(|s| s.fields.iter().map(|f| f.id.as_str()))
            .collect()
    }

    /// Find a field by ID
    pub fn get_field(&self, field_id: &str) -> Option<&SheetField> {
        self.sections
            .iter()
            .flat_map(|s| s.fields.iter())
            .find(|f| f.id == field_id)
    }
}

/// A section of the character sheet (e.g., "Attributes", "Skills", "Combat")
#[derive(Debug, Clone)]
pub struct SheetSection {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    /// Fields in this section in display order
    pub fields: Vec<SheetField>,
    /// Layout hint for the section
    pub layout: SectionLayout,
    /// Whether this section is collapsible
    pub collapsible: bool,
    /// Whether this section starts collapsed
    pub collapsed_by_default: bool,
    /// Display order (lower = higher)
    pub order: u32,
}

impl SheetSection {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            fields: Vec::new(),
            layout: SectionLayout::Vertical,
            collapsible: false,
            collapsed_by_default: false,
            order: 0,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_field(mut self, field: SheetField) -> Self {
        self.fields.push(field);
        self
    }

    pub fn with_layout(mut self, layout: SectionLayout) -> Self {
        self.layout = layout;
        self
    }

    pub fn collapsible(mut self) -> Self {
        self.collapsible = true;
        self
    }

    pub fn collapsed(mut self) -> Self {
        self.collapsible = true;
        self.collapsed_by_default = true;
        self
    }

    pub fn with_order(mut self, order: u32) -> Self {
        self.order = order;
        self
    }
}

/// Layout hint for a section
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionLayout {
    /// Fields stack vertically
    Vertical,
    /// Fields arranged in a grid
    Grid { columns: u8 },
    /// Fields flow horizontally and wrap
    Flow,
    /// Two-column layout (label left, value right)
    TwoColumn,
}

/// A single field in the character sheet
#[derive(Debug, Clone)]
pub struct SheetField {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    /// The type of field and its configuration
    pub field_type: FieldType,
    /// Whether this field is required
    pub required: bool,
    /// Whether this field is read-only (for derived values)
    pub read_only: bool,
    /// Display order within section
    pub order: u32,
}

impl SheetField {
    pub fn new(id: impl Into<String>, name: impl Into<String>, field_type: FieldType) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            field_type,
            required: false,
            read_only: false,
            order: 0,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }

    pub fn with_order(mut self, order: u32) -> Self {
        self.order = order;
        self
    }
}

/// The type of a field and its specific configuration
#[derive(Debug, Clone)]
pub enum FieldType {
    /// A numeric value (e.g., attribute score, HP)
    Number {
        min: Option<i32>,
        max: Option<i32>,
        default: Option<i32>,
    },
    /// A text value (e.g., name, backstory)
    Text {
        multiline: bool,
        max_length: Option<usize>,
    },
    /// A boolean checkbox
    Checkbox { default: bool },
    /// A selection from predefined options
    Select { options: Vec<SelectOption> },
    /// A reference to a skill from the world's skill list
    SkillReference {
        /// Which skill categories to show
        categories: Option<Vec<String>>,
        /// Whether to show the skill's base attribute
        show_attribute: bool,
    },
    /// A value derived from other fields
    Derived {
        /// Formula to calculate the value (e.g., "floor((STR - 10) / 2)")
        formula: String,
        /// Fields this depends on
        depends_on: Vec<String>,
    },
    /// A resource with current/max values (e.g., HP, spell slots)
    Resource {
        max_field: Option<String>,
        default_max: Option<i32>,
    },
    /// A list of items (for inventory, features, etc.)
    ItemList {
        item_type: ItemListType,
        max_items: Option<usize>,
    },
    /// A list of skill values with proficiency
    SkillList {
        /// Whether to include modifier calculation
        show_modifier: bool,
        /// Whether to include proficiency checkbox
        show_proficiency: bool,
    },
}

/// Option in a select field
#[derive(Debug, Clone)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
    pub description: Option<String>,
}

impl SelectOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            description: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Type of items in an item list field
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemListType {
    /// Inventory items
    Inventory,
    /// Class features, racial traits, etc.
    Features,
    /// Spells or abilities
    Spells,
    /// Custom notes
    Notes,
}

/// Character data that conforms to a template
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CharacterSheetData {
    /// Map of field_id -> value
    pub values: HashMap<String, FieldValue>,
}

impl CharacterSheetData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, field_id: impl Into<String>, value: FieldValue) {
        self.values.insert(field_id.into(), value);
    }

    pub fn get(&self, field_id: &str) -> Option<&FieldValue> {
        self.values.get(field_id)
    }

    pub fn get_number(&self, field_id: &str) -> Option<i32> {
        match self.values.get(field_id)? {
            FieldValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn get_text(&self, field_id: &str) -> Option<&str> {
        match self.values.get(field_id)? {
            FieldValue::Text(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn get_bool(&self, field_id: &str) -> Option<bool> {
        match self.values.get(field_id)? {
            FieldValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    /// Get skill modifier by skill ID.
    /// Searches all FieldValue::SkillEntry values for a matching skill_id and returns the bonus.
    pub fn get_skill_modifier(&self, skill_id: &str) -> Option<i32> {
        for value in self.values.values() {
            if let FieldValue::SkillEntry {
                skill_id: entry_skill_id,
                bonus,
                ..
            } = value
            {
                if entry_skill_id == skill_id {
                    return Some(*bonus);
                }
            }
        }
        None
    }

    /// Get skill modifier by skill name (case-insensitive match).
    /// Useful when the skill ID isn't available but the name is.
    pub fn get_skill_modifier_by_name(&self, skill_name: &str) -> Option<i32> {
        let skill_name_lower = skill_name.to_lowercase();
        for value in self.values.values() {
            if let FieldValue::SkillEntry {
                skill_id,
                bonus,
                ..
            } = value
            {
                // Match if skill_id contains the skill name (case-insensitive)
                if skill_id.to_lowercase().contains(&skill_name_lower)
                    || skill_name_lower.contains(&skill_id.to_lowercase())
                {
                    return Some(*bonus);
                }
            }
        }
        None
    }
}

/// A value stored for a field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FieldValue {
    Number(i32),
    Text(String),
    Boolean(bool),
    /// Current and max for resources
    Resource { current: i32, max: i32 },
    /// List of strings (for item lists)
    List(Vec<String>),
    /// Skill proficiency entry
    SkillEntry {
        skill_id: String,
        proficient: bool,
        bonus: i32,
    },
}

// ============================================================================
// Default Templates per Rule System
// ============================================================================

impl CharacterSheetTemplate {
    /// Create the default template for a rule system variant
    pub fn default_for_variant(world_id: WorldId, variant: &RuleSystemVariant) -> Self {
        match variant {
            RuleSystemVariant::Dnd5e => Self::dnd5e_template(world_id),
            RuleSystemVariant::Pathfinder2e => Self::pathfinder2e_template(world_id),
            RuleSystemVariant::GenericD20 => Self::generic_d20_template(world_id),
            RuleSystemVariant::CallOfCthulhu7e => Self::coc7e_template(world_id),
            RuleSystemVariant::RuneQuest => Self::runequest_template(world_id),
            RuleSystemVariant::GenericD100 => Self::generic_d100_template(world_id),
            RuleSystemVariant::KidsOnBikes => Self::kids_on_bikes_template(world_id),
            RuleSystemVariant::FateCore => Self::fate_template(world_id),
            RuleSystemVariant::PoweredByApocalypse => Self::pbta_template(world_id),
            RuleSystemVariant::Custom(_) => Self::minimal_template(world_id),
        }
    }

    /// D&D 5th Edition template
    fn dnd5e_template(world_id: WorldId) -> Self {
        Self::new(world_id, "D&D 5e Character Sheet", RuleSystemVariant::Dnd5e)
            .with_description("Standard D&D 5th Edition character sheet")
            .as_default()
            // Attributes section
            .with_section(
                SheetSection::new("attributes", "Ability Scores")
                    .with_layout(SectionLayout::Grid { columns: 3 })
                    .with_order(0)
                    .with_field(SheetField::new("STR", "Strength", FieldType::Number { min: Some(1), max: Some(30), default: Some(10) }).with_order(0))
                    .with_field(SheetField::new("DEX", "Dexterity", FieldType::Number { min: Some(1), max: Some(30), default: Some(10) }).with_order(1))
                    .with_field(SheetField::new("CON", "Constitution", FieldType::Number { min: Some(1), max: Some(30), default: Some(10) }).with_order(2))
                    .with_field(SheetField::new("INT", "Intelligence", FieldType::Number { min: Some(1), max: Some(30), default: Some(10) }).with_order(3))
                    .with_field(SheetField::new("WIS", "Wisdom", FieldType::Number { min: Some(1), max: Some(30), default: Some(10) }).with_order(4))
                    .with_field(SheetField::new("CHA", "Charisma", FieldType::Number { min: Some(1), max: Some(30), default: Some(10) }).with_order(5))
            )
            // Modifiers section (derived)
            .with_section(
                SheetSection::new("modifiers", "Ability Modifiers")
                    .with_layout(SectionLayout::Grid { columns: 3 })
                    .with_order(1)
                    .with_field(SheetField::new("STR_MOD", "STR Mod", FieldType::Derived { formula: "floor((STR - 10) / 2)".into(), depends_on: vec!["STR".into()] }).read_only().with_order(0))
                    .with_field(SheetField::new("DEX_MOD", "DEX Mod", FieldType::Derived { formula: "floor((DEX - 10) / 2)".into(), depends_on: vec!["DEX".into()] }).read_only().with_order(1))
                    .with_field(SheetField::new("CON_MOD", "CON Mod", FieldType::Derived { formula: "floor((CON - 10) / 2)".into(), depends_on: vec!["CON".into()] }).read_only().with_order(2))
                    .with_field(SheetField::new("INT_MOD", "INT Mod", FieldType::Derived { formula: "floor((INT - 10) / 2)".into(), depends_on: vec!["INT".into()] }).read_only().with_order(3))
                    .with_field(SheetField::new("WIS_MOD", "WIS Mod", FieldType::Derived { formula: "floor((WIS - 10) / 2)".into(), depends_on: vec!["WIS".into()] }).read_only().with_order(4))
                    .with_field(SheetField::new("CHA_MOD", "CHA Mod", FieldType::Derived { formula: "floor((CHA - 10) / 2)".into(), depends_on: vec!["CHA".into()] }).read_only().with_order(5))
            )
            // Combat section
            .with_section(
                SheetSection::new("combat", "Combat")
                    .with_layout(SectionLayout::Grid { columns: 2 })
                    .with_order(2)
                    .with_field(SheetField::new("HP", "Hit Points", FieldType::Resource { max_field: Some("HP_MAX".into()), default_max: Some(10) }).with_order(0))
                    .with_field(SheetField::new("HP_MAX", "Max HP", FieldType::Number { min: Some(1), max: None, default: Some(10) }).with_order(1))
                    .with_field(SheetField::new("AC", "Armor Class", FieldType::Number { min: Some(0), max: None, default: Some(10) }).with_order(2))
                    .with_field(SheetField::new("INITIATIVE", "Initiative", FieldType::Derived { formula: "DEX_MOD".into(), depends_on: vec!["DEX_MOD".into()] }).read_only().with_order(3))
                    .with_field(SheetField::new("SPEED", "Speed", FieldType::Number { min: Some(0), max: None, default: Some(30) }).with_order(4))
                    .with_field(SheetField::new("PROF_BONUS", "Proficiency Bonus", FieldType::Number { min: Some(2), max: Some(6), default: Some(2) }).with_order(5))
            )
            // Skills section
            .with_section(
                SheetSection::new("skills", "Skills")
                    .with_layout(SectionLayout::Vertical)
                    .with_order(3)
                    .collapsible()
                    .with_field(SheetField::new("SKILLS", "Character Skills", FieldType::SkillList { show_modifier: true, show_proficiency: true }))
            )
            // Features section
            .with_section(
                SheetSection::new("features", "Features & Traits")
                    .with_layout(SectionLayout::Vertical)
                    .with_order(4)
                    .collapsible()
                    .collapsed()
                    .with_field(SheetField::new("FEATURES", "Features", FieldType::ItemList { item_type: ItemListType::Features, max_items: None }))
            )
            // Inventory section
            .with_section(
                SheetSection::new("inventory", "Inventory")
                    .with_layout(SectionLayout::Vertical)
                    .with_order(5)
                    .collapsible()
                    .collapsed()
                    .with_field(SheetField::new("INVENTORY", "Items", FieldType::ItemList { item_type: ItemListType::Inventory, max_items: None }))
            )
    }

    /// Pathfinder 2e template
    fn pathfinder2e_template(world_id: WorldId) -> Self {
        Self::new(world_id, "Pathfinder 2e Character Sheet", RuleSystemVariant::Pathfinder2e)
            .with_description("Pathfinder 2nd Edition character sheet")
            .as_default()
            .with_section(
                SheetSection::new("attributes", "Ability Scores")
                    .with_layout(SectionLayout::Grid { columns: 3 })
                    .with_order(0)
                    .with_field(SheetField::new("STR", "Strength", FieldType::Number { min: Some(1), max: None, default: Some(10) }))
                    .with_field(SheetField::new("DEX", "Dexterity", FieldType::Number { min: Some(1), max: None, default: Some(10) }))
                    .with_field(SheetField::new("CON", "Constitution", FieldType::Number { min: Some(1), max: None, default: Some(10) }))
                    .with_field(SheetField::new("INT", "Intelligence", FieldType::Number { min: Some(1), max: None, default: Some(10) }))
                    .with_field(SheetField::new("WIS", "Wisdom", FieldType::Number { min: Some(1), max: None, default: Some(10) }))
                    .with_field(SheetField::new("CHA", "Charisma", FieldType::Number { min: Some(1), max: None, default: Some(10) }))
            )
            .with_section(
                SheetSection::new("combat", "Combat")
                    .with_layout(SectionLayout::Grid { columns: 2 })
                    .with_order(1)
                    .with_field(SheetField::new("HP", "Hit Points", FieldType::Resource { max_field: Some("HP_MAX".into()), default_max: Some(10) }))
                    .with_field(SheetField::new("HP_MAX", "Max HP", FieldType::Number { min: Some(1), max: None, default: Some(10) }))
                    .with_field(SheetField::new("AC", "Armor Class", FieldType::Number { min: Some(0), max: None, default: Some(10) }))
                    .with_field(SheetField::new("PERCEPTION", "Perception", FieldType::Number { min: None, max: None, default: Some(0) }))
            )
            .with_section(
                SheetSection::new("skills", "Skills")
                    .with_layout(SectionLayout::Vertical)
                    .with_order(2)
                    .with_field(SheetField::new("SKILLS", "Character Skills", FieldType::SkillList { show_modifier: true, show_proficiency: true }))
            )
    }

    /// Generic D20 template
    fn generic_d20_template(world_id: WorldId) -> Self {
        Self::new(world_id, "Generic D20 Character Sheet", RuleSystemVariant::GenericD20)
            .with_description("Simple d20-based character sheet")
            .as_default()
            .with_section(
                SheetSection::new("attributes", "Attributes")
                    .with_layout(SectionLayout::Grid { columns: 2 })
                    .with_order(0)
                    .with_field(SheetField::new("STR", "Strength", FieldType::Number { min: Some(1), max: Some(20), default: Some(10) }))
                    .with_field(SheetField::new("DEX", "Dexterity", FieldType::Number { min: Some(1), max: Some(20), default: Some(10) }))
                    .with_field(SheetField::new("CON", "Constitution", FieldType::Number { min: Some(1), max: Some(20), default: Some(10) }))
                    .with_field(SheetField::new("INT", "Intelligence", FieldType::Number { min: Some(1), max: Some(20), default: Some(10) }))
                    .with_field(SheetField::new("WIS", "Wisdom", FieldType::Number { min: Some(1), max: Some(20), default: Some(10) }))
                    .with_field(SheetField::new("CHA", "Charisma", FieldType::Number { min: Some(1), max: Some(20), default: Some(10) }))
            )
            .with_section(
                SheetSection::new("vitals", "Vitals")
                    .with_layout(SectionLayout::TwoColumn)
                    .with_order(1)
                    .with_field(SheetField::new("HP", "Hit Points", FieldType::Resource { max_field: Some("HP_MAX".into()), default_max: Some(10) }))
                    .with_field(SheetField::new("HP_MAX", "Max HP", FieldType::Number { min: Some(1), max: None, default: Some(10) }))
            )
            .with_section(
                SheetSection::new("skills", "Skills")
                    .with_layout(SectionLayout::Vertical)
                    .with_order(2)
                    .with_field(SheetField::new("SKILLS", "Character Skills", FieldType::SkillList { show_modifier: true, show_proficiency: false }))
            )
    }

    /// Call of Cthulhu 7e template
    fn coc7e_template(world_id: WorldId) -> Self {
        Self::new(world_id, "Call of Cthulhu 7e Character Sheet", RuleSystemVariant::CallOfCthulhu7e)
            .with_description("Call of Cthulhu 7th Edition investigator sheet")
            .as_default()
            .with_section(
                SheetSection::new("characteristics", "Characteristics")
                    .with_layout(SectionLayout::Grid { columns: 3 })
                    .with_order(0)
                    .with_field(SheetField::new("STR", "Strength", FieldType::Number { min: Some(0), max: Some(100), default: Some(50) }))
                    .with_field(SheetField::new("CON", "Constitution", FieldType::Number { min: Some(0), max: Some(100), default: Some(50) }))
                    .with_field(SheetField::new("SIZ", "Size", FieldType::Number { min: Some(0), max: Some(100), default: Some(50) }))
                    .with_field(SheetField::new("DEX", "Dexterity", FieldType::Number { min: Some(0), max: Some(100), default: Some(50) }))
                    .with_field(SheetField::new("APP", "Appearance", FieldType::Number { min: Some(0), max: Some(100), default: Some(50) }))
                    .with_field(SheetField::new("INT", "Intelligence", FieldType::Number { min: Some(0), max: Some(100), default: Some(50) }))
                    .with_field(SheetField::new("POW", "Power", FieldType::Number { min: Some(0), max: Some(100), default: Some(50) }))
                    .with_field(SheetField::new("EDU", "Education", FieldType::Number { min: Some(0), max: Some(100), default: Some(50) }))
            )
            .with_section(
                SheetSection::new("derived", "Derived Attributes")
                    .with_layout(SectionLayout::Grid { columns: 2 })
                    .with_order(1)
                    .with_field(SheetField::new("HP", "Hit Points", FieldType::Resource { max_field: Some("HP_MAX".into()), default_max: Some(10) }))
                    .with_field(SheetField::new("HP_MAX", "Max HP", FieldType::Derived { formula: "floor((CON + SIZ) / 10)".into(), depends_on: vec!["CON".into(), "SIZ".into()] }).read_only())
                    .with_field(SheetField::new("SAN", "Sanity", FieldType::Resource { max_field: Some("SAN_MAX".into()), default_max: Some(50) }))
                    .with_field(SheetField::new("SAN_MAX", "Max Sanity", FieldType::Number { min: Some(0), max: Some(99), default: Some(99) }))
                    .with_field(SheetField::new("LUCK", "Luck", FieldType::Number { min: Some(0), max: Some(100), default: Some(50) }))
                    .with_field(SheetField::new("MP", "Magic Points", FieldType::Resource { max_field: Some("MP_MAX".into()), default_max: Some(10) }))
                    .with_field(SheetField::new("MP_MAX", "Max MP", FieldType::Derived { formula: "floor(POW / 5)".into(), depends_on: vec!["POW".into()] }).read_only())
            )
            .with_section(
                SheetSection::new("skills", "Skills")
                    .with_layout(SectionLayout::Vertical)
                    .with_order(2)
                    .with_field(SheetField::new("SKILLS", "Investigator Skills", FieldType::SkillList { show_modifier: false, show_proficiency: false }))
            )
    }

    /// RuneQuest template
    fn runequest_template(world_id: WorldId) -> Self {
        Self::new(world_id, "RuneQuest Character Sheet", RuleSystemVariant::RuneQuest)
            .with_description("RuneQuest percentile character sheet")
            .as_default()
            .with_section(
                SheetSection::new("characteristics", "Characteristics")
                    .with_layout(SectionLayout::Grid { columns: 2 })
                    .with_order(0)
                    .with_field(SheetField::new("STR", "Strength", FieldType::Number { min: Some(1), max: Some(21), default: Some(10) }))
                    .with_field(SheetField::new("CON", "Constitution", FieldType::Number { min: Some(1), max: Some(21), default: Some(10) }))
                    .with_field(SheetField::new("SIZ", "Size", FieldType::Number { min: Some(1), max: Some(21), default: Some(10) }))
                    .with_field(SheetField::new("INT", "Intelligence", FieldType::Number { min: Some(1), max: Some(21), default: Some(10) }))
                    .with_field(SheetField::new("POW", "Power", FieldType::Number { min: Some(1), max: Some(21), default: Some(10) }))
                    .with_field(SheetField::new("DEX", "Dexterity", FieldType::Number { min: Some(1), max: Some(21), default: Some(10) }))
                    .with_field(SheetField::new("CHA", "Charisma", FieldType::Number { min: Some(1), max: Some(21), default: Some(10) }))
            )
            .with_section(
                SheetSection::new("combat", "Combat")
                    .with_layout(SectionLayout::TwoColumn)
                    .with_order(1)
                    .with_field(SheetField::new("HP", "Hit Points", FieldType::Resource { max_field: Some("HP_MAX".into()), default_max: Some(10) }))
                    .with_field(SheetField::new("HP_MAX", "Max HP", FieldType::Number { min: Some(1), max: None, default: Some(10) }))
            )
            .with_section(
                SheetSection::new("skills", "Skills")
                    .with_layout(SectionLayout::Vertical)
                    .with_order(2)
                    .with_field(SheetField::new("SKILLS", "Character Skills", FieldType::SkillList { show_modifier: false, show_proficiency: false }))
            )
    }

    /// Generic D100 template
    fn generic_d100_template(world_id: WorldId) -> Self {
        Self::new(world_id, "Generic D100 Character Sheet", RuleSystemVariant::GenericD100)
            .with_description("Simple percentile-based character sheet")
            .as_default()
            .with_section(
                SheetSection::new("characteristics", "Characteristics")
                    .with_layout(SectionLayout::Grid { columns: 2 })
                    .with_order(0)
                    .with_field(SheetField::new("STR", "Strength", FieldType::Number { min: Some(0), max: Some(100), default: Some(50) }))
                    .with_field(SheetField::new("DEX", "Dexterity", FieldType::Number { min: Some(0), max: Some(100), default: Some(50) }))
                    .with_field(SheetField::new("INT", "Intelligence", FieldType::Number { min: Some(0), max: Some(100), default: Some(50) }))
                    .with_field(SheetField::new("POW", "Power", FieldType::Number { min: Some(0), max: Some(100), default: Some(50) }))
            )
            .with_section(
                SheetSection::new("vitals", "Vitals")
                    .with_layout(SectionLayout::TwoColumn)
                    .with_order(1)
                    .with_field(SheetField::new("HP", "Hit Points", FieldType::Resource { max_field: Some("HP_MAX".into()), default_max: Some(10) }))
                    .with_field(SheetField::new("HP_MAX", "Max HP", FieldType::Number { min: Some(1), max: None, default: Some(10) }))
            )
            .with_section(
                SheetSection::new("skills", "Skills")
                    .with_layout(SectionLayout::Vertical)
                    .with_order(2)
                    .with_field(SheetField::new("SKILLS", "Character Skills", FieldType::SkillList { show_modifier: false, show_proficiency: false }))
            )
    }

    /// Kids on Bikes template
    fn kids_on_bikes_template(world_id: WorldId) -> Self {
        Self::new(world_id, "Kids on Bikes Character Sheet", RuleSystemVariant::KidsOnBikes)
            .with_description("Kids on Bikes trope-based character sheet")
            .as_default()
            .with_section(
                SheetSection::new("stats", "Stats")
                    .with_layout(SectionLayout::Grid { columns: 2 })
                    .with_order(0)
                    .with_field(SheetField::new("BRAINS", "Brains", FieldType::Select {
                        options: vec![
                            SelectOption::new("d4", "d4"),
                            SelectOption::new("d6", "d6"),
                            SelectOption::new("d8", "d8"),
                            SelectOption::new("d10", "d10"),
                            SelectOption::new("d12", "d12"),
                            SelectOption::new("d20", "d20"),
                        ]
                    }))
                    .with_field(SheetField::new("BRAWN", "Brawn", FieldType::Select {
                        options: vec![
                            SelectOption::new("d4", "d4"),
                            SelectOption::new("d6", "d6"),
                            SelectOption::new("d8", "d8"),
                            SelectOption::new("d10", "d10"),
                            SelectOption::new("d12", "d12"),
                            SelectOption::new("d20", "d20"),
                        ]
                    }))
                    .with_field(SheetField::new("FIGHT", "Fight", FieldType::Select {
                        options: vec![
                            SelectOption::new("d4", "d4"),
                            SelectOption::new("d6", "d6"),
                            SelectOption::new("d8", "d8"),
                            SelectOption::new("d10", "d10"),
                            SelectOption::new("d12", "d12"),
                            SelectOption::new("d20", "d20"),
                        ]
                    }))
                    .with_field(SheetField::new("FLIGHT", "Flight", FieldType::Select {
                        options: vec![
                            SelectOption::new("d4", "d4"),
                            SelectOption::new("d6", "d6"),
                            SelectOption::new("d8", "d8"),
                            SelectOption::new("d10", "d10"),
                            SelectOption::new("d12", "d12"),
                            SelectOption::new("d20", "d20"),
                        ]
                    }))
                    .with_field(SheetField::new("CHARM", "Charm", FieldType::Select {
                        options: vec![
                            SelectOption::new("d4", "d4"),
                            SelectOption::new("d6", "d6"),
                            SelectOption::new("d8", "d8"),
                            SelectOption::new("d10", "d10"),
                            SelectOption::new("d12", "d12"),
                            SelectOption::new("d20", "d20"),
                        ]
                    }))
                    .with_field(SheetField::new("GRIT", "Grit", FieldType::Select {
                        options: vec![
                            SelectOption::new("d4", "d4"),
                            SelectOption::new("d6", "d6"),
                            SelectOption::new("d8", "d8"),
                            SelectOption::new("d10", "d10"),
                            SelectOption::new("d12", "d12"),
                            SelectOption::new("d20", "d20"),
                        ]
                    }))
            )
            .with_section(
                SheetSection::new("adversity", "Adversity Tokens")
                    .with_layout(SectionLayout::TwoColumn)
                    .with_order(1)
                    .with_field(SheetField::new("ADVERSITY", "Tokens", FieldType::Number { min: Some(0), max: Some(10), default: Some(0) }))
            )
            .with_section(
                SheetSection::new("strengths", "Strengths & Flaws")
                    .with_layout(SectionLayout::Vertical)
                    .with_order(2)
                    .with_field(SheetField::new("STRENGTHS", "Strengths", FieldType::ItemList { item_type: ItemListType::Features, max_items: Some(2) }))
                    .with_field(SheetField::new("FLAWS", "Flaws", FieldType::ItemList { item_type: ItemListType::Features, max_items: Some(2) }))
            )
    }

    /// FATE Core template
    fn fate_template(world_id: WorldId) -> Self {
        Self::new(world_id, "FATE Core Character Sheet", RuleSystemVariant::FateCore)
            .with_description("FATE Core character sheet with aspects and stunts")
            .as_default()
            .with_section(
                SheetSection::new("aspects", "Aspects")
                    .with_layout(SectionLayout::Vertical)
                    .with_order(0)
                    .with_field(SheetField::new("HIGH_CONCEPT", "High Concept", FieldType::Text { multiline: false, max_length: Some(100) }).required())
                    .with_field(SheetField::new("TROUBLE", "Trouble", FieldType::Text { multiline: false, max_length: Some(100) }).required())
                    .with_field(SheetField::new("ASPECT_1", "Aspect", FieldType::Text { multiline: false, max_length: Some(100) }))
                    .with_field(SheetField::new("ASPECT_2", "Aspect", FieldType::Text { multiline: false, max_length: Some(100) }))
                    .with_field(SheetField::new("ASPECT_3", "Aspect", FieldType::Text { multiline: false, max_length: Some(100) }))
            )
            .with_section(
                SheetSection::new("skills", "Skills")
                    .with_layout(SectionLayout::Vertical)
                    .with_order(1)
                    .with_field(SheetField::new("SKILLS", "Character Skills", FieldType::SkillList { show_modifier: true, show_proficiency: false }))
            )
            .with_section(
                SheetSection::new("stress", "Stress & Consequences")
                    .with_layout(SectionLayout::TwoColumn)
                    .with_order(2)
                    .with_field(SheetField::new("PHYSICAL_STRESS", "Physical Stress", FieldType::Number { min: Some(0), max: Some(4), default: Some(0) }))
                    .with_field(SheetField::new("MENTAL_STRESS", "Mental Stress", FieldType::Number { min: Some(0), max: Some(4), default: Some(0) }))
                    .with_field(SheetField::new("MILD", "Mild Consequence", FieldType::Text { multiline: false, max_length: Some(50) }))
                    .with_field(SheetField::new("MODERATE", "Moderate Consequence", FieldType::Text { multiline: false, max_length: Some(50) }))
                    .with_field(SheetField::new("SEVERE", "Severe Consequence", FieldType::Text { multiline: false, max_length: Some(50) }))
            )
            .with_section(
                SheetSection::new("refresh", "Refresh & Fate Points")
                    .with_layout(SectionLayout::TwoColumn)
                    .with_order(3)
                    .with_field(SheetField::new("REFRESH", "Refresh", FieldType::Number { min: Some(1), max: Some(10), default: Some(3) }))
                    .with_field(SheetField::new("FATE_POINTS", "Fate Points", FieldType::Number { min: Some(0), max: None, default: Some(3) }))
            )
            .with_section(
                SheetSection::new("stunts", "Stunts")
                    .with_layout(SectionLayout::Vertical)
                    .with_order(4)
                    .collapsible()
                    .with_field(SheetField::new("STUNTS", "Stunts", FieldType::ItemList { item_type: ItemListType::Features, max_items: Some(5) }))
            )
    }

    /// Powered by the Apocalypse template
    fn pbta_template(world_id: WorldId) -> Self {
        Self::new(world_id, "PbtA Character Sheet", RuleSystemVariant::PoweredByApocalypse)
            .with_description("Powered by the Apocalypse playbook sheet")
            .as_default()
            .with_section(
                SheetSection::new("stats", "Stats")
                    .with_layout(SectionLayout::Grid { columns: 3 })
                    .with_order(0)
                    .with_field(SheetField::new("COOL", "Cool", FieldType::Number { min: Some(-2), max: Some(3), default: Some(0) }))
                    .with_field(SheetField::new("HARD", "Hard", FieldType::Number { min: Some(-2), max: Some(3), default: Some(0) }))
                    .with_field(SheetField::new("HOT", "Hot", FieldType::Number { min: Some(-2), max: Some(3), default: Some(0) }))
                    .with_field(SheetField::new("SHARP", "Sharp", FieldType::Number { min: Some(-2), max: Some(3), default: Some(0) }))
                    .with_field(SheetField::new("WEIRD", "Weird", FieldType::Number { min: Some(-2), max: Some(3), default: Some(0) }))
            )
            .with_section(
                SheetSection::new("harm", "Harm")
                    .with_layout(SectionLayout::TwoColumn)
                    .with_order(1)
                    .with_field(SheetField::new("HARM", "Harm", FieldType::Number { min: Some(0), max: Some(6), default: Some(0) }))
                    .with_field(SheetField::new("STABILIZED", "Stabilized", FieldType::Checkbox { default: false }))
            )
            .with_section(
                SheetSection::new("experience", "Experience")
                    .with_layout(SectionLayout::TwoColumn)
                    .with_order(2)
                    .with_field(SheetField::new("XP", "Experience", FieldType::Number { min: Some(0), max: Some(5), default: Some(0) }))
            )
            .with_section(
                SheetSection::new("moves", "Moves")
                    .with_layout(SectionLayout::Vertical)
                    .with_order(3)
                    .with_field(SheetField::new("SKILLS", "Basic Moves", FieldType::SkillList { show_modifier: false, show_proficiency: false }))
            )
            .with_section(
                SheetSection::new("playbook_moves", "Playbook Moves")
                    .with_layout(SectionLayout::Vertical)
                    .with_order(4)
                    .collapsible()
                    .with_field(SheetField::new("PLAYBOOK_MOVES", "Special Moves", FieldType::ItemList { item_type: ItemListType::Features, max_items: None }))
            )
    }

    /// Minimal template for custom systems
    fn minimal_template(world_id: WorldId) -> Self {
        Self::new(world_id, "Custom Character Sheet", RuleSystemVariant::Custom(String::new()))
            .with_description("Minimal character sheet for custom rule systems")
            .as_default()
            .with_section(
                SheetSection::new("basic", "Basic Info")
                    .with_layout(SectionLayout::Vertical)
                    .with_order(0)
                    .with_field(SheetField::new("NOTES", "Character Notes", FieldType::Text { multiline: true, max_length: None }))
            )
            .with_section(
                SheetSection::new("skills", "Skills")
                    .with_layout(SectionLayout::Vertical)
                    .with_order(1)
                    .with_field(SheetField::new("SKILLS", "Character Skills", FieldType::SkillList { show_modifier: false, show_proficiency: false }))
            )
    }
}
