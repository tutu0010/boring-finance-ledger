use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    pub id: u64,
    pub timestamp: DateTime<Utc>,
    pub event: Event,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum Event {
    Expense {
        amount: Decimal,
        category: String,
        description: String,
    },
    Income {
        amount: Decimal,
        source: String,
        description: String,
    },
    LoanGiven {
        amount: Decimal,
        person: String,
        description: String,
    },
    LoanTaken {
        amount: Decimal,
        person: String,
        description: String,
    },
    RepaymentReceived {
        amount: Decimal,
        person: String,
    },
    RepaymentMade {
        amount: Decimal,
        person: String,
    },
    SubscriptionCreated {
        amount: Decimal,
        service: String,
        frequency: String,
    },
}

impl EventRecord {
    pub fn new(id: u64, event: Event) -> Self {
        Self {
            id,
            timestamp: Utc::now(),
            event,
        }
    }
}

fn fmt_amount(amount: Decimal) -> String {
    format!("₹{amount:.2}")
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Expense {
                amount,
                category,
                description,
            } => {
                write!(
                    f,
                    "Expense | {} | {} | {}",
                    fmt_amount(*amount),
                    category,
                    description
                )
            }
            Self::Income {
                amount,
                source,
                description,
            } => {
                write!(
                    f,
                    "Income | {} | {} | {}",
                    fmt_amount(*amount),
                    source,
                    description
                )
            }
            Self::LoanGiven {
                amount,
                person,
                description,
            } => {
                write!(
                    f,
                    "Loan Given | {} | {} | {}",
                    fmt_amount(*amount),
                    person,
                    description
                )
            }
            Self::LoanTaken {
                amount,
                person,
                description,
            } => {
                write!(
                    f,
                    "Loan Taken | {} | {} | {}",
                    fmt_amount(*amount),
                    person,
                    description
                )
            }
            Self::RepaymentReceived { amount, person } => {
                write!(
                    f,
                    "Repayment Received | {} | {}",
                    fmt_amount(*amount),
                    person
                )
            }
            Self::RepaymentMade { amount, person } => {
                write!(f, "Repayment Made | {} | {}", fmt_amount(*amount), person)
            }
            Self::SubscriptionCreated {
                amount,
                service,
                frequency,
            } => {
                write!(
                    f,
                    "Subscription | {} | {} | {}",
                    fmt_amount(*amount),
                    service,
                    frequency
                )
            }
        }
    }
}

impl fmt::Display for EventRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} | #{} | {}",
            self.timestamp
                .with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M"),
            self.id,
            self.event
        )
    }
}
