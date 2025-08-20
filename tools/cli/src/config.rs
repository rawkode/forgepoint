use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgepointConfig {
    pub schema_path: PathBuf,
    pub exclude_patterns: Vec<String>,
    pub rules: ValidationRules,
    pub output: OutputConfig,
    pub templates: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRules {
    pub require_id: bool,
    pub enforce_structure: bool,
    pub validate_references: bool,
    pub check_id_uniqueness: bool,
    pub max_title_length: Option<usize>,
    pub required_attributes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub format: String,
    pub verbose: bool,
    pub show_suggestions: bool,
    pub color: bool,
}

impl Default for ForgepointConfig {
    fn default() -> Self {
        Self {
            schema_path: PathBuf::from("schema"),
            exclude_patterns: vec![
                "node_modules/**".to_string(),
                "target/**".to_string(),
                "dist/**".to_string(),
                ".git/**".to_string(),
                "*.tmp.adoc".to_string(),
            ],
            rules: ValidationRules {
                require_id: true,
                enforce_structure: true,
                validate_references: true,
                check_id_uniqueness: true,
                max_title_length: Some(100),
                required_attributes: vec![
                    "forgepoint-type".to_string(),
                    "id".to_string(),
                    "schema-version".to_string(),
                ],
            },
            output: OutputConfig {
                format: "text".to_string(),
                verbose: false,
                show_suggestions: true,
                color: true,
            },
            templates: None,
        }
    }
}

impl ForgepointConfig {
    /// Load configuration from file or use defaults
    pub fn load(config_path: Option<&PathBuf>) -> Result<Self> {
        let config_paths = if let Some(path) = config_path {
            vec![path.clone()]
        } else {
            vec![
                PathBuf::from(".forgepoint.toml"),
                PathBuf::from(".forgepoint.yaml"),
                PathBuf::from(".forgepoint.yml"),
                PathBuf::from(".forgepointrc.json"),
                PathBuf::from("forgepoint.toml"),
            ]
        };

        for path in config_paths {
            if path.exists() {
                return Self::load_from_file(&path);
            }
        }

        // No config file found, use defaults
        Ok(Self::default())
    }

    fn load_from_file(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        
        match path.extension().and_then(|s| s.to_str()) {
            Some("toml") => {
                Ok(toml::from_str(&content)?)
            }
            Some("yaml") | Some("yml") => {
                Ok(serde_yaml::from_str(&content)?)
            }
            Some("json") => {
                Ok(serde_json::from_str(&content)?)
            }
            _ => {
                // Try to detect format from content
                if content.trim_start().starts_with('{') {
                    Ok(serde_json::from_str(&content)?)
                } else if content.contains("---") || content.contains(':') {
                    Ok(serde_yaml::from_str(&content)?)
                } else {
                    Ok(toml::from_str(&content)?)
                }
            }
        }
    }

    /// Merge CLI arguments into configuration
    pub fn merge_cli_args(
        mut self,
        schema_path: Option<PathBuf>,
        verbose: bool,
    ) -> Self {
        if let Some(path) = schema_path {
            self.schema_path = path;
        }
        
        if verbose {
            self.output.verbose = true;
        }

        self
    }

    /// Resolve relative paths to absolute paths
    pub fn resolve_paths(mut self, base_dir: Option<&PathBuf>) -> Self {
        let base = base_dir
            .cloned()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        if self.schema_path.is_relative() {
            self.schema_path = base.join(&self.schema_path);
        }

        self
    }
}