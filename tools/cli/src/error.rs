use thiserror::Error;

#[derive(Error, Debug)]
pub enum ForgepointError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Schema validation error: {0}")]
    Schema(String),

    #[error("Document parsing error: {0}")]
    Parsing(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Invalid document type: {0}")]
    InvalidDocumentType(String),

    #[error("Invalid ID format: {0}")]
    InvalidIdFormat(String),

    #[error("Reference error: {0}")]
    Reference(String),
}

pub type Result<T> = std::result::Result<T, ForgepointError>;