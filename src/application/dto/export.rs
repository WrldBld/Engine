use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ExportQueryDto {
    #[serde(default)]
    pub format: Option<String>,
}

