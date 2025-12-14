//! Data Transfer Objects - For API boundaries
//!
//! DTOs live in the application layer so infrastructure (HTTP/WebSocket) can
//! serialize/deserialize without pulling serde into the domain model.

pub mod rule_system;
pub mod character;
pub mod skill;
pub mod challenge;
pub mod interaction;
pub mod suggestion;
pub mod event_chain;
pub mod narrative_event;
pub mod sheet_template;
pub mod export;
pub mod workflow;
pub mod asset;
pub mod location;
pub mod scene;
pub mod world;

pub use rule_system::*;
pub use character::*;
pub use skill::*;
pub use challenge::*;
pub use interaction::*;
pub use suggestion::*;
pub use event_chain::*;
pub use narrative_event::*;
pub use sheet_template::*;
pub use export::*;
pub use workflow::*;
pub use asset::*;
pub use location::*;
pub use scene::*;
pub use world::*;
