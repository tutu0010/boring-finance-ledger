use crate::errors::LedgerError;
use chrono::{DateTime, Datelike, NaiveDate, Utc};

#[derive(Debug)]
pub enum Action {
    Command(CommandType),
    Query(QueryType),
}

#[derive(Debug)]
pub enum CommandType {
    Expense {
        amount: f64,
        category: String,
        description: String,
    },
    Income {
        amount: f64,
        source: String,
        description: String,
    },
    Lend {
        amount: f64,
        person: String,
        description: String,
    },
    Borrow {
        amount: f64,
        person: String,
        description: String,
    },
    Repay {
        amount: f64,
        person: String,
    },
    Receive {
        amount: f64,
        person: String,
    },
    Subscribe {
        amount: f64,
        service: String,
        frequency: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListKind {
    Expenses,
    Income,
    Loans,
    Subscriptions,
}

#[derive(Debug)]
pub enum QueryType {
    History,
    Summary {
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
    },
    Owed,
    Debts,
    List(ListKind),
    Find(String),
}

fn parse_date_bound(s: &str, end: bool) -> Result<DateTime<Utc>, LedgerError> {
    let date = match s.len() {
        7 => NaiveDate::parse_from_str(&format!("{s}-01"), "%Y-%m-%d").map_err(|_| {
            LedgerError::Parse(format!(
                "Invalid date format (use YYYY-MM or YYYY-MM-DD): {}",
                s
            ))
        })?,
        10 => NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| {
            LedgerError::Parse(format!(
                "Invalid date format (use YYYY-MM or YYYY-MM-DD): {}",
                s
            ))
        })?,
        _ => {
            return Err(LedgerError::Parse(format!(
                "Invalid date format (use YYYY-MM or YYYY-MM-DD): {}",
                s
            )));
        }
    };

    let date = if end {
        if s.len() == 7 {
            let (y, m) = (date.year(), date.month());
            let (y, m) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
            NaiveDate::from_ymd_opt(y, m, 1).ok_or_else(|| {
                LedgerError::Parse(format!(
                    "Invalid date format (use YYYY-MM or YYYY-MM-DD): {}",
                    s
                ))
            })?
        } else {
            date.succ_opt().ok_or_else(|| {
                LedgerError::Parse(format!(
                    "Invalid date format (use YYYY-MM or YYYY-MM-DD): {}",
                    s
                ))
            })?
        }
    } else {
        date
    };

    Ok(date.and_hms_opt(0, 0, 0).unwrap().and_utc())
}

pub fn parse(args: &[String]) -> Result<Action, LedgerError> {
    if args.is_empty() {
        return Err(LedgerError::Syntax("No command provided".into()));
    }

    let cmd = args[0].as_str();
    let parse_f64 = |s: &String| {
        s.parse::<f64>()
            .map_err(|_| LedgerError::Parse(format!("Invalid amount: {}", s)))
    };

    match cmd {
        "expense" if args.len() >= 4 => Ok(Action::Command(CommandType::Expense {
            amount: parse_f64(&args[1])?,
            category: args[2].clone(),
            description: args[3..].join(" "),
        })),
        "income" if args.len() >= 4 => Ok(Action::Command(CommandType::Income {
            amount: parse_f64(&args[1])?,
            source: args[2].clone(),
            description: args[3..].join(" "),
        })),
        "lend" if args.len() >= 4 => Ok(Action::Command(CommandType::Lend {
            amount: parse_f64(&args[1])?,
            person: args[2].clone(),
            description: args[3..].join(" "),
        })),
        "borrow" if args.len() >= 4 => Ok(Action::Command(CommandType::Borrow {
            amount: parse_f64(&args[1])?,
            person: args[2].clone(),
            description: args[3..].join(" "),
        })),
        "repay" if args.len() == 3 => Ok(Action::Command(CommandType::Repay {
            amount: parse_f64(&args[1])?,
            person: args[2].clone(),
        })),
        "receive" if args.len() == 3 => Ok(Action::Command(CommandType::Receive {
            amount: parse_f64(&args[1])?,
            person: args[2].clone(),
        })),
        "subscribe" if args.len() == 4 => Ok(Action::Command(CommandType::Subscribe {
            amount: parse_f64(&args[1])?,
            service: args[2].clone(),
            frequency: args[3].clone(),
        })),

        "history" => Ok(Action::Query(QueryType::History)),
        "summary" if args.len() <= 3 => {
            let start = args
                .get(1)
                .map(|s| parse_date_bound(s, false))
                .transpose()?;
            let end = args.get(2).map(|s| parse_date_bound(s, true)).transpose()?;

            if let (Some(s), Some(e)) = (&start, &end) {
                if s >= e {
                    return Err(LedgerError::Syntax(
                        "summary start must be before end".into(),
                    ));
                }
            }

            Ok(Action::Query(QueryType::Summary { start, end }))
        }
        "owed" => Ok(Action::Query(QueryType::Owed)),
        "debts" => Ok(Action::Query(QueryType::Debts)),
        "list" if args.len() == 2 => {
            let kind = match args[1].to_lowercase().as_str() {
                "expense" | "expenses" => ListKind::Expenses,
                "income" => ListKind::Income,
                "loan" | "loans" => ListKind::Loans,
                "subscription" | "subscriptions" => ListKind::Subscriptions,
                _ => {
                    return Err(LedgerError::Parse(format!(
                        "Unknown list kind: {}",
                        args[1]
                    )));
                }
            };
            Ok(Action::Query(QueryType::List(kind)))
        }
        "find" if args.len() >= 2 => Ok(Action::Query(QueryType::Find(args[1..].join(" ")))),

        _ => Err(LedgerError::Syntax(format!(
            "Unknown or incomplete command: {}",
            cmd
        ))),
    }
}
