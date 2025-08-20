use crate::schema::DocumentTypeDefinition;
use crate::validator::{ValidationResult, ValidationError, Severity};
use colored::*;
use serde_json;
use std::collections::HashMap;

pub struct ResultFormatter;

impl ResultFormatter {
    /// Format validation results as human-readable text
    pub fn format_text(results: &[ValidationResult], verbose: bool) -> String {
        let mut output = String::new();

        for result in results {
            let status = if result.valid {
                "✓".green()
            } else {
                "✗".red()
            };

            let file_name = result.file_path.cyan();
            let doc_info = if let (Some(doc_type), Some(doc_id)) = (&result.document_type, &result.document_id) {
                format!(" ({}:{})", doc_type, doc_id).dimmed()
            } else {
                String::new().dimmed()
            };

            output.push_str(&format!("{} {}{}\n", status, file_name, doc_info));

            if !result.valid || verbose {
                // Show errors
                for error in &result.errors {
                    output.push_str(&Self::format_error(error, "error"));
                }

                // Show warnings if verbose or if there are no errors
                if verbose || result.errors.is_empty() {
                    for warning in &result.warnings {
                        output.push_str(&Self::format_error(warning, "warning"));
                    }
                }
            }

            output.push('\n');
        }

        output
    }

    /// Format a single validation error
    fn format_error(error: &ValidationError, level: &str) -> String {
        let icon = match level {
            "error" => "  ✗".red(),
            "warning" => "  ⚠".yellow(),
            _ => "  •".white(),
        };

        let message = error.message.white();
        let mut output = format!("{} {}", icon, message);

        if let Some(location) = &error.location {
            let mut location_parts = Vec::new();
            if let Some(line) = location.line {
                location_parts.push(format!("line {}", line));
            }
            if let Some(column) = location.column {
                location_parts.push(format!("col {}", column));
            }
            if let Some(section) = &location.section {
                location_parts.push(format!("section \"{}\"", section));
            }

            if !location_parts.is_empty() {
                output.push_str(&format!(" ({})", location_parts.join(", ")).dimmed().to_string());
            }
        }

        if let Some(rule) = &error.rule {
            output.push_str(&format!(" [{}]", rule).dimmed().to_string());
        }

        output.push('\n');

        if let Some(suggestion) = &error.suggestion {
            output.push_str(&format!("    Suggestion: {}\n", suggestion).dimmed().to_string());
        }

        output
    }

    /// Format validation results as JSON
    pub fn format_json(results: &[ValidationResult]) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(results)
    }

    /// Format validation results as JUnit XML
    pub fn format_junit(results: &[ValidationResult]) -> String {
        let total_tests = results.len();
        let failures = results.iter().filter(|r| !r.valid).count();

        let mut xml = String::new();
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str(&format!(
            "<testsuite name=\"Forgepoint Linter\" tests=\"{}\" failures=\"{}\" time=\"0\">\n",
            total_tests, failures
        ));

        for result in results {
            let test_name = result
                .file_path
                .chars()
                .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' { c } else { '_' })
                .collect::<String>();

            xml.push_str(&format!(
                "  <testcase name=\"{}\" classname=\"Forgepoint\"",
                test_name
            ));

            if result.valid {
                xml.push_str(" />\n");
            } else {
                xml.push_str(">\n");

                for error in &result.errors {
                    xml.push_str("    <failure type=\"ValidationError\">");
                    xml.push_str("<![CDATA[");
                    xml.push_str(&error.message);
                    if let Some(location) = &error.location {
                        if let Some(line) = location.line {
                            xml.push_str(&format!(" (line {})", line));
                        }
                    }
                    if let Some(rule) = &error.rule {
                        xml.push_str(&format!(" [{}]", rule));
                    }
                    xml.push_str("]]></failure>\n");
                }

                xml.push_str("  </testcase>\n");
            }
        }

        xml.push_str("</testsuite>\n");
        xml
    }

    /// Format summary statistics
    pub fn format_summary(results: &[ValidationResult]) -> String {
        let total_files = results.len();
        let valid_files = results.iter().filter(|r| r.valid).count();
        let total_errors: usize = results.iter().map(|r| r.errors.len()).sum();
        let total_warnings: usize = results.iter().map(|r| r.warnings.len()).sum();

        let mut output = String::new();
        output.push_str(&format!("\n{}\n", "Summary:".bold()));
        output.push_str(&format!("Files processed: {}\n", total_files));
        output.push_str(&format!("Valid files: {}\n", valid_files.to_string().green()));
        output.push_str(&format!(
            "Invalid files: {}\n",
            (total_files - valid_files).to_string().red()
        ));
        output.push_str(&format!("Total errors: {}\n", total_errors.to_string().red()));
        output.push_str(&format!(
            "Total warnings: {}\n",
            total_warnings.to_string().yellow()
        ));

        let success_rate = if total_files > 0 {
            (valid_files as f64 / total_files as f64 * 100.0)
        } else {
            0.0
        };

        output.push_str(&format!("Success rate: {:.1}%\n", success_rate));

        output
    }

    /// Format available document types
    pub fn format_document_types(document_types: &[DocumentTypeDefinition]) -> String {
        let mut output = String::new();
        output.push_str(&format!("{}\n\n", "Available Document Types:".bold()));

        let categories = ["discovery", "design", "development", "testing", "release"];

        for category in &categories {
            let category_types: Vec<_> = document_types
                .iter()
                .filter(|dt| dt.category == *category)
                .collect();

            if category_types.is_empty() {
                continue;
            }

            output.push_str(&format!("{}:\n", category.to_uppercase().cyan().bold()));

            for doc_type in category_types {
                output.push_str(&format!(
                    "  {:<20} {}\n",
                    doc_type.doc_type.yellow(),
                    doc_type.name
                ));
                output.push_str(&format!("    {}\n\n", doc_type.description.dimmed()));
            }
        }

        output
    }

    /// Get summary statistics
    pub fn get_summary_stats(results: &[ValidationResult]) -> SummaryStats {
        let total_files = results.len();
        let valid_files = results.iter().filter(|r| r.valid).count();
        let total_errors: usize = results.iter().map(|r| r.errors.len()).sum();
        let total_warnings: usize = results.iter().map(|r| r.warnings.len()).sum();

        let mut errors_by_type: HashMap<String, usize> = HashMap::new();
        let mut errors_by_rule: HashMap<String, usize> = HashMap::new();

        for result in results {
            for error in &result.errors {
                let error_type = format!("{:?}", error.error_type);
                *errors_by_type.entry(error_type).or_insert(0) += 1;

                if let Some(rule) = &error.rule {
                    *errors_by_rule.entry(rule.clone()).or_insert(0) += 1;
                }
            }

            for warning in &result.warnings {
                let warning_type = format!("{:?}", warning.error_type);
                *errors_by_type.entry(warning_type).or_insert(0) += 1;

                if let Some(rule) = &warning.rule {
                    *errors_by_rule.entry(rule.clone()).or_insert(0) += 1;
                }
            }
        }

        SummaryStats {
            total_files,
            valid_files,
            total_errors,
            total_warnings,
            errors_by_type,
            errors_by_rule,
        }
    }
}

#[derive(Debug)]
pub struct SummaryStats {
    pub total_files: usize,
    pub valid_files: usize,
    pub total_errors: usize,
    pub total_warnings: usize,
    pub errors_by_type: HashMap<String, usize>,
    pub errors_by_rule: HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validator::{ErrorType, Location};

    #[test]
    fn test_format_text() {
        let results = vec![
            ValidationResult {
                file_path: "test.adoc".to_string(),
                document_type: Some("story".to_string()),
                document_id: Some("test-story".to_string()),
                valid: true,
                errors: Vec::new(),
                warnings: Vec::new(),
            },
            ValidationResult {
                file_path: "invalid.adoc".to_string(),
                document_type: None,
                document_id: None,
                valid: false,
                errors: vec![ValidationError {
                    error_type: ErrorType::Structure,
                    severity: Severity::Error,
                    message: "Missing required attributes".to_string(),
                    location: Some(Location {
                        line: Some(1),
                        column: None,
                        section: None,
                    }),
                    rule: Some("require-structure".to_string()),
                    suggestion: Some("Add required attributes".to_string()),
                }],
                warnings: Vec::new(),
            },
        ];

        let output = ResultFormatter::format_text(&results, false);
        assert!(output.contains("✓"));
        assert!(output.contains("✗"));
        assert!(output.contains("test.adoc"));
        assert!(output.contains("invalid.adoc"));
    }

    #[test]
    fn test_format_json() {
        let results = vec![ValidationResult {
            file_path: "test.adoc".to_string(),
            document_type: Some("story".to_string()),
            document_id: Some("test-story".to_string()),
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }];

        let json_output = ResultFormatter::format_json(&results).unwrap();
        assert!(json_output.contains("test.adoc"));
        assert!(json_output.contains("story"));
    }
}