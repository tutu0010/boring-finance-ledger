use crate::command::Command;
use crate::errors::LedgerError;
use crate::models::{Event, EventRecord};
use crate::store::EventStore;
use crate::subscription;
use chrono::Utc;
use std::collections::HashSet;

/// What happened as a result of `Ledger::record`. Returned as data instead
/// of printed, so this core logic is testable without capturing stdout and
/// reusable by any future frontend (TUI, mobile dashboard, whatever).
pub enum RecordOutcome {
    Recorded(EventRecord),
}

/// What happened as a result of `Ledger::undo`.
pub enum UndoOutcome {
    Reversed {
        original: EventRecord,
        marker: EventRecord,
    },
}

pub struct Ledger<S: EventStore> {
    events: Vec<EventRecord>,
    next_id: u64,
    store: S,
}

impl<S: EventStore> Ledger<S> {
    pub fn new(store: S, events: Vec<EventRecord>) -> Self {
        let next_id = events.iter().map(|r| r.id).max().unwrap_or(0) + 1;
        Self {
            events,
            next_id,
            store,
        }
    }

    pub fn events(&self) -> &[EventRecord] {
        &self.events
    }

    fn push(&mut self, event: Event) -> Result<EventRecord, LedgerError> {
        let record = EventRecord::new(self.next_id, event);
        self.store.append(&record)?;
        self.events.push(record.clone());
        self.next_id += 1;
        Ok(record)
    }

    pub fn record(&mut self, cmd: Command) -> Result<RecordOutcome, LedgerError> {
        let event = match cmd {
            Command::Expense {
                amount,
                category,
                description,
            } => Event::Expense {
                amount,
                category,
                description,
            },
            Command::Income {
                amount,
                source,
                description,
            } => Event::Income {
                amount,
                source,
                description,
            },
            Command::Lend {
                amount,
                person,
                description,
            } => Event::LoanGiven {
                amount,
                person,
                description,
            },
            Command::Borrow {
                amount,
                person,
                description,
            } => Event::LoanTaken {
                amount,
                person,
                description,
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
        };

        let record = self.push(event)?;
        Ok(RecordOutcome::Recorded(record))
    }

    /// Reverses the most recent non-reversed event by appending a
    /// compensating `Event::Reversed` marker. Nothing is ever deleted from
    /// the log: a future second device that already pulled the original
    /// event will see the reversal too, instead of the event just
    /// vanishing with no trace of why.
    pub fn undo(&mut self) -> Result<UndoOutcome, LedgerError> {
        let already_reversed: HashSet<u64> = self
            .events
            .iter()
            .filter_map(|r| match &r.event {
                Event::Reversed { original_id, .. } => Some(*original_id),
                _ => None,
            })
            .collect();

        let target = self
            .events
            .iter()
            .rev()
            .find(|r| {
                !matches!(r.event, Event::Reversed { .. }) && !already_reversed.contains(&r.id)
            })
            .cloned()
            .ok_or(LedgerError::EmptyLedger)?;

        let marker = self.push(Event::Reversed {
            original_id: target.id,
            reason: None,
        })?;

        Ok(UndoOutcome::Reversed {
            original: target,
            marker,
        })
    }

    /// Walks every non-reversed `SubscriptionCreated` event, figures out how
    /// many billing periods have elapsed since its last charge, and appends
    /// one `SubscriptionCharged` event per elapsed period. Without this, a
    /// `Subscribe` event just sits there being decorative — no money ever
    /// actually leaves.
    pub fn materialize_due_subscriptions(&mut self) -> Result<Vec<EventRecord>, LedgerError> {
        let now = Utc::now();

        let reversed: HashSet<u64> = self
            .events
            .iter()
            .filter_map(|r| match &r.event {
                Event::Reversed { original_id, .. } => Some(*original_id),
                _ => None,
            })
            .collect();

        struct Due {
            subscription_id: u64,
            service: String,
            amount: crate::money::Money,
            frequency: String,
            last_charge: chrono::DateTime<Utc>,
        }

        let mut due_list = Vec::new();
        for record in &self.events {
            if reversed.contains(&record.id) {
                continue;
            }
            if let Event::SubscriptionCreated {
                amount,
                service,
                frequency,
            } = &record.event
            {
                let last_charge = self
                    .events
                    .iter()
                    .filter_map(|r| match &r.event {
                        Event::SubscriptionCharged {
                            subscription_id, ..
                        } if *subscription_id == record.id => Some(r.timestamp),
                        _ => None,
                    })
                    .max()
                    .unwrap_or(record.timestamp);

                due_list.push(Due {
                    subscription_id: record.id,
                    service: service.clone(),
                    amount: *amount,
                    frequency: frequency.clone(),
                    last_charge,
                });
            }
        }

        let mut new_records = Vec::new();
        for due in due_list {
            let mut cursor = due.last_charge;
            // Hard ceiling so a bad frequency or clock skew can never loop
            // forever or flood the ledger with thousands of charges.
            for _ in 0..500 {
                let next = subscription::advance(cursor, &due.frequency)?;
                if next > now {
                    break;
                }
                let record = self.push(Event::SubscriptionCharged {
                    subscription_id: due.subscription_id,
                    amount: due.amount,
                    service: due.service.clone(),
                })?;
                new_records.push(record);
                cursor = next;
            }
        }

        Ok(new_records)
    }
}
