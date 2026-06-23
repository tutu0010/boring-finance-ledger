use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub event: Event,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum Event {
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
    LoanGiven {
        amount: f64,
        person: String,
        description: String,
    },
    LoanTaken {
        amount: f64,
        person: String,
        description: String,
    },
    RepaymentReceived {
        amount: f64,
        person: String,
    },
    RepaymentMade {
        amount: f64,
        person: String,
    },
    SubscriptionCreated {
        amount: f64,
        service: String,
        frequency: String,
    },
}

impl EventRecord {
    pub fn new(event: Event) -> Self {
        let now = Utc::now();
        Self {
            id: now.timestamp_nanos_opt().unwrap_or(0).to_string(),
            timestamp: now,
            event,
        }
    }
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Expense {
                amount,
                category,
                description,
            } => write!(f, "Expense | ₹{amount:.2} | {category} | {description}"),
            Self::Income {
                amount,
                source,
                description,
            } => write!(f, "Income | ₹{amount:.2} | {source} | {description}"),
            Self::LoanGiven {
                amount,
                person,
                description,
            } => write!(f, "Loan Given | ₹{amount:.2} | {person} | {description}"),
            Self::LoanTaken {
                amount,
                person,
                description,
            } => write!(f, "Loan Taken | ₹{amount:.2} | {person} | {description}"),
            Self::RepaymentReceived { amount, person } => {
                write!(f, "Repayment Received | ₹{amount:.2} | {person}")
            }
            Self::RepaymentMade { amount, person } => {
                write!(f, "Repayment Made | ₹{amount:.2} | {person}")
            }
            Self::SubscriptionCreated {
                amount,
                service,
                frequency,
            } => write!(f, "Subscription | ₹{amount:.2} | {service} | {frequency}"),
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
