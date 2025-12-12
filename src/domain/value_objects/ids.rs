//! Strongly-typed identifiers for domain entities

use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! define_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            pub fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            pub fn as_uuid(&self) -> &Uuid {
                &self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<Uuid> for $name {
            fn from(uuid: Uuid) -> Self {
                Self(uuid)
            }
        }

        impl From<$name> for Uuid {
            fn from(id: $name) -> Uuid {
                id.0
            }
        }
    };
}

define_id!(WorldId);
define_id!(ActId);
define_id!(SceneId);
define_id!(LocationId);
define_id!(CharacterId);
define_id!(ItemId);
define_id!(RelationshipId);
define_id!(WantId);
define_id!(GridMapId);
define_id!(SessionId);
define_id!(ParticipantId);
define_id!(ActionId);
define_id!(EventId);
define_id!(InteractionId);
define_id!(AssetId);
define_id!(BatchId);
define_id!(WorkflowConfigId);
define_id!(SkillId);
define_id!(ChallengeId);
