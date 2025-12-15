//! World export functionality
//!
//! This module provides export capabilities for worlds,
//! allowing them to be serialized to JSON for the Player to consume.
//!
//! Two export formats are available:
//! - [`JsonExporter`] / [`WorldSnapshot`]: Full export with all data for archival/backup
//! - `PlayerWorldSnapshot`: Streamlined snapshot for real-time Player client transmission
//!   (now defined in application/ports/outbound/world_exporter_port.rs)

mod json_exporter;
mod world_snapshot;

pub use world_snapshot::Neo4jWorldExporter;
