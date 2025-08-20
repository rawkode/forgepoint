use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod cli;
mod config;
mod document;
mod linter;
mod parser;
mod schema;
mod validator;
mod formatter;
mod error;

use cli::*;

#[derive(Parser)]
#[command(
    name = "forgepoint",
    about = "A Git-native product planning platform linter and validator",
    version = "0.1.0",
    long_about = "Forgepoint CLI validates AsciiDoc documents against comprehensive schemas covering the entire SDLC - from PRFAQs to retrospectives."
)]
pub struct Cli {
    /// Path to schema directory
    #[arg(long, global = true, env = "FORGEPOINT_SCHEMA_PATH")]
    pub schema_path: Option<PathBuf>,

    /// Configuration file path
    #[arg(long, global = true, env = "FORGEPOINT_CONFIG")]
    pub config: Option<PathBuf>,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Validate AsciiDoc documents against Forgepoint schemas
    Lint {
        /// File patterns to lint
        #[arg(default_values = &["**/*.adoc"])]
        patterns: Vec<String>,

        /// Output format
        #[arg(short, long, value_enum, default_value = "text")]
        format: OutputFormat,

        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Exclude patterns (comma-separated)
        #[arg(long)]
        exclude: Option<String>,

        /// Skip ID uniqueness check
        #[arg(long)]
        no_check_ids: bool,

        /// Skip reference validation
        #[arg(long)]
        no_check_refs: bool,

        /// Treat warnings as failures
        #[arg(long)]
        fail_on_warnings: bool,
    },

    /// Create a new document from template
    Create {
        /// Document type
        document_type: String,

        /// Document ID
        id: String,

        /// Document title
        #[arg(long)]
        title: Option<String>,

        /// Document author
        #[arg(long)]
        author: Option<String>,

        /// Output file (default: <id>.adoc)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// List all available document types
    #[command(name = "list-types")]
    ListTypes,

    /// Check a single file
    Check {
        /// File to check
        file: PathBuf,
    },

    /// Initialize Forgepoint in current directory
    Init {
        /// Create example documents
        #[arg(long)]
        example: bool,
    },

    /// Show configuration
    Config {
        /// Show resolved configuration
        #[arg(long)]
        show: bool,
    },
}

#[derive(clap::ValueEnum, Clone, Copy)]
pub enum OutputFormat {
    Text,
    Json,
    Junit,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Set up logging based on verbosity
    if cli.verbose {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Debug)
            .init();
    }

    match cli.command {
        Commands::Lint {
            patterns,
            format,
            output,
            exclude,
            no_check_ids,
            no_check_refs,
            fail_on_warnings,
        } => {
            lint_command(LintArgs {
                cli,
                patterns,
                format,
                output,
                exclude,
                no_check_ids,
                no_check_refs,
                fail_on_warnings,
            })
            .await
        }
        Commands::Create {
            document_type,
            id,
            title,
            author,
            output,
        } => {
            create_command(CreateArgs {
                cli,
                document_type,
                id,
                title,
                author,
                output,
            })
            .await
        }
        Commands::ListTypes => list_types_command(cli).await,
        Commands::Check { file } => check_command(cli, file).await,
        Commands::Init { example } => init_command(cli, example).await,
        Commands::Config { show } => config_command(cli, show).await,
    }
}