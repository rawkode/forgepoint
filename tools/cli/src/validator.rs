use crate::document::{CrossReference, ForgepointDocument};
use crate::error::{ForgepointError, Result};
use crate::schema::SchemaLoader;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub file_path: String,
    pub document_type: Option<String>,
    pub document_id: Option<String>,
    pub valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub error_type: ErrorType,
    pub severity: Severity,
    pub message: String,
    pub location: Option<Location>,
    pub rule: Option<String>,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorType {
    Schema,
    Structure,
    Reference,
    IdConflict,
    Format,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub section: Option<String>,
}

pub struct DocumentValidator {
    schema_loader: SchemaLoader,
    document_index: HashMap<String, HashMap<String, DocumentInfo>>,
}

#[derive(Debug, Clone)]
struct DocumentInfo {
    file_path: String,
    title: Option<String>,
}

impl DocumentValidator {
    pub fn new(schema_loader: SchemaLoader) -> Self {
        Self {
            schema_loader,
            document_index: HashMap::new(),
        }
    }

    /// Validate a single document
    pub fn validate_document(&mut self, doc: &ForgepointDocument) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Check if document has Forgepoint structure
        if !doc.has_forgepoint_structure() {
            return ValidationResult {
                file_path: doc.file_path.to_string_lossy().to_string(),
                document_type: None,
                document_id: None,
                valid: false,
                errors: vec![ValidationError {
                    error_type: ErrorType::Structure,
                    severity: Severity::Error,
                    message: "Document missing required Forgepoint attributes (:forgepoint-type:, :id:, :schema-version:)".to_string(),
                    location: None,
                    rule: Some("require-forgepoint-structure".to_string()),
                    suggestion: Some("Add the required attributes to the document header".to_string()),
                }],
                warnings: Vec::new(),
            };
        }

        let document_type = doc.document_type().cloned();
        let document_id = doc.document_id().cloned();

        // Validate document type exists
        if let Some(ref doc_type) = document_type {
            if !self.schema_loader.is_valid_document_type(doc_type) {
                errors.push(ValidationError {
                    error_type: ErrorType::Schema,
                    severity: Severity::Error,
                    message: format!("Unknown document type: {}", doc_type),
                    location: None,
                    rule: Some("valid-document-type".to_string()),
                    suggestion: Some("Use 'forgepoint list-types' to see available document types".to_string()),
                });
            } else {
                // Validate attributes against schema
                match self.schema_loader.validate_attributes(doc_type, &doc.attributes) {
                    Ok(schema_errors) => {
                        for error in schema_errors {
                            errors.push(ValidationError {
                                error_type: ErrorType::Schema,
                                severity: Severity::Error,
                                message: error,
                                location: Some(Location {
                                    line: None,
                                    column: None,
                                    section: Some("attributes".to_string()),
                                }),
                                rule: Some("schema-validation".to_string()),
                                suggestion: None,
                            });
                        }
                    }
                    Err(e) => {
                        errors.push(ValidationError {
                            error_type: ErrorType::Schema,
                            severity: Severity::Error,
                            message: format!("Schema validation failed: {}", e),
                            location: None,
                            rule: Some("schema-validation".to_string()),
                            suggestion: None,
                        });
                    }
                }

                // Validate required sections
                let required_sections = self.schema_loader.get_required_sections(doc_type);
                let document_sections: Vec<String> = doc.level_2_sections()
                    .iter()
                    .map(|s| s.title.clone())
                    .collect();

                for required_section in required_sections {
                    if !document_sections.contains(&required_section) {
                        errors.push(ValidationError {
                            error_type: ErrorType::Structure,
                            severity: Severity::Error,
                            message: format!("Missing required section: {}", required_section),
                            location: None,
                            rule: Some("required-sections".to_string()),
                            suggestion: Some(format!("Add a '== {}' section to your document", required_section)),
                        });
                    }
                }

                // Validate abstract requirement
                if self.schema_loader.is_abstract_required(doc_type) && doc.abstract_content().is_none() {
                    errors.push(ValidationError {
                        error_type: ErrorType::Structure,
                        severity: Severity::Error,
                        message: "Document requires an abstract".to_string(),
                        location: None,
                        rule: Some("required-abstract".to_string()),
                        suggestion: Some("Add an [abstract] block after the title".to_string()),
                    });
                }

                // Validate title format
                if let Some(title_format) = self.schema_loader.get_title_format(doc_type) {
                    if title_format.contains('{') {
                        warnings.push(ValidationError {
                            error_type: ErrorType::Format,
                            severity: Severity::Warning,
                            message: format!("Consider following the recommended title format: {}", title_format),
                            location: None,
                            rule: Some("title-format".to_string()),
                            suggestion: None,
                        });
                    }
                }
            }
        }

        // Validate ID format
        if let Err(e) = doc.validate_id_format() {
            errors.push(ValidationError {
                error_type: ErrorType::Format,
                severity: Severity::Error,
                message: e.to_string(),
                location: None,
                rule: Some("id-format".to_string()),
                suggestion: Some("Use only lowercase letters, numbers, and hyphens".to_string()),
            });
        }

        // Validate cross-references
        let reference_errors = self.validate_references(doc);
        errors.extend(reference_errors.errors);
        warnings.extend(reference_errors.warnings);

        // Index this document for cross-reference validation
        self.index_document(doc);

        ValidationResult {
            file_path: doc.file_path.to_string_lossy().to_string(),
            document_type,
            document_id,
            valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    /// Validate cross-references in a document
    fn validate_references(&self, doc: &ForgepointDocument) -> ValidationResults {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        let references = doc.extract_cross_references();

        for reference in references {
            if reference.external {
                // External references - warn that they can't be validated
                warnings.push(ValidationError {
                    error_type: ErrorType::Reference,
                    severity: Severity::Warning,
                    message: format!("External reference cannot be validated: {}:{}", reference.ref_type, reference.id),
                    location: reference.line_number.map(|line| Location {
                        line: Some(line),
                        column: None,
                        section: None,
                    }),
                    rule: Some("external-reference".to_string()),
                    suggestion: None,
                });
            } else {
                // Internal references - check if target exists in index
                let target_exists = self
                    .document_index
                    .get(&reference.ref_type)
                    .and_then(|docs| docs.get(&reference.id))
                    .is_some();

                if !target_exists {
                    errors.push(ValidationError {
                        error_type: ErrorType::Reference,
                        severity: Severity::Error,
                        message: format!("Reference to non-existent document: {}:{}", reference.ref_type, reference.id),
                        location: reference.line_number.map(|line| Location {
                            line: Some(line),
                            column: None,
                            section: None,
                        }),
                        rule: Some("reference-integrity".to_string()),
                        suggestion: Some("Create the referenced document or fix the reference".to_string()),
                    });
                }
            }
        }

        ValidationResults { errors, warnings }
    }

    /// Index a document for cross-reference validation
    fn index_document(&mut self, doc: &ForgepointDocument) {
        if let (Some(doc_type), Some(doc_id)) = (doc.document_type(), doc.document_id()) {
            let doc_type = doc_type.clone();
            let doc_id = doc_id.clone();

            self.document_index
                .entry(doc_type)
                .or_insert_with(HashMap::new)
                .insert(
                    doc_id,
                    DocumentInfo {
                        file_path: doc.file_path.to_string_lossy().to_string(),
                        title: doc.title.clone(),
                    },
                );
        }
    }

    /// Check for duplicate IDs across all documents
    pub fn check_id_uniqueness(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        let mut all_ids: HashMap<String, Vec<(String, String)>> = HashMap::new(); // id -> [(type, file_path)]

        // Collect all IDs
        for (doc_type, docs) in &self.document_index {
            for (doc_id, doc_info) in docs {
                all_ids
                    .entry(doc_id.clone())
                    .or_insert_with(Vec::new)
                    .push((doc_type.clone(), doc_info.file_path.clone()));
            }
        }

        // Find duplicates
        for (id, occurrences) in all_ids {
            if occurrences.len() > 1 {
                for (doc_type, file_path) in &occurrences {
                    let other_occurrences: Vec<String> = occurrences
                        .iter()
                        .filter(|(_, fp)| fp != file_path)
                        .map(|(dt, fp)| format!("{} in {}", dt, fp))
                        .collect();

                    errors.push(ValidationError {
                        error_type: ErrorType::IdConflict,
                        severity: Severity::Error,
                        message: format!(
                            "Duplicate ID '{}' found in {} (conflicts with {})",
                            id,
                            doc_type,
                            other_occurrences.join(", ")
                        ),
                        location: None,
                        rule: Some("unique-ids".to_string()),
                        suggestion: Some("Change one of the conflicting IDs".to_string()),
                    });
                }
            }
        }

        errors
    }

    /// Clear the document index
    pub fn clear_index(&mut self) {
        self.document_index.clear();
    }

    /// Get the document index
    pub fn get_document_index(&self) -> &HashMap<String, HashMap<String, DocumentInfo>> {
        &self.document_index
    }
}

struct ValidationResults {
    errors: Vec<ValidationError>,
    warnings: Vec<ValidationError>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::DocumentParser;
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn test_validate_document_structure() {
        let mut schema_loader = SchemaLoader::new("test");
        let validator = DocumentValidator::new(schema_loader);

        // Test document without Forgepoint structure
        let doc = ForgepointDocument {
            file_path: PathBuf::from("test.adoc"),
            title: Some("Test".to_string()),
            attributes: HashMap::new(),
            content: "test content".to_string(),
            sections: Vec::new(),
        };

        let mut validator = DocumentValidator::new(SchemaLoader::new("test"));
        let result = validator.validate_document(&doc);

        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].message.contains("missing required Forgepoint attributes"));
    }
}