use crate::models::EventRecord;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Read from `SUPABASE_URL` / `SUPABASE_KEY`. Absence of either means the
/// feature is simply off — this is opt-in, not a hard requirement to run
/// the CLI at all.
pub struct SupabaseConfig {
    pub url: String,
    pub key: String,
}

impl SupabaseConfig {
    pub fn from_env() -> Option<Self> {
        let url = std::env::var("SUPABASE_URL").ok()?;
        let key = std::env::var("SUPABASE_KEY").ok()?;
        if url.trim().is_empty() || key.trim().is_empty() {
            return None;
        }
        Some(Self { url, key })
    }
}

/// Tracks the highest event id we've successfully confirmed Supabase has.
/// Desktop-only, single-writer, monotonically increasing ids make "push
/// everything newer than N" a correct and sufficient sync strategy — no
/// merge logic needed because there's exactly one writer today.
struct SyncState {
    path: PathBuf,
    last_synced_id: u64,
}

impl SyncState {
    fn load(dir: &Path) -> Self {
        let path = dir.join("ledger.sync");
        let last_synced_id = fs::read_to_string(&path)
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0);
        Self {
            path,
            last_synced_id,
        }
    }

    fn save(&self) {
        let _ = fs::write(&self.path, self.last_synced_id.to_string());
    }
}

/// Result of a sync attempt, meant to be printed as a one-line status by
/// main.rs — never fatal to the command the user actually ran.
pub struct SyncReport {
    pub pushed: usize,
    pub pending: usize,
    pub error: Option<String>,
}

/// Pushes every local event newer than the last confirmed sync point to
/// Supabase. Uses PostgREST upsert semantics (`on_conflict=id` +
/// `resolution=merge-duplicates`) so re-pushing an already-synced event is
/// harmless — this makes retries safe without needing precise bookkeeping.
///
/// If Supabase is unreachable (Mac is offline, DNS fails, whatever), this
/// returns a report describing what's still pending instead of erroring —
/// the local encrypted store is always the source of truth regardless of
/// whether the remote push succeeds.
pub fn sync_pending(events: &[EventRecord], dir: &Path) -> SyncReport {
    let Some(config) = SupabaseConfig::from_env() else {
        return SyncReport {
            pushed: 0,
            pending: 0,
            error: None,
        };
    };

    let mut state = SyncState::load(dir);
    let pending: Vec<&EventRecord> = events
        .iter()
        .filter(|r| r.id > state.last_synced_id)
        .collect();

    if pending.is_empty() {
        return SyncReport {
            pushed: 0,
            pending: 0,
            error: None,
        };
    }

    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(5))
        .build();

    let endpoint = format!(
        "{}/rest/v1/events?on_conflict=id",
        config.url.trim_end_matches('/')
    );
    let mut pushed = 0;

    for record in &pending {
        let body = push_body(record);
        let result = agent
            .post(&endpoint)
            .set("apikey", &config.key)
            .set("Authorization", &format!("Bearer {}", config.key))
            .set("Content-Type", "application/json")
            .set("Prefer", "resolution=merge-duplicates,return=minimal")
            .send_json(body);

        match result {
            Ok(_resp) => {
                pushed += 1;
                state.last_synced_id = record.id;
            }
            Err(ureq::Error::Status(status, resp)) => {
                let text = resp.into_string().unwrap_or_default();
                state.save();
                return SyncReport {
                    pushed,
                    pending: pending.len() - pushed,
                    error: Some(format!(
                        "Supabase rejected event #{}: {status} {text}",
                        record.id
                    )),
                };
            }
            Err(e) => {
                state.save();
                return SyncReport {
                    pushed,
                    pending: pending.len() - pushed,
                    error: Some(format!("network error reaching Supabase: {e}")),
                };
            }
        }
    }

    state.save();
    SyncReport {
        pushed,
        pending: 0,
        error: None,
    }
}

fn push_body(record: &EventRecord) -> serde_json::Value {
    serde_json::json!({
        "id": record.id,
        "timestamp": record.timestamp.to_rfc3339(),
        "event": record.event,
    })
}

#[allow(dead_code)]
pub fn schema_sql() -> &'static str {
    r#"
create table if not exists events (
    id bigint primary key,
    timestamp timestamptz not null,
    event jsonb not null,
    synced_at timestamptz not null default now()
);

-- Single-user personal ledger accessed only via the service_role key from
-- your own machine: row level security stays off rather than modeling a
-- multi-tenant policy that doesn't apply here.
alter table events disable row level security;
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_config_means_silent_noop() {
        std::env::remove_var("SUPABASE_URL");
        std::env::remove_var("SUPABASE_KEY");
        let dir = tempfile::tempdir().unwrap();
        let report = sync_pending(&[], dir.path());
        assert_eq!(report.pushed, 0);
        assert!(report.error.is_none());
    }
}
