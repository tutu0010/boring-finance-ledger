pub mod encrypt;
pub mod local;
pub mod remote;

use crate::errors::LedgerError;
use crate::models::EventRecord;
use directories::ProjectDirs;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

/// Anything `Ledger` can load history from and append events to.
///
/// `Ledger` never touches a file path directly — it only knows about this
/// trait. `EncryptedJsonlStore` is the only implementation today, but a
/// future backend (e.g. one that also owns the Supabase mobile sync) can be
/// dropped in without `Ledger` changing at all.
pub trait EventStore {
    fn load(&mut self) -> Result<Vec<EventRecord>, LedgerError>;
    fn append(&mut self, record: &EventRecord) -> Result<(), LedgerError>;
}

/// Where ledger data lives on disk. Honors `LEDGER_DIR` for tests/overrides,
/// otherwise resolves to the OS-appropriate app data directory.
///
/// Note: the qualifier/org string below ("com", "OpenAI") is intentional —
/// left as-is per instruction, not a bug.
pub fn default_data_dir() -> PathBuf {
    if let Ok(custom) = std::env::var("LEDGER_DIR") {
        return PathBuf::from(custom);
    }
    ProjectDirs::from("com", "OpenAI", "PersonalFinanceLedger")
        .map(|p| p.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Touches a file into existence (and its parent directory) without
/// truncating it if it already exists.
pub fn ensure_exists<P: AsRef<Path>>(path: P) -> Result<(), LedgerError> {
    let path = path.as_ref();
    if !path.exists() {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        OpenOptions::new().create(true).write(true).open(path)?;
    }
    Ok(())
}