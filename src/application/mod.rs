//! Application layer - Use cases and orchestration
//!
//! This layer contains:
//! - Ports: Interface definitions (traits) for inbound and outbound communication
//! - Services: Use case implementations
//! - DTOs: Data transfer objects for API boundaries

pub mod dto;
pub mod ports;
pub mod services;
