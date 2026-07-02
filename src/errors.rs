use thiserror::Error;

#[derive(Error, Debug)]
pub enum LedgerError {
    #[error("storage error: {0}")]
    Storage(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("invalid command syntax: {0}")]
    Syntax(String),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("ledger is empty")]
    EmptyLedger,
    #[error("ledger data is corrupt: {0}")]
    Corrupt(String),
    #[error("encryption error: {0}")]
    Crypto(String),
}
