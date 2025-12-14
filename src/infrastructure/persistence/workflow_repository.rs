//! Neo4j repository for workflow configurations

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use neo4rs::query;
use uuid::Uuid;

use crate::application::ports::outbound::WorkflowRepositoryPort;
use crate::application::dto::{InputDefaultDto, PromptMappingDto};
use crate::domain::entities::{
    InputDefault, PromptMapping, WorkflowConfiguration, WorkflowSlot,
};
use crate::domain::value_objects::WorkflowConfigId;
use crate::infrastructure::persistence::Neo4jConnection;

/// Repository for workflow configuration persistence
#[derive(Clone)]
pub struct Neo4jWorkflowRepository {
    connection: Neo4jConnection,
}

impl Neo4jWorkflowRepository {
    pub fn new(connection: Neo4jConnection) -> Self {
        Self { connection }
    }

    /// Create or update a workflow configuration
    pub async fn save(&self, config: &WorkflowConfiguration) -> Result<()> {
        let id = config.id.to_string();
        let slot = config.slot.as_str();
        let name = &config.name;
        let workflow_json = serde_json::to_string(&config.workflow_json)?;
        let prompt_mappings = serde_json::to_string(
            &config
                .prompt_mappings
                .iter()
                .cloned()
                .map(PromptMappingDto::from)
                .collect::<Vec<_>>(),
        )?;
        let input_defaults = serde_json::to_string(
            &config
                .input_defaults
                .iter()
                .cloned()
                .map(InputDefaultDto::from)
                .collect::<Vec<_>>(),
        )?;
        let locked_inputs = serde_json::to_string(&config.locked_inputs)?;
        let created_at = config.created_at.to_rfc3339();
        let updated_at = config.updated_at.to_rfc3339();

        let q = query(
            r#"
            MERGE (w:WorkflowConfiguration {slot: $slot})
            SET w.id = $id,
                w.name = $name,
                w.workflow_json = $workflow_json,
                w.prompt_mappings = $prompt_mappings,
                w.input_defaults = $input_defaults,
                w.locked_inputs = $locked_inputs,
                w.created_at = $created_at,
                w.updated_at = $updated_at
            RETURN w
            "#,
        )
        .param("id", id)
        .param("slot", slot)
        .param("name", name.clone())
        .param("workflow_json", workflow_json)
        .param("prompt_mappings", prompt_mappings)
        .param("input_defaults", input_defaults)
        .param("locked_inputs", locked_inputs)
        .param("created_at", created_at)
        .param("updated_at", updated_at);

        self.connection.graph().run(q).await?;
        Ok(())
    }

    /// Get a workflow configuration by slot
    pub async fn get_by_slot(&self, slot: WorkflowSlot) -> Result<Option<WorkflowConfiguration>> {
        let slot_str = slot.as_str();

        let q = query(
            r#"
            MATCH (w:WorkflowConfiguration {slot: $slot})
            RETURN w.id AS id,
                   w.slot AS slot,
                   w.name AS name,
                   w.workflow_json AS workflow_json,
                   w.prompt_mappings AS prompt_mappings,
                   w.input_defaults AS input_defaults,
                   w.locked_inputs AS locked_inputs,
                   w.created_at AS created_at,
                   w.updated_at AS updated_at
            "#,
        )
        .param("slot", slot_str);

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            let config = self.row_to_config(&row)?;
            Ok(Some(config))
        } else {
            Ok(None)
        }
    }

    /// Get all workflow configurations
    pub async fn list_all(&self) -> Result<Vec<WorkflowConfiguration>> {
        let q = query(
            r#"
            MATCH (w:WorkflowConfiguration)
            RETURN w.id AS id,
                   w.slot AS slot,
                   w.name AS name,
                   w.workflow_json AS workflow_json,
                   w.prompt_mappings AS prompt_mappings,
                   w.input_defaults AS input_defaults,
                   w.locked_inputs AS locked_inputs,
                   w.created_at AS created_at,
                   w.updated_at AS updated_at
            ORDER BY w.slot
            "#,
        );

        let mut result = self.connection.graph().execute(q).await?;
        let mut configs = Vec::new();

        while let Some(row) = result.next().await? {
            let config = self.row_to_config(&row)?;
            configs.push(config);
        }

        Ok(configs)
    }

    /// Delete a workflow configuration by slot
    pub async fn delete_by_slot(&self, slot: WorkflowSlot) -> Result<bool> {
        let slot_str = slot.as_str();

        let q = query(
            r#"
            MATCH (w:WorkflowConfiguration {slot: $slot})
            DELETE w
            RETURN count(w) AS deleted
            "#,
        )
        .param("slot", slot_str);

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            let deleted: i64 = row.get("deleted").unwrap_or(0);
            Ok(deleted > 0)
        } else {
            Ok(false)
        }
    }

    /// Check which slots have configurations
    pub async fn get_configured_slots(&self) -> Result<Vec<WorkflowSlot>> {
        let q = query(
            r#"
            MATCH (w:WorkflowConfiguration)
            RETURN w.slot AS slot
            ORDER BY w.slot
            "#,
        );

        let mut result = self.connection.graph().execute(q).await?;
        let mut slots = Vec::new();

        while let Some(row) = result.next().await? {
            let slot_str: String = row.get("slot").unwrap_or_default();
            if let Some(slot) = WorkflowSlot::from_str(&slot_str) {
                slots.push(slot);
            }
        }

        Ok(slots)
    }

    /// Convert a Neo4j row to a WorkflowConfiguration
    fn row_to_config(&self, row: &neo4rs::Row) -> Result<WorkflowConfiguration> {
        let id_str: String = row.get("id").unwrap_or_default();
        let id = Uuid::parse_str(&id_str)
            .map(WorkflowConfigId::from_uuid)
            .unwrap_or_else(|_| WorkflowConfigId::new());

        let slot_str: String = row.get("slot").unwrap_or_default();
        let slot = WorkflowSlot::from_str(&slot_str)
            .ok_or_else(|| anyhow::anyhow!("Invalid workflow slot: {}", slot_str))?;

        let name: String = row.get("name").unwrap_or_default();

        let workflow_json_str: String = row.get("workflow_json").unwrap_or_default();
        let workflow_json: serde_json::Value = serde_json::from_str(&workflow_json_str)
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

        let prompt_mappings_str: String = row.get("prompt_mappings").unwrap_or_default();
        let prompt_mappings: Vec<PromptMapping> = serde_json::from_str::<Vec<PromptMappingDto>>(
            &prompt_mappings_str,
        )
        .unwrap_or_default()
        .into_iter()
        .map(Into::into)
        .collect();

        let input_defaults_str: String = row.get("input_defaults").unwrap_or_default();
        let input_defaults: Vec<InputDefault> = serde_json::from_str::<Vec<InputDefaultDto>>(
            &input_defaults_str,
        )
        .unwrap_or_default()
        .into_iter()
        .map(Into::into)
        .collect();

        let locked_inputs_str: String = row.get("locked_inputs").unwrap_or_default();
        let locked_inputs: Vec<String> =
            serde_json::from_str(&locked_inputs_str).unwrap_or_default();

        let created_at_str: String = row.get("created_at").unwrap_or_default();
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let updated_at_str: String = row.get("updated_at").unwrap_or_default();
        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(WorkflowConfiguration {
            id,
            slot,
            name,
            workflow_json,
            prompt_mappings,
            input_defaults,
            locked_inputs,
            created_at,
            updated_at,
        })
    }
}

// =============================================================================
// Port Implementation
// =============================================================================

#[async_trait]
impl WorkflowRepositoryPort for Neo4jWorkflowRepository {
    async fn save(&self, config: &WorkflowConfiguration) -> Result<()> {
        self.save(config).await
    }

    async fn get_by_slot(&self, slot: WorkflowSlot) -> Result<Option<WorkflowConfiguration>> {
        self.get_by_slot(slot).await
    }

    async fn delete_by_slot(&self, slot: WorkflowSlot) -> Result<bool> {
        self.delete_by_slot(slot).await
    }

    async fn list_all(&self) -> Result<Vec<WorkflowConfiguration>> {
        self.list_all().await
    }
}
