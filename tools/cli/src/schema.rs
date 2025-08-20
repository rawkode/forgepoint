use crate::error::{ForgepointError, Result};
use jsonschema::{Draft, JSONSchema};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaRegistry {
    #[serde(rename = "schemaVersion")]
    pub schema_version: String,
    pub schemas: HashMap<String, SchemaRef>,
    #[serde(rename = "documentTypes")]
    pub document_types: Vec<DocumentTypeDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaRef {
    #[serde(rename = "$ref")]
    pub reference: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentTypeDefinition {
    #[serde(rename = "type")]
    pub doc_type: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub schema: String,
}

#[derive(Debug, Clone)]
pub struct CompiledSchema {
    pub definition: DocumentTypeDefinition,
    pub json_schema: JSONSchema,
    pub structural_requirements: StructuralRequirements,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuralRequirements {
    pub title: Option<TitleRequirement>,
    pub sections: Option<SectionRequirements>,
    #[serde(rename = "abstract")]
    pub abstract_req: Option<AbstractRequirement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TitleRequirement {
    pub required: Option<bool>,
    pub format: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionRequirements {
    pub required: Option<Vec<String>>,
    pub optional: Option<Vec<String>>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbstractRequirement {
    pub required: Option<bool>,
    pub description: Option<String>,
}

pub struct SchemaLoader {
    schema_path: PathBuf,
    registry: Option<SchemaRegistry>,
    compiled_schemas: HashMap<String, CompiledSchema>,
}

impl SchemaLoader {
    pub fn new<P: AsRef<Path>>(schema_path: P) -> Self {
        Self {
            schema_path: schema_path.as_ref().to_path_buf(),
            registry: None,
            compiled_schemas: HashMap::new(),
        }
    }

    /// Load and compile all schemas
    pub fn load_schemas(&mut self) -> Result<()> {
        // Load the schema registry index
        let index_path = self.schema_path.join("index.json");
        if !index_path.exists() {
            return Err(ForgepointError::FileNotFound(format!(
                "Schema index not found at {}",
                index_path.display()
            )));
        }

        let index_content = fs::read_to_string(&index_path)?;
        let registry: SchemaRegistry = serde_json::from_str(&index_content)?;

        // Load and compile each schema
        for doc_type in &registry.document_types {
            self.load_schema(&doc_type.doc_type, &doc_type.schema)?;
        }

        self.registry = Some(registry);
        
        println!("Loaded {} schemas", self.compiled_schemas.len());
        Ok(())
    }

    /// Load a specific schema file
    fn load_schema(&mut self, doc_type: &str, schema_file: &str) -> Result<()> {
        let schema_path = self.schema_path.join(schema_file);
        
        if !schema_path.exists() {
            eprintln!("Warning: Schema file not found: {}", schema_path.display());
            return Ok(());
        }

        let schema_content = fs::read_to_string(&schema_path)?;
        let schema_json: Value = serde_json::from_str(&schema_content)?;

        // Extract structural requirements
        let structural_requirements = schema_json
            .get("structuralRequirements")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or(StructuralRequirements {
                title: None,
                sections: None,
                abstract_req: None,
            });

        // Compile the JSON schema
        let json_schema = JSONSchema::options()
            .with_draft(Draft::Draft7)
            .compile(&schema_json)
            .map_err(|e| ForgepointError::Schema(format!("Failed to compile schema for {}: {}", doc_type, e)))?;

        // Find the document type definition
        let definition = self
            .registry
            .as_ref()
            .and_then(|r| r.document_types.iter().find(|dt| dt.doc_type == doc_type))
            .cloned()
            .ok_or_else(|| ForgepointError::Schema(format!("Document type definition not found: {}", doc_type)))?;

        let compiled_schema = CompiledSchema {
            definition,
            json_schema,
            structural_requirements,
        };

        self.compiled_schemas.insert(doc_type.to_string(), compiled_schema);
        Ok(())
    }

    /// Get compiled schema for a document type
    pub fn get_schema(&self, doc_type: &str) -> Option<&CompiledSchema> {
        self.compiled_schemas.get(doc_type)
    }

    /// Get all available document types
    pub fn get_document_types(&self) -> Vec<DocumentTypeDefinition> {
        self.registry
            .as_ref()
            .map(|r| r.document_types.clone())
            .unwrap_or_default()
    }

    /// Check if a document type is valid
    pub fn is_valid_document_type(&self, doc_type: &str) -> bool {
        self.compiled_schemas.contains_key(doc_type)
    }

    /// Validate document attributes against schema
    pub fn validate_attributes(&self, doc_type: &str, attributes: &HashMap<String, String>) -> Result<Vec<String>> {
        let schema = self
            .get_schema(doc_type)
            .ok_or_else(|| ForgepointError::InvalidDocumentType(doc_type.to_string()))?;

        // Convert attributes to JSON for validation
        let attributes_json = serde_json::to_value(attributes)?;

        let validation_result = schema.json_schema.validate(&attributes_json);
        
        let mut errors = Vec::new();
        if let Err(validation_errors) = validation_result {
            for error in validation_errors {
                errors.push(format!("Validation error at {}: {}", error.instance_path, error));
            }
        }

        Ok(errors)
    }

    /// Get required sections for a document type
    pub fn get_required_sections(&self, doc_type: &str) -> Vec<String> {
        self.get_schema(doc_type)
            .and_then(|s| s.structural_requirements.sections.as_ref())
            .and_then(|s| s.required.clone())
            .unwrap_or_default()
    }

    /// Get optional sections for a document type
    pub fn get_optional_sections(&self, doc_type: &str) -> Vec<String> {
        self.get_schema(doc_type)
            .and_then(|s| s.structural_requirements.sections.as_ref())
            .and_then(|s| s.optional.clone())
            .unwrap_or_default()
    }

    /// Check if abstract is required for a document type
    pub fn is_abstract_required(&self, doc_type: &str) -> bool {
        self.get_schema(doc_type)
            .and_then(|s| s.structural_requirements.abstract_req.as_ref())
            .and_then(|a| a.required)
            .unwrap_or(false)
    }

    /// Get title format requirement for a document type
    pub fn get_title_format(&self, doc_type: &str) -> Option<String> {
        self.get_schema(doc_type)
            .and_then(|s| s.structural_requirements.title.as_ref())
            .and_then(|t| t.format.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_schema_loading() {
        let temp_dir = TempDir::new().unwrap();
        let schema_dir = temp_dir.path();

        // Create a simple schema index
        let index = r#"{
            "schemaVersion": "1.0",
            "schemas": {
                "story": { "$ref": "story.json" }
            },
            "documentTypes": [{
                "type": "story",
                "name": "User Story", 
                "description": "Test story",
                "category": "design",
                "schema": "story.json"
            }]
        }"#;
        fs::write(schema_dir.join("index.json"), index).unwrap();

        // Create a simple story schema
        let story_schema = r#"{
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "forgepoint-type": { "const": "story" },
                "id": { "type": "string" }
            },
            "required": ["forgepoint-type", "id"]
        }"#;
        fs::write(schema_dir.join("story.json"), story_schema).unwrap();

        let mut loader = SchemaLoader::new(schema_dir);
        assert!(loader.load_schemas().is_ok());
        assert!(loader.is_valid_document_type("story"));
        assert!(!loader.is_valid_document_type("invalid"));
    }
}