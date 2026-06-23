use crate::errors::LedgerError;
use crate::models::EventRecord;
use directories::ProjectDirs;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

pub struct LoadResult {
    pub events: Vec<EventRecord>,
    pub recovered_from_backup: bool,
}

pub fn default_path() -> PathBuf {
    ProjectDirs::from("com", "OpenAI", "PersonalFinanceLedger")
        .map(|p| p.data_dir().join("ledger.json"))
        .unwrap_or_else(|| PathBuf::from("ledger.json"))
}

fn backup_path(path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.bak", path.display()))
}

fn try_load_file(path: &Path) -> Result<Vec<EventRecord>, LedgerError> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    if contents.trim().is_empty() {
        return Ok(Vec::new());
    }
    Ok(serde_json::from_str(&contents)?)
}

pub fn load<P: AsRef<Path>>(path: P) -> Result<LoadResult, LedgerError> {
    let path = path.as_ref();
    match try_load_file(path) {
        Ok(events) => Ok(LoadResult {
            events,
            recovered_from_backup: false,
        }),
        Err(_) => {
            let bak = backup_path(path);
            let events = try_load_file(&bak).map_err(|_| {
                LedgerError::Corrupt(format!(
                    "could not read {} or {}",
                    path.display(),
                    bak.display()
                ))
            })?;
            Ok(LoadResult {
                events,
                recovered_from_backup: true,
            })
        }
    }
}

pub fn save<P: AsRef<Path>>(path: P, events: &[EventRecord]) -> Result<(), LedgerError> {
    let path = path.as_ref();
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }

    if path.exists() {
        fs::copy(path, backup_path(path))?;
    }

    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut tmp = NamedTempFile::new_in(dir)?;
    let json = serde_json::to_string_pretty(events)?;
    tmp.write_all(json.as_bytes())?;
    tmp.flush()?;
    tmp.as_file().sync_all()?;
    tmp.persist(path).map_err(|e| e.error)?;
    Ok(())
}

pub fn ensure_exists<P: AsRef<Path>>(path: P) -> Result<(), LedgerError> {
    let path = path.as_ref();
    if !path.exists() {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Event, EventRecord};
    use rust_decimal::Decimal;
    use tempfile::tempdir;

    #[test]
    fn round_trip() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("ledger.json");
        let events = vec![EventRecord::new(
            1,
            Event::Expense {
                amount: Decimal::new(20000, 2),
                category: "food".into(),
                description: "burger".into(),
            },
        )];
        save(&file, &events).unwrap();
        let loaded = load(&file).unwrap();
        assert_eq!(loaded.events.len(), 1);
        assert!(!loaded.recovered_from_backup);
    }

    #[test]
    fn backup_recovers_corrupt_main() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("ledger.json");
        let bak = backup_path(&file);
        fs::write(&file, "not json").unwrap();
        fs::write(
            &bak,
            serde_json::to_string(&vec![EventRecord::new(
                1,
                Event::Income {
                    amount: Decimal::new(50000, 2),
                    source: "father".into(),
                    description: "fees".into(),
                },
            )])
            .unwrap(),
        )
        .unwrap();

        let loaded = load(&file).unwrap();
        assert_eq!(loaded.events.len(), 1);
        assert!(loaded.recovered_from_backup);
    }
}
