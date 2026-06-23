use crate::command::Command;
use crate::errors::LedgerError;
use crate::models::{Event, EventRecord};

pub struct Ledger {
    events: Vec<EventRecord>,
    next_id: u64,
    file_path: String,
}

impl Ledger {
    pub fn new(file_path: String, events: Vec<EventRecord>) -> Self {
        let next_id = events.iter().map(|r| r.id).max().unwrap_or(0) + 1;
        Self {
            events,
            next_id,
            file_path,
        }
    }

    pub fn events(&self) -> &[EventRecord] {
        &self.events
    }

    pub fn record(&mut self, cmd: Command) -> Result<(), LedgerError> {
        let event = match cmd {
            Command::Expense {
                amount,
                category,
                description,
            } => Event::Expense {
                amount,
                category,
                description: crate::command::Command::description(&description),
            },
            Command::Income {
                amount,
                source,
                description,
            } => Event::Income {
                amount,
                source,
                description: crate::command::Command::description(&description),
            },
            Command::Lend {
                amount,
                person,
                description,
            } => Event::LoanGiven {
                amount,
                person,
                description: crate::command::Command::description(&description),
            },
            Command::Borrow {
                amount,
                person,
                description,
            } => Event::LoanTaken {
                amount,
                person,
                description: crate::command::Command::description(&description),
            },
            Command::Repay { amount, person } => Event::RepaymentMade { amount, person },
            Command::Receive { amount, person } => Event::RepaymentReceived { amount, person },
            Command::Subscribe {
                amount,
                service,
                frequency,
            } => Event::SubscriptionCreated {
                amount,
                service,
                frequency,
            },
            _ => return Err(LedgerError::Syntax("query passed to record".into())),
        };

        self.events.push(EventRecord::new(self.next_id, event));
        self.next_id += 1;
        crate::storage::save(&self.file_path, &self.events)?;
        println!("Recorded successfully.");
        Ok(())
    }

    pub fn undo(&mut self) -> Result<(), LedgerError> {
        self.events.pop().ok_or(LedgerError::EmptyLedger)?;
        self.next_id = self.events.iter().map(|r| r.id).max().unwrap_or(0) + 1;
        crate::storage::save(&self.file_path, &self.events)?;
        println!("Last event removed.");
        Ok(())
    }
}
