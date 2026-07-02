use crate::money::Money;
use chrono::{DateTime, Utc};
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
        amount: Money,
        category: String,
        description: String,
    },
    Income {
        amount: Money,
        source: String,
        description: String,
    },
    LoanGiven {
        amount: Money,
        person: String,
        description: String,
    },
    LoanTaken {
        amount: Money,
        person: String,
        description: String,
    },
    RepaymentReceived {
        amount: Money,
        person: String,
    },
    RepaymentMade {
        amount: Money,
        person: String,
    },
    SubscriptionCreated {
        amount: Money,
        service: String,
        frequency: String,
    },
    /// A materialized recurring charge generated from a `SubscriptionCreated`
    /// event. `subscription_id` points at the `EventRecord::id` of the
    /// `SubscriptionCreated` event it was billed from, so charges can always
    /// be traced back to the subscription that produced them.
    SubscriptionCharged {
        subscription_id: u64,
        amount: Money,
        service: String,
    },
    /// A compensating event produced by `undo`. Rather than deleting history
    /// (which a second device that already synced could never learn about),
    /// this points at the record it reverses. Every fold over the ledger
    /// (summaries, balances, search) skips both the marker and the event it
    /// points at.
    Reversed {
        original_id: u64,
        reason: Option<String>,
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

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Expense {
                amount,
                category,
                description,
            } => write!(f, "Expense | {amount} | {category} | {description}"),
            Self::Income {
                amount,
                source,
                description,
            } => write!(f, "Income | {amount} | {source} | {description}"),
            Self::LoanGiven {
                amount,
                person,
                description,
            } => write!(f, "Loan Given | {amount} | {person} | {description}"),
            Self::LoanTaken {
                amount,
                person,
                description,
            } => write!(f, "Loan Taken | {amount} | {person} | {description}"),
            Self::RepaymentReceived { amount, person } => {
                write!(f, "Repayment Received | {amount} | {person}")
            }
            Self::RepaymentMade { amount, person } => {
                write!(f, "Repayment Made | {amount} | {person}")
            }
            Self::SubscriptionCreated {
                amount,
                service,
                frequency,
            } => write!(f, "Subscription | {amount} | {service} | {frequency}"),
            Self::SubscriptionCharged {
                amount, service, ..
            } => write!(f, "Subscription Charge | {amount} | {service}"),
            Self::Reversed {
                original_id,
                reason,
            } => match reason {
                Some(r) => write!(f, "Reversed #{original_id} | {r}"),
                None => write!(f, "Reversed #{original_id}"),
            },
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
