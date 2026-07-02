use crate::errors::LedgerError;
use chrono::{DateTime, Duration, Months, Utc};

/// Normalizes and validates a subscription frequency string at the moment
/// the subscription is created, so a typo surfaces immediately at the CLI
/// instead of silently never charging anything later.
pub fn validate_frequency(raw: &str) -> Result<String, LedgerError> {
    let normalized = raw.trim().to_lowercase();
    match normalized.as_str() {
        "daily" | "weekly" | "monthly" | "yearly" => Ok(normalized),
        "annual" | "annually" => Ok("yearly".to_string()),
        other => Err(LedgerError::Validation(format!(
            "unknown frequency '{other}', expected one of: daily, weekly, monthly, yearly"
        ))),
    }
}

/// Advances `from` by one billing period of `frequency`. `frequency` is
/// assumed to already be normalized via `validate_frequency`.
pub fn advance(from: DateTime<Utc>, frequency: &str) -> Result<DateTime<Utc>, LedgerError> {
    match frequency {
        "daily" => Ok(from + Duration::days(1)),
        "weekly" => Ok(from + Duration::weeks(1)),
        "monthly" => from
            .checked_add_months(Months::new(1))
            .ok_or_else(|| LedgerError::Validation("date overflow while advancing month".into())),
        "yearly" => from
            .checked_add_months(Months::new(12))
            .ok_or_else(|| LedgerError::Validation("date overflow while advancing year".into())),
        other => Err(LedgerError::Validation(format!(
            "unknown frequency '{other}'"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn validates_known_frequencies() {
        assert_eq!(validate_frequency("Monthly").unwrap(), "monthly");
        assert_eq!(validate_frequency("ANNUALLY").unwrap(), "yearly");
        assert!(validate_frequency("fortnightly").is_err());
    }

    #[test]
    fn monthly_advance_handles_year_rollover() {
        let dec = Utc.with_ymd_and_hms(2025, 12, 15, 0, 0, 0).unwrap();
        let next = advance(dec, "monthly").unwrap();
        assert_eq!(next.date_naive().to_string(), "2026-01-15");
    }
}
