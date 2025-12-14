//! Character Sheet Template repository implementation for Neo4j

use anyhow::Result;
use async_trait::async_trait;
use neo4rs::{query, Row};

use super::connection::Neo4jConnection;
use crate::application::ports::outbound::SheetTemplateRepositoryPort;
use crate::application::dto::SheetTemplateStorageDto;
use crate::domain::entities::{CharacterSheetTemplate, SheetTemplateId};
use crate::domain::value_objects::WorldId;

/// Repository for CharacterSheetTemplate operations
pub struct Neo4jSheetTemplateRepository {
    connection: Neo4jConnection,
}

impl Neo4jSheetTemplateRepository {
    pub fn new(connection: Neo4jConnection) -> Self {
        Self { connection }
    }

    /// Create a new sheet template
    pub async fn create(&self, template: &CharacterSheetTemplate) -> Result<()> {
        // Serialize the template to JSON for storage
        let template_json = serde_json::to_string(&SheetTemplateStorageDto::from(template))?;

        let q = query(
            "MATCH (w:World {id: $world_id})
            CREATE (t:SheetTemplate {
                id: $id,
                world_id: $world_id,
                name: $name,
                is_default: $is_default,
                template_data: $template_data
            })
            CREATE (w)-[:HAS_SHEET_TEMPLATE]->(t)
            RETURN t.id as id",
        )
        .param("id", template.id.0.clone())
        .param("world_id", template.world_id.to_string())
        .param("name", template.name.clone())
        .param("is_default", template.is_default)
        .param("template_data", template_json);

        self.connection.graph().run(q).await?;
        tracing::debug!("Created sheet template: {}", template.name);
        Ok(())
    }

    /// Get a sheet template by ID
    pub async fn get(&self, id: &SheetTemplateId) -> Result<Option<CharacterSheetTemplate>> {
        let q = query(
            "MATCH (t:SheetTemplate {id: $id})
            RETURN t",
        )
        .param("id", id.0.clone());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            Ok(Some(row_to_template(row)?))
        } else {
            Ok(None)
        }
    }

    /// Get the default template for a world
    pub async fn get_default_for_world(
        &self,
        world_id: &WorldId,
    ) -> Result<Option<CharacterSheetTemplate>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_SHEET_TEMPLATE]->(t:SheetTemplate {is_default: true})
            RETURN t
            LIMIT 1",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            Ok(Some(row_to_template(row)?))
        } else {
            Ok(None)
        }
    }

    /// List all templates for a world
    pub async fn list_by_world(&self, world_id: &WorldId) -> Result<Vec<CharacterSheetTemplate>> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_SHEET_TEMPLATE]->(t:SheetTemplate)
            RETURN t
            ORDER BY t.is_default DESC, t.name",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;
        let mut templates = Vec::new();

        while let Some(row) = result.next().await? {
            templates.push(row_to_template(row)?);
        }

        Ok(templates)
    }

    /// Update a sheet template
    pub async fn update(&self, template: &CharacterSheetTemplate) -> Result<()> {
        let template_json = serde_json::to_string(&SheetTemplateStorageDto::from(template))?;

        let q = query(
            "MATCH (t:SheetTemplate {id: $id})
            SET t.name = $name,
                t.is_default = $is_default,
                t.template_data = $template_data
            RETURN t.id as id",
        )
        .param("id", template.id.0.clone())
        .param("name", template.name.clone())
        .param("is_default", template.is_default)
        .param("template_data", template_json);

        self.connection.graph().run(q).await?;
        tracing::debug!("Updated sheet template: {}", template.name);
        Ok(())
    }

    /// Delete a sheet template
    pub async fn delete(&self, id: &SheetTemplateId) -> Result<()> {
        let q = query(
            "MATCH (t:SheetTemplate {id: $id})
            DETACH DELETE t",
        )
        .param("id", id.0.clone());

        self.connection.graph().run(q).await?;
        tracing::debug!("Deleted sheet template: {}", id.0);
        Ok(())
    }

    /// Delete all templates for a world
    pub async fn delete_all_for_world(&self, world_id: &WorldId) -> Result<()> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_SHEET_TEMPLATE]->(t:SheetTemplate)
            DETACH DELETE t",
        )
        .param("world_id", world_id.to_string());

        self.connection.graph().run(q).await?;
        tracing::debug!("Deleted all sheet templates for world: {}", world_id);
        Ok(())
    }

    /// Check if a world has any templates
    pub async fn has_templates(&self, world_id: &WorldId) -> Result<bool> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:HAS_SHEET_TEMPLATE]->(t:SheetTemplate)
            RETURN count(t) as count",
        )
        .param("world_id", world_id.to_string());

        let mut result = self.connection.graph().execute(q).await?;

        if let Some(row) = result.next().await? {
            let count: i64 = row.get("count")?;
            Ok(count > 0)
        } else {
            Ok(false)
        }
    }
}

/// Convert a Neo4j row to a CharacterSheetTemplate
fn row_to_template(row: Row) -> Result<CharacterSheetTemplate> {
    let node: neo4rs::Node = row.get("t")?;

    let template_data: String = node.get("template_data")?;
    let stored: SheetTemplateStorageDto = serde_json::from_str(&template_data)?;
    let template: CharacterSheetTemplate = stored.try_into()?;

    Ok(template)
}

// =============================================================================
// SheetTemplateRepositoryPort Implementation
// =============================================================================

#[async_trait]
impl SheetTemplateRepositoryPort for Neo4jSheetTemplateRepository {
    async fn create(&self, template: &CharacterSheetTemplate) -> Result<()> {
        self.create(template).await
    }

    async fn get(&self, id: &SheetTemplateId) -> Result<Option<CharacterSheetTemplate>> {
        self.get(id).await
    }

    async fn get_default_for_world(&self, world_id: &WorldId) -> Result<Option<CharacterSheetTemplate>> {
        self.get_default_for_world(world_id).await
    }

    async fn list_by_world(&self, world_id: &WorldId) -> Result<Vec<CharacterSheetTemplate>> {
        self.list_by_world(world_id).await
    }

    async fn update(&self, template: &CharacterSheetTemplate) -> Result<()> {
        self.update(template).await
    }

    async fn delete(&self, id: &SheetTemplateId) -> Result<()> {
        self.delete(id).await
    }

    async fn delete_all_for_world(&self, world_id: &WorldId) -> Result<()> {
        self.delete_all_for_world(world_id).await
    }

    async fn has_templates(&self, world_id: &WorldId) -> Result<bool> {
        self.has_templates(world_id).await
    }
}
