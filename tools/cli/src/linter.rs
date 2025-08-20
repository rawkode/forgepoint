use crate::config::ForgepointConfig;
use crate::document::ForgepointDocument;
use crate::parser::DocumentParser;
use crate::schema::SchemaLoader;
use crate::validator::{DocumentValidator, ValidationResult};
use crate::error::Result;
use glob::glob;
use rayon::prelude::*;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct ForgepointLinter {
    config: ForgepointConfig,
    schema_loader: SchemaLoader,
    parser: DocumentParser,
}

impl ForgepointLinter {
    pub fn new(config: ForgepointConfig) -> Self {
        let schema_loader = SchemaLoader::new(&config.schema_path);
        let parser = DocumentParser::new();

        Self {
            config,
            schema_loader,
            parser,
        }
    }

    pub fn initialize(&mut self) -> Result<()> {
        self.schema_loader.load_schemas()?;
        Ok(())
    }

    pub fn lint_files(&self, patterns: &[String]) -> Result<Vec<ValidationResult>> {
        let files = self.find_files(patterns)?;
        
        if files.is_empty() {
            return Ok(Vec::new());
        }

        let schema_loader_clone = SchemaLoader::new(&self.config.schema_path);
        let validator = Mutex::new(DocumentValidator::new(schema_loader_clone));
        
        // First pass: Parse and validate individual documents
        let results: Vec<ValidationResult> = files
            .par_iter()
            .map(|file_path| {
                match self.parser.parse_file(file_path) {
                    Ok(doc) => {
                        let mut validator = validator.lock().unwrap();
                        validator.validate_document(&doc)
                    }
                    Err(e) => self.create_parse_error_result(file_path, &e),
                }
            })
            .collect();

        // Second pass: Check for ID uniqueness across all documents
        let mut final_results = results;
        if self.config.rules.check_id_uniqueness {
            let validator = validator.into_inner().unwrap();
            let duplicate_errors = validator.check_id_uniqueness();
            
            // Add duplicate ID errors to affected files
            for error in duplicate_errors {
                for result in &mut final_results {
                    if let Some(doc_id) = &result.document_id {
                        if error.message.contains(doc_id) {
                            result.errors.push(error.clone());
                            result.valid = false;
                        }
                    }
                }
            }
        }

        Ok(final_results)
    }

    pub fn lint_file(&self, file_path: &PathBuf) -> Result<ValidationResult> {
        match self.parser.parse_file(file_path) {
            Ok(doc) => {
                let schema_loader_clone = SchemaLoader::new(&self.config.schema_path);
                let mut validator = DocumentValidator::new(schema_loader_clone);
                Ok(validator.validate_document(&doc))
            }
            Err(e) => Ok(self.create_parse_error_result(file_path, &e)),
        }
    }

    pub fn create_document_template(
        &self,
        doc_type: &str,
        id: &str,
        title: Option<&str>,
        author: Option<&str>,
    ) -> Result<String> {
        let document_types = self.schema_loader.get_document_types();
        let doc_type_def = document_types
            .iter()
            .find(|dt| dt.doc_type == doc_type)
            .ok_or_else(|| crate::error::ForgepointError::InvalidDocumentType(doc_type.to_string()))?;

        let title = title.unwrap_or(&doc_type_def.name);
        let author = author.unwrap_or("Author Name");
        let date = chrono::Utc::now().format("%Y-%m-%d").to_string();

        let required_sections = self.schema_loader.get_required_sections(doc_type);
        let is_abstract_required = self.schema_loader.is_abstract_required(doc_type);

        let mut template = format!("= {}\n", title);
        template.push_str(&format!(":forgepoint-type: {}\n", doc_type));
        template.push_str(&format!(":id: {}\n", id));
        template.push_str(":status: draft\n");
        template.push_str(&format!(":created: {}\n", date));
        template.push_str(&format!(":author: {}\n", author));
        template.push_str(":schema-version: 1.0\n\n");

        if is_abstract_required {
            template.push_str("[abstract]\n");
            template.push_str(&format!("Brief description of this {}.\n\n", doc_type_def.name.to_lowercase()));
        }

        for section in required_sections {
            template.push_str(&format!("== {}\n\n", section));
            template.push_str(&format!("// TODO: Add content for {}\n\n", section));
        }

        Ok(template)
    }

    fn find_files(&self, patterns: &[String]) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        
        for pattern in patterns {
            for entry in glob(pattern)
                .map_err(|e| crate::error::ForgepointError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Invalid glob pattern '{}': {}", pattern, e)
                )))?
            {
                let path = entry.map_err(|e| crate::error::ForgepointError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to read glob entry: {}", e)
                )))?;
                
                if DocumentParser::is_asciidoc_file(&path) {
                    let should_exclude = self.config.exclude_patterns.iter().any(|exclude_pattern| {
                        glob::Pattern::new(exclude_pattern)
                            .map(|p| p.matches_path(&path))
                            .unwrap_or(false)
                    });
                    
                    if !should_exclude {
                        files.push(path);
                    }
                }
            }
        }
        
        files.sort();
        files.dedup();
        Ok(files)
    }

    fn create_parse_error_result(&self, file_path: &PathBuf, error: &crate::error::ForgepointError) -> ValidationResult {
        use crate::validator::{ValidationError, ErrorType, Severity};
        
        ValidationResult {
            file_path: file_path.to_string_lossy().to_string(),
            document_type: None,
            document_id: None,
            valid: false,
            errors: vec![ValidationError {
                error_type: ErrorType::Format,
                severity: Severity::Error,
                message: format!("Failed to parse file: {}", error),
                location: None,
                rule: Some("file-parsing".to_string()),
                suggestion: None,
            }],
            warnings: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_linter_creation() {
        let config = ForgepointConfig::default();
        let linter = ForgepointLinter::new(config);
        
        // Just test that we can create a linter without panicking
        assert!(true);
    }

    #[test]
    fn test_find_files() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        
        // Create some test files
        fs::write(temp_path.join("test1.adoc"), "= Test 1").unwrap();
        fs::write(temp_path.join("test2.adoc"), "= Test 2").unwrap();
        fs::write(temp_path.join("readme.md"), "# Readme").unwrap();
        
        let config = ForgepointConfig::default();
        let linter = ForgepointLinter::new(config);
        
        let pattern = format!("{}/**/*.adoc", temp_path.display());
        let files = linter.find_files(&[pattern]).unwrap();
        
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.file_name().unwrap() == "test1.adoc"));
        assert!(files.iter().any(|f| f.file_name().unwrap() == "test2.adoc"));
    }
}