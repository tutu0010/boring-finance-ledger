use super::encrypt::{self, Cipher, Meta};
use super::{ensure_exists, EventStore};
use crate::errors::LedgerError;
use crate::models::EventRecord;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// Append-only, per-line-encrypted event log.
///
/// Each `EventRecord` is serialized to JSON, then encrypted independently
/// with its own random nonce and written as one base64 line. This keeps the
/// "append is O(1), never touches earlier lines" property of the original
/// JSONL design even though the file as a whole is opaque ciphertext.
pub struct EncryptedJsonlStore {
    events_path: PathBuf,
    cipher: Cipher,
}

pub struct OpenResult {
    pub store: EncryptedJsonlStore,
    pub migrated_legacy_events: usize,
}

impl EncryptedJsonlStore {
    /// `is_new_setup` tells the caller (main.rs) whether it should have
    /// prompted for a passphrase *confirmation* — true the first time this
    /// data directory is ever unlocked.
    pub fn is_new_setup(dir: &Path) -> bool {
        !dir.join("ledger.meta.json").exists()
    }

    pub fn open(dir: &Path, passphrase: &str) -> Result<OpenResult, LedgerError> {
        fs::create_dir_all(dir)?;
        let meta_path = dir.join("ledger.meta.json");
        let events_path = dir.join("ledger.jsonl");
        ensure_exists(&events_path)?;

        let is_new = !meta_path.exists();

        let cipher = if is_new {
            let salt = encrypt::new_salt();
            let cipher = Cipher::derive(passphrase, &salt)?;
            let meta = encrypt::create_meta(&cipher, &salt)?;
            fs::write(&meta_path, serde_json::to_string_pretty(&meta)?)?;
            cipher
        } else {
            let raw = fs::read_to_string(&meta_path)?;
            let meta: Meta = serde_json::from_str(&raw)
                .map_err(|e| LedgerError::Corrupt(format!("unreadable meta file: {e}")))?;
            let salt = encrypt::decode_salt(&meta)?;
            let cipher = Cipher::derive(passphrase, &salt)?;
            encrypt::verify_canary(&cipher, &meta)?;
            cipher
        };

        let mut store = EncryptedJsonlStore {
            events_path,
            cipher,
        };

        let migrated_legacy_events = if is_new {
            store.migrate_legacy(dir)?
        } else {
            0
        };

        Ok(OpenResult {
            store,
            migrated_legacy_events,
        })
    }

    /// Best-effort migration of the pre-encryption plaintext `ledger.json`
    /// format (a single JSON array) into the new encrypted JSONL log. Only
    /// runs on a brand-new setup, since that's the only time "meta file
    /// doesn't exist yet, but old data might" is a meaningful state. On any
    /// failure this warns rather than aborting startup — a missing or
    /// unreadable legacy file is not itself an error.
    fn migrate_legacy(&mut self, dir: &Path) -> Result<usize, LedgerError> {
        let legacy_path = dir.join("ledger.json");
        if !legacy_path.exists() {
            return Ok(0);
        }

        let contents = fs::read_to_string(&legacy_path)?;
        if contents.trim().is_empty() {
            return Ok(0);
        }

        let legacy_events: Vec<EventRecord> = match serde_json::from_str(&contents) {
            Ok(events) => events,
            Err(e) => {
                eprintln!(
                    "Warning: found legacy ledger.json but couldn't parse it ({e}); leaving it untouched."
                );
                return Ok(0);
            }
        };

        for record in &legacy_events {
            self.append(record)?;
        }

        let migrated_marker = legacy_path.with_extension("json.migrated");
        fs::rename(&legacy_path, &migrated_marker)?;

        Ok(legacy_events.len())
    }
}

impl EventStore for EncryptedJsonlStore {
    fn load(&mut self) -> Result<Vec<EventRecord>, LedgerError> {
        let file = File::open(&self.events_path)?;
        let reader = BufReader::new(file);
        let lines: Vec<String> = reader.lines().collect::<Result<_, _>>()?;

        let mut events = Vec::with_capacity(lines.len());
        let last_index = lines.iter().rposition(|l| !l.trim().is_empty());

        for (i, line) in lines.iter().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            match self.cipher.decrypt(line) {
                Ok(plaintext) => {
                    let record: EventRecord = serde_json::from_slice(&plaintext)
                        .map_err(|e| LedgerError::Corrupt(format!("line {}: {e}", i + 1)))?;
                    events.push(record);
                }
                Err(e) => {
                    if Some(i) == last_index {
                        // A broken final line most likely means the process
                        // was killed mid-write. Warn and drop it rather than
                        // failing the whole ledger over one interrupted append.
                        eprintln!(
                            "Warning: last line of ledger is unreadable ({e}), skipping it. \
                             If this keeps happening, restore from ledger.jsonl.bak."
                        );
                    } else {
                        return Err(LedgerError::Corrupt(format!(
                            "line {} failed to decrypt: {e}",
                            i + 1
                        )));
                    }
                }
            }
        }

        Ok(events)
    }

    fn append(&mut self, record: &EventRecord) -> Result<(), LedgerError> {
        let plaintext = serde_json::to_vec(record)?;
        let line = self.cipher.encrypt(&plaintext)?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.events_path)?;
        writeln!(file, "{line}")?;
        file.sync_all()?;

        // Refresh a full-file backup periodically rather than on every
        // append — copying the whole file on every write would defeat the
        // point of an O(1) append, but a backup that's at most 20 events
        // stale is still a solid recovery point.
        if record.id % 20 == 0 {
            let _ = fs::copy(
                &self.events_path,
                self.events_path.with_extension("jsonl.bak"),
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Event;
    use crate::money::Money;
    use rust_decimal::Decimal;
    use tempfile::tempdir;

    #[test]
    fn round_trip_encrypted() {
        let dir = tempdir().unwrap();
        let opened = EncryptedJsonlStore::open(dir.path(), "hunter2-but-better").unwrap();
        let mut store = opened.store;

        let record = EventRecord::new(
            1,
            Event::Expense {
                amount: Money::new(Decimal::new(20000, 2)).unwrap(),
                category: "Food".into(),
                description: "burger".into(),
            },
        );
        store.append(&record).unwrap();

        let loaded = store.load().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, 1);
    }

    #[test]
    fn wrong_passphrase_on_reopen_fails() {
        let dir = tempdir().unwrap();
        let _ = EncryptedJsonlStore::open(dir.path(), "correct-passphrase").unwrap();
        let reopened = EncryptedJsonlStore::open(dir.path(), "wrong-passphrase");
        assert!(reopened.is_err());
    }

    #[test]
    fn on_disk_file_is_not_plaintext() {
        let dir = tempdir().unwrap();
        let opened = EncryptedJsonlStore::open(dir.path(), "secret").unwrap();
        let mut store = opened.store;
        store
            .append(&EventRecord::new(
                1,
                Event::Income {
                    amount: Money::new(Decimal::new(50000, 2)).unwrap(),
                    source: "Scholarship".into(),
                    description: "monthly stipend".into(),
                },
            ))
            .unwrap();

        let raw = fs::read_to_string(dir.path().join("ledger.jsonl")).unwrap();
        assert!(!raw.contains("Scholarship"));
        assert!(!raw.contains("Income"));
    }
}
