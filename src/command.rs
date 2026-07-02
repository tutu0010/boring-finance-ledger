use crate::errors::LedgerError;
use crate::money::Money;
use crate::subscription;
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "ft", version, about = "Personal Finance Ledger")]
pub struct Cli {
    #[command(subcommand)]
    pub action: Action,
}

/// The raw shape clap parses the command line into. This is intentionally
/// "everything in one enum" for clap's ergonomics; `Dispatch::try_from`
/// immediately sorts it into `Command` (mutations), `Undo`, or `Query`
/// (reads) so nothing downstream has to guess which bucket a variant
/// belongs in.
#[derive(Debug, Subcommand)]
pub enum Action {
    Expense {
        #[arg(allow_hyphen_values = true)]
        amount: Money,
        category: String,
        #[arg(num_args = 1.., value_name = "DESCRIPTION")]
        description: Vec<String>,
    },
    Income {
        #[arg(allow_hyphen_values = true)]
        amount: Money,
        source: String,
        #[arg(num_args = 1.., value_name = "DESCRIPTION")]
        description: Vec<String>,
    },
    Lend {
        #[arg(allow_hyphen_values = true)]
        amount: Money,
        person: String,
        #[arg(num_args = 1.., value_name = "DESCRIPTION")]
        description: Vec<String>,
    },
    Borrow {
        #[arg(allow_hyphen_values = true)]
        amount: Money,
        person: String,
        #[arg(num_args = 1.., value_name = "DESCRIPTION")]
        description: Vec<String>,
    },
    Repay {
        #[arg(allow_hyphen_values = true)]
        amount: Money,
        person: String,
    },
    Receive {
        #[arg(allow_hyphen_values = true)]
        amount: Money,
        person: String,
    },
    Subscribe {
        #[arg(allow_hyphen_values = true)]
        amount: Money,
        service: String,
        frequency: String,
    },
    History,
    Summary {
        start: Option<String>,
        end: Option<String>,
    },
    Owed,
    Debts,
    List {
        kind: ListKind,
    },
    Find {
        #[arg(num_args = 1.., value_name = "TERM")]
        term: Vec<String>,
    },
    Undo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ListKind {
    Expenses,
    Income,
    Loans,
    Subscriptions,
}

/// Mutating operations. `Ledger::record` accepts only this type, so it is
/// structurally impossible to hand a read-only query to the write path.
#[derive(Debug, Clone)]
pub enum Command {
    Expense {
        amount: Money,
        category: String,
        description: String,
    },
    Income {
        amount: Money,
        source: String,
        description: String,
    },
    Lend {
        amount: Money,
        person: String,
        description: String,
    },
    Borrow {
        amount: Money,
        person: String,
        description: String,
    },
    Repay {
        amount: Money,
        person: String,
    },
    Receive {
        amount: Money,
        person: String,
    },
    Subscribe {
        amount: Money,
        service: String,
        frequency: String,
    },
}

/// Read-only operations. `query::execute` accepts only this type.
#[derive(Debug, Clone)]
pub enum Query {
    History,
    Summary {
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
    },
    Owed,
    Debts,
    List {
        kind: ListKind,
    },
    Find {
        term: String,
    },
}

/// The fully-validated, fully-classified result of parsing. This is what
/// `main.rs` actually matches on.
pub enum Dispatch {
    Write(Command),
    Undo,
    Read(Query),
}

impl TryFrom<Action> for Dispatch {
    type Error = LedgerError;

    fn try_from(action: Action) -> Result<Self, LedgerError> {
        Ok(match action {
            Action::Expense {
                amount,
                category,
                description,
            } => Dispatch::Write(Command::Expense {
                amount,
                category: normalize(&category),
                description: join_description(&description),
            }),
            Action::Income {
                amount,
                source,
                description,
            } => Dispatch::Write(Command::Income {
                amount,
                source: normalize(&source),
                description: join_description(&description),
            }),
            Action::Lend {
                amount,
                person,
                description,
            } => Dispatch::Write(Command::Lend {
                amount,
                person: normalize(&person),
                description: join_description(&description),
            }),
            Action::Borrow {
                amount,
                person,
                description,
            } => Dispatch::Write(Command::Borrow {
                amount,
                person: normalize(&person),
                description: join_description(&description),
            }),
            Action::Repay { amount, person } => Dispatch::Write(Command::Repay {
                amount,
                person: normalize(&person),
            }),
            Action::Receive { amount, person } => Dispatch::Write(Command::Receive {
                amount,
                person: normalize(&person),
            }),
            Action::Subscribe {
                amount,
                service,
                frequency,
            } => Dispatch::Write(Command::Subscribe {
                amount,
                service: normalize(&service),
                frequency: subscription::validate_frequency(&frequency)?,
            }),
            Action::Undo => Dispatch::Undo,
            Action::History => Dispatch::Read(Query::History),
            Action::Summary { start, end } => {
                let (start, end) = summary_bounds(start.as_deref(), end.as_deref())?;
                Dispatch::Read(Query::Summary { start, end })
            }
            Action::Owed => Dispatch::Read(Query::Owed),
            Action::Debts => Dispatch::Read(Query::Debts),
            Action::List { kind } => Dispatch::Read(Query::List { kind }),
            Action::Find { term } => Dispatch::Read(Query::Find {
                term: term.join(" "),
            }),
        })
    }
}

/// Trims, collapses internal whitespace, and title-cases freeform names so
/// "Food" and "food" land in the same bucket in balances/summaries instead
/// of silently splitting into two.
fn normalize(raw: &str) -> String {
    raw.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => {
                    first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn join_description(text: &[String]) -> String {
    text.join(" ")
}

fn parse_date_bound(s: &str, end: bool) -> Result<DateTime<Utc>, LedgerError> {
    let base = match s.len() {
        7 => NaiveDate::parse_from_str(&format!("{s}-01"), "%Y-%m-%d"),
        10 => NaiveDate::parse_from_str(s, "%Y-%m-%d"),
        _ => {
            return Err(LedgerError::Parse(format!(
                "use YYYY-MM or YYYY-MM-DD: {s}"
            )));
        }
    }
    .map_err(|_| LedgerError::Parse(format!("invalid date: {s}")))?;

    let target = if end {
        if s.len() == 7 {
            let (y, m) = (base.year(), base.month());
            let (y, m) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
            NaiveDate::from_ymd_opt(y, m, 1)
                .ok_or_else(|| LedgerError::Parse(format!("invalid date: {s}")))?
        } else {
            base.succ_opt()
                .ok_or_else(|| LedgerError::Parse(format!("invalid date: {s}")))?
        }
    } else {
        base
    };

    Ok(target.and_hms_opt(0, 0, 0).unwrap().and_utc())
}

/// Free function, not a method that reconstructs `Command::Summary` just to
/// ask it a question about itself. Takes raw `&str` bounds straight from
/// clap, returns validated `DateTime<Utc>` bounds with no outer `Option`
/// wrapping the whole tuple.
pub fn summary_bounds(
    start: Option<&str>,
    end: Option<&str>,
) -> Result<(Option<DateTime<Utc>>, Option<DateTime<Utc>>), LedgerError> {
    let start = start.map(|s| parse_date_bound(s, false)).transpose()?;
    let end = end.map(|s| parse_date_bound(s, true)).transpose()?;
    if let (Some(s), Some(e)) = (&start, &end) {
        if s >= e {
            return Err(LedgerError::Syntax(
                "summary start must be before end".into(),
            ));
        }
    }
    Ok((start, end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_case_and_whitespace() {
        assert_eq!(normalize("  food   "), "Food");
        assert_eq!(normalize("SINAN"), "Sinan");
        assert_eq!(normalize("east  side  cafe"), "East Side Cafe");
    }

    #[test]
    fn summary_bounds_rejects_start_after_end() {
        let result = summary_bounds(Some("2026-02"), Some("2026-01"));
        assert!(result.is_err());
    }

    #[test]
    fn summary_bounds_month_shorthand_expands() {
        let (start, end) = summary_bounds(Some("2026-01"), None).unwrap();
        assert_eq!(start.unwrap().date_naive().to_string(), "2026-01-01");
        assert!(end.is_none());
    }
}
