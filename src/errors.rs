use thiserror::Error;

#[derive(Error, Debug)]
pub enum LedgerError {
    #[error("Storage error: {0}")]
    Storage(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Invalid command syntax: {0}")]
    Syntax(String),
}
