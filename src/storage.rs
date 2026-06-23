use crate::errors::LedgerError;
use crate::models::EventRecord;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

pub fn load<P: AsRef<Path>>(path: P) -> Result<Vec<EventRecord>, LedgerError> {
    if !path.as_ref().exists() {
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

pub fn save<P: AsRef<Path>>(path: P, events: &[EventRecord]) -> Result<(), LedgerError> {
    let json = serde_json::to_string_pretty(events)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    file.write_all(json.as_bytes())?;
    Ok(())
}
