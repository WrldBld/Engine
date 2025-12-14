use serde::{Deserialize, Serialize};

use crate::domain::entities::Scene;

#[derive(Debug, Deserialize)]
pub struct CreateSceneRequestDto {
    pub name: String,
    pub location_id: String,
    #[serde(default)]
    pub time_context: Option<String>,
    #[serde(default)]
    pub backdrop_override: Option<String>,
    #[serde(default)]
    pub featured_characters: Vec<String>,
    #[serde(default)]
    pub directorial_notes: String,
    #[serde(default)]
    pub order: u32,
}

#[derive(Debug, Deserialize)]
pub struct UpdateNotesRequestDto {
    pub notes: String,
}

#[derive(Debug, Serialize)]
pub struct SceneResponseDto {
    pub id: String,
    pub act_id: String,
    pub name: String,
    pub location_id: String,
    pub time_context: String,
    pub backdrop_override: Option<String>,
    pub featured_characters: Vec<String>,
    pub directorial_notes: String,
    pub order: u32,
}

impl From<Scene> for SceneResponseDto {
    fn from(s: Scene) -> Self {
        Self {
            id: s.id.to_string(),
            act_id: s.act_id.to_string(),
            name: s.name,
            location_id: s.location_id.to_string(),
            time_context: format!("{:?}", s.time_context),
            backdrop_override: s.backdrop_override,
            featured_characters: s.featured_characters.iter().map(|c| c.to_string()).collect(),
            directorial_notes: s.directorial_notes,
            order: s.order,
        }
    }
}

