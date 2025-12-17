//! Approval-related value objects
//!
//! # Architectural Note (ADR-001: Domain Serialization)
//!
//! `ProposedToolInfo` and `ApprovalDecision` include serde derives because:
//! 1. They are serialized directly in WebSocket messages for DM approval workflow
//! 2. The wire format IS the domain contract - no translation layer adds value
//! 3. These are simple value objects with no domain behavior
//!
//! This is an accepted exception to the "no serde in domain" rule.

use serde::{Deserialize, Serialize};

/// Proposed tool call information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedToolInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub arguments: serde_json::Value,
}

/// DM's decision on an approval request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "decision")]
pub enum ApprovalDecision {
    Accept,
    AcceptWithModification {
        modified_dialogue: String,
        approved_tools: Vec<String>,
        rejected_tools: Vec<String>,
    },
    Reject {
        feedback: String,
    },
    TakeOver {
        dm_response: String,
    },
}
