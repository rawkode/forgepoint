use crate::error::{ForgepointError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgepointDocument {
    pub file_path: PathBuf,
    pub title: Option<String>,
    pub attributes: HashMap<String, String>,
    pub content: String,
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    pub level: usize,
    pub title: String,
    pub content: String,
    pub line_number: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossReference {
    pub ref_type: String,
    pub id: String,
    pub line_number: Option<usize>,
    pub external: bool,
    pub version: Option<String>,
    pub repository: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecklistItem {
    pub text: String,
    pub checked: bool,
    pub line_number: usize,
}

impl ForgepointDocument {
    /// Check if document has the required Forgepoint structure
    pub fn has_forgepoint_structure(&self) -> bool {
        let required_attrs = ["forgepoint-type", "id", "schema-version"];
        required_attrs
            .iter()
            .all(|attr| self.attributes.contains_key(*attr))
    }

    /// Get the document type
    pub fn document_type(&self) -> Option<&String> {
        self.attributes.get("forgepoint-type")
    }

    /// Get the document ID
    pub fn document_id(&self) -> Option<&String> {
        self.attributes.get("id")
    }

    /// Get the schema version
    pub fn schema_version(&self) -> Option<&String> {
        self.attributes.get("schema-version")
    }

    /// Validate ID format
    pub fn validate_id_format(&self) -> Result<()> {
        let id = self.document_id().ok_or_else(|| {
            ForgepointError::InvalidIdFormat("Missing document ID".to_string())
        })?;

        // ID must be lowercase alphanumeric with hyphens
        let id_regex = regex::Regex::new(r"^[a-z0-9-]+$").unwrap();
        if !id_regex.is_match(id) {
            return Err(ForgepointError::InvalidIdFormat(format!(
                "ID '{}' must contain only lowercase letters, numbers, and hyphens",
                id
            )));
        }

        // ID cannot start or end with hyphen
        if id.starts_with('-') || id.ends_with('-') {
            return Err(ForgepointError::InvalidIdFormat(
                "ID cannot start or end with a hyphen".to_string(),
            ));
        }

        // ID cannot contain consecutive hyphens
        if id.contains("--") {
            return Err(ForgepointError::InvalidIdFormat(
                "ID cannot contain consecutive hyphens".to_string(),
            ));
        }

        Ok(())
    }

    /// Get all sections with a specific title
    pub fn sections_with_title(&self, title: &str) -> Vec<&Section> {
        self.sections
            .iter()
            .filter(|section| section.title == title)
            .collect()
    }

    /// Get all level-2 sections (== sections)
    pub fn level_2_sections(&self) -> Vec<&Section> {
        self.sections
            .iter()
            .filter(|section| section.level == 2)
            .collect()
    }

    /// Get the abstract content if it exists
    pub fn abstract_content(&self) -> Option<String> {
        // Look for [abstract] block in content
        let lines: Vec<&str> = self.content.lines().collect();
        let mut in_abstract = false;
        let mut abstract_lines = Vec::new();

        for line in lines {
            if line.trim() == "[abstract]" {
                in_abstract = true;
                continue;
            }

            if in_abstract {
                if line.starts_with('=') || line.starts_with('[') && line.ends_with(']') {
                    // End of abstract block
                    break;
                }
                if line.trim().is_empty() && !abstract_lines.is_empty() {
                    // End of abstract on empty line
                    break;
                }
                if !line.trim().is_empty() {
                    abstract_lines.push(line);
                }
            }
        }

        if abstract_lines.is_empty() {
            None
        } else {
            Some(abstract_lines.join("\n").trim().to_string())
        }
    }

    /// Extract cross-references from the document
    pub fn extract_cross_references(&self) -> Vec<CrossReference> {
        let mut references = Vec::new();
        let lines: Vec<&str> = self.content.lines().collect();

        // Regex for internal references: xref:type:id[]
        let internal_regex = regex::Regex::new(r"xref:([a-z-]+):([a-z0-9-]+)(?:\[[^\]]*\])?").unwrap();
        
        // Regex for external references: xref:github.com/org/repo#type:id@version[]
        let external_regex = regex::Regex::new(r"xref:([^#]+)#([a-z-]+):([a-z0-9-]+)(?:@([^[\]]+))?(?:\[[^\]]*\])?").unwrap();

        for (line_no, line) in lines.iter().enumerate() {
            // Check for internal references
            for cap in internal_regex.captures_iter(line) {
                references.push(CrossReference {
                    ref_type: cap[1].to_string(),
                    id: cap[2].to_string(),
                    line_number: Some(line_no + 1),
                    external: false,
                    version: None,
                    repository: None,
                });
            }

            // Check for external references
            for cap in external_regex.captures_iter(line) {
                references.push(CrossReference {
                    ref_type: cap[2].to_string(),
                    id: cap[3].to_string(),
                    line_number: Some(line_no + 1),
                    external: true,
                    version: cap.get(4).map(|m| m.as_str().to_string()),
                    repository: Some(cap[1].to_string()),
                });
            }
        }

        references
    }

    /// Extract checklist items from the document
    pub fn extract_checklist_items(&self) -> Vec<ChecklistItem> {
        let mut items = Vec::new();
        let lines: Vec<&str> = self.content.lines().collect();

        // Regex for checklist items: * [ ] or * [x]
        let checklist_regex = regex::Regex::new(r"^\s*\*\s+\[([x ])\]\s+(.+)$").unwrap();

        for (line_no, line) in lines.iter().enumerate() {
            if let Some(cap) = checklist_regex.captures(line) {
                items.push(ChecklistItem {
                    text: cap[2].trim().to_string(),
                    checked: &cap[1] == "x",
                    line_number: line_no + 1,
                });
            }
        }

        items
    }
}