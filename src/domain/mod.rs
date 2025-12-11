//! Domain layer - Core business logic with no external dependencies
//!
//! This layer contains:
//! - Entities: World, Scene, Character, Location, etc.
//! - Value Objects: Archetype, Want, Relationship types
//! - Aggregates: World aggregate root
//! - Domain Events: State changes and notifications
//! - Domain Services: Pure business logic operations

pub mod aggregates;
pub mod entities;
pub mod events;
pub mod services;
pub mod value_objects;
