use crate::errors::LedgerError;
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use clap::{Parser, Subcommand, ValueEnum};
use rust_decimal::Decimal;
use std::str::FromStr;

#[derive(Debug, Parser)]
#[command(name = "ledger", version, about = "Personal Finance Ledger")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Expense {
        amount: Decimal,
        category: String,
        #[arg(num_args = 1.., value_name = "DESCRIPTION")]
        description: Vec<String>,
    },
    Income {
        amount: Decimal,
        source: String,
        #[arg(num_args = 1.., value_name = "DESCRIPTION")]
        description: Vec<String>,
    },
    Lend {
        amount: Decimal,
        person: String,
        #[arg(num_args = 1.., value_name = "DESCRIPTION")]
        description: Vec<String>,
    },
    Borrow {
        amount: Decimal,
        person: String,
        #[arg(num_args = 1.., value_name = "DESCRIPTION")]
        description: Vec<String>,
    },
    Repay {
        amount: Decimal,
        person: String,
    },
    Receive {
        amount: Decimal,
        person: String,
    },
    Subscribe {
        amount: Decimal,
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
                .ok_or_else(|| LedgerError::Parse(format!("invalid date: {s}")))?;
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

impl Command {
    pub fn summary_bounds(
        &self,
    ) -> Result<Option<(Option<DateTime<Utc>>, Option<DateTime<Utc>>)>, LedgerError> {
        match self {
            Self::Summary { start, end } => {
                let start = start
                    .as_deref()
                    .map(|s| parse_date_bound(s, false))
                    .transpose()?;
                let end = end
                    .as_deref()
                    .map(|s| parse_date_bound(s, true))
                    .transpose()?;
                if let (Some(s), Some(e)) = (&start, &end) {
                    if s >= e {
                        return Err(LedgerError::Syntax(
                            "summary start must be before end".into(),
                        ));
                    }
                }
                Ok(Some((start, end)))
            }
            _ => Ok(None),
        }
    }

    pub fn find_term(&self) -> Option<String> {
        match self {
            Self::Find { term } => Some(term.join(" ")),
            _ => None,
        }
    }

    pub fn description(text: &[String]) -> String {
        text.join(" ")
    }

    pub fn amount_from_str(s: &str) -> Result<Decimal, LedgerError> {
        Decimal::from_str(s).map_err(|_| LedgerError::Parse(format!("invalid amount: {s}")))
    }
}
