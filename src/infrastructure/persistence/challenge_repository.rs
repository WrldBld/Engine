//! Challenge repository implementation for Neo4j
//!
//! ## Graph-First Design (Phase 0.E)
//!
//! This repository uses Neo4j edges for all Challenge relationships:
//! - `(Challenge)-[:REQUIRES_SKILL]->(Skill)` - Skill tested by this challenge
//! - `(Challenge)-[:TIED_TO_SCENE]->(Scene)` - Scene this challenge appears in
//! - `(Challenge)-[:REQUIRES_COMPLETION_OF {success_required}]->(Challenge)` - Prerequisites
//! - `(Challenge)-[:AVAILABLE_AT {always_available, time_restriction}]->(Location)` - Location availability
//! - `(Challenge)-[:ON_SUCCESS_UNLOCKS]->(Location)` - Location unlocked on success

use anyhow::Result;
use async_trait::async_trait;
use neo4rs::{query, Row};

use super::connection::Neo4jConnection;
use crate::application::dto::{DifficultyRequestDto, OutcomesRequestDto, TriggerConditionRequestDto};
use crate::application::ports::outbound::ChallengeRepositoryPort;
use crate::domain::entities::{
    Challenge, ChallengeLocationAvailability, ChallengePrerequisite, ChallengeType,
};
use crate::domain::value_objects::{ChallengeId, LocationId, SceneId, SkillId, WorldId};

/// Repository for Challenge operations
pub struct Neo4jChallengeRepository {
    connection: Neo4jConnection,
}

impl Neo4jChallengeRepository {
    pub fn new(connection: Neo4jConnection) -> Self {
        Self { connection }
    }
}

// =============================================================================
// ChallengeRepositoryPort Implementation
// =============================================================================

#[async_trait]
impl ChallengeRepositoryPort for Neo4jChallengeRepository {
    // -------------------------------------------------------------------------
    // Core CRUD
    // -------------------------------------------------------------------------

    async fn create(&self, challenge: &Challenge) -> Result<()> {
        // Serialize complex fields as JSON
        let outcomes_json =
            serde_json::to_string(&OutcomesRequestDto::from(challenge.outcomes.clone()))?;
        let triggers_json = serde_json::to_string(
            &challenge
                .trigger_conditions
                .iter()
                .cloned()
                .map(TriggerConditionRequestDto::from)
                .collect::<Vec<_>>(),
        )?;
        let tags_json = serde_json::to_string(&challenge.tags)?;

        let q = query(
            "MATCH (w:World {id: $world_id})
            CREATE (c:Challenge {
                id: $id,
                world_id: $world_id,
                name: $name,
                description: $description,
                challenge_type: $challenge_type,
                difficulty_json: $difficulty_json,
                outcomes_json: $outcomes_json,
                triggers_json: $triggers_json,
                active: $active,
                challenge_order: $challenge_order,
                is_favorite: $is_favorite,
                tags_json: $tags_json
            })
            CREATE (w)-[:CONTAINS_CHALLENGE]->(c)
            RETURN c.id as id",
        )
        .param("id", challenge.id.to_string())
        .param("world_id", challenge.world_id.to_string())
        .param("name", challenge.name.clone())
        .param("description", challenge.description.clone())
        .param("challenge_type", format!("{:?}", challenge.challenge_type))
        .param(
            "difficulty_json",
            serde_json::to_string(&DifficultyRequestDto::from(challenge.difficulty.clone()))?,
        )
        .param("outcomes_json", outcomes_json)
        .param("triggers_json", triggers_json)
        .param("active", challenge.active)
        .param("challenge_order", challenge.order as i64)
        .param("is_favorite", challenge.is_favorite)
        .param("tags_json", tags_json);

        self.connection.graph().run(q).await?;
        tracing::debug!("Created challenge: {}", challenge.name);

        Ok(())
    }

    async fn get(&self, id: ChallengeId) -> Result<Option<Challenge>> {
        let q = query(
            "MATCH (c:Challenge {id: $id})
            RETURN c",
        )
        .param("id", id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            Ok(Some(row_to_challenge(row)?))
        } else {
            Ok(None)
        }
    }

    async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<Challenge>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:CONTAINS_CHALLENGE]->(c:Challenge)
            RETURN c
            ORDER BY c.is_favorite DESC, c.challenge_order",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut challenges = Vec::new();

        while let Some(row) = result.next().await? {
            challenges.push(row_to_challenge(row)?);
        }

        Ok(challenges)
    }

    async fn list_by_scene(&self, scene_id: SceneId) -> Result<Vec<Challenge>> {
        let q = query(
            "MATCH (c:Challenge)-[:TIED_TO_SCENE]->(s:Scene {id: $scene_id})
            RETURN c
            ORDER BY c.is_favorite DESC, c.challenge_order",
        )
        .param("scene_id", scene_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut challenges = Vec::new();

        while let Some(row) = result.next().await? {
            challenges.push(row_to_challenge(row)?);
        }

        Ok(challenges)
    }

    async fn list_by_location(&self, location_id: LocationId) -> Result<Vec<Challenge>> {
        let q = query(
            "MATCH (c:Challenge)-[:AVAILABLE_AT]->(l:Location {id: $location_id})
            WHERE c.active = true
            RETURN c
            ORDER BY c.is_favorite DESC, c.challenge_order",
        )
        .param("location_id", location_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut challenges = Vec::new();

        while let Some(row) = result.next().await? {
            challenges.push(row_to_challenge(row)?);
        }

        Ok(challenges)
    }

    async fn list_active(&self, world_id: WorldId) -> Result<Vec<Challenge>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:CONTAINS_CHALLENGE]->(c:Challenge {active: true})
            RETURN c
            ORDER BY c.challenge_order",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut challenges = Vec::new();

        while let Some(row) = result.next().await? {
            challenges.push(row_to_challenge(row)?);
        }

        Ok(challenges)
    }

    async fn list_favorites(&self, world_id: WorldId) -> Result<Vec<Challenge>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:CONTAINS_CHALLENGE]->(c:Challenge {is_favorite: true})
            RETURN c
            ORDER BY c.challenge_order",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut challenges = Vec::new();

        while let Some(row) = result.next().await? {
            challenges.push(row_to_challenge(row)?);
        }

        Ok(challenges)
    }

    async fn update(&self, challenge: &Challenge) -> Result<()> {
        let outcomes_json =
            serde_json::to_string(&OutcomesRequestDto::from(challenge.outcomes.clone()))?;
        let triggers_json = serde_json::to_string(
            &challenge
                .trigger_conditions
                .iter()
                .cloned()
                .map(TriggerConditionRequestDto::from)
                .collect::<Vec<_>>(),
        )?;
        let tags_json = serde_json::to_string(&challenge.tags)?;

        let q = query(
            "MATCH (c:Challenge {id: $id})
            SET c.name = $name,
                c.description = $description,
                c.challenge_type = $challenge_type,
                c.difficulty_json = $difficulty_json,
                c.outcomes_json = $outcomes_json,
                c.triggers_json = $triggers_json,
                c.active = $active,
                c.challenge_order = $challenge_order,
                c.is_favorite = $is_favorite,
                c.tags_json = $tags_json
            RETURN c.id as id",
        )
        .param("id", challenge.id.to_string())
        .param("name", challenge.name.clone())
        .param("description", challenge.description.clone())
        .param("challenge_type", format!("{:?}", challenge.challenge_type))
        .param(
            "difficulty_json",
            serde_json::to_string(&DifficultyRequestDto::from(challenge.difficulty.clone()))?,
        )
        .param("outcomes_json", outcomes_json)
        .param("triggers_json", triggers_json)
        .param("active", challenge.active)
        .param("challenge_order", challenge.order as i64)
        .param("is_favorite", challenge.is_favorite)
        .param("tags_json", tags_json);

        self.connection.graph().run(q).await?;
        tracing::debug!("Updated challenge: {}", challenge.name);
        Ok(())
    }

    async fn delete(&self, id: ChallengeId) -> Result<()> {
        let q = query(
            "MATCH (c:Challenge {id: $id})
            DETACH DELETE c",
        )
        .param("id", id.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!("Deleted challenge: {}", id);
        Ok(())
    }

    async fn set_active(&self, id: ChallengeId, active: bool) -> Result<()> {
        let q = query(
            "MATCH (c:Challenge {id: $id})
            SET c.active = $active
            RETURN c.id as id",
        )
        .param("id", id.to_string())
        .param("active", active);

        self.connection.graph().run(q).await?;
        tracing::debug!("Set challenge {} active: {}", id, active);
        Ok(())
    }

    async fn toggle_favorite(&self, id: ChallengeId) -> Result<bool> {
        let q = query(
            "MATCH (c:Challenge {id: $id})
            SET c.is_favorite = NOT coalesce(c.is_favorite, false)
            RETURN c.is_favorite as is_favorite",
        )
        .param("id", id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        if let Some(row) = result.next().await? {
            let is_favorite: bool = row.get("is_favorite")?;
            Ok(is_favorite)
        } else {
            Ok(false)
        }
    }

    // -------------------------------------------------------------------------
    // Skill Edge (REQUIRES_SKILL)
    // -------------------------------------------------------------------------

    async fn set_required_skill(
        &self,
        challenge_id: ChallengeId,
        skill_id: SkillId,
    ) -> Result<()> {
        // Remove existing skill edge first, then create new one
        let q = query(
            "MATCH (c:Challenge {id: $challenge_id})
            OPTIONAL MATCH (c)-[old:REQUIRES_SKILL]->()
            DELETE old
            WITH c
            MATCH (s:Skill {id: $skill_id})
            CREATE (c)-[:REQUIRES_SKILL]->(s)",
        )
        .param("challenge_id", challenge_id.to_string())
        .param("skill_id", skill_id.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!(
            "Set required skill {} for challenge {}",
            skill_id,
            challenge_id
        );
        Ok(())
    }

    async fn get_required_skill(&self, challenge_id: ChallengeId) -> Result<Option<SkillId>> {
        let q = query(
            "MATCH (c:Challenge {id: $challenge_id})-[:REQUIRES_SKILL]->(s:Skill)
            RETURN s.id as skill_id",
        )
        .param("challenge_id", challenge_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            let skill_id_str: String = row.get("skill_id")?;
            Ok(Some(SkillId::from_uuid(uuid::Uuid::parse_str(
                &skill_id_str,
            )?)))
        } else {
            Ok(None)
        }
    }

    async fn remove_required_skill(&self, challenge_id: ChallengeId) -> Result<()> {
        let q = query(
            "MATCH (c:Challenge {id: $challenge_id})-[r:REQUIRES_SKILL]->()
            DELETE r",
        )
        .param("challenge_id", challenge_id.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!("Removed required skill from challenge {}", challenge_id);
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Scene Edge (TIED_TO_SCENE)
    // -------------------------------------------------------------------------

    async fn tie_to_scene(&self, challenge_id: ChallengeId, scene_id: SceneId) -> Result<()> {
        // Remove existing scene edge first, then create new one
        let q = query(
            "MATCH (c:Challenge {id: $challenge_id})
            OPTIONAL MATCH (c)-[old:TIED_TO_SCENE]->()
            DELETE old
            WITH c
            MATCH (s:Scene {id: $scene_id})
            CREATE (c)-[:TIED_TO_SCENE]->(s)",
        )
        .param("challenge_id", challenge_id.to_string())
        .param("scene_id", scene_id.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!(
            "Tied challenge {} to scene {}",
            challenge_id,
            scene_id
        );
        Ok(())
    }

    async fn get_tied_scene(&self, challenge_id: ChallengeId) -> Result<Option<SceneId>> {
        let q = query(
            "MATCH (c:Challenge {id: $challenge_id})-[:TIED_TO_SCENE]->(s:Scene)
            RETURN s.id as scene_id",
        )
        .param("challenge_id", challenge_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            let scene_id_str: String = row.get("scene_id")?;
            Ok(Some(SceneId::from_uuid(uuid::Uuid::parse_str(
                &scene_id_str,
            )?)))
        } else {
            Ok(None)
        }
    }

    async fn untie_from_scene(&self, challenge_id: ChallengeId) -> Result<()> {
        let q = query(
            "MATCH (c:Challenge {id: $challenge_id})-[r:TIED_TO_SCENE]->()
            DELETE r",
        )
        .param("challenge_id", challenge_id.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!("Untied challenge {} from scene", challenge_id);
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Prerequisite Edges (REQUIRES_COMPLETION_OF)
    // -------------------------------------------------------------------------

    async fn add_prerequisite(
        &self,
        challenge_id: ChallengeId,
        prerequisite: ChallengePrerequisite,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Challenge {id: $challenge_id}), (prereq:Challenge {id: $prereq_id})
            MERGE (c)-[r:REQUIRES_COMPLETION_OF]->(prereq)
            SET r.success_required = $success_required",
        )
        .param("challenge_id", challenge_id.to_string())
        .param("prereq_id", prerequisite.challenge_id.to_string())
        .param("success_required", prerequisite.success_required);

        self.connection.graph().run(q).await?;
        tracing::debug!(
            "Added prerequisite {} for challenge {}",
            prerequisite.challenge_id,
            challenge_id
        );
        Ok(())
    }

    async fn get_prerequisites(
        &self,
        challenge_id: ChallengeId,
    ) -> Result<Vec<ChallengePrerequisite>> {
        let q = query(
            "MATCH (c:Challenge {id: $challenge_id})-[r:REQUIRES_COMPLETION_OF]->(prereq:Challenge)
            RETURN prereq.id as prereq_id, r.success_required as success_required",
        )
        .param("challenge_id", challenge_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut prerequisites = Vec::new();

        while let Some(row) = result.next().await? {
            let prereq_id_str: String = row.get("prereq_id")?;
            let success_required: bool = row.get("success_required").unwrap_or(false);

            prerequisites.push(ChallengePrerequisite {
                challenge_id: ChallengeId::from_uuid(uuid::Uuid::parse_str(&prereq_id_str)?),
                success_required,
            });
        }

        Ok(prerequisites)
    }

    async fn remove_prerequisite(
        &self,
        challenge_id: ChallengeId,
        prerequisite_id: ChallengeId,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Challenge {id: $challenge_id})-[r:REQUIRES_COMPLETION_OF]->(prereq:Challenge {id: $prereq_id})
            DELETE r",
        )
        .param("challenge_id", challenge_id.to_string())
        .param("prereq_id", prerequisite_id.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!(
            "Removed prerequisite {} from challenge {}",
            prerequisite_id,
            challenge_id
        );
        Ok(())
    }

    async fn get_dependent_challenges(
        &self,
        challenge_id: ChallengeId,
    ) -> Result<Vec<ChallengeId>> {
        let q = query(
            "MATCH (dependent:Challenge)-[:REQUIRES_COMPLETION_OF]->(c:Challenge {id: $challenge_id})
            RETURN dependent.id as dependent_id",
        )
        .param("challenge_id", challenge_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut dependents = Vec::new();

        while let Some(row) = result.next().await? {
            let dependent_id_str: String = row.get("dependent_id")?;
            dependents.push(ChallengeId::from_uuid(uuid::Uuid::parse_str(
                &dependent_id_str,
            )?));
        }

        Ok(dependents)
    }

    // -------------------------------------------------------------------------
    // Location Availability Edges (AVAILABLE_AT)
    // -------------------------------------------------------------------------

    async fn add_location_availability(
        &self,
        challenge_id: ChallengeId,
        availability: ChallengeLocationAvailability,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Challenge {id: $challenge_id}), (l:Location {id: $location_id})
            MERGE (c)-[r:AVAILABLE_AT]->(l)
            SET r.always_available = $always_available,
                r.time_restriction = $time_restriction",
        )
        .param("challenge_id", challenge_id.to_string())
        .param("location_id", availability.location_id.to_string())
        .param("always_available", availability.always_available)
        .param(
            "time_restriction",
            availability.time_restriction.unwrap_or_default(),
        );

        self.connection.graph().run(q).await?;
        tracing::debug!(
            "Added location availability {} for challenge {}",
            availability.location_id,
            challenge_id
        );
        Ok(())
    }

    async fn get_location_availabilities(
        &self,
        challenge_id: ChallengeId,
    ) -> Result<Vec<ChallengeLocationAvailability>> {
        let q = query(
            "MATCH (c:Challenge {id: $challenge_id})-[r:AVAILABLE_AT]->(l:Location)
            RETURN l.id as location_id, r.always_available as always_available, r.time_restriction as time_restriction",
        )
        .param("challenge_id", challenge_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut availabilities = Vec::new();

        while let Some(row) = result.next().await? {
            let location_id_str: String = row.get("location_id")?;
            let always_available: bool = row.get("always_available").unwrap_or(true);
            let time_restriction: String = row.get("time_restriction").unwrap_or_default();

            availabilities.push(ChallengeLocationAvailability {
                location_id: LocationId::from_uuid(uuid::Uuid::parse_str(&location_id_str)?),
                always_available,
                time_restriction: if time_restriction.is_empty() {
                    None
                } else {
                    Some(time_restriction)
                },
            });
        }

        Ok(availabilities)
    }

    async fn remove_location_availability(
        &self,
        challenge_id: ChallengeId,
        location_id: LocationId,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Challenge {id: $challenge_id})-[r:AVAILABLE_AT]->(l:Location {id: $location_id})
            DELETE r",
        )
        .param("challenge_id", challenge_id.to_string())
        .param("location_id", location_id.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!(
            "Removed location availability {} from challenge {}",
            location_id,
            challenge_id
        );
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Unlock Edges (ON_SUCCESS_UNLOCKS)
    // -------------------------------------------------------------------------

    async fn add_unlock_location(
        &self,
        challenge_id: ChallengeId,
        location_id: LocationId,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Challenge {id: $challenge_id}), (l:Location {id: $location_id})
            MERGE (c)-[:ON_SUCCESS_UNLOCKS]->(l)",
        )
        .param("challenge_id", challenge_id.to_string())
        .param("location_id", location_id.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!(
            "Added unlock location {} for challenge {}",
            location_id,
            challenge_id
        );
        Ok(())
    }

    async fn get_unlock_locations(&self, challenge_id: ChallengeId) -> Result<Vec<LocationId>> {
        let q = query(
            "MATCH (c:Challenge {id: $challenge_id})-[:ON_SUCCESS_UNLOCKS]->(l:Location)
            RETURN l.id as location_id",
        )
        .param("challenge_id", challenge_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut locations = Vec::new();

        while let Some(row) = result.next().await? {
            let location_id_str: String = row.get("location_id")?;
            locations.push(LocationId::from_uuid(uuid::Uuid::parse_str(
                &location_id_str,
            )?));
        }

        Ok(locations)
    }

    async fn remove_unlock_location(
        &self,
        challenge_id: ChallengeId,
        location_id: LocationId,
    ) -> Result<()> {
        let q = query(
            "MATCH (c:Challenge {id: $challenge_id})-[r:ON_SUCCESS_UNLOCKS]->(l:Location {id: $location_id})
            DELETE r",
        )
        .param("challenge_id", challenge_id.to_string())
        .param("location_id", location_id.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!(
            "Removed unlock location {} from challenge {}",
            location_id,
            challenge_id
        );
        Ok(())
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Convert a Neo4j row to a Challenge
fn row_to_challenge(row: Row) -> Result<Challenge> {
    let node: neo4rs::Node = row.get("c")?;

    let id_str: String = node.get("id")?;
    let world_id_str: String = node.get("world_id")?;
    let name: String = node.get("name")?;
    let description: String = node.get("description").unwrap_or_default();
    let challenge_type_str: String = node.get("challenge_type")?;
    let difficulty_json: String = node.get("difficulty_json")?;
    let outcomes_json: String = node.get("outcomes_json")?;
    let triggers_json: String = node.get("triggers_json")?;
    let active: bool = node.get("active").unwrap_or(true);
    let order: i64 = node.get("challenge_order").unwrap_or(0);
    let is_favorite: bool = node.get("is_favorite").unwrap_or(false);
    let tags_json: String = node.get("tags_json").unwrap_or_else(|_| "[]".to_string());

    Ok(Challenge {
        id: ChallengeId::from_uuid(uuid::Uuid::parse_str(&id_str)?),
        world_id: WorldId::from_uuid(uuid::Uuid::parse_str(&world_id_str)?),
        name,
        description,
        challenge_type: parse_challenge_type(&challenge_type_str),
        difficulty: serde_json::from_str::<DifficultyRequestDto>(&difficulty_json)?.into(),
        outcomes: serde_json::from_str::<OutcomesRequestDto>(&outcomes_json)?.into(),
        trigger_conditions: serde_json::from_str::<Vec<TriggerConditionRequestDto>>(&triggers_json)?
            .into_iter()
            .map(Into::into)
            .collect(),
        active,
        order: order as u32,
        is_favorite,
        tags: serde_json::from_str(&tags_json).unwrap_or_default(),
    })
}

/// Parse ChallengeType from string
fn parse_challenge_type(s: &str) -> ChallengeType {
    match s {
        "SkillCheck" => ChallengeType::SkillCheck,
        "AbilityCheck" => ChallengeType::AbilityCheck,
        "SavingThrow" => ChallengeType::SavingThrow,
        "OpposedCheck" => ChallengeType::OpposedCheck,
        "ComplexChallenge" => ChallengeType::ComplexChallenge,
        _ => ChallengeType::SkillCheck,
    }
}
