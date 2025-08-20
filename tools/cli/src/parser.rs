use crate::document::{ForgepointDocument, Section};
use crate::error::{ForgepointError, Result};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub struct DocumentParser {
    // Pre-compiled regexes for better performance
    title_regex: Regex,
    section_regex: Regex,
    attribute_regex: Regex,
}

impl DocumentParser {
    pub fn new() -> Self {
        Self {
            // Level-0 title: = Title
            title_regex: Regex::new(r"^=\s+(.+)$").unwrap(),
            // Section headers: == Section, === Subsection, etc.
            section_regex: Regex::new(r"^(=+)\s+(.+)$").unwrap(),
            // Document attributes: :key: value
            attribute_regex: Regex::new(r"^:([^:]+):\s*(.*)$").unwrap(),
        }
    }

    /// Parse an AsciiDoc file into a Forgepoint document
    pub fn parse_file<P: AsRef<Path>>(&self, file_path: P) -> Result<ForgepointDocument> {
        let path = file_path.as_ref();
        let content = fs::read_to_string(path).map_err(|e| {
            ForgepointError::Parsing(format!("Failed to read file '{}': {}", path.display(), e))
        })?;

        self.parse_content(&content, path.to_path_buf())
    }

    /// Parse AsciiDoc content into a Forgepoint document
    pub fn parse_content(&self, content: &str, file_path: std::path::PathBuf) -> Result<ForgepointDocument> {
        let lines: Vec<&str> = content.lines().collect();
        let mut title = None;
        let mut attributes = HashMap::new();
        let mut sections = Vec::new();

        let mut current_section: Option<Section> = None;
        let mut in_header = true; // We're in the document header until we hit a section

        for (line_no, line) in lines.iter().enumerate() {
            let line_number = line_no + 1;

            // Skip empty lines
            if line.trim().is_empty() {
                continue;
            }

            // Check for document title (only if we haven't found one and we're in header)
            if title.is_none() && in_header {
                if let Some(cap) = self.title_regex.captures(line) {
                    title = Some(cap[1].trim().to_string());
                    continue;
                }
            }

            // Check for document attributes (only in header)
            if in_header {
                if let Some(cap) = self.attribute_regex.captures(line) {
                    let key = cap[1].trim().to_string();
                    let value = cap[2].trim().to_string();
                    attributes.insert(key, value);
                    continue;
                }
            }

            // Check for section headers
            if let Some(cap) = self.section_regex.captures(line) {
                in_header = false; // We're no longer in the header

                // Save the previous section if exists
                if let Some(section) = current_section.take() {
                    sections.push(section);
                }

                let level = cap[1].len();
                let section_title = cap[2].trim().to_string();

                // Start a new section
                current_section = Some(Section {
                    level,
                    title: section_title,
                    content: String::new(),
                    line_number: Some(line_number),
                });
                continue;
            }

            // If we're not in header and not a section header, it's section content
            if !in_header {
                if let Some(ref mut section) = current_section {
                    if !section.content.is_empty() {
                        section.content.push('\n');
                    }
                    section.content.push_str(line);
                } else {
                    // Content before any section - create an implicit content section
                    current_section = Some(Section {
                        level: 0,
                        title: "Content".to_string(),
                        content: line.to_string(),
                        line_number: Some(line_number),
                    });
                }
            }
        }

        // Don't forget the last section
        if let Some(section) = current_section {
            sections.push(section);
        }

        Ok(ForgepointDocument {
            file_path,
            title,
            attributes,
            content: content.to_string(),
            sections,
        })
    }

    /// Quick check if a file looks like an AsciiDoc file
    pub fn is_asciidoc_file<P: AsRef<Path>>(file_path: P) -> bool {
        let path = file_path.as_ref();
        
        // Check file extension
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            matches!(ext.to_lowercase().as_str(), "adoc" | "asciidoc" | "asc")
        } else {
            false
        }
    }

    /// Check if content looks like AsciiDoc
    pub fn is_asciidoc_content(content: &str) -> bool {
        let lines: Vec<&str> = content.lines().take(20).collect(); // Check first 20 lines
        
        let mut asciidoc_indicators = 0;
        
        for line in lines {
            let trimmed = line.trim();
            
            // Check for AsciiDoc-specific patterns
            if trimmed.starts_with('=') {
                asciidoc_indicators += 2; // Titles are strong indicators
            } else if trimmed.starts_with(':') && trimmed.ends_with(':') {
                asciidoc_indicators += 1; // Attributes
            } else if trimmed.starts_with("//") {
                asciidoc_indicators += 1; // Comments
            } else if trimmed.contains("xref:") || trimmed.contains("<<") {
                asciidoc_indicators += 1; // Cross-references
            }
        }
        
        asciidoc_indicators >= 2
    }
}

impl Default for DocumentParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_parse_basic_document() {
        let content = r#"= Test Document
:forgepoint-type: story
:id: test-story
:schema-version: 1.0

[abstract]
This is a test document.

== Section One

Some content here.

== Section Two

More content here.
"#;

        let parser = DocumentParser::new();
        let doc = parser.parse_content(content, "test.adoc".into()).unwrap();

        assert_eq!(doc.title, Some("Test Document".to_string()));
        assert_eq!(doc.attributes.get("forgepoint-type"), Some(&"story".to_string()));
        assert_eq!(doc.attributes.get("id"), Some(&"test-story".to_string()));
        assert_eq!(doc.sections.len(), 2);
        assert_eq!(doc.sections[0].title, "Section One");
        assert_eq!(doc.sections[1].title, "Section Two");
    }

    #[test]
    fn test_parse_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "= Test\n:forgepoint-type: story\n:id: test").unwrap();

        let parser = DocumentParser::new();
        let doc = parser.parse_file(temp_file.path()).unwrap();

        assert_eq!(doc.title, Some("Test".to_string()));
        assert_eq!(doc.attributes.get("forgepoint-type"), Some(&"story".to_string()));
    }

    #[test]
    fn test_is_asciidoc_file() {
        assert!(DocumentParser::is_asciidoc_file("test.adoc"));
        assert!(DocumentParser::is_asciidoc_file("test.asciidoc"));
        assert!(DocumentParser::is_asciidoc_file("test.asc"));
        assert!(!DocumentParser::is_asciidoc_file("test.md"));
        assert!(!DocumentParser::is_asciidoc_file("test.txt"));
    }

    #[test]
    fn test_is_asciidoc_content() {
        let asciidoc_content = "= Title\n:attr: value\n\n== Section\n";
        let markdown_content = "# Title\n\n## Section\n";
        
        assert!(DocumentParser::is_asciidoc_content(asciidoc_content));
        assert!(!DocumentParser::is_asciidoc_content(markdown_content));
    }
}