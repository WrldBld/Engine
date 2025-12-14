//! Data Transfer Objects - For API boundaries
//!
//! DTOs live in the application layer so infrastructure (HTTP/WebSocket) can
//! serialize/deserialize without pulling serde into the domain model.

pub mod rule_system;
pub mod world;

pub use rule_system::*;
pub use world::*;
