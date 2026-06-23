use crate::command::CommandType;
use crate::errors::LedgerError;
use crate::models::{Event, EventRecord};

pub struct Ledger {
    events: Vec<EventRecord>,
    file_path: String,
}

impl Ledger {
    pub fn new(file_path: String, events: Vec<EventRecord>) -> Self {
        Self { events, file_path }
    }

    pub fn events(&self) -> &[EventRecord] {
        &self.events
    }

    pub fn record(&mut self, cmd: CommandType) -> Result<(), LedgerError> {
        let event = match cmd {
            CommandType::Expense {
                amount,
                category,
                description,
            } => Event::Expense {
                amount,
                category,
                description,
            },
            CommandType::Income {
                amount,
                source,
                description,
            } => Event::Income {
                amount,
                source,
                description,
            },
            CommandType::Lend {
                amount,
                person,
                description,
            } => Event::LoanGiven {
                amount,
                person,
                description,
            },
            CommandType::Borrow {
                amount,
                person,
                description,
            } => Event::LoanTaken {
                amount,
                person,
                description,
            },
            CommandType::Repay { amount, person } => Event::RepaymentMade { amount, person },
            CommandType::Receive { amount, person } => Event::RepaymentReceived { amount, person },
            CommandType::Subscribe {
                amount,
                service,
                frequency,
            } => Event::SubscriptionCreated {
                amount,
                service,
                frequency,
            },
        };

        self.events.push(EventRecord::new(event));
        crate::storage::save(&self.file_path, &self.events)?;
        println!("Recorded successfully.");
        Ok(())
    }
}
