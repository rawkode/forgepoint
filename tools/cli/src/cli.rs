use crate::config::ForgepointConfig;
use crate::document::ForgepointDocument;
use crate::formatter::ResultFormatter;
use crate::parser::DocumentParser;
use crate::schema::SchemaLoader;
use crate::validator::DocumentValidator;
use crate::{Cli, OutputFormat};
use anyhow::{Context, Result};
use glob::glob;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct LintArgs {
    pub cli: Cli,
    pub patterns: Vec<String>,
    pub format: OutputFormat,
    pub output: Option<PathBuf>,
    pub exclude: Option<String>,
    pub no_check_ids: bool,
    pub no_check_refs: bool,
    pub fail_on_warnings: bool,
}

pub struct CreateArgs {
    pub cli: Cli,
    pub document_type: String,
    pub id: String,
    pub title: Option<String>,
    pub author: Option<String>,
    pub output: Option<PathBuf>,
}

pub async fn lint_command(args: LintArgs) -> Result<()> {
    let config = load_config(&args.cli)?;
    
    println!("{}", "Loading schemas...".dimmed());
    let mut schema_loader = SchemaLoader::new(&config.schema_path);
    schema_loader.load_schemas()
        .context("Failed to load schemas")?;

    println!("{}", "Finding documents...".dimmed());
    let files = find_files(&args.patterns, &get_exclude_patterns(&args, &config))?;
    
    if files.is_empty() {
        eprintln!("No AsciiDoc files found matching patterns: {}", args.patterns.join(", "));
        return Ok(());
    }

    println!("Found {} documents to validate", files.len());
    
    let progress = ProgressBar::new(files.len() as u64);
    progress.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
            .unwrap(),
    );

    let parser = DocumentParser::new();
    let validator = Mutex::new(DocumentValidator::new(schema_loader));
    
    // First pass: Parse and validate documents
    let results: Vec<_> = files
        .par_iter()
        .map(|file_path| {
            progress.set_message(format!("Processing {}", file_path.file_name().unwrap_or_default().to_string_lossy()));
            progress.inc(1);

            let result = match parser.parse_file(file_path) {
                Ok(doc) => {
                    let mut validator = validator.lock().unwrap();
                    validator.validate_document(&doc)
                }
                Err(e) => {
                    use crate::validator::{ValidationResult, ValidationError, ErrorType, Severity};
                    ValidationResult {
                        file_path: file_path.to_string_lossy().to_string(),
                        document_type: None,
                        document_id: None,
                        valid: false,
                        errors: vec![ValidationError {
                            error_type: ErrorType::Format,
                            severity: Severity::Error,
                            message: format!("Failed to parse file: {}", e),
                            location: None,
                            rule: Some("file-parsing".to_string()),
                            suggestion: None,
                        }],
                        warnings: Vec::new(),
                    }
                }
            };

            result
        })
        .collect();

    progress.finish_with_message("Validation complete");

    // Second pass: Check ID uniqueness if enabled
    let mut final_results = results;
    if !args.no_check_ids && config.rules.check_id_uniqueness {
        let validator = validator.into_inner().unwrap();
        let duplicate_errors = validator.check_id_uniqueness();
        
        // Add duplicate errors to affected results
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

    // Format and output results
    let output_text = match args.format {
        OutputFormat::Text => {
            let mut text = ResultFormatter::format_text(&final_results, config.output.verbose);
            if !config.output.verbose {
                text.push_str(&ResultFormatter::format_summary(&final_results));
            }
            text
        }
        OutputFormat::Json => ResultFormatter::format_json(&final_results)?,
        OutputFormat::Junit => ResultFormatter::format_junit(&final_results),
    };

    if let Some(output_file) = args.output {
        fs::write(&output_file, output_text)
            .with_context(|| format!("Failed to write output to {}", output_file.display()))?;
        println!("Results written to {}", output_file.display());
    } else {
        print!("{}", output_text);
    }

    // Determine exit code
    let has_errors = final_results.iter().any(|r| !r.valid);
    let has_warnings = final_results.iter().any(|r| !r.warnings.is_empty());

    if has_errors || (args.fail_on_warnings && has_warnings) {
        std::process::exit(1);
    }

    Ok(())
}

pub async fn create_command(args: CreateArgs) -> Result<()> {
    let config = load_config(&args.cli)?;
    
    let mut schema_loader = SchemaLoader::new(&config.schema_path);
    schema_loader.load_schemas()
        .context("Failed to load schemas")?;

    let template = create_document_template(
        &schema_loader,
        &args.document_type,
        &args.id,
        args.title.as_deref(),
        args.author.as_deref(),
    )?;

    let output_file = args.output.unwrap_or_else(|| PathBuf::from(format!("{}.adoc", args.id)));
    
    fs::write(&output_file, template)
        .with_context(|| format!("Failed to write template to {}", output_file.display()))?;

    println!("Created {} document: {}", args.document_type, output_file.display());
    Ok(())
}

pub async fn list_types_command(cli: Cli) -> Result<()> {
    let config = load_config(&cli)?;
    
    let mut schema_loader = SchemaLoader::new(&config.schema_path);
    schema_loader.load_schemas()
        .context("Failed to load schemas")?;

    let document_types = schema_loader.get_document_types();
    println!("{}", ResultFormatter::format_document_types(&document_types));
    
    Ok(())
}

pub async fn check_command(cli: Cli, file: PathBuf) -> Result<()> {
    let config = load_config(&cli)?;
    
    let mut schema_loader = SchemaLoader::new(&config.schema_path);
    schema_loader.load_schemas()
        .context("Failed to load schemas")?;

    let parser = DocumentParser::new();
    let mut validator = DocumentValidator::new(schema_loader);

    let doc = parser.parse_file(&file)
        .with_context(|| format!("Failed to parse file {}", file.display()))?;

    let result = validator.validate_document(&doc);
    let output = ResultFormatter::format_text(&[result.clone()], true);
    
    print!("{}", output);
    
    if !result.valid {
        std::process::exit(1);
    }

    Ok(())
}

pub async fn init_command(cli: Cli, _example: bool) -> Result<()> {
    println!("Initializing Forgepoint...");
    
    // Create .forgepoint.toml configuration file
    let config = ForgepointConfig::default();
    let config_toml = toml::to_string_pretty(&config)
        .context("Failed to serialize default configuration")?;
    
    fs::write(".forgepoint.toml", config_toml)
        .context("Failed to write configuration file")?;
    
    println!("Created .forgepoint.toml configuration file");
    
    // TODO: Create example documents if requested
    
    println!("Forgepoint initialized successfully!");
    Ok(())
}

pub async fn config_command(cli: Cli, show: bool) -> Result<()> {
    let config = load_config(&cli)?;
    
    if show {
        let config_json = serde_json::to_string_pretty(&config)
            .context("Failed to serialize configuration")?;
        println!("{}", config_json);
    } else {
        println!("Configuration file locations checked:");
        let config_paths = [
            ".forgepoint.toml",
            ".forgepoint.yaml", 
            ".forgepoint.yml",
            ".forgepointrc.json",
            "forgepoint.toml",
        ];
        
        for path in &config_paths {
            let exists = std::path::Path::new(path).exists();
            println!("  {} {}", path, if exists { "✓" } else { "✗" });
        }
    }
    
    Ok(())
}

fn load_config(cli: &Cli) -> Result<ForgepointConfig> {
    let config = ForgepointConfig::load(cli.config.as_ref())
        .context("Failed to load configuration")?
        .merge_cli_args(cli.schema_path.clone(), cli.verbose)
        .resolve_paths(None);
    
    Ok(config)
}

fn find_files(patterns: &[String], exclude_patterns: &[String]) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    
    for pattern in patterns {
        for entry in glob(pattern)
            .with_context(|| format!("Invalid glob pattern: {}", pattern))?
        {
            let path = entry.context("Failed to read glob entry")?;
            
            if DocumentParser::is_asciidoc_file(&path) {
                let should_exclude = exclude_patterns.iter().any(|exclude_pattern| {
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

fn get_exclude_patterns(args: &LintArgs, config: &ForgepointConfig) -> Vec<String> {
    let mut patterns = config.exclude_patterns.clone();
    
    if let Some(exclude) = &args.exclude {
        patterns.extend(
            exclude
                .split(',')
                .map(|s| s.trim().to_string())
                .collect::<Vec<_>>()
        );
    }
    
    patterns
}

fn create_document_template(
    schema_loader: &SchemaLoader,
    doc_type: &str,
    id: &str,
    title: Option<&str>,
    author: Option<&str>,
) -> Result<String> {
    let document_types = schema_loader.get_document_types();
    let doc_type_def = document_types
        .iter()
        .find(|dt| dt.doc_type == doc_type)
        .ok_or_else(|| anyhow::anyhow!("Unknown document type: {}", doc_type))?;

    let title = title.unwrap_or(&doc_type_def.name);
    let author = author.unwrap_or("Author Name");
    let date = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let required_sections = schema_loader.get_required_sections(doc_type);
    let is_abstract_required = schema_loader.is_abstract_required(doc_type);

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

use colored::Colorize; // Add this import for .dimmed()