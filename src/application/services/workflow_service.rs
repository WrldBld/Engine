//! Workflow Service - Parsing and managing ComfyUI workflow configurations
//!
//! This service handles:
//! - Parsing ComfyUI API format workflow JSON
//! - Extracting configurable inputs from workflows
//! - Applying prompts and overrides to workflows
//! - Managing workflow configurations

use anyhow::{anyhow, Result};
use rand::Rng;

use crate::application::dto::WorkflowConfigExportDto;
use crate::domain::entities::{
    InputDefault, InputType, PromptMapping, PromptMappingType, WorkflowAnalysis,
    WorkflowConfiguration, WorkflowInput,
};

/// Service for working with ComfyUI workflows
pub struct WorkflowService;

impl WorkflowService {
    /// Analyze a ComfyUI API format workflow JSON
    ///
    /// Extracts all configurable inputs (non-connection values) from the workflow.
    pub fn analyze_workflow(workflow_json: &serde_json::Value) -> WorkflowAnalysis {
        let mut inputs = Vec::new();
        let mut text_inputs = Vec::new();
        let mut errors = Vec::new();
        let mut node_count = 0;

        // The workflow should be an object with node IDs as keys
        let nodes = match workflow_json.as_object() {
            Some(nodes) => nodes,
            None => {
                errors.push("Workflow JSON must be an object with node IDs as keys".to_string());
                return WorkflowAnalysis {
                    node_count: 0,
                    inputs,
                    text_inputs,
                    errors,
                };
            }
        };

        for (node_id, node) in nodes {
            node_count += 1;

            // Get the class_type (node type)
            let class_type = node
                .get("class_type")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string();

            // Get the node title from _meta if available
            let node_title = node
                .get("_meta")
                .and_then(|m| m.get("title"))
                .and_then(|t| t.as_str())
                .map(String::from);

            // Get inputs
            let node_inputs = match node.get("inputs").and_then(|v| v.as_object()) {
                Some(inputs) => inputs,
                None => continue, // Node has no inputs
            };

            for (input_name, value) in node_inputs {
                // Skip connection inputs (arrays like ["node_id", output_index])
                if value.is_array() {
                    continue;
                }

                let input_type = InputType::from_value(value);

                let workflow_input = WorkflowInput {
                    node_id: node_id.clone(),
                    node_type: class_type.clone(),
                    node_title: node_title.clone(),
                    input_name: input_name.clone(),
                    input_type: input_type.clone(),
                    current_value: value.clone(),
                };

                // Track text inputs separately (potential prompt fields)
                if input_type == InputType::Text {
                    text_inputs.push(workflow_input.clone());
                }

                inputs.push(workflow_input);
            }
        }

        // Sort inputs by node_id then input_name for consistent ordering
        inputs.sort_by(|a, b| {
            a.node_id
                .cmp(&b.node_id)
                .then(a.input_name.cmp(&b.input_name))
        });

        text_inputs.sort_by(|a, b| {
            a.node_id
                .cmp(&b.node_id)
                .then(a.input_name.cmp(&b.input_name))
        });

        WorkflowAnalysis {
            node_count,
            inputs,
            text_inputs,
            errors,
        }
    }

    /// Validate a workflow JSON is in ComfyUI API format
    pub fn validate_workflow(workflow_json: &serde_json::Value) -> Result<()> {
        let nodes = workflow_json
            .as_object()
            .ok_or_else(|| anyhow!("Workflow must be a JSON object"))?;

        if nodes.is_empty() {
            return Err(anyhow!("Workflow has no nodes"));
        }

        // Check that at least some nodes have the expected structure
        let mut valid_nodes = 0;
        for (_node_id, node) in nodes {
            if !node.is_object() {
                continue;
            }

            // Check for class_type (required in API format)
            if node.get("class_type").is_some() {
                valid_nodes += 1;
            }
        }

        if valid_nodes == 0 {
            return Err(anyhow!(
                "No valid ComfyUI nodes found. Make sure you're using the API format (Save API Format from ComfyUI)"
            ));
        }

        Ok(())
    }

    /// Prepare a workflow for execution by applying prompt and overrides
    ///
    /// Returns a modified copy of the workflow with:
    /// - Prompts injected into mapped text fields
    /// - Default values applied
    /// - Override values applied
    /// - Random seed generated (if seed input exists)
    pub fn prepare_workflow(
        config: &WorkflowConfiguration,
        prompt: &str,
        negative_prompt: Option<&str>,
        overrides: &[InputDefault],
    ) -> Result<serde_json::Value> {
        let mut workflow = config.workflow_json.clone();

        // Apply prompt mappings
        for mapping in &config.prompt_mappings {
            let text = match mapping.mapping_type {
                PromptMappingType::Primary => prompt.to_string(),
                PromptMappingType::Negative => {
                    negative_prompt.unwrap_or("").to_string()
                }
            };

            Self::set_input(&mut workflow, &mapping.node_id, &mapping.input_name, text.into())?;
        }

        // Apply defaults (for inputs not in overrides)
        for default in &config.input_defaults {
            let is_overridden = overrides
                .iter()
                .any(|o| o.node_id == default.node_id && o.input_name == default.input_name);

            if !is_overridden {
                Self::set_input(
                    &mut workflow,
                    &default.node_id,
                    &default.input_name,
                    default.default_value.clone(),
                )?;
            }
        }

        // Apply overrides
        for override_val in overrides {
            Self::set_input(
                &mut workflow,
                &override_val.node_id,
                &override_val.input_name,
                override_val.default_value.clone(),
            )?;
        }

        // Randomize seed inputs
        Self::randomize_seeds(&mut workflow);

        Ok(workflow)
    }

    /// Set an input value in the workflow
    fn set_input(
        workflow: &mut serde_json::Value,
        node_id: &str,
        input_name: &str,
        value: serde_json::Value,
    ) -> Result<()> {
        let node = workflow
            .get_mut(node_id)
            .ok_or_else(|| anyhow!("Node '{}' not found in workflow", node_id))?;

        let inputs = node
            .get_mut("inputs")
            .ok_or_else(|| anyhow!("Node '{}' has no inputs", node_id))?;

        inputs[input_name] = value;
        Ok(())
    }

    /// Randomize all seed inputs in the workflow
    ///
    /// ComfyUI won't generate new images if the seed hasn't changed,
    /// so we randomize seeds to ensure unique outputs.
    fn randomize_seeds(workflow: &mut serde_json::Value) {
        let mut rng = rand::thread_rng();

        if let Some(nodes) = workflow.as_object_mut() {
            for (_node_id, node) in nodes {
                if let Some(inputs) = node.get_mut("inputs").and_then(|i| i.as_object_mut()) {
                    // Look for common seed input names
                    for seed_name in ["seed", "noise_seed", "random_seed"] {
                        if inputs.contains_key(seed_name) {
                            // Generate a random i64 seed
                            let new_seed: i64 = rng.gen();
                            inputs.insert(
                                seed_name.to_string(),
                                serde_json::Value::Number(new_seed.into()),
                            );
                        }
                    }
                }
            }
        }
    }

    /// Find common node types in a workflow
    pub fn find_nodes_by_type(
        workflow: &serde_json::Value,
        class_type: &str,
    ) -> Vec<(String, serde_json::Value)> {
        let mut found = Vec::new();

        if let Some(nodes) = workflow.as_object() {
            for (node_id, node) in nodes {
                if let Some(ct) = node.get("class_type").and_then(|v| v.as_str()) {
                    if ct == class_type {
                        found.push((node_id.clone(), node.clone()));
                    }
                }
            }
        }

        found
    }

    /// Auto-detect prompt mappings from common node types
    pub fn auto_detect_prompt_mappings(workflow: &serde_json::Value) -> Vec<PromptMapping> {
        let mut mappings = Vec::new();

        // Look for CLIPTextEncode nodes (most common for prompts)
        let clip_nodes = Self::find_nodes_by_type(workflow, "CLIPTextEncode");

        for (node_id, node) in clip_nodes {
            // Check the node title to guess if it's positive or negative
            let title = node
                .get("_meta")
                .and_then(|m| m.get("title"))
                .and_then(|t| t.as_str())
                .unwrap_or("");

            let is_negative = title.to_lowercase().contains("negative")
                || title.to_lowercase().contains("neg");

            let mapping_type = if is_negative {
                PromptMappingType::Negative
            } else {
                // Default to primary for the first positive prompt found
                if mappings.iter().any(|m: &PromptMapping| m.mapping_type == PromptMappingType::Primary) {
                    continue; // Skip additional positive prompts
                }
                PromptMappingType::Primary
            };

            mappings.push(PromptMapping {
                node_id: node_id.clone(),
                input_name: "text".to_string(),
                mapping_type,
            });
        }

        mappings
    }

    /// Export all workflow configurations to a single JSON for backup
    pub fn export_configs(configs: &[WorkflowConfiguration]) -> serde_json::Value {
        let exported: Vec<WorkflowConfigExportDto> =
            configs.iter().cloned().map(Into::into).collect();
        serde_json::json!({
            "version": "1.0",
            "exported_at": chrono::Utc::now().to_rfc3339(),
            "workflows": exported,
        })
    }

    /// Import workflow configurations from exported JSON
    pub fn import_configs(json: &serde_json::Value) -> Result<Vec<WorkflowConfiguration>> {
        let workflows = json
            .get("workflows")
            .ok_or_else(|| anyhow!("Missing 'workflows' field in import data"))?;

        let dtos: Vec<WorkflowConfigExportDto> = serde_json::from_value(workflows.clone())
            .map_err(|e| anyhow!("Failed to parse workflow configurations: {}", e))?;

        let configs: Vec<WorkflowConfiguration> = dtos
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>>>()?;

        Ok(configs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_workflow() -> serde_json::Value {
        serde_json::json!({
            "3": {
                "class_type": "KSampler",
                "_meta": {"title": "KSampler"},
                "inputs": {
                    "seed": 12345,
                    "steps": 20,
                    "cfg": 7.5,
                    "sampler_name": "euler_ancestral",
                    "scheduler": "normal",
                    "denoise": 1.0,
                    "model": ["4", 0],
                    "positive": ["6", 0],
                    "negative": ["7", 0],
                    "latent_image": ["5", 0]
                }
            },
            "5": {
                "class_type": "EmptyLatentImage",
                "_meta": {"title": "Empty Latent Image"},
                "inputs": {
                    "width": 512,
                    "height": 512,
                    "batch_size": 1
                }
            },
            "6": {
                "class_type": "CLIPTextEncode",
                "_meta": {"title": "Positive Prompt"},
                "inputs": {
                    "text": "a beautiful landscape",
                    "clip": ["4", 1]
                }
            },
            "7": {
                "class_type": "CLIPTextEncode",
                "_meta": {"title": "Negative Prompt"},
                "inputs": {
                    "text": "ugly, blurry",
                    "clip": ["4", 1]
                }
            }
        })
    }

    #[test]
    fn test_analyze_workflow() {
        let workflow = sample_workflow();
        let analysis = WorkflowService::analyze_workflow(&workflow);

        assert_eq!(analysis.node_count, 4);
        assert!(analysis.errors.is_empty());

        // Should have extracted non-connection inputs
        assert!(analysis.inputs.len() > 0);

        // Should have found prompt text inputs (positive and negative prompts)
        let prompt_text_inputs: Vec<_> = analysis
            .text_inputs
            .iter()
            .filter(|i| i.node_type == "CLIPTextEncode" && i.input_name == "text")
            .collect();
        assert_eq!(prompt_text_inputs.len(), 2);

        // Should not include connection inputs (arrays)
        assert!(!analysis
            .inputs
            .iter()
            .any(|i| i.input_name == "model" || i.input_name == "positive"));
    }

    #[test]
    fn test_validate_workflow() {
        let workflow = sample_workflow();
        assert!(WorkflowService::validate_workflow(&workflow).is_ok());

        // Invalid: not an object
        let invalid = serde_json::json!([1, 2, 3]);
        assert!(WorkflowService::validate_workflow(&invalid).is_err());

        // Invalid: empty object
        let empty = serde_json::json!({});
        assert!(WorkflowService::validate_workflow(&empty).is_err());
    }

    #[test]
    fn test_auto_detect_prompt_mappings() {
        let workflow = sample_workflow();
        let mappings = WorkflowService::auto_detect_prompt_mappings(&workflow);

        assert_eq!(mappings.len(), 2);

        // Should have detected primary prompt
        assert!(mappings
            .iter()
            .any(|m| m.mapping_type == PromptMappingType::Primary));

        // Should have detected negative prompt
        assert!(mappings
            .iter()
            .any(|m| m.mapping_type == PromptMappingType::Negative));
    }

    #[test]
    fn test_input_type_detection() {
        assert_eq!(
            InputType::from_value(&serde_json::json!("hello")),
            InputType::Text
        );
        assert_eq!(
            InputType::from_value(&serde_json::json!(42)),
            InputType::Integer
        );
        assert_eq!(
            InputType::from_value(&serde_json::json!(3.14)),
            InputType::Float
        );
        assert_eq!(
            InputType::from_value(&serde_json::json!(true)),
            InputType::Boolean
        );
    }
}
